use crate::tournament_manager::{
    BlackWhiteBundle, Penalty, PenaltyKind, TournamentManager, TournamentManagerError,
};
use derivative::Derivative;
use std::{
    mem,
    sync::{Arc, Mutex, MutexGuard},
};
use thiserror::Error;
use tokio::time::Instant;
use uwh_common::game_snapshot::Color;

const MAX_LIST_LEN: usize = 8;

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Origin {
    color: Color,
    index: usize,
}

#[derive(Derivative)]
#[derivative(Debug, Default, PartialEq, Eq, Clone)]
enum EditablePenalty {
    Original(Origin, Penalty),
    Edited(Origin, Penalty),
    Deleted(Origin, Penalty),
    #[derivative(Default)]
    New(PenaltyKind, u8),
}

#[derive(Debug)]
pub struct PenaltyEditor {
    penalties: BlackWhiteBundle<Vec<EditablePenalty>>,
    tm: Arc<Mutex<TournamentManager>>,
    session_started: bool,
}

impl PenaltyEditor {
    pub fn new(tm: Arc<Mutex<TournamentManager>>) -> Self {
        Self {
            penalties: BlackWhiteBundle {
                black: vec![],
                white: vec![],
            },
            tm,
            session_started: false,
        }
    }

    pub fn start_session(&mut self) -> Result<()> {
        if self.session_started {
            return Err(PenaltyEditorError::ExistingSession);
        }
        let penalties = self.tm.lock()?.get_penalties();

        let init = |vec: &Vec<Penalty>, color: Color| {
            vec.iter()
                .enumerate()
                .map(|(index, pen)| EditablePenalty::Original(Origin { color, index }, pen.clone()))
                .collect()
        };

        self.penalties.black = init(&penalties.black, Color::Black);
        self.penalties.white = init(&penalties.white, Color::White);
        self.session_started = true;
        Ok(())
    }

