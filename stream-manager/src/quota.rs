//! This program's share of YouTube's daily allowance ("quota", ADR 026 §7).
//!
//! The allowance belongs to the Google Cloud project, so every court's Stream Manager shares it.
//! Each program only counts what it used itself, keeps that count in a small file so a restart
//! doesn't lose it, and starts again from zero when Google's day resets (midnight US Pacific
//! time). When what's left of its share would no longer cover the rest of the day's switches,
//! the extras (chat message, "Next game" link) stop. The switches themselves never do.

use crate::{BoxError, portal::EventPlan};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, sync::Mutex};
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, UtcOffset, macros::time};

/// Units each switch can cost: go live and end (50 each) plus status checks while waiting.
pub const SWITCH_COST: u32 = 120;
/// Kept spare on top of the switches still to come.
pub const MARGIN: u32 = 200;
pub const LEDGER_FILE: &str = "youtube-allowance.json";

/// Only one load-add-save of the ledger file runs at a time, so two YouTube calls finishing
/// together can't lose each other's units.
static LEDGER_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    pub day: Option<Date>,
    pub used: u32,
}

/// The `n`th Sunday (1-based) of a month.
fn nth_sunday(year: i32, month: Month, n: u8) -> Option<Date> {
    let first = Date::from_calendar_date(year, month, 1).ok()?;
    let to_sunday = (7 - first.weekday().number_days_from_sunday()) % 7;
    let day = 1 + to_sunday + 7 * (n - 1);
    Date::from_calendar_date(year, month, day).ok()
}

/// The date in US Pacific time (Google's allowance day), with US daylight-saving rules:
/// UTC−7 from the second Sunday in March 02:00 to the first Sunday in November 02:00, else UTC−8.
pub fn pacific_date(now: OffsetDateTime) -> Date {
    let utc = now.to_offset(UtcOffset::UTC);
    let year = utc.year();
    // 02:00 local time on the change days: 10:00 UTC in March (still PST, UTC−8) and 09:00 UTC
    // in November (still PDT, UTC−7).
    let summer_start = nth_sunday(year, Month::March, 2)
        .map(|d| PrimitiveDateTime::new(d, time!(10:00)).assume_utc());
    let summer_end = nth_sunday(year, Month::November, 1)
        .map(|d| PrimitiveDateTime::new(d, time!(09:00)).assume_utc());
    let summer =
        matches!((summer_start, summer_end), (Some(start), Some(end)) if start <= utc && utc < end);
    let hours = if summer { 7 } else { 8 };
    (utc - time::Duration::hours(hours)).date()
}

