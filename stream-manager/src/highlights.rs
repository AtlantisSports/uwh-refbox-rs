//! A game's goals, penalties and fouls, worked out from the refbox feed while the game is played,
//! and the "Game highlights" list of clickable times written into its video's description once
//! the game is over.
//!
//! The feed says what the scorekeeper has entered, not when it happened on the pool floor, so
//! each item points a little before the moment it was entered ([`LEAD`]). An item entered during a
//! referee timeout, or soon after one, points before the start of that timeout instead: the refs
//! stopped play to discuss it, and what they discussed happened before.

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use time::{Duration, OffsetDateTime};
use uwh_common::{
    color::Color,
    game_snapshot::{GameSnapshot, Infraction, PenaltyTime, TimeoutSnapshot},
};

/// How far before its anchor (when it was entered, or when the referee timeout started) each item
/// points, so viewers see the play that led to it.
pub const LEAD: Duration = Duration::seconds(15);
/// An item entered this long after a referee timeout ended still counts as decided in it.
pub const REF_TIMEOUT_WINDOW: Duration = Duration::seconds(10);
/// An item entered this long before its video's 0:00 belongs to an earlier video of the game
/// (one deleted and made again), so it isn't listed.
pub const STALE: Duration = Duration::minutes(30);
/// A penalty gone from the refbox's list this soon after it appeared was entered by mistake. Served
/// penalties also leave the list, but never this soon: none is shorter than a minute.
pub const PENALTY_MISTAKE: Duration = Duration::seconds(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Goal,
    Penalty,
    Foul,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Black,
    White,
}

impl Side {
    fn of(color: Color) -> Self {
        match color {
            Color::Black => Side::Black,
            Color::White => Side::White,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Side::Black => "Black",
            Side::White => "White",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PenaltyLength {
    Minutes(u16),
    Dismissal,
}

impl PenaltyLength {
    /// The feed gives the time a penalty has left, which is a second or so short of its length
    /// when first seen: rounding up to whole minutes gives the length the referee gave.
    fn from_time_left(time: PenaltyTime) -> Self {
        match time {
            PenaltyTime::Seconds(secs) => PenaltyLength::Minutes(secs.div_ceil(60).max(1)),
            PenaltyTime::TotalDismissal => PenaltyLength::Dismissal,
        }
    }
}

/// One goal, penalty or foul.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Highlight {
    pub kind: Kind,
    /// The team; a foul may have none.
    pub side: Option<Side>,
    /// The player's cap number, when the scorekeeper entered one.
    pub cap: Option<u8>,
    /// The foul's name (for a penalty: the foul it was given for).
    #[serde(default)]
    pub infraction: Option<String>,
    #[serde(default)]
    pub length: Option<PenaltyLength>,
    /// When the scorekeeper entered it (this program's clock).
    #[serde(with = "time::serde::rfc3339")]
    pub entered: OffsetDateTime,
    /// The moment it points to before [`LEAD`]: when it was entered, or when the referee timeout
    /// it was decided in started.
    #[serde(with = "time::serde::rfc3339")]
    pub anchor: OffsetDateTime,
}

/// What one game's video description lists, kept in the event's record as the game goes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameHighlights {
    pub items: Vec<Highlight>,
    /// The score the refbox last showed for this game: black, white.
    #[serde(default)]
    pub score: Option<[u8; 2]>,
}

/// A penalty or foul as the feed lists it: enough to tell entries apart and to notice one added
/// or removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    side: Option<Side>,
    cap: Option<u8>,
    infraction: Infraction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Seen {
    score: [u8; 2],
    penalties: Vec<Entry>,
    fouls: Vec<Entry>,
}

impl Seen {
    /// How many goals, penalties and fouls the feed shows.
    fn size(&self) -> usize {
        usize::from(self.score[0])
            + usize::from(self.score[1])
            + self.penalties.len()
            + self.fouls.len()
    }