    pub fn add_penalty(
        &mut self,
        color: Color,
        player_number: u8,
        kind: PenaltyKind,
    ) -> Result<()> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }
        let pen = EditablePenalty::New(kind, player_number);
        match color {
            Color::Black => self.penalties.black.push(pen),
            Color::White => self.penalties.white.push(pen),
        }
        Ok(())
    }

    pub fn get_penalty(&self, color: Color, index: usize) -> Result<PenaltyDetails> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }
        let pen = match color {
            Color::Black => self.penalties.black.get(index),
            Color::White => self.penalties.white.get(index),
        }
        .ok_or(PenaltyEditorError::InvalidIndex(color, index))?;
        Ok(match pen {
            EditablePenalty::Original(_, p)
            | EditablePenalty::Edited(_, p)
            | EditablePenalty::Deleted(_, p) => PenaltyDetails {
                kind: p.kind,
                player_number: p.player_number,
                color,
            },
            EditablePenalty::New(kind, player_number) => PenaltyDetails {
                kind: *kind,
                player_number: *player_number,
                color,
            },
        })
    }

    pub fn delete_penalty(&mut self, color: Color, index: usize) -> Result<()> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }
        let pen = match color {
            Color::Black => self.penalties.black.get_mut(index),
            Color::White => self.penalties.white.get_mut(index),
        }
        .ok_or(PenaltyEditorError::InvalidIndex(color, index))?;
        let mut remove = false;

        *pen = if let EditablePenalty::Original(o, p)
        | EditablePenalty::Edited(o, p)
        | EditablePenalty::Deleted(o, p) = pen
        {
            EditablePenalty::Deleted(*o, mem::take(p))
        } else {
            remove = true;
            mem::take(pen)
        };

        if remove {
            match color {
                Color::Black => self.penalties.black.remove(index),
                Color::White => self.penalties.white.remove(index),
            };
        }
        Ok(())
    }

    pub fn edit_penalty(
        &mut self,
        old_color: Color,
        index: usize,
        new_color: Color,
        new_player_number: u8,
        new_kind: PenaltyKind,
    ) -> Result<()> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }
        let pen = match old_color {
            Color::Black => self.penalties.black.get_mut(index),
            Color::White => self.penalties.white.get_mut(index),
        }
        .ok_or(PenaltyEditorError::InvalidIndex(old_color, index))?;

        *pen = if let EditablePenalty::Original(o, p)
        | EditablePenalty::Edited(o, p)
        | EditablePenalty::Deleted(o, p) = pen
        {
            p.kind = new_kind;
            p.player_number = new_player_number;
            EditablePenalty::Edited(*o, mem::take(p))
        } else {
            EditablePenalty::New(new_kind, new_player_number)
        };

        if new_color != old_color {
            match old_color {
                Color::Black => self
                    .penalties
                    .white
                    .push(self.penalties.black.remove(index)),
                Color::White => self
                    .penalties
                    .black
                    .push(self.penalties.white.remove(index)),
            }
        }
        Ok(())
    }

    pub fn get_printable_lists(
        &self,
        now: Instant,
    ) -> Result<BlackWhiteBundle<Vec<(String, FormatHint, PenaltyKind)>>> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }
        let tm = self.tm.lock()?;
        Ok(BlackWhiteBundle {
            black: generate_printable_list(&tm, &self.penalties.black, now)
                .ok_or(PenaltyEditorError::InvalidNowValue)?,
            white: generate_printable_list(&tm, &self.penalties.white, now)
                .ok_or(PenaltyEditorError::InvalidNowValue)?,
        })
    }

    pub fn apply_changes(&mut self, now: Instant) -> Result<()> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }

        enum Action {
            Edit,
            Delete,
        }

        let mut new_pens: Vec<(Color, PenaltyKind, u8)> = vec![];
        let mut modified_pens: Vec<(Origin, Penalty, Color, Action)> = vec![];

        for (pen, color) in self
            .penalties
            .black
            .drain(..)
            .zip([Color::Black].into_iter().cycle())
            .chain(
                self.penalties
                    .white
                    .drain(..)
                    .zip([Color::White].into_iter().cycle()),
            )
        {
            match pen {
                EditablePenalty::Original(_, _) => {}
                EditablePenalty::Edited(o, p) => modified_pens.push((o, p, color, Action::Edit)),
                EditablePenalty::Deleted(o, p) => modified_pens.push((o, p, color, Action::Delete)),
                EditablePenalty::New(kind, num) => new_pens.push((color, kind, num)),
            }
        }

        modified_pens.sort_by(|a, b| a.0.index.cmp(&b.0.index));

        let mut tm = self.tm.lock()?;

        for (origin, pen, new_color, action) in modified_pens.into_iter().rev() {
            match action {
                Action::Edit => tm.edit_penalty(
                    origin.color,
                    origin.index,
                    new_color,
                    pen.player_number,
                    pen.kind,
                )?,
                Action::Delete => tm.delete_penalty(origin.color, origin.index)?,
            }
        }

        for (color, kind, player_number) in new_pens.into_iter() {
            tm.start_penalty(color, player_number, kind, now)?;
        }

        let b_too_long = match tm.limit_pen_list_len(Color::Black, MAX_LIST_LEN, now) {
            Ok(()) => false,
            Err(TournamentManagerError::TooManyPenalties(_)) => true,
            Err(e) => return Err(e.into()),
        };
        let w_too_long = match tm.limit_pen_list_len(Color::White, MAX_LIST_LEN, now) {
            Ok(()) => false,
            Err(TournamentManagerError::TooManyPenalties(_)) => true,
            Err(e) => return Err(e.into()),
        };

        std::mem::drop(tm);

        self.abort_session();

        if b_too_long & w_too_long {
            return Err(PenaltyEditorError::ListTooLong("Black and White"));
        } else if b_too_long {
            return Err(PenaltyEditorError::ListTooLong("Black"));
        } else if w_too_long {
            return Err(PenaltyEditorError::ListTooLong("White"));
        };

        Ok(())
    }

    pub fn abort_session(&mut self) {
        self.penalties.black.clear();
        self.penalties.white.clear();
        self.session_started = false;
    }
}

