//! Decides, for one court, when to switch from one game's video to the next.
//!
//! The rules (see `docs/decisions/026-per-game-youtube-streams.md`, §3 and §4):
//! - A game's video keeps running after the final whistle, so interviews stay in it.
//! - During the break, switch when the countdown to the next game reaches `switch_lead_secs`
//!   (3:15), just before the overlay starts showing the next game's rosters.
//! - Never switch automatically while the rosters are on screen
//!   (`roster_end_secs..=roster_start_secs`); wait until they finish.
//! - If a game is already being played and its video is not live (e.g. Hold was on), switch
//!   to it straight away.
//! - Hold stops all automatic switching. Switch now always switches.
//!
//! This module only decides; carrying out the switch on YouTube/vMix happens elsewhere.

use uwh_common::game_snapshot::{GamePeriod, GameSnapshot};

pub type GameNumber = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchRules {
    pub switch_lead_secs: u32,
    pub roster_start_secs: u32,
    pub roster_end_secs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    StartDay,
    EndDay,
    Hold,
    Release,
    SwitchNow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// First video of the day goes live.
    GoLive(GameNumber),
    /// End `from`'s video and make `to`'s video live.
    Switch { from: GameNumber, to: GameNumber },
    /// Last video of the day ends.
    End(GameNumber),
}

/// Where the court is in its timeline, as far as switching is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// No snapshot from the refbox yet.
    Unknown,
    /// A game is being played (including half time, overtime, sudden death).
    Playing(GameNumber),
    /// Break before `upcoming`, with `secs_left` on the countdown.
    Break {
        upcoming: GameNumber,
        secs_left: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub day_running: bool,
    pub live: Option<GameNumber>,
    pub hold: bool,
    pub phase: Phase,
    /// Seconds until the automatic switch, when one is coming up in this break.
    pub secs_until_switch: Option<u32>,
    /// Seconds until the overlay starts showing rosters, during the break.
    pub secs_until_rosters: Option<u32>,
    /// The game that goes live next: the one Start day would put live before the day starts,
    /// or the one Switch now would during it. None while that isn't known.
    pub next: Option<GameNumber>,
}

#[derive(Debug)]
pub struct CourtSwitcher {
    rules: SwitchRules,
    day_running: bool,
    live: Option<GameNumber>,
    hold: bool,
    last: Option<GameSnapshot>,
}

impl CourtSwitcher {
    pub fn new(rules: SwitchRules) -> Self {
        Self {
            rules,
            day_running: false,
            live: None,
            hold: false,
            last: None,
        }
    }

    /// Changes the timing rules, keeping everything else (the day, Hold, the last snapshot).
    pub fn set_rules(&mut self, rules: SwitchRules) {
        self.rules = rules;
    }

    pub fn status(&self) -> Status {
        let phase = self.last.as_ref().map_or(Phase::Unknown, phase_of);
        let (secs_until_switch, secs_until_rosters) = match &phase {
            Phase::Break {
                upcoming,
                secs_left,
            } => {
                let switch = (self.day_running
                    && !upcoming.is_empty()
                    && self.live.as_ref() != Some(upcoming))
                .then(|| secs_left.saturating_sub(self.rules.switch_lead_secs));
                let rosters = (*secs_left > self.rules.roster_start_secs)
                    .then(|| secs_left - self.rules.roster_start_secs);
                (switch, rosters)
            }
            _ => (None, None),
        };
        Status {
            day_running: self.day_running,
            live: self.live.clone(),
            hold: self.hold,
            phase,
            secs_until_switch,
            secs_until_rosters,
            next: self.next_target(),
        }
    }

    /// The game Start day (day not running) or Switch now (day running) would put live.
    fn next_target(&self) -> Option<GameNumber> {
        let snapshot = self.last.as_ref()?;
        let phase = phase_of(snapshot);
        let Some(live) = self.live.as_ref().filter(|_| self.day_running) else {
            return match phase {
                Phase::Playing(game) | Phase::Break { upcoming: game, .. } => Some(game),
                Phase::Unknown => None,
            };
        };
        let target = match phase {
            Phase::Playing(game) | Phase::Break { upcoming: game, .. } if game != *live => game,
            // The live video already shows the current/upcoming game, so move on to the one
            // after it.
            _ => snapshot.next_game_number.clone(),
        };
        (target != *live && !target.is_empty()).then_some(target)
    }

    /// Feed every snapshot the refbox sends. Returns an action when it's time to switch.
    pub fn on_snapshot(&mut self, snapshot: &GameSnapshot) -> Option<Action> {
        self.last = Some(snapshot.clone());
        self.evaluate()
    }