    fn of(snapshot: &GameSnapshot) -> Self {
        let mut penalties = Vec::new();
        for (color, list) in [
            (Color::Black, &snapshot.penalties.black),
            (Color::White, &snapshot.penalties.white),
        ] {
            penalties.extend(list.iter().map(|p| Entry {
                side: Some(Side::of(color)),
                cap: Some(p.player_number),
                infraction: p.infraction,
            }));
        }
        let mut fouls = Vec::new();
        for (side, list) in [
            (Some(Side::Black), &snapshot.fouls.black),
            (None, &snapshot.fouls.equal),
            (Some(Side::White), &snapshot.fouls.white),
        ] {
            fouls.extend(list.iter().map(|f| Entry {
                side,
                cap: f.player_number,
                infraction: f.infraction,
            }));
        }
        Seen {
            score: [snapshot.scores.black, snapshot.scores.white],
            penalties,
            fouls,
        }
    }
}

/// The entries of `new` that `old` doesn't account for, and those of `old` that `new` no longer
/// has: each entry counted as many times as it appears.
fn added_and_removed(old: &[Entry], new: &[Entry]) -> (Vec<Entry>, Vec<Entry>) {
    let mut unmatched_old: Vec<Option<&Entry>> = old.iter().map(Some).collect();
    let mut added = Vec::new();
    for entry in new {
        match unmatched_old
            .iter_mut()
            .find(|o| o.is_some_and(|o| o == entry))
        {
            Some(slot) => *slot = None,
            None => added.push(*entry),
        }
    }
    let removed = unmatched_old.into_iter().flatten().copied().collect();
    (added, removed)
}

/// Follows one court's refbox feed and keeps the current game's list up to date.
#[derive(Debug, Default)]
pub struct Tracker {
    /// The game the list belongs to, and what the feed last showed for it.
    game: Option<(String, Seen)>,
    list: GameHighlights,
    /// When the current referee timeout started, while one is running.
    ref_timeout: Option<OffsetDateTime>,
    /// The last referee timeout that ended: when it started and when it ended.
    last_ref_timeout: Option<(OffsetDateTime, OffsetDateTime)>,
}

impl Tracker {
    /// Takes in one snapshot received at `now`. Returns the game and its list when the list
    /// changed, to be saved. `saved` gives a game's list as recorded, so a list kept before this
    /// program restarted carries on.
    ///
    /// The first snapshot of a game (and the first after a restart or a new connection, see
    /// [`Tracker::reset`]) only sets the starting point: what it already shows was entered at an
    /// unknown time, so it isn't listed.
    pub fn on_snapshot(
        &mut self,
        snapshot: &GameSnapshot,
        now: OffsetDateTime,
        saved: impl FnOnce(&str) -> GameHighlights,
    ) -> Option<(String, GameHighlights)> {
        self.follow_ref_timeouts(snapshot, now);
        // Between games the refbox shows the finished game until it is reset for the next one;
        // a new game number means a new list.
        let game = snapshot.game_number.to_string();
        let seen = Seen::of(snapshot);
        let previous = match self.game.take() {
            Some((g, previous)) if g == game => previous,
            _ => {
                self.list = saved(&game);
                let changed = self.list.score != Some(seen.score);
                self.list.score = Some(seen.score);
                self.game = Some((game.clone(), seen));
                return changed.then(|| (game, self.list.clone()));
            }
        };
        if previous.size() >= 2 && seen.size() == 0 {
            // Everything gone at once: the refbox was reset (or restarted), not corrected. What
            // was listed stays, and the cleared state is the new starting point.
            self.game = Some((game, seen));
            return None;
        }
        let before = self.list.clone();
        self.goals(&previous, &seen, snapshot, now);
        self.penalties(&previous, &seen, snapshot, now);
        self.fouls(&previous, &seen, now);
        self.list.score = Some(seen.score);
        self.game = Some((game.clone(), seen));
        (self.list != before).then(|| (game, self.list.clone()))
    }

    /// The game being followed and its list so far.
    pub fn current(&self) -> Option<(&str, &GameHighlights)> {
        self.game
            .as_ref()
            .map(|(game, _)| (game.as_str(), &self.list))
    }

    /// The connection to the refbox was lost or reconnected: the next snapshot is a new starting
    /// point, since anything entered meanwhile was entered at an unknown time.
    pub fn reset(&mut self) {
        self.game = None;
        self.ref_timeout = None;
    }

    fn follow_ref_timeouts(&mut self, snapshot: &GameSnapshot, now: OffsetDateTime) {
        let in_ref_timeout = matches!(snapshot.timeout, Some(TimeoutSnapshot::Ref(_)));
        match (in_ref_timeout, self.ref_timeout) {
            (true, None) => self.ref_timeout = Some(now),
            (false, Some(start)) => {
                self.ref_timeout = None;
                self.last_ref_timeout = Some((start, now));
            }
            _ => {}
        }
    }