fn generate_printable_list(
    tm: &TournamentManager,
    penalties: &[EditablePenalty],
    now: Instant,
) -> Option<Vec<(String, FormatHint, PenaltyKind)>> {
    penalties
        .iter()
        .map(|pen| {
            let (p_num, time, kind) = match pen {
                EditablePenalty::Original(_, p)
                | EditablePenalty::Edited(_, p)
                | EditablePenalty::Deleted(_, p) => {
                    Some((p.player_number, tm.printable_penalty_time(p, now)?, p.kind))
                }
                EditablePenalty::New(kind, num) => Some((*num, String::from("Pending"), *kind)),
            }?;
            let hint = match pen {
                EditablePenalty::Original(_, _) => FormatHint::NoChange,
                EditablePenalty::Edited(_, _) => FormatHint::Edited,
                EditablePenalty::Deleted(_, _) => FormatHint::Deleted,
                EditablePenalty::New(_, _) => FormatHint::New,
            };
            let kind_str = match kind {
                PenaltyKind::OneMinute => "1m",
                PenaltyKind::TwoMinute => "2m",
                PenaltyKind::FiveMinute => "5m",
                PenaltyKind::TotalDismissal => "DSMS",
            };
            Some((format!("Player {p_num} - {time} ({kind_str})"), hint, kind))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PenaltyDetails {
    pub kind: PenaltyKind,
    pub player_number: u8,
    pub color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatHint {
    NoChange,
    Edited,
    Deleted,
    New,
}

#[derive(Debug, PartialEq, Error)]
pub enum PenaltyEditorError {
    #[error("The Mutex was poisoned")]
    MutexPoisoned,
    #[error("There is already a session in progress")]
    ExistingSession,
    #[error("There is no session in progress")]
    NotInSession,
    #[error("No {0} penalty exists at the index {1}")]
    InvalidIndex(Color, usize),
    #[error("The `now` value passed is not valid")]
    InvalidNowValue,
    #[error("The {0} penalty list(s) are too long")]
    ListTooLong(&'static str),
    #[error(transparent)]
    TMError(#[from] TournamentManagerError),
}

impl From<std::sync::PoisonError<MutexGuard<'_, TournamentManager>>> for PenaltyEditorError {
    fn from(_: std::sync::PoisonError<MutexGuard<TournamentManager>>) -> Self {
        PenaltyEditorError::MutexPoisoned
    }
}

pub type Result<T> = std::result::Result<T, PenaltyEditorError>;

#[cfg(test)]
mod test {
    use super::*;
    use crate::tournament_manager::Penalty;
    use std::time::Duration;
    use uwh_common::{config::Game as GameConfig, game_snapshot::GamePeriod};

    fn b_origin(index: usize) -> Origin {
        Origin {
            color: Color::Black,
            index,
        }
    }

    fn w_origin(index: usize) -> Origin {
        Origin {
            color: Color::White,
            index,
        }
    }

    #[test]
    fn test_add_penalty() {
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let mut now = Instant::now();
        let mut tm = TournamentManager::new(config);
        tm.start_play_now(now).unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut pen_edit = PenaltyEditor::new(tm.clone());

        let b_pen = Penalty {
            kind: PenaltyKind::OneMinute,
            player_number: 3,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(880),
        };

        let w_pen = Penalty {
            kind: PenaltyKind::TwoMinute,
            player_number: 13,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(880),
        };

        assert_eq!(
            pen_edit.add_penalty(Color::Black, 4, PenaltyKind::OneMinute),
            Err(PenaltyEditorError::NotInSession)
        );

        pen_edit.start_session().unwrap();
        pen_edit
            .add_penalty(Color::Black, b_pen.player_number, b_pen.kind)
            .unwrap();

        assert_eq!(
            pen_edit.get_penalty(Color::Black, 0),
            Ok(PenaltyDetails {
                kind: b_pen.kind,
                player_number: b_pen.player_number,
                color: Color::Black,
            })
        );

        pen_edit
            .add_penalty(Color::White, w_pen.player_number, w_pen.kind)
            .unwrap();

        assert_eq!(
            pen_edit.get_penalty(Color::White, 0),
            Ok(PenaltyDetails {
                kind: w_pen.kind,
                player_number: w_pen.player_number,
                color: Color::White,
            })
        );

        now += Duration::from_secs(20);
        pen_edit.apply_changes(now).unwrap();
        assert_eq!(
            tm.lock().unwrap().get_penalties(),
            BlackWhiteBundle {
                black: vec![b_pen],
                white: vec![w_pen],
            }
        );
    }

    #[test]
    fn test_delete_penalty() {
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let mut now = Instant::now();
        let mut tm = TournamentManager::new(config);
        tm.start_play_now(now).unwrap();

        now += Duration::from_secs(5);

        let b_pen_0 = Penalty {
            kind: PenaltyKind::OneMinute,
            player_number: 7,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
        };

        let w_pen_0 = Penalty {
            kind: PenaltyKind::FiveMinute,
            player_number: 4,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
        };

        let b_pen_1 = Penalty {
            kind: PenaltyKind::TotalDismissal,
            player_number: 13,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
        };

        let w_pen_1 = Penalty {
            kind: PenaltyKind::OneMinute,
            player_number: 6,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
        };

        tm.start_penalty(Color::Black, b_pen_0.player_number, b_pen_0.kind, now)
            .unwrap();
        tm.start_penalty(Color::White, w_pen_0.player_number, w_pen_0.kind, now)
            .unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut pen_edit = PenaltyEditor::new(tm.clone());

        assert_eq!(
            pen_edit.delete_penalty(Color::Black, 0),
            Err(PenaltyEditorError::NotInSession)
        );

        pen_edit.start_session().unwrap();

        assert_eq!(
            pen_edit.get_penalty(Color::Black, 0),
            Ok(PenaltyDetails {
                kind: b_pen_0.kind,
                player_number: b_pen_0.player_number,
                color: Color::Black,
            })
        );

        assert_eq!(
            pen_edit.get_penalty(Color::White, 0),
            Ok(PenaltyDetails {
                kind: w_pen_0.kind,
                player_number: w_pen_0.player_number,
                color: Color::White,
            })
        );

        assert_eq!(
            pen_edit.penalties.black,
            vec![EditablePenalty::Original(b_origin(0), b_pen_0.clone())]
        );
        assert_eq!(
            pen_edit.penalties.white,
            vec![EditablePenalty::Original(w_origin(0), w_pen_0.clone())]
        );

        pen_edit.delete_penalty(Color::Black, 0).unwrap();
        pen_edit.delete_penalty(Color::White, 0).unwrap();

        assert_eq!(
            pen_edit.get_penalty(Color::Black, 0),
            Ok(PenaltyDetails {
                kind: b_pen_0.kind,
                player_number: b_pen_0.player_number,
                color: Color::Black,
            })
        );

        assert_eq!(
            pen_edit.get_penalty(Color::White, 0),
            Ok(PenaltyDetails {
                kind: w_pen_0.kind,
                player_number: w_pen_0.player_number,
                color: Color::White,
            })
        );

        assert_eq!(
            pen_edit.penalties.black,
            vec![EditablePenalty::Deleted(b_origin(0), b_pen_0.clone())]
        );
        assert_eq!(
            pen_edit.penalties.white,
            vec![EditablePenalty::Deleted(w_origin(0), w_pen_0.clone())]
        );

        // The original ones should be re-deletable without any changes
        pen_edit.delete_penalty(Color::Black, 0).unwrap();
        pen_edit.delete_penalty(Color::White, 0).unwrap();

        assert_eq!(
            pen_edit.penalties.black,
            vec![EditablePenalty::Deleted(b_origin(0), b_pen_0.clone())]
        );
        assert_eq!(
            pen_edit.penalties.white,
            vec![EditablePenalty::Deleted(w_origin(0), w_pen_0.clone())]
        );

        pen_edit
            .penalties
            .black
            .push(EditablePenalty::Edited(b_origin(1), b_pen_1.clone()));
        pen_edit
            .penalties
            .black
            .push(EditablePenalty::New(PenaltyKind::TwoMinute, 9));

        pen_edit
            .penalties
            .white
            .push(EditablePenalty::Edited(w_origin(1), w_pen_1.clone()));
        pen_edit
            .penalties
            .white
            .push(EditablePenalty::New(PenaltyKind::TwoMinute, 3));

        pen_edit.delete_penalty(Color::Black, 1).unwrap();
        pen_edit.delete_penalty(Color::Black, 2).unwrap();
        pen_edit.delete_penalty(Color::White, 1).unwrap();
        pen_edit.delete_penalty(Color::White, 2).unwrap();

        assert_eq!(
            pen_edit.penalties.black,
            vec![
                EditablePenalty::Deleted(b_origin(0), b_pen_0),
                EditablePenalty::Deleted(b_origin(1), b_pen_1)
            ]
        );
        assert_eq!(
            pen_edit.penalties.white,
            vec![
                EditablePenalty::Deleted(w_origin(0), w_pen_0),
                EditablePenalty::Deleted(w_origin(1), w_pen_1)
            ]
        );

        assert_eq!(
            pen_edit.delete_penalty(Color::Black, 3),
            Err(PenaltyEditorError::InvalidIndex(Color::Black, 3))
        );
        assert_eq!(
            pen_edit.delete_penalty(Color::White, 2),
            Err(PenaltyEditorError::InvalidIndex(Color::White, 2))
        );

        pen_edit.penalties.black.remove(1);
        pen_edit.penalties.white.remove(1);

        now += Duration::from_secs(20);
        pen_edit.apply_changes(now).unwrap();
        assert_eq!(
            tm.lock().unwrap().get_penalties(),
            BlackWhiteBundle {
                black: vec![],
                white: vec![],
            }
        );
    }

    #[test]
    fn test_edit_penalty() {
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let mut now = Instant::now();
        let mut tm = TournamentManager::new(config);
        tm.start_play_now(now).unwrap();

        now += Duration::from_secs(5);

        let b_pen_0 = Penalty {
            kind: PenaltyKind::OneMinute,
            player_number: 7,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
        };

        let b_pen_0_ed = Penalty {
            kind: PenaltyKind::TwoMinute,
            ..b_pen_0
        };

        let w_pen_0 = Penalty {
            kind: PenaltyKind::FiveMinute,
            player_number: 4,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
        };

        let w_pen_0_ed = Penalty {
            kind: PenaltyKind::TwoMinute,
            ..w_pen_0
        };

        let b_pen_1 = Penalty {
            kind: PenaltyKind::TotalDismissal,
            player_number: 13,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
        };

        let b_pen_1_ed = Penalty {
            kind: PenaltyKind::FiveMinute,
            ..b_pen_1
        };

        let w_pen_1 = Penalty {
            kind: PenaltyKind::OneMinute,
            player_number: 6,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
        };

        let w_pen_1_ed = Penalty {
            kind: PenaltyKind::FiveMinute,
            ..w_pen_1
        };

        let b_pen_2 = Penalty {
            kind: PenaltyKind::FiveMinute,
            player_number: 1,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
        };

        let b_pen_2_ed = Penalty {
            kind: PenaltyKind::TwoMinute,
            ..b_pen_2
        };

        let w_pen_2 = Penalty {
            kind: PenaltyKind::TwoMinute,
            player_number: 8,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
        };

        let w_pen_2_ed = Penalty {
            kind: PenaltyKind::OneMinute,
            player_number: 9,
            ..w_pen_2
        };

        tm.start_penalty(Color::Black, b_pen_0.player_number, b_pen_0.kind, now)
            .unwrap();
        tm.start_penalty(Color::White, w_pen_0.player_number, w_pen_0.kind, now)
            .unwrap();
        tm.start_penalty(Color::Black, b_pen_1.player_number, b_pen_1.kind, now)
            .unwrap();
        tm.start_penalty(Color::White, w_pen_1.player_number, w_pen_1.kind, now)
            .unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut pen_edit = PenaltyEditor::new(tm.clone());

        assert_eq!(
            pen_edit.edit_penalty(Color::Black, 0, Color::Black, 2, PenaltyKind::OneMinute),
            Err(PenaltyEditorError::NotInSession)
        );

        pen_edit.start_session().unwrap();

        // Edit Original without color change
        pen_edit
            .edit_penalty(
                Color::Black,
                0,
                Color::Black,
                b_pen_0_ed.player_number,
                b_pen_0_ed.kind,
            )
            .unwrap();
        pen_edit
            .edit_penalty(
                Color::White,
                0,
                Color::White,
                w_pen_0_ed.player_number,
                w_pen_0_ed.kind,
            )
            .unwrap();

        // Edit Original with color change
        pen_edit
            .edit_penalty(
                Color::Black,
                1,
                Color::White,
                b_pen_1.player_number,
                b_pen_1.kind,
            )
            .unwrap();
        pen_edit
            .edit_penalty(
                Color::White,
                1,
                Color::Black,
                w_pen_1.player_number,
                w_pen_1.kind,
            )
            .unwrap();

        assert_eq!(
            pen_edit.penalties.black,
            vec![
                EditablePenalty::Edited(b_origin(0), b_pen_0_ed.clone()),
                EditablePenalty::Edited(w_origin(1), w_pen_1)
            ]
        );
        assert_eq!(
            pen_edit.penalties.white,
            vec![
                EditablePenalty::Edited(w_origin(0), w_pen_0_ed.clone()),
                EditablePenalty::Edited(b_origin(1), b_pen_1)
            ]
        );

        // Edit Edited
        pen_edit
            .edit_penalty(
                Color::Black,
                1,
                Color::Black,
                w_pen_1_ed.player_number,
                w_pen_1_ed.kind,
            )
            .unwrap();
        pen_edit
            .edit_penalty(
                Color::White,
                1,
                Color::White,
                b_pen_1_ed.player_number,
                b_pen_1_ed.kind,
            )
            .unwrap();

        assert_eq!(
            pen_edit.penalties.black,
            vec![
                EditablePenalty::Edited(b_origin(0), b_pen_0_ed.clone()),
                EditablePenalty::Edited(w_origin(1), w_pen_1_ed.clone())
            ]
        );
        assert_eq!(
            pen_edit.penalties.white,
            vec![
                EditablePenalty::Edited(w_origin(0), w_pen_0_ed.clone()),
                EditablePenalty::Edited(b_origin(1), b_pen_1_ed.clone())
            ]
        );

        // Edit Deleted and New
        pen_edit
            .penalties
            .black
            .push(EditablePenalty::Deleted(b_origin(2), b_pen_2));
        pen_edit
            .penalties
            .white
            .push(EditablePenalty::Deleted(w_origin(2), w_pen_2));

        pen_edit
            .penalties
            .black
            .push(EditablePenalty::New(PenaltyKind::TotalDismissal, 15));
        pen_edit
            .penalties
            .white
            .push(EditablePenalty::New(PenaltyKind::TwoMinute, 2));

        pen_edit
            .edit_penalty(
                Color::Black,
                2,
                Color::Black,
                b_pen_2_ed.player_number,
                b_pen_2_ed.kind,
            )
            .unwrap();
        pen_edit
            .edit_penalty(
                Color::White,
                2,
                Color::White,
                w_pen_2_ed.player_number,
                w_pen_2_ed.kind,
            )
            .unwrap();

        pen_edit
            .edit_penalty(
                Color::Black,
                3,
                Color::Black,
                14,
                PenaltyKind::TotalDismissal,
            )
            .unwrap();
        pen_edit
            .edit_penalty(Color::White, 3, Color::White, 3, PenaltyKind::FiveMinute)
            .unwrap();

        assert_eq!(
            pen_edit.penalties.black,
            vec![
                EditablePenalty::Edited(b_origin(0), b_pen_0_ed.clone()),
                EditablePenalty::Edited(w_origin(1), w_pen_1_ed.clone()),
                EditablePenalty::Edited(b_origin(2), b_pen_2_ed),
                EditablePenalty::New(PenaltyKind::TotalDismissal, 14)
            ]
        );
        assert_eq!(
            pen_edit.penalties.white,
            vec![
                EditablePenalty::Edited(w_origin(0), w_pen_0_ed.clone()),
                EditablePenalty::Edited(b_origin(1), b_pen_1_ed.clone()),
                EditablePenalty::Edited(w_origin(2), w_pen_2_ed),
                EditablePenalty::New(PenaltyKind::FiveMinute, 3)
            ]
        );

        // Test applying changes
        pen_edit.penalties.black.remove(3);
        pen_edit.penalties.black.remove(2);
        pen_edit.penalties.white.remove(3);
        pen_edit.penalties.white.remove(2);

        now += Duration::from_secs(20);
        pen_edit.apply_changes(now).unwrap();
        assert_eq!(
            tm.lock().unwrap().get_penalties(),
            BlackWhiteBundle {
                black: vec![b_pen_0_ed, w_pen_1_ed],
                white: vec![w_pen_0_ed, b_pen_1_ed],
            }
        );
    }
}