    pub fn on_command(&mut self, command: Command) -> Option<Action> {
        match command {
            Command::StartDay => {
                if self.day_running {
                    return None;
                }
                let target = self.next_target()?;
                self.day_running = true;
                self.live = Some(target.clone());
                Some(Action::GoLive(target))
            }
            Command::EndDay => {
                self.day_running = false;
                self.hold = false;
                self.live.take().map(Action::End)
            }
            Command::Hold => {
                self.hold = true;
                None
            }
            Command::Release => {
                self.hold = false;
                self.evaluate()
            }
            Command::SwitchNow => {
                self.live.as_ref().filter(|_| self.day_running)?;
                let target = self.next_target()?;
                // Switching by hand ends any hold.
                self.hold = false;
                self.switch_to(target)
            }
        }
    }

    /// After a restart, carries on with `live` as the live video, as if Start day had been
    /// pressed. No action: the video is already live on YouTube.
    pub fn resume(&mut self, live: GameNumber) {
        self.day_running = true;
        self.live = Some(live);
        self.hold = false;
    }

    /// A switch couldn't be carried out. `actually_live` is the game whose video is really live
    /// now. Automatic switching pauses (Hold) so it isn't retried every second; the operator
    /// retries with Switch now.
    pub fn switch_failed(&mut self, actually_live: Option<GameNumber>) {
        self.day_running = actually_live.is_some();
        self.hold = self.day_running;
        self.live = actually_live;
    }

    fn evaluate(&mut self) -> Option<Action> {
        if !self.day_running || self.hold {
            return None;
        }
        let live = self.live.as_ref()?;
        let target = match phase_of(self.last.as_ref()?) {
            Phase::Unknown => return None,
            // Catch up: a game is in progress but its video isn't live.
            Phase::Playing(game) => (game != *live).then_some(game)?,
            Phase::Break {
                upcoming,
                secs_left,
            } => {
                let in_rosters = (self.rules.roster_end_secs..=self.rules.roster_start_secs)
                    .contains(&secs_left);
                // After the court's last game the refbox sends a blank upcoming game: there is
                // no next video, so the last one simply waits for End day (ADR 026).
                (!upcoming.is_empty()
                    && upcoming != *live
                    && secs_left <= self.rules.switch_lead_secs
                    && !in_rosters)
                    .then_some(upcoming)?
            }
        };
        self.switch_to(target)
    }

    fn switch_to(&mut self, target: GameNumber) -> Option<Action> {
        let from = self.live.replace(target.clone())?;
        Some(Action::Switch { from, to: target })
    }
}