    /// Where an item entered at `now` points: the start of the referee timeout it was decided in,
    /// if any, otherwise `now`.
    fn anchor(&self, now: OffsetDateTime) -> OffsetDateTime {
        if let Some(start) = self.ref_timeout {
            return start;
        }
        match self.last_ref_timeout {
            Some((start, end)) if now - end <= REF_TIMEOUT_WINDOW => start,
            _ => now,
        }
    }

    fn push(
        &mut self,
        kind: Kind,
        entry: Entry,
        length: Option<PenaltyLength>,
        now: OffsetDateTime,
    ) {
        self.list.items.push(Highlight {
            kind,
            side: entry.side,
            cap: entry.cap,
            infraction: (kind != Kind::Goal).then(|| entry.infraction.short_name().to_string()),
            length,
            entered: now,
            anchor: self.anchor(now),
        });
    }

    /// Removes the latest item of `kind` matching `matches`, if `allowed` says it may go.
    fn remove_latest(
        &mut self,
        kind: Kind,
        matches: impl Fn(&Highlight) -> bool,
        allowed: impl Fn(&Highlight) -> bool,
    ) {
        let latest = self
            .list
            .items
            .iter()
            .rposition(|h| h.kind == kind && matches(h));
        if let Some(i) = latest.filter(|&i| allowed(&self.list.items[i])) {
            self.list.items.remove(i);
        }
    }

    fn goals(&mut self, old: &Seen, new: &Seen, snapshot: &GameSnapshot, now: OffsetDateTime) {
        for (i, side) in [(0, Side::Black), (1, Side::White)] {
            let (was, is) = (old.score[i], new.score[i]);
            if is > was {
                // The feed names the scorer of the latest goal only.
                let scorer = snapshot
                    .recent_goal
                    .filter(|(color, _)| Side::of(*color) == side && is - was == 1)
                    .map(|(_, cap)| cap);
                for _ in was..is {
                    let entry = Entry {
                        side: Some(side),
                        cap: scorer,
                        infraction: Infraction::Unknown,
                    };
                    self.push(Kind::Goal, entry, None, now);
                }
            } else {
                for _ in is..was {
                    self.remove_latest(Kind::Goal, |h| h.side == Some(side), |_| true);
                }
            }
        }
    }

    fn penalties(&mut self, old: &Seen, new: &Seen, snapshot: &GameSnapshot, now: OffsetDateTime) {
        let (added, removed) = added_and_removed(&old.penalties, &new.penalties);
        for entry in removed {
            // Gone this soon: deleted. Later: served (the refbox clears served penalties).
            self.remove_latest(
                Kind::Penalty,
                |h| same_entry(h, &entry),
                |h| now - h.entered < PENALTY_MISTAKE,
            );
        }
        for entry in added {
            let time_left = [&snapshot.penalties.black, &snapshot.penalties.white]
                .into_iter()
                .flatten()
                .find(|p| Some(p.player_number) == entry.cap && p.infraction == entry.infraction)
                .map(|p| p.time);
            let length = time_left.map(PenaltyLength::from_time_left);
            self.push(Kind::Penalty, entry, length, now);
        }
    }