impl Ledger {
    /// Missing or unreadable file → an empty ledger.
    pub fn load(path: &Path) -> Ledger {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), BoxError> {
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Adds units used at `now`, starting from zero when the Pacific day has changed.
    pub fn record(&mut self, units: u32, now: OffsetDateTime) {
        let today = pacific_date(now);
        if self.day != Some(today) {
            self.day = Some(today);
            self.used = 0;
        }
        self.used = self.used.saturating_add(units);
    }

    pub fn used_today(&self, now: OffsetDateTime) -> u32 {
        if self.day == Some(pacific_date(now)) {
            self.used
        } else {
            0
        }
    }
}

/// Adds units to the ledger file (load, add, save, one at a time across the program).
pub fn record_to_file(path: &Path, units: u32, now: OffsetDateTime) -> Result<(), BoxError> {
    // A panic while holding the lock leaves nothing half-done in memory; keep going.
    let _guard = LEDGER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut ledger = Ledger::load(path);
    ledger.record(units, now);
    ledger.save(path)
}

pub fn share(limit: u32, percent: u8) -> u32 {
    let units = u64::from(limit) * u64::from(percent) / 100;
    u32::try_from(units).unwrap_or(u32::MAX)
}

/// Extras are allowed while what's left still covers the remaining switches plus MARGIN.
pub fn extras_allowed(remaining: u32, switches_left: u32) -> bool {
    let needed = switches_left
        .saturating_mul(SWITCH_COST)
        .saturating_add(MARGIN);
    remaining >= needed
}

/// The schedule day whose games on `court` start on today's date (in the games' own time zone).
pub fn todays_day(plan: &EventPlan, court: &str, now: OffsetDateTime) -> Option<usize> {
    plan.games
        .iter()
        .find(|g| g.court == court && g.start.date() == now.to_offset(g.start.offset()).date())
        .map(|g| g.day)
}

/// Switches still to come today on one court: its games after the live one, on the schedule
/// day of the live (or else the next) game. Without a live game, all of the court's games that
/// day. Without either game, the day is the one whose games start on today's date.
pub fn court_switches_left(
    plan: &EventPlan,
    court: &str,
    live: Option<&str>,
    next: Option<&str>,
    now: OffsetDateTime,
) -> u32 {
    let day = live
        .or(next)
        .and_then(|game| plan.game(game))
        .map(|game| game.day)
        .or_else(|| todays_day(plan, court, now));
    let Some(day) = day else {
        return 0;
    };
    let todays: Vec<&str> = plan
        .court_games(court, day)
        .map(|g| g.number.as_str())
        .collect();
    let left = match live.and_then(|l| todays.iter().position(|g| *g == l)) {
        Some(position) => todays.len() - position - 1,
        None => todays.len(),
    };
    u32::try_from(left).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portal::parse_event_plan;
    use time::macros::{date, datetime};

    #[test]
    fn pacific_date_in_winter_is_utc_minus_8() {
        assert_eq!(
            pacific_date(datetime!(2026-01-15 07:59 UTC)),
            date!(2026 - 01 - 14)
        );
        assert_eq!(
            pacific_date(datetime!(2026-01-15 08:00 UTC)),
            date!(2026 - 01 - 15)
        );
    }

    #[test]
    fn pacific_date_in_summer_is_utc_minus_7() {
        assert_eq!(
            pacific_date(datetime!(2026-07-15 06:59 UTC)),
            date!(2026 - 07 - 14)
        );
        assert_eq!(
            pacific_date(datetime!(2026-07-15 07:00 UTC)),
            date!(2026 - 07 - 15)
        );
    }

    #[test]
    fn pacific_date_follows_the_2026_change_days() {
        // 8 March 2026: the day starts on PST and ends on PDT.
        assert_eq!(
            pacific_date(datetime!(2026-03-08 07:59 UTC)),
            date!(2026 - 03 - 07)
        );
        assert_eq!(
            pacific_date(datetime!(2026-03-08 08:00 UTC)),
            date!(2026 - 03 - 08)
        );
        assert_eq!(
            pacific_date(datetime!(2026-03-09 06:59 UTC)),
            date!(2026 - 03 - 08)
        );
        assert_eq!(
            pacific_date(datetime!(2026-03-09 07:00 UTC)),
            date!(2026 - 03 - 09)
        );
        // 1 November 2026: the day starts on PDT and ends on PST.
        assert_eq!(
            pacific_date(datetime!(2026-11-01 06:59 UTC)),
            date!(2026 - 10 - 31)
        );
        assert_eq!(
            pacific_date(datetime!(2026-11-01 07:00 UTC)),
            date!(2026 - 11 - 01)
        );
        assert_eq!(
            pacific_date(datetime!(2026-11-02 07:59 UTC)),
            date!(2026 - 11 - 01)
        );
        assert_eq!(
            pacific_date(datetime!(2026-11-02 08:00 UTC)),
            date!(2026 - 11 - 02)
        );
    }

    #[test]
    fn pacific_date_accepts_any_offset() {
        // 09:00 in Sydney on 15 January is 22:00 UTC on the 14th: 14:00 PST on the 14th.
        assert_eq!(
            pacific_date(datetime!(2026-01-15 09:00 +11)),
            date!(2026 - 01 - 14)
        );
    }

    #[test]
    fn record_adds_within_a_day_and_resets_on_a_new_pacific_day() {
        let mut ledger = Ledger::default();
        ledger.record(50, datetime!(2026-01-15 09:00 UTC));
        ledger.record(1, datetime!(2026-01-16 07:59 UTC)); // still the 15th in Pacific time
        assert_eq!(ledger.used, 51);
        assert_eq!(ledger.day, Some(date!(2026 - 01 - 15)));
        assert_eq!(ledger.used_today(datetime!(2026-01-16 07:59 UTC)), 51);
        assert_eq!(ledger.used_today(datetime!(2026-01-16 08:00 UTC)), 0);

        ledger.record(50, datetime!(2026-01-16 08:00 UTC));
        assert_eq!(ledger.used, 50);
        assert_eq!(ledger.day, Some(date!(2026 - 01 - 16)));
    }

    #[test]
    fn missing_file_loads_as_empty_and_a_saved_ledger_loads_back() {
        let dir = std::env::temp_dir().join(format!("stream-manager-quota-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(LEDGER_FILE);
        let _ = fs::remove_file(&path);
        assert_eq!(Ledger::load(&path), Ledger::default());

        record_to_file(&path, 50, datetime!(2026-01-15 09:00 UTC)).unwrap();
        record_to_file(&path, 1, datetime!(2026-01-15 10:00 UTC)).unwrap();
        assert_eq!(
            Ledger::load(&path),
            Ledger {
                day: Some(date!(2026 - 01 - 15)),
                used: 51
            }
        );

        fs::write(&path, "not json").unwrap();
        assert_eq!(Ledger::load(&path), Ledger::default());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn extras_stop_when_the_rest_no_longer_covers_the_switches_plus_margin() {
        // 3 switches: 3 × 120 + 200 = 560.
        assert!(extras_allowed(561, 3));
        assert!(extras_allowed(560, 3));
        assert!(!extras_allowed(559, 3));
        assert!(extras_allowed(MARGIN, 0));
        assert!(!extras_allowed(0, 0));
        assert!(!extras_allowed(u32::MAX - 1, u32::MAX));
    }

    #[test]
    fn share_rounds_down() {
        assert_eq!(share(10_000, 50), 5_000);
        assert_eq!(share(10_000, 100), 10_000);
        assert_eq!(share(10_001, 50), 5_000);
        assert_eq!(share(999, 33), 329);
        assert_eq!(share(u32::MAX, 100), u32::MAX);
    }

    #[test]
    fn switches_left_counts_this_courts_games_after_the_live_one_that_day() {
        let game = |number: &str, start: &str, court: &str| {
            format!(
                r#"{{ "number": "{number}", "startsOn": "{start}", "court": "{court}",
                    "dark": {{ "assignment": null }}, "light": {{ "assignment": null }} }}"#
            )
        };
        let plan = parse_event_plan(&format!(
            r#"{{ "event": {{ "name": "Test Cup" }}, "games": [ {}, {}, {}, {}, {} ] }}"#,
            game("1", "2026-08-01T09:00:00+10:00", "1"),
            game("2", "2026-08-01T09:00:00+10:00", "2"),
            game("3", "2026-08-01T10:00:00+10:00", "1"),
            game("5", "2026-08-01T11:00:00+10:00", "1"),
            game("7", "2026-08-02T09:00:00+10:00", "1"),
        ))
        .unwrap();
        let day_one = datetime!(2026-08-01 08:00 +10);
        let day_two = datetime!(2026-08-02 08:00 +10);
        assert_eq!(
            court_switches_left(&plan, "1", Some("3"), Some("5"), day_one),
            1
        );
        assert_eq!(court_switches_left(&plan, "1", Some("5"), None, day_one), 0);
        // Day not started yet: every game that day.
        assert_eq!(court_switches_left(&plan, "1", None, Some("1"), day_one), 3);
        // Nothing known from the refbox: the games starting on today's date.
        assert_eq!(court_switches_left(&plan, "1", None, None, day_one), 3);
        assert_eq!(court_switches_left(&plan, "1", None, None, day_two), 1);
        assert_eq!(court_switches_left(&plan, "2", None, None, day_one), 1);
        assert_eq!(
            court_switches_left(&plan, "1", None, None, datetime!(2026-08-05 08:00 +10)),
            0
        );
    }
}