fn phase_of(snapshot: &GameSnapshot) -> Phase {
    match snapshot.current_period {
        GamePeriod::BetweenGames => Phase::Break {
            // The refbox only changes `game_number` when the next game kicks off, so during the
            // whole break it is still the previous game ("0" before the first game of the day).
            // The upcoming game is always `next_game_number`.
            upcoming: snapshot.next_game_number.clone(),
            secs_left: snapshot.secs_in_period,
        },
        _ => Phase::Playing(snapshot.game_number.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: SwitchRules = SwitchRules {
        switch_lead_secs: 195,
        roster_start_secs: 181,
        roster_end_secs: 30,
    };

    fn playing(game: &str, next: &str) -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::SecondHalf,
            secs_in_period: 300,
            game_number: game.to_string(),
            next_game_number: next.to_string(),
            ..Default::default()
        }
    }

    /// Break after `old` finished, refbox still showing its result.
    fn break_old(old: &str, next: &str, secs: u32) -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::BetweenGames,
            secs_in_period: secs,
            is_old_game: true,
            game_number: old.to_string(),
            next_game_number: next.to_string(),
            ..Default::default()
        }
    }

    /// Break after the refbox has been reset for the next game (scores cleared, overlay shows
    /// rosters). As in the real refbox, `game_number` is still the previous game until kickoff.
    fn break_new(previous: &str, upcoming: &str, secs: u32) -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::BetweenGames,
            secs_in_period: secs,
            is_old_game: false,
            game_number: previous.to_string(),
            next_game_number: upcoming.to_string(),
            ..Default::default()
        }
    }

    fn started(snapshot: GameSnapshot) -> CourtSwitcher {
        let mut s = CourtSwitcher::new(RULES);
        s.on_snapshot(&snapshot);
        s.on_command(Command::StartDay);
        s
    }

    fn switch(from: &str, to: &str) -> Option<Action> {
        Some(Action::Switch {
            from: from.into(),
            to: to.into(),
        })
    }

    #[test]
    fn before_the_first_game_the_upcoming_game_is_not_game_0() {
        // Refbox freshly set to an event: placeholder game "0", counting down to game 1.
        let mut s = CourtSwitcher::new(RULES);
        s.on_snapshot(&break_new("0", "1", 900));
        assert_eq!(
            s.on_command(Command::StartDay),
            Some(Action::GoLive("1".into()))
        );
        // Kickoff of game 1: nothing more to do.
        assert_eq!(s.on_snapshot(&playing("1", "3")), None);
    }

    #[test]
    fn nothing_happens_before_start_day() {
        let mut s = CourtSwitcher::new(RULES);
        assert_eq!(s.on_snapshot(&break_old("14", "15", 100)), None);
        assert_eq!(s.on_snapshot(&playing("15", "16")), None);
        assert_eq!(s.status().live, None);
    }

    #[test]
    fn start_day_goes_live_with_current_or_upcoming_game() {
        let mut s = CourtSwitcher::new(RULES);
        assert_eq!(s.on_command(Command::StartDay), None, "no refbox data yet");
        s.on_snapshot(&break_new("0", "1", 600));
        assert_eq!(
            s.on_command(Command::StartDay),
            Some(Action::GoLive("1".into()))
        );
        assert_eq!(s.on_command(Command::StartDay), None, "already running");

        let mut s = CourtSwitcher::new(RULES);
        s.on_snapshot(&playing("7", "8"));
        assert_eq!(
            s.on_command(Command::StartDay),
            Some(Action::GoLive("7".into()))
        );
    }

    #[test]
    fn video_keeps_running_after_the_game_until_lead_time() {
        let mut s = started(playing("14", "15"));
        // Final whistle, interview time: no switch.
        for secs in [600, 400, 300, 196] {
            assert_eq!(
                s.on_snapshot(&break_old("14", "15", secs)),
                None,
                "at {secs}s"
            );
        }
        // 3:15 before Game 15.
        assert_eq!(
            s.on_snapshot(&break_old("14", "15", 195)),
            switch("14", "15")
        );
        assert_eq!(s.status().live.as_deref(), Some("15"));
        // No repeat switching.
        assert_eq!(s.on_snapshot(&break_new("14", "15", 150)), None);
        assert_eq!(s.on_snapshot(&playing("15", "16")), None);
    }

    #[test]
    fn after_the_last_game_the_video_waits_for_end_day() {
        // Last game of the court's day: the refbox's break has a blank upcoming game.
        let mut s = started(playing("20", ""));
        for secs in [600, 195, 100, 10, 0] {
            assert_eq!(
                s.on_snapshot(&break_old("20", "", secs)),
                None,
                "at {secs}s"
            );
        }
        let status = s.status();
        assert_eq!(status.live.as_deref(), Some("20"));
        assert!(!status.hold);
        assert_eq!(status.secs_until_switch, None);
        assert_eq!(status.next, None);
    }

    #[test]
    fn switches_when_refbox_has_already_moved_on_to_the_next_game() {
        let mut s = started(playing("14", "15"));
        assert_eq!(
            s.on_snapshot(&break_new("14", "15", 190)),
            switch("14", "15")
        );
    }

    #[test]
    fn never_switches_automatically_during_rosters() {
        let mut s = started(playing("14", "15"));
        // Program missed the lead window (e.g. refbox reconnected inside the rosters).
        for secs in [181, 120, 30] {
            assert_eq!(
                s.on_snapshot(&break_new("14", "15", secs)),
                None,
                "at {secs}s"
            );
        }
        assert_eq!(
            s.on_snapshot(&break_new("14", "15", 29)),
            switch("14", "15")
        );
    }

    #[test]
    fn hold_blocks_switching_and_release_waits_for_rosters_to_end() {
        let mut s = started(playing("14", "15"));
        s.on_command(Command::Hold);
        assert_eq!(s.on_snapshot(&break_old("14", "15", 195)), None);
        assert_eq!(s.on_snapshot(&break_new("14", "15", 150)), None);
        assert!(s.status().hold);
        // Released while rosters are on screen: still wait.
        assert_eq!(s.on_command(Command::Release), None);
        assert_eq!(s.on_snapshot(&break_new("14", "15", 31)), None);
        assert_eq!(
            s.on_snapshot(&break_new("14", "15", 29)),
            switch("14", "15")
        );
    }

    #[test]
    fn release_before_rosters_switches_straight_away() {
        let mut s = started(playing("14", "15"));
        s.on_command(Command::Hold);
        assert_eq!(s.on_snapshot(&break_old("14", "15", 190)), None);
        assert_eq!(s.on_command(Command::Release), switch("14", "15"));
    }

    #[test]
    fn hold_keeps_old_video_even_after_next_game_starts() {
        let mut s = started(playing("14", "15"));
        s.on_command(Command::Hold);
        assert_eq!(s.on_snapshot(&playing("15", "16")), None);
        // Releasing during play catches up to the game in progress.
        assert_eq!(s.on_command(Command::Release), switch("14", "15"));
    }

    #[test]
    fn switch_now_works_anywhere_and_clears_hold() {
        // During the interview, long before lead time.
        let mut s = started(playing("14", "15"));
        s.on_snapshot(&break_old("14", "15", 500));
        assert_eq!(s.on_command(Command::SwitchNow), switch("14", "15"));

        // During the rosters, with hold on.
        let mut s = started(playing("14", "15"));
        s.on_command(Command::Hold);
        s.on_snapshot(&break_new("14", "15", 100));
        assert_eq!(s.on_command(Command::SwitchNow), switch("14", "15"));
        assert!(!s.status().hold);

        // Live video already shows the current game: move to the one after it.
        let mut s = started(playing("15", "16"));
        assert_eq!(s.on_command(Command::SwitchNow), switch("15", "16"));
    }

    #[test]
    fn switch_now_does_nothing_before_start_day() {
        let mut s = CourtSwitcher::new(RULES);
        s.on_snapshot(&break_old("14", "15", 500));
        assert_eq!(s.on_command(Command::SwitchNow), None);
    }

    #[test]
    fn end_day_ends_the_live_video_and_stops_switching() {
        let mut s = started(playing("22", "23"));
        assert_eq!(
            s.on_command(Command::EndDay),
            Some(Action::End("22".into()))
        );
        assert_eq!(s.on_snapshot(&break_old("22", "23", 100)), None);
        assert_eq!(s.on_command(Command::EndDay), None);
    }

    #[test]
    fn failed_switch_restores_the_live_game_and_holds() {
        let mut s = started(playing("14", "15"));
        assert_eq!(
            s.on_snapshot(&break_old("14", "15", 195)),
            switch("14", "15")
        );
        s.switch_failed(Some("14".into()));
        let status = s.status();
        assert_eq!((status.live.as_deref(), status.hold), (Some("14"), true));
        // No automatic retry...
        assert_eq!(s.on_snapshot(&break_old("14", "15", 190)), None);
        // ...but Switch now retries.
        assert_eq!(s.on_command(Command::SwitchNow), switch("14", "15"));

        // A failed start leaves the day not started.
        let mut s = started(playing("14", "15"));
        s.switch_failed(None);
        assert!(!s.status().day_running);
        assert_eq!(
            s.on_command(Command::StartDay),
            Some(Action::GoLive("14".into()))
        );
    }

    #[test]
    fn resume_carries_on_from_the_live_video_as_after_start_day() {
        // Restarted during the break after Game 14, whose video is still live on YouTube.
        let mut s = CourtSwitcher::new(RULES);
        s.resume("14".into());
        let status = s.status();
        assert!(status.day_running);
        assert_eq!((status.live.as_deref(), status.hold), (Some("14"), false));
        assert_eq!(s.on_snapshot(&break_old("14", "15", 196)), None);
        assert_eq!(
            s.on_snapshot(&break_old("14", "15", 195)),
            switch("14", "15")
        );
        assert_eq!(s.on_command(Command::StartDay), None, "already running");

        // Hold and Switch now work as after Start day.
        let mut s = CourtSwitcher::new(RULES);
        s.resume("14".into());
        s.on_command(Command::Hold);
        assert_eq!(s.on_snapshot(&break_old("14", "15", 195)), None);
        assert_eq!(s.on_command(Command::SwitchNow), switch("14", "15"));
        assert!(!s.status().hold);

        // A hold from before the restart doesn't carry over.
        let mut s = started(playing("14", "15"));
        s.on_command(Command::Hold);
        s.resume("14".into());
        assert!(!s.status().hold);
    }

    #[test]
    fn status_counts_down_to_switch_and_rosters() {
        let mut s = started(playing("14", "15"));
        s.on_snapshot(&break_old("14", "15", 240));
        let status = s.status();
        assert_eq!(status.secs_until_switch, Some(45));
        assert_eq!(status.secs_until_rosters, Some(59));
        assert_eq!(
            status.phase,
            Phase::Break {
                upcoming: "15".into(),
                secs_left: 240
            }
        );
    }
}