    fn fouls(&mut self, old: &Seen, new: &Seen, now: OffsetDateTime) {
        let (added, removed) = added_and_removed(&old.fouls, &new.fouls);
        for entry in removed {
            self.remove_latest(Kind::Foul, |h| same_entry(h, &entry), |_| true);
        }
        for entry in added {
            self.push(Kind::Foul, entry, None, now);
        }
    }
}

fn same_entry(h: &Highlight, entry: &Entry) -> bool {
    h.side == entry.side
        && h.cap == entry.cap
        && h.infraction.as_deref() == Some(entry.infraction.short_name())
}

/// One item as listed, without its time: "Goal — White #7", "Penalty — Black #3, 1 min
/// (Obstruction)", "Foul — White #4 (Stick Foul)".
pub fn describe(h: &Highlight) -> String {
    let who = match (h.side, h.cap) {
        (Some(side), Some(cap)) => format!("{} #{cap}", side.name()),
        (Some(side), None) => side.name().to_string(),
        (None, Some(cap)) => format!("#{cap}"),
        (None, None) => "both teams".to_string(),
    };
    let foul = h
        .infraction
        .as_deref()
        .map(|f| format!(" ({f})"))
        .unwrap_or_default();
    match h.kind {
        Kind::Goal => format!("Goal — {who}"),
        Kind::Penalty => {
            let length = match h.length {
                Some(PenaltyLength::Minutes(m)) => format!(", {m} min"),
                Some(PenaltyLength::Dismissal) => ", dismissal".to_string(),
                None => String::new(),
            };
            format!("Penalty — {who}{length}{foul}")
        }
        Kind::Foul => format!("Foul — {who}{foul}"),
    }
}

/// A line for the court's log when the list of a game changed from `before` to `after`: the item
/// added, or that one was taken off.
pub fn log_line(game: &str, before: &GameHighlights, after: &GameHighlights) -> Option<String> {
    if after.items.len() > before.items.len() {
        let added: Vec<String> = after.items[before.items.len()..]
            .iter()
            .map(describe)
            .collect();
        Some(format!("Game {game}: noted {}", added.join("; ")))
    } else if after.items.len() < before.items.len() {
        let gone = before
            .items
            .iter()
            .find(|h| !after.items.contains(h))
            .map(describe)
            .unwrap_or_default();
        Some(format!("Game {game}: taken off the list: {gone}"))
    } else {
        None
    }
}

/// `M:SS`, or `H:MM:SS` from an hour, as YouTube turns into a link.
fn video_time(offset: Duration) -> String {
    let secs = offset.whole_seconds().max(0);
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// YouTube refuses descriptions longer than this (in characters). The list leaves room for the
/// rest of the description.
const MAX_BLOCK_CHARS: usize = 3500;

/// The "Game highlights" block for a video that went live at `video_start`, or `None` when there
/// is nothing to list. Each line points [`LEAD`] before its anchor, never before 0:00. If the
/// list would be too long for YouTube, fouls are left out first.
pub fn description_block(list: &GameHighlights, video_start: OffsetDateTime) -> Option<String> {
    let mut items: Vec<&Highlight> = list
        .items
        .iter()
        .filter(|h| h.entered >= video_start - STALE)
        .collect();
    if items.is_empty() {
        return None;
    }
    items.sort_by_key(|h| (h.anchor, h.entered));
    // The running score is shown only when every goal was seen, so it can't be wrong.
    let goals = |side| {
        items
            .iter()
            .filter(|h| h.kind == Kind::Goal && h.side == Some(side))
            .count()
    };
    let complete = list.score == Some([goals(Side::Black) as u8, goals(Side::White) as u8]);
    let mut score = [0u8; 2];
    let mut lines = Vec::new();
    for h in &items {
        let at = video_time(h.anchor - LEAD - video_start);
        let mut line = format!("{at} {}", describe(h));
        if h.kind == Kind::Goal {
            let i = usize::from(h.side == Some(Side::White));
            score[i] = score[i].saturating_add(1);
            if complete {
                line.push_str(&format!(" (White {}–{} Black)", score[1], score[0]));
            }
        }
        lines.push((h.kind, line.replace(['<', '>'], "")));
    }
    let final_line = list
        .score
        .map(|[black, white]| format!("Final score: White {white}–{black} Black"));
    let build = |with_fouls: bool| {
        let mut block = String::from("Game highlights\n0:00 Start\n");
        let mut left_out = false;
        for (kind, line) in &lines {
            if *kind == Kind::Foul && !with_fouls {
                left_out = true;
                continue;
            }
            let _ = writeln!(block, "{line}");
        }
        if left_out {
            block.push_str("(more fouls not shown)\n");
        }
        if let Some(line) = &final_line {
            let _ = writeln!(block, "{line}");
        }
        block
    };
    let block = build(true);
    if block.chars().count() <= MAX_BLOCK_CHARS {
        Some(block)
    } else {
        Some(build(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uwh_common::{
        bundles::{BlackWhiteBundle, OptColorBundle},
        game_snapshot::{GamePeriod, InfractionSnapshot, PenaltySnapshot},
    };

    fn at(secs: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_800_000_000 + secs).unwrap()
    }

    fn snap(game: &str, black: u8, white: u8) -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::FirstHalf,
            game_number: game.into(),
            scores: BlackWhiteBundle { black, white },
            ..Default::default()
        }
    }

    fn penalty(cap: u8, secs: u16, infraction: Infraction) -> PenaltySnapshot {
        PenaltySnapshot {
            player_number: cap,
            time: PenaltyTime::Seconds(secs),
            infraction,
        }
    }

    fn foul(cap: u8, infraction: Infraction) -> InfractionSnapshot {
        InfractionSnapshot {
            player_number: Some(cap),
            infraction,
        }
    }

    /// A tracker that has already seen game "5" at 0–0, at time 0.
    fn started() -> Tracker {
        let mut t = Tracker::default();
        t.on_snapshot(&snap("5", 0, 0), at(0), |_| GameHighlights::default());
        t
    }

    fn feed(t: &mut Tracker, s: GameSnapshot, secs: i64) -> GameHighlights {
        t.on_snapshot(&s, at(secs), |_| GameHighlights::default());
        t.list.clone()
    }

    #[test]
    fn a_goal_points_15_seconds_before_it_was_entered() {
        let mut t = started();
        let mut s = snap("5", 0, 1);
        s.recent_goal = Some((Color::White, 7));
        let list = feed(&mut t, s, 300);
        assert_eq!(list.items.len(), 1);
        let goal = &list.items[0];
        assert_eq!(
            (goal.kind, goal.side, goal.cap),
            (Kind::Goal, Some(Side::White), Some(7))
        );
        assert_eq!(goal.anchor, at(300));
        let block = description_block(&list, at(0)).unwrap();
        assert!(
            block.contains("4:45 Goal — White #7 (White 1–0 Black)"),
            "{block}"
        );
    }

    #[test]
    fn what_the_first_snapshot_already_shows_is_not_listed() {
        let mut t = Tracker::default();
        let changed = t.on_snapshot(&snap("5", 2, 1), at(0), |_| GameHighlights::default());
        assert_eq!(
            changed.map(|(g, l)| (g, l.items.len(), l.score)),
            Some(("5".into(), 0, Some([2, 1])))
        );
    }

    #[test]
    fn an_item_entered_during_or_soon_after_a_referee_timeout_points_before_its_start() {
        let mut t = started();
        let mut s = snap("5", 0, 0);
        s.timeout = Some(TimeoutSnapshot::Ref(0));
        feed(&mut t, s.clone(), 100);
        // Entered during the referee timeout.
        s.scores.black = 1;
        let list = feed(&mut t, s, 220);
        assert_eq!(list.items[0].anchor, at(100));
        // The timeout ends at 300; a penalty 8 s later still counts, a foul 20 s later doesn't.
        feed(&mut t, snap("5", 1, 0), 300);
        let mut s = snap("5", 1, 0);
        s.penalties.black = vec![penalty(3, 59, Infraction::Obstruction)];
        let list = feed(&mut t, s.clone(), 308);
        assert_eq!(list.items[1].anchor, at(100));
        s.fouls.white = vec![foul(4, Infraction::StickInfringement)];
        let list = feed(&mut t, s, 320);
        assert_eq!(list.items[2].anchor, at(320));
    }

    #[test]
    fn team_timeouts_do_not_move_the_anchor() {
        let mut t = started();
        let mut s = snap("5", 0, 0);
        s.timeout = Some(TimeoutSnapshot::Black(0));
        feed(&mut t, s.clone(), 100);
        s.scores.white = 1;
        let list = feed(&mut t, s, 150);
        assert_eq!(list.items[0].anchor, at(150));
    }

    #[test]
    fn a_refbox_reset_keeps_the_list_and_starts_afresh() {
        let mut t = started();
        feed(&mut t, snap("5", 1, 0), 100);
        let mut s = snap("5", 1, 0);
        s.fouls.white = vec![foul(4, Infraction::Obstruction)];
        feed(&mut t, s, 150);
        // Everything cleared at once: a reset, not two corrections.
        let list = feed(&mut t, snap("5", 0, 0), 200);
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.score, Some([1, 0]));
        // Play goes on from the cleared state.
        let list = feed(&mut t, snap("5", 0, 1), 300);
        assert_eq!(list.items.len(), 3);
    }

    #[test]
    fn items_from_long_before_the_video_are_left_out() {
        let mut t = started();
        feed(&mut t, snap("5", 1, 0), 100);
        let list = feed(&mut t, snap("5", 1, 1), 10_000);
        // A video made again for the game, starting long after the first goal.
        let block = description_block(&list, at(9_900)).unwrap();
        assert!(!block.contains("Goal — Black"), "{block}");
        assert!(block.contains("1:25 Goal — White"), "{block}");
        assert_eq!(
            description_block(
                &GameHighlights {
                    items: list.items[..1].to_vec(),
                    ..list
                },
                at(9_900)
            ),
            None
        );
    }

    #[test]
    fn a_goal_taken_back_is_removed() {
        let mut t = started();
        feed(&mut t, snap("5", 1, 0), 100);
        feed(&mut t, snap("5", 1, 1), 200);
        let list = feed(&mut t, snap("5", 0, 1), 260);
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].side, Some(Side::White));
    }

    #[test]
    fn a_penalty_names_its_foul_and_length() {
        let mut t = started();
        let mut s = snap("5", 0, 0);
        s.penalties.black = vec![penalty(3, 119, Infraction::Obstruction)];
        let list = feed(&mut t, s, 100);
        let block = description_block(&list, at(0)).unwrap();
        assert!(
            block.contains("1:25 Penalty — Black #3, 2 min (Obstruction)"),
            "{block}"
        );
    }

    #[test]
    fn a_penalty_gone_quickly_was_a_mistake_but_a_served_one_stays() {
        let mut t = started();
        let mut s = snap("5", 0, 0);
        s.penalties.black = vec![penalty(3, 59, Infraction::Obstruction)];
        feed(&mut t, s, 100);
        let list = feed(&mut t, snap("5", 0, 0), 110);
        assert!(list.items.is_empty());

        let mut s = snap("5", 0, 0);
        s.penalties.white = vec![penalty(8, 59, Infraction::FreeArm)];
        feed(&mut t, s, 200);
        let list = feed(&mut t, snap("5", 0, 0), 270);
        assert_eq!(list.items.len(), 1);
    }

    #[test]
    fn a_foul_removed_from_the_refbox_is_removed() {
        let mut t = started();
        let mut s = snap("5", 0, 0);
        s.fouls = OptColorBundle {
            black: vec![foul(2, Infraction::StickInfringement)],
            equal: vec![],
            white: vec![foul(4, Infraction::Obstruction)],
        };
        feed(&mut t, s.clone(), 100);
        s.fouls.black.clear();
        let list = feed(&mut t, s, 500);
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].cap, Some(4));
    }

    #[test]
    fn a_new_game_number_starts_a_new_list_and_a_saved_list_carries_on() {
        let mut t = started();
        feed(&mut t, snap("5", 1, 0), 100);
        let saved = t.list.clone();
        // A restart: the saved list for game 5 is picked up and goes on growing.
        let mut t = Tracker::default();
        t.on_snapshot(&snap("5", 1, 0), at(200), |_| saved.clone());
        let list = feed(&mut t, snap("5", 2, 0), 300);
        assert_eq!(list.items.len(), 2);
        // The next game starts empty.
        let changed = t.on_snapshot(&snap("6", 0, 0), at(400), |_| GameHighlights::default());
        assert_eq!(
            changed.map(|(g, l)| (g, l.items.len())),
            Some(("6".into(), 0))
        );
    }

    #[test]
    fn times_never_go_before_the_start_and_an_hour_shows_hours() {
        assert_eq!(video_time(Duration::seconds(-20)), "0:00");
        assert_eq!(video_time(Duration::seconds(3725)), "1:02:05");
        let list = GameHighlights {
            items: vec![Highlight {
                kind: Kind::Goal,
                side: Some(Side::Black),
                cap: None,
                infraction: None,
                length: None,
                entered: at(5),
                anchor: at(5),
            }],
            score: Some([1, 0]),
        };
        let block = description_block(&list, at(0)).unwrap();
        assert!(
            block.contains("0:00 Goal — Black (White 0–1 Black)"),
            "{block}"
        );
        assert!(block.ends_with("Final score: White 0–1 Black\n"), "{block}");
    }

    #[test]
    fn the_running_score_is_left_out_when_a_goal_was_missed() {
        let mut t = started();
        let list = feed(&mut t, snap("5", 0, 1), 100);
        let list = GameHighlights {
            score: Some([0, 2]),
            ..list
        };
        let block = description_block(&list, at(0)).unwrap();
        assert!(block.contains("1:25 Goal — White\n"), "{block}");
        assert!(block.contains("Final score: White 2–0 Black"), "{block}");
    }

    #[test]
    fn nothing_to_list_gives_no_block() {
        assert_eq!(description_block(&GameHighlights::default(), at(0)), None);
    }

    #[test]
    fn a_very_long_list_leaves_fouls_out_first() {
        let mut t = started();
        let mut s = snap("5", 0, 0);
        for i in 0..200 {
            s.fouls
                .black
                .push(foul((i % 15) as u8 + 1, Infraction::StickInfringement));
            feed(&mut t, s.clone(), 10 + i64::from(i));
        }
        s.scores.white = 1;
        let list = feed(&mut t, s, 400);
        let block = description_block(&list, at(0)).unwrap();
        assert!(block.chars().count() <= MAX_BLOCK_CHARS);
        assert!(block.contains("Goal — White"), "{block}");
        assert!(block.contains("(more fouls not shown)"), "{block}");
    }
}
