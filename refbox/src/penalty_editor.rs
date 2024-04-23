use crate::tournament_manager::{
    infraction::InfractionDetails,
    penalty::{Penalty, PenaltyKind},
    BlackWhiteBundle, OptColorBundle, TournamentManager, TournamentManagerError,
};
use std::{
    fmt::Debug,
    mem,
    ops::{Index, IndexMut},
    sync::{Arc, Mutex, MutexGuard},
};
use thiserror::Error;
use tokio::time::Instant;
use uwh_common::game_snapshot::{Color, Infraction};

pub trait ColorIndex: Debug + Default + PartialEq + Eq + Clone + Copy {
    type Structure<T: Default>;
}

impl ColorIndex for Color {
    type Structure<T: Default> = BlackWhiteBundle<T>;
}

impl ColorIndex for Option<Color> {
    type Structure<T: Default> = OptColorBundle<T>;
}

pub trait IterHelp<C: ColorIndex, T> {
    fn iter<'a>(&'a self) -> impl Iterator<Item = (C, &'a T)>
    where
        T: 'a;

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = (C, &'a mut T)>
    where
        T: 'a;
}

impl<T> IterHelp<Color, T> for BlackWhiteBundle<T> {
    fn iter<'a>(&'a self) -> impl Iterator<Item = (Color, &'a T)>
    where
        T: 'a,
    {
        self.iter()
    }

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = (Color, &'a mut T)>
    where
        T: 'a,
    {
        std::iter::once((Color::Black, &mut self.black))
            .chain(std::iter::once((Color::White, &mut self.white)))
    }
}

impl<T> IterHelp<Option<Color>, T> for OptColorBundle<T> {
    fn iter<'a>(&'a self) -> impl Iterator<Item = (Option<Color>, &'a T)>
    where
        T: 'a,
    {
        self.iter()
    }

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = (Option<Color>, &'a mut T)>
    where
        T: 'a,
    {
        std::iter::once((Some(Color::Black), &mut self.black))
            .chain(std::iter::once((None, &mut self.equal)))
            .chain(std::iter::once((Some(Color::White), &mut self.white)))
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Origin<C: ColorIndex> {
    color: C,
    index: usize,
}

type TmGuard<'a> = std::sync::MutexGuard<'a, TournamentManager>;

pub(super) trait Editable<C: ColorIndex>: Debug + PartialEq + Eq + Clone {
    type Number: Debug + Default + PartialEq + Eq + Clone + Copy;
    type Kind: Debug + Default + PartialEq + Eq + Clone + Copy;
    type Summary;
    type PrintableSummary;
    const MAX_LIST_LEN: usize;

    fn get_items_from_tm<'a>(tm: &'a TmGuard) -> &'a C::Structure<Vec<Self>>;

    fn into_summary(&self, color: C) -> Self::Summary;

    fn new_summary(kind: Self::Kind, player_number: Self::Number, color: C) -> Self::Summary;

    fn edit(&mut self, kind: Self::Kind, player_number: Self::Number, infraction: Infraction);

    #[allow(private_interfaces)]
    fn generate_printable_list(
        tm: &TmGuard,
        items: &[EditableItem<Self, C>],
        now: Instant,
    ) -> Option<Vec<Self::PrintableSummary>>;

    fn edit_in_tm(
        tm: &mut TmGuard,
        old_color: C,
        index: usize,
        new_color: C,
        new_item: Self,
    ) -> std::result::Result<(), TournamentManagerError>;

    fn delete_in_tm(
        tm: &mut TmGuard,
        color: C,
        index: usize,
    ) -> std::result::Result<(), TournamentManagerError>;

    fn add_to_tm(
        tm: &mut TmGuard,
        color: C,
        player_number: Self::Number,
        kind: Self::Kind,
        now: Instant,
        infraction: Infraction,
    ) -> std::result::Result<(), TournamentManagerError>;

    fn limit_list_len(
        tm: &mut TmGuard,
        color: C,
        limit: usize,
        now: Instant,
    ) -> std::result::Result<(), TournamentManagerError>;
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum EditableItem<T, C>
where
    C: ColorIndex,
    T: Editable<C>,
{
    Original(Origin<C>, T),
    Edited(Origin<C>, T),
    Deleted(Origin<C>, T),
    New(T::Kind, T::Number, Infraction),
}

impl<T: Editable<C>, C: ColorIndex> Default for EditableItem<T, C> {
    fn default() -> Self {
        Self::New(Default::default(), Default::default(), Default::default())
    }
}

#[derive(Debug)]
pub struct ListEditor<T: Editable<C>, C: ColorIndex> {
    items: C::Structure<Vec<EditableItem<T, C>>>,
    tm: Arc<Mutex<TournamentManager>>,
    session_started: bool,
}

#[allow(private_bounds)]
impl<T: Editable<C>, C: ColorIndex> ListEditor<T, C>
where
    <C as ColorIndex>::Structure<Vec<EditableItem<T, C>>>: std::default::Default
        + Index<C, Output = Vec<EditableItem<T, C>>>
        + IndexMut<C, Output = Vec<EditableItem<T, C>>>
        + IntoIterator<Item = (C, Vec<EditableItem<T, C>>)>
        + FromIterator<(C, Vec<EditableItem<T, C>>)>
        + IterHelp<C, Vec<EditableItem<T, C>>>,
    <C as ColorIndex>::Structure<Vec<T>>: IntoIterator<Item = (C, Vec<T>)> + IterHelp<C, Vec<T>>,
    <C as ColorIndex>::Structure<bool>: Debug
        + Default
        + IntoIterator<Item = (C, bool)>
        + IterHelp<C, bool>
        + FromIterator<(C, bool)>,
    <C as ColorIndex>::Structure<Vec<T::PrintableSummary>>:
        FromIterator<(C, Vec<T::PrintableSummary>)>,
{
    pub fn new(tm: Arc<Mutex<TournamentManager>>) -> Self {
        Self {
            items: Default::default(),
            tm,
            session_started: false,
        }
    }

    pub fn start_session(&mut self) -> Result<()> {
        if self.session_started {
            return Err(PenaltyEditorError::ExistingSession);
        }
        let tm = self.tm.lock()?;
        let items = T::get_items_from_tm(&tm);

        let init = |color: C, vec: &Vec<T>| {
            (
                color,
                vec.iter()
                    .enumerate()
                    .map(|(index, item)| {
                        EditableItem::Original(Origin { color, index }, item.clone())
                    })
                    .collect(),
            )
        };

        self.items = items.iter().map(|(color, vec)| init(color, vec)).collect();
        self.session_started = true;
        Ok(())
    }

    pub fn add_item(
        &mut self,
        color: C,
        player_number: T::Number,
        kind: T::Kind,
        infraction: Infraction,
    ) -> Result<()> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }
        let item = EditableItem::New(kind, player_number, infraction);
        self.items[color].push(item);
        Ok(())
    }

    pub fn get_item(&self, color: C, index: usize) -> Result<T::Summary> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }
        let item = self.items[color]
            .get(index)
            .ok_or(PenaltyEditorError::InvalidIndex(
                format!("{color:?}"),
                index,
            ))?;
        Ok(match item {
            EditableItem::Original(_, p)
            | EditableItem::Edited(_, p)
            | EditableItem::Deleted(_, p) => p.into_summary(color),
            EditableItem::New(kind, player_number, _) => {
                T::new_summary(*kind, *player_number, color)
            }
        })
    }

    pub fn delete_item(&mut self, color: C, index: usize) -> Result<()> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }
        let item = self.items[color]
            .get_mut(index)
            .ok_or(PenaltyEditorError::InvalidIndex(
                format!("{color:?}"),
                index,
            ))?;
        let mut remove = false;

        *item = if let EditableItem::Original(o, i)
        | EditableItem::Edited(o, i)
        | EditableItem::Deleted(o, i) = item
        {
            EditableItem::Deleted(*o, i.clone())
        } else {
            remove = true;
            mem::take(item)
        };

        if remove {
            self.items[color].remove(index);
        }
        Ok(())
    }

    pub fn edit_item(
        &mut self,
        old_color: C,
        index: usize,
        new_color: C,
        new_player_number: T::Number,
        new_kind: T::Kind,
        new_infraction: Infraction,
    ) -> Result<()> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }
        let item = self.items[old_color]
            .get_mut(index)
            .ok_or(PenaltyEditorError::InvalidIndex(
                format!("{old_color:?}"),
                index,
            ))?;

        *item = if let EditableItem::Original(o, i)
        | EditableItem::Edited(o, i)
        | EditableItem::Deleted(o, i) = item
        {
            i.edit(new_kind, new_player_number, new_infraction);
            EditableItem::Edited(*o, i.clone())
        } else {
            EditableItem::New(new_kind, new_player_number, new_infraction)
        };

        if new_color != old_color {
            let item = self.items[old_color].remove(index);
            self.items[new_color].push(item);
        }
        Ok(())
    }

    pub fn get_printable_lists(
        &self,
        now: Instant,
    ) -> Result<C::Structure<Vec<T::PrintableSummary>>> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }
        let tm = self.tm.lock()?;
        Ok(self
            .items
            .iter()
            .map(|(color, vec)| {
                (
                    color,
                    T::generate_printable_list(&tm, vec, now).unwrap_or_default(),
                )
            })
            .collect())
    }

    pub fn apply_changes(&mut self, now: Instant) -> Result<()> {
        if !self.session_started {
            return Err(PenaltyEditorError::NotInSession);
        }

        enum Action {
            Edit,
            Delete,
        }

        let mut new_pens: Vec<(C, T::Kind, T::Number, Infraction)> = vec![];
        let mut modified_pens: Vec<(Origin<C>, T, C, Action)> = vec![];

        for (item, color) in self
            .items
            .iter_mut()
            .flat_map(|(color, vec)| vec.drain(..).zip([color].into_iter().cycle()))
        {
            match item {
                EditableItem::Original(_, _) => {}
                EditableItem::Edited(o, i) => modified_pens.push((o, i, color, Action::Edit)),
                EditableItem::Deleted(o, i) => modified_pens.push((o, i, color, Action::Delete)),
                EditableItem::New(kind, num, infraction) => {
                    new_pens.push((color, kind, num, infraction))
                }
            }
        }

        modified_pens.sort_by(|a, b| a.0.index.cmp(&b.0.index));

        let mut tm = self.tm.lock()?;

        for (origin, pen, new_color, action) in modified_pens.into_iter().rev() {
            match action {
                Action::Edit => T::edit_in_tm(&mut tm, origin.color, origin.index, new_color, pen)?,
                Action::Delete => T::delete_in_tm(&mut tm, origin.color, origin.index)?,
            }
        }

        for (color, kind, player_number, infraction) in new_pens.into_iter() {
            T::add_to_tm(&mut tm, color, player_number, kind, now, infraction)?;
        }

        let too_long: C::Structure<bool> = Default::default();
        let too_long = too_long
            .into_iter()
            .map(
                |(c, _)| match T::limit_list_len(&mut tm, c, T::MAX_LIST_LEN, now) {
                    Ok(()) => Ok((c, false)),
                    Err(TournamentManagerError::TooManyPenalties(_)) => Ok((c, true)),
                    Err(e) => return Err(e.into()),
                },
            )
            .collect::<Result<C::Structure<bool>>>()?;

        std::mem::drop(tm);

        self.abort_session();

        if too_long.iter().any(|(_, x)| *x) {
            return Err(PenaltyEditorError::ListTooLong(format!("{too_long:?}")));
        };

        Ok(())
    }

    pub fn abort_session(&mut self) {
        self.items.iter_mut().map(|(_, v)| v.clear()).for_each(drop);
        self.session_started = false;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintablePenaltySummary {
    pub text: String,
    pub hint: FormatHint,
    pub kind: PenaltyKind,
    pub infraction: Infraction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PenaltySummary {
    pub kind: PenaltyKind,
    pub player_number: u8,
    pub color: Color,
}

impl Editable<Color> for Penalty {
    type Number = u8;
    type Kind = PenaltyKind;
    type Summary = PenaltySummary;
    type PrintableSummary = PrintablePenaltySummary;

    const MAX_LIST_LEN: usize = 8;

    fn get_items_from_tm<'a>(tm: &'a TmGuard) -> &'a BlackWhiteBundle<Vec<Self>> {
        tm.get_penalties()
    }

    fn into_summary(&self, color: Color) -> Self::Summary {
        Self::Summary {
            kind: self.kind,
            player_number: self.player_number,
            color,
        }
    }

    fn new_summary(kind: Self::Kind, player_number: Self::Number, color: Color) -> Self::Summary {
        Self::Summary {
            kind,
            player_number,
            color,
        }
    }

    fn edit(&mut self, kind: Self::Kind, player_number: Self::Number, infraction: Infraction) {
        self.kind = kind;
        self.player_number = player_number;
        self.infraction = infraction;
    }

    #[allow(private_interfaces)]
    fn generate_printable_list(
        tm: &TmGuard,
        items: &[EditableItem<Self, Color>],
        now: Instant,
    ) -> Option<Vec<Self::PrintableSummary>> {
        items
            .iter()
            .map(|pen| {
                let (p_num, time, kind, infraction) = match pen {
                    EditableItem::Original(_, p)
                    | EditableItem::Edited(_, p)
                    | EditableItem::Deleted(_, p) => (
                        p.player_number,
                        tm.printable_penalty_time(p, now)?,
                        p.kind,
                        p.infraction,
                    ),
                    EditableItem::New(kind, num, infraction) => {
                        (*num, String::from("Pending"), *kind, *infraction)
                    }
                };
                let hint = match pen {
                    EditableItem::Original(_, _) => FormatHint::NoChange,
                    EditableItem::Edited(_, _) => FormatHint::Edited,
                    EditableItem::Deleted(_, _) => FormatHint::Deleted,
                    EditableItem::New(_, _, _) => FormatHint::New,
                };
                let kind_str = match kind {
                    PenaltyKind::ThirtySecond => "30s",
                    PenaltyKind::OneMinute => "1m",
                    PenaltyKind::TwoMinute => "2m",
                    PenaltyKind::FourMinute => "4m",
                    PenaltyKind::FiveMinute => "5m",
                    PenaltyKind::TotalDismissal => "DSMS",
                };
                Some(Self::PrintableSummary {
                    text: format!("Player {p_num} - {time} ({kind_str})"),
                    hint,
                    kind,
                    infraction,
                })
            })
            .collect()
    }

    fn edit_in_tm(
        tm: &mut TmGuard,
        old_color: Color,
        index: usize,
        new_color: Color,
        new_item: Self,
    ) -> std::result::Result<(), TournamentManagerError> {
        tm.edit_penalty(
            old_color,
            index,
            new_color,
            new_item.player_number,
            new_item.kind,
            new_item.infraction,
        )
    }

    fn delete_in_tm(
        tm: &mut TmGuard,
        color: Color,
        index: usize,
    ) -> std::result::Result<(), TournamentManagerError> {
        tm.delete_penalty(color, index)
    }

    fn add_to_tm(
        tm: &mut TmGuard,
        color: Color,
        player_number: Self::Number,
        kind: Self::Kind,
        now: Instant,
        infraction: Infraction,
    ) -> std::result::Result<(), TournamentManagerError> {
        tm.start_penalty(color, player_number, kind, now, infraction)
    }

    fn limit_list_len(
        tm: &mut TmGuard,
        color: Color,
        limit: usize,
        now: Instant,
    ) -> std::result::Result<(), TournamentManagerError> {
        tm.limit_pen_list_len(color, limit, now)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintableInfractionSummary {
    pub text: String,
    pub hint: FormatHint,
    pub infraction: Infraction,
    pub team: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarningSummary {
    pub player_number: Option<u8>,
    pub color: Color,
}

impl Editable<Color> for InfractionDetails {
    type Number = Option<u8>;
    type Kind = ();
    type Summary = WarningSummary;
    type PrintableSummary = PrintableInfractionSummary;

    const MAX_LIST_LEN: usize = usize::MAX;

    fn get_items_from_tm<'a>(tm: &'a TmGuard) -> &'a BlackWhiteBundle<Vec<Self>> {
        tm.get_warnings()
    }

    fn into_summary(&self, color: Color) -> Self::Summary {
        Self::Summary {
            player_number: self.player_number,
            color,
        }
    }

    fn new_summary(_kind: Self::Kind, player_number: Self::Number, color: Color) -> Self::Summary {
        Self::Summary {
            player_number,
            color,
        }
    }

    fn edit(&mut self, _kind: Self::Kind, player_number: Self::Number, infraction: Infraction) {
        self.player_number = player_number;
        self.infraction = infraction;
    }

    #[allow(private_interfaces)]
    fn generate_printable_list(
        _tm: &TmGuard,
        items: &[EditableItem<Self, Color>],
        _now: Instant,
    ) -> Option<Vec<Self::PrintableSummary>> {
        generate_printable_infraction_list(items, InfractionPrintMode::Warning)
    }

    fn edit_in_tm(
        tm: &mut TmGuard,
        old_color: Color,
        index: usize,
        new_color: Color,
        new_item: Self,
    ) -> std::result::Result<(), TournamentManagerError> {
        tm.edit_warning(
            old_color,
            index,
            new_color,
            new_item.player_number,
            new_item.infraction,
        )
    }

    fn delete_in_tm(
        tm: &mut TmGuard,
        color: Color,
        index: usize,
    ) -> std::result::Result<(), TournamentManagerError> {
        tm.delete_warning(color, index)
    }

    fn add_to_tm(
        tm: &mut TmGuard,
        color: Color,
        player_number: Self::Number,
        _kind: Self::Kind,
        now: Instant,
        infraction: Infraction,
    ) -> std::result::Result<(), TournamentManagerError> {
        tm.add_warning(color, player_number, infraction, now)
    }

    fn limit_list_len(
        _tm: &mut TmGuard,
        _color: Color,
        _limit: usize,
        _now: Instant,
    ) -> std::result::Result<(), TournamentManagerError> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoulSummary {
    pub player_number: Option<u8>,
    pub color: Option<Color>,
}

impl Editable<Option<Color>> for InfractionDetails {
    type Number = Option<u8>;
    type Kind = ();
    type Summary = FoulSummary;
    type PrintableSummary = PrintableInfractionSummary;

    const MAX_LIST_LEN: usize = usize::MAX;

    fn get_items_from_tm<'a>(tm: &'a TmGuard) -> &'a OptColorBundle<Vec<Self>> {
        tm.get_fouls()
    }

    fn into_summary(&self, color: Option<Color>) -> Self::Summary {
        Self::Summary {
            player_number: self.player_number,
            color,
        }
    }

    fn new_summary(
        _kind: Self::Kind,
        player_number: Self::Number,
        color: Option<Color>,
    ) -> Self::Summary {
        Self::Summary {
            player_number,
            color,
        }
    }

    fn edit(&mut self, _kind: Self::Kind, player_number: Self::Number, infraction: Infraction) {
        self.player_number = player_number;
        self.infraction = infraction;
    }

    #[allow(private_interfaces)]
    fn generate_printable_list(
        _tm: &TmGuard,
        items: &[EditableItem<Self, Option<Color>>],
        _now: Instant,
    ) -> Option<Vec<Self::PrintableSummary>> {
        generate_printable_infraction_list(items, InfractionPrintMode::Foul)
    }

    fn edit_in_tm(
        tm: &mut TmGuard,
        old_color: Option<Color>,
        index: usize,
        new_color: Option<Color>,
        new_item: Self,
    ) -> std::result::Result<(), TournamentManagerError> {
        tm.edit_foul(
            old_color,
            index,
            new_color,
            new_item.player_number,
            new_item.infraction,
        )
    }

    fn delete_in_tm(
        tm: &mut TmGuard,
        color: Option<Color>,
        index: usize,
    ) -> std::result::Result<(), TournamentManagerError> {
        tm.delete_foul(color, index)
    }

    fn add_to_tm(
        tm: &mut TmGuard,
        color: Option<Color>,
        player_number: Self::Number,
        _kind: Self::Kind,
        now: Instant,
        infraction: Infraction,
    ) -> std::result::Result<(), TournamentManagerError> {
        tm.add_foul(color, player_number, infraction, now)
    }

    fn limit_list_len(
        _tm: &mut TmGuard,
        _color: Option<Color>,
        _limit: usize,
        _now: Instant,
    ) -> std::result::Result<(), TournamentManagerError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InfractionPrintMode {
    Warning,
    Foul,
}

fn generate_printable_infraction_list<C: ColorIndex>(
    items: &[EditableItem<InfractionDetails, C>],
    mode: InfractionPrintMode,
) -> Option<Vec<PrintableInfractionSummary>>
where
    InfractionDetails: Editable<C, Number = Option<u8>>,
{
    items
        .iter()
        .map(|pen| {
            let (p_num, infraction, team) = match pen {
                EditableItem::Original(_, p)
                | EditableItem::Edited(_, p)
                | EditableItem::Deleted(_, p) => {
                    (p.player_number, p.infraction, p.player_number.is_none())
                }
                EditableItem::New(_, num, infraction) => (*num, *infraction, num.is_none()),
            };
            let hint = match pen {
                EditableItem::Original(_, _) => FormatHint::NoChange,
                EditableItem::Edited(_, _) => FormatHint::Edited,
                EditableItem::Deleted(_, _) => FormatHint::Deleted,
                EditableItem::New(_, _, _) => FormatHint::New,
            };

            let text = match mode {
                InfractionPrintMode::Warning => {
                    let who = if let Some(p_num) = p_num {
                        format!("#{p_num}")
                    } else {
                        "T".to_string()
                    };
                    format!("{who} - {}", infraction.short_name())
                }
                InfractionPrintMode::Foul => {
                    let who = if let Some(p_num) = p_num {
                        format!("#{p_num} - ")
                    } else {
                        "".to_string()
                    };
                    format!("{who}{}", infraction.short_name())
                }
            };

            Some(PrintableInfractionSummary {
                text,
                hint,
                infraction,
                team,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatHint {
    NoChange,
    Edited,
    Deleted,
    New,
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum PenaltyEditorError {
    #[error("The Mutex was poisoned")]
    MutexPoisoned,
    #[error("There is already a session in progress")]
    ExistingSession,
    #[error("There is no session in progress")]
    NotInSession,
    #[error("No {0} penalty exists at the index {1}")]
    InvalidIndex(String, usize),
    #[error("The {0} list(s) are too long")]
    ListTooLong(String),
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
    use crate::tournament_manager::penalty::Penalty;
    use std::time::Duration;
    use uwh_common::{config::Game as GameConfig, game_snapshot::GamePeriod};

    fn b_origin(index: usize) -> Origin<Color> {
        Origin {
            color: Color::Black,
            index,
        }
    }

    fn b_o_origin(index: usize) -> Origin<Option<Color>> {
        Origin {
            color: Some(Color::Black),
            index,
        }
    }

    fn w_origin(index: usize) -> Origin<Color> {
        Origin {
            color: Color::White,
            index,
        }
    }

    fn w_o_origin(index: usize) -> Origin<Option<Color>> {
        Origin {
            color: Some(Color::White),
            index,
        }
    }

    fn e_o_origin(index: usize) -> Origin<Option<Color>> {
        Origin { color: None, index }
    }

    #[test]
    fn test_add_penalty() {
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let now = Instant::now();
        let apply_time = now + Duration::from_secs(20);
        let mut tm = TournamentManager::new(config);
        tm.start_play_now(now).unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut pen_edit = ListEditor::<Penalty, Color>::new(tm.clone());

        let b_pen = Penalty {
            kind: PenaltyKind::OneMinute,
            player_number: 3,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(880),
            start_instant: apply_time,
            infraction: Infraction::Unknown,
        };

        let w_pen = Penalty {
            kind: PenaltyKind::TwoMinute,
            player_number: 13,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(880),
            start_instant: apply_time,
            infraction: Infraction::DelayOfGame,
        };

        assert_eq!(
            pen_edit.add_item(Color::Black, 4, PenaltyKind::OneMinute, Infraction::Unknown),
            Err(PenaltyEditorError::NotInSession)
        );

        pen_edit.start_session().unwrap();
        pen_edit
            .add_item(
                Color::Black,
                b_pen.player_number,
                b_pen.kind,
                b_pen.infraction,
            )
            .unwrap();

        assert_eq!(
            pen_edit.get_item(Color::Black, 0),
            Ok(PenaltySummary {
                kind: b_pen.kind,
                player_number: b_pen.player_number,
                color: Color::Black,
            })
        );

        pen_edit
            .add_item(
                Color::White,
                w_pen.player_number,
                w_pen.kind,
                w_pen.infraction,
            )
            .unwrap();

        assert_eq!(
            pen_edit.get_item(Color::White, 0),
            Ok(PenaltySummary {
                kind: w_pen.kind,
                player_number: w_pen.player_number,
                color: Color::White,
            })
        );

        pen_edit.apply_changes(apply_time).unwrap();
        assert_eq!(
            tm.lock().unwrap().get_penalties(),
            &BlackWhiteBundle {
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
            start_instant: now,
            infraction: Infraction::Unknown,
        };

        let w_pen_0 = Penalty {
            kind: PenaltyKind::FiveMinute,
            player_number: 4,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::DelayOfGame,
        };

        let b_pen_1 = Penalty {
            kind: PenaltyKind::TotalDismissal,
            player_number: 13,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::FalseStart,
        };

        let w_pen_1 = Penalty {
            kind: PenaltyKind::OneMinute,
            player_number: 6,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::FreeArm,
        };

        tm.start_penalty(
            Color::Black,
            b_pen_0.player_number,
            b_pen_0.kind,
            now,
            b_pen_0.infraction,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            w_pen_0.player_number,
            w_pen_0.kind,
            now,
            w_pen_0.infraction,
        )
        .unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut pen_edit = ListEditor::new(tm.clone());

        assert_eq!(
            pen_edit.delete_item(Color::Black, 0),
            Err(PenaltyEditorError::NotInSession)
        );

        pen_edit.start_session().unwrap();

        assert_eq!(
            pen_edit.get_item(Color::Black, 0),
            Ok(PenaltySummary {
                kind: b_pen_0.kind,
                player_number: b_pen_0.player_number,
                color: Color::Black,
            })
        );

        assert_eq!(
            pen_edit.get_item(Color::White, 0),
            Ok(PenaltySummary {
                kind: w_pen_0.kind,
                player_number: w_pen_0.player_number,
                color: Color::White,
            })
        );

        assert_eq!(
            pen_edit.items.black,
            vec![EditableItem::Original(b_origin(0), b_pen_0.clone())]
        );
        assert_eq!(
            pen_edit.items.white,
            vec![EditableItem::Original(w_origin(0), w_pen_0.clone())]
        );

        pen_edit.delete_item(Color::Black, 0).unwrap();
        pen_edit.delete_item(Color::White, 0).unwrap();

        assert_eq!(
            pen_edit.get_item(Color::Black, 0),
            Ok(PenaltySummary {
                kind: b_pen_0.kind,
                player_number: b_pen_0.player_number,
                color: Color::Black,
            })
        );

        assert_eq!(
            pen_edit.get_item(Color::White, 0),
            Ok(PenaltySummary {
                kind: w_pen_0.kind,
                player_number: w_pen_0.player_number,
                color: Color::White,
            })
        );

        assert_eq!(
            pen_edit.items.black,
            vec![EditableItem::Deleted(b_origin(0), b_pen_0.clone())]
        );
        assert_eq!(
            pen_edit.items.white,
            vec![EditableItem::Deleted(w_origin(0), w_pen_0.clone())]
        );

        // The original ones should be re-deletable without any changes
        pen_edit.delete_item(Color::Black, 0).unwrap();
        pen_edit.delete_item(Color::White, 0).unwrap();

        assert_eq!(
            pen_edit.items.black,
            vec![EditableItem::Deleted(b_origin(0), b_pen_0.clone())]
        );
        assert_eq!(
            pen_edit.items.white,
            vec![EditableItem::Deleted(w_origin(0), w_pen_0.clone())]
        );

        pen_edit
            .items
            .black
            .push(EditableItem::Edited(b_origin(1), b_pen_1.clone()));
        pen_edit.items.black.push(EditableItem::New(
            PenaltyKind::TwoMinute,
            9,
            Infraction::Unknown,
        ));

        pen_edit
            .items
            .white
            .push(EditableItem::Edited(w_origin(1), w_pen_1.clone()));
        pen_edit.items.white.push(EditableItem::New(
            PenaltyKind::TwoMinute,
            3,
            Infraction::Unknown,
        ));

        pen_edit.delete_item(Color::Black, 1).unwrap();
        pen_edit.delete_item(Color::Black, 2).unwrap();
        pen_edit.delete_item(Color::White, 1).unwrap();
        pen_edit.delete_item(Color::White, 2).unwrap();

        assert_eq!(
            pen_edit.items.black,
            vec![
                EditableItem::Deleted(b_origin(0), b_pen_0),
                EditableItem::Deleted(b_origin(1), b_pen_1)
            ]
        );
        assert_eq!(
            pen_edit.items.white,
            vec![
                EditableItem::Deleted(w_origin(0), w_pen_0),
                EditableItem::Deleted(w_origin(1), w_pen_1)
            ]
        );

        assert_eq!(
            pen_edit.delete_item(Color::Black, 3),
            Err(PenaltyEditorError::InvalidIndex("Black".to_string(), 3))
        );
        assert_eq!(
            pen_edit.delete_item(Color::White, 2),
            Err(PenaltyEditorError::InvalidIndex("White".to_string(), 2))
        );

        pen_edit.items.black.remove(1);
        pen_edit.items.white.remove(1);

        now += Duration::from_secs(20);
        pen_edit.apply_changes(now).unwrap();
        assert_eq!(
            tm.lock().unwrap().get_penalties(),
            &BlackWhiteBundle {
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
            start_instant: now,
            infraction: Infraction::Unknown,
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
            start_instant: now,
            infraction: Infraction::DelayOfGame,
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
            start_instant: now,
            infraction: Infraction::FalseStart,
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
            start_instant: now,
            infraction: Infraction::FreeArm,
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
            start_instant: now,
            infraction: Infraction::GrabbingTheBarrier,
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
            start_instant: now,
            infraction: Infraction::IllegalAdvancement,
        };

        let w_pen_2_ed = Penalty {
            kind: PenaltyKind::OneMinute,
            player_number: 9,
            ..w_pen_2
        };

        tm.start_penalty(
            Color::Black,
            b_pen_0.player_number,
            b_pen_0.kind,
            now,
            b_pen_0.infraction,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            w_pen_0.player_number,
            w_pen_0.kind,
            now,
            w_pen_0.infraction,
        )
        .unwrap();
        tm.start_penalty(
            Color::Black,
            b_pen_1.player_number,
            b_pen_1.kind,
            now,
            b_pen_1.infraction,
        )
        .unwrap();
        tm.start_penalty(
            Color::White,
            w_pen_1.player_number,
            w_pen_1.kind,
            now,
            w_pen_1.infraction,
        )
        .unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut pen_edit = ListEditor::new(tm.clone());

        assert_eq!(
            pen_edit.edit_item(
                Color::Black,
                0,
                Color::Black,
                2,
                PenaltyKind::OneMinute,
                Infraction::Unknown
            ),
            Err(PenaltyEditorError::NotInSession)
        );

        pen_edit.start_session().unwrap();

        // Edit Original without color change
        pen_edit
            .edit_item(
                Color::Black,
                0,
                Color::Black,
                b_pen_0_ed.player_number,
                b_pen_0_ed.kind,
                b_pen_0_ed.infraction,
            )
            .unwrap();
        pen_edit
            .edit_item(
                Color::White,
                0,
                Color::White,
                w_pen_0_ed.player_number,
                w_pen_0_ed.kind,
                w_pen_0_ed.infraction,
            )
            .unwrap();

        // Edit Original with color change
        pen_edit
            .edit_item(
                Color::Black,
                1,
                Color::White,
                b_pen_1.player_number,
                b_pen_1.kind,
                b_pen_1.infraction,
            )
            .unwrap();
        pen_edit
            .edit_item(
                Color::White,
                1,
                Color::Black,
                w_pen_1.player_number,
                w_pen_1.kind,
                w_pen_1.infraction,
            )
            .unwrap();

        assert_eq!(
            pen_edit.items.black,
            vec![
                EditableItem::Edited(b_origin(0), b_pen_0_ed.clone()),
                EditableItem::Edited(w_origin(1), w_pen_1)
            ]
        );
        assert_eq!(
            pen_edit.items.white,
            vec![
                EditableItem::Edited(w_origin(0), w_pen_0_ed.clone()),
                EditableItem::Edited(b_origin(1), b_pen_1)
            ]
        );

        // Edit Edited
        pen_edit
            .edit_item(
                Color::Black,
                1,
                Color::Black,
                w_pen_1_ed.player_number,
                w_pen_1_ed.kind,
                w_pen_1_ed.infraction,
            )
            .unwrap();
        pen_edit
            .edit_item(
                Color::White,
                1,
                Color::White,
                b_pen_1_ed.player_number,
                b_pen_1_ed.kind,
                b_pen_1_ed.infraction,
            )
            .unwrap();

        assert_eq!(
            pen_edit.items.black,
            vec![
                EditableItem::Edited(b_origin(0), b_pen_0_ed.clone()),
                EditableItem::Edited(w_origin(1), w_pen_1_ed.clone())
            ]
        );
        assert_eq!(
            pen_edit.items.white,
            vec![
                EditableItem::Edited(w_origin(0), w_pen_0_ed.clone()),
                EditableItem::Edited(b_origin(1), b_pen_1_ed.clone())
            ]
        );

        // Edit Deleted and New
        pen_edit
            .items
            .black
            .push(EditableItem::Deleted(b_origin(2), b_pen_2));
        pen_edit
            .items
            .white
            .push(EditableItem::Deleted(w_origin(2), w_pen_2));

        pen_edit.items.black.push(EditableItem::New(
            PenaltyKind::TotalDismissal,
            15,
            Infraction::IllegalSubstitution,
        ));
        pen_edit.items.white.push(EditableItem::New(
            PenaltyKind::TwoMinute,
            2,
            Infraction::IllegallyStoppingThePuck,
        ));

        pen_edit
            .edit_item(
                Color::Black,
                2,
                Color::Black,
                b_pen_2_ed.player_number,
                b_pen_2_ed.kind,
                b_pen_2_ed.infraction,
            )
            .unwrap();
        pen_edit
            .edit_item(
                Color::White,
                2,
                Color::White,
                w_pen_2_ed.player_number,
                w_pen_2_ed.kind,
                w_pen_2_ed.infraction,
            )
            .unwrap();

        pen_edit
            .edit_item(
                Color::Black,
                3,
                Color::Black,
                14,
                PenaltyKind::TotalDismissal,
                Infraction::IllegalSubstitution,
            )
            .unwrap();
        pen_edit
            .edit_item(
                Color::White,
                3,
                Color::White,
                3,
                PenaltyKind::FiveMinute,
                Infraction::IllegallyStoppingThePuck,
            )
            .unwrap();

        assert_eq!(
            pen_edit.items.black,
            vec![
                EditableItem::Edited(b_origin(0), b_pen_0_ed.clone()),
                EditableItem::Edited(w_origin(1), w_pen_1_ed.clone()),
                EditableItem::Edited(b_origin(2), b_pen_2_ed),
                EditableItem::New(
                    PenaltyKind::TotalDismissal,
                    14,
                    Infraction::IllegalSubstitution
                )
            ]
        );
        assert_eq!(
            pen_edit.items.white,
            vec![
                EditableItem::Edited(w_origin(0), w_pen_0_ed.clone()),
                EditableItem::Edited(b_origin(1), b_pen_1_ed.clone()),
                EditableItem::Edited(w_origin(2), w_pen_2_ed),
                EditableItem::New(
                    PenaltyKind::FiveMinute,
                    3,
                    Infraction::IllegallyStoppingThePuck
                )
            ]
        );

        // Test applying changes
        pen_edit.items.black.remove(3);
        pen_edit.items.black.remove(2);
        pen_edit.items.white.remove(3);
        pen_edit.items.white.remove(2);

        now += Duration::from_secs(20);
        pen_edit.apply_changes(now).unwrap();
        assert_eq!(
            tm.lock().unwrap().get_penalties(),
            &BlackWhiteBundle {
                black: vec![b_pen_0_ed, w_pen_1_ed],
                white: vec![w_pen_0_ed, b_pen_1_ed],
            }
        );
    }

    #[test]
    fn test_add_warning() {
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let now = Instant::now();
        let apply_time = now + Duration::from_secs(20);
        let mut tm = TournamentManager::new(config);
        tm.start_play_now(now).unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut warn_edit = ListEditor::<InfractionDetails, Color>::new(tm.clone());

        let b_warn = InfractionDetails {
            player_number: Some(3),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(880),
            start_instant: apply_time,
            infraction: Infraction::Unknown,
        };

        let w_warn = InfractionDetails {
            player_number: Some(13),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(880),
            start_instant: apply_time,
            infraction: Infraction::DelayOfGame,
        };

        assert_eq!(
            warn_edit.add_item(Color::Black, Some(4), (), Infraction::Unknown),
            Err(PenaltyEditorError::NotInSession)
        );

        warn_edit.start_session().unwrap();
        warn_edit
            .add_item(Color::Black, b_warn.player_number, (), b_warn.infraction)
            .unwrap();

        assert_eq!(
            warn_edit.get_item(Color::Black, 0),
            Ok(WarningSummary {
                player_number: b_warn.player_number,
                color: Color::Black,
            })
        );

        warn_edit
            .add_item(Color::White, w_warn.player_number, (), w_warn.infraction)
            .unwrap();

        assert_eq!(
            warn_edit.get_item(Color::White, 0),
            Ok(WarningSummary {
                player_number: w_warn.player_number,
                color: Color::White,
            })
        );

        warn_edit.apply_changes(apply_time).unwrap();
        assert_eq!(
            tm.lock().unwrap().get_warnings(),
            &BlackWhiteBundle {
                black: vec![b_warn],
                white: vec![w_warn],
            }
        );
    }

    #[test]
    fn test_delete_warning() {
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let mut now = Instant::now();
        let mut tm = TournamentManager::new(config);
        tm.start_play_now(now).unwrap();

        now += Duration::from_secs(5);

        let b_warn_0 = InfractionDetails {
            player_number: Some(7),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::Unknown,
        };

        let w_warn_0 = InfractionDetails {
            player_number: Some(4),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::DelayOfGame,
        };

        let b_warn_1 = InfractionDetails {
            player_number: Some(13),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::FalseStart,
        };

        let w_warn_1 = InfractionDetails {
            player_number: Some(6),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::FreeArm,
        };

        tm.add_warning(
            Color::Black,
            b_warn_0.player_number,
            b_warn_0.infraction,
            now,
        )
        .unwrap();
        tm.add_warning(
            Color::White,
            w_warn_0.player_number,
            w_warn_0.infraction,
            now,
        )
        .unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut warn_edit = ListEditor::new(tm.clone());

        assert_eq!(
            warn_edit.delete_item(Color::Black, 0),
            Err(PenaltyEditorError::NotInSession)
        );

        warn_edit.start_session().unwrap();

        assert_eq!(
            warn_edit.get_item(Color::Black, 0),
            Ok(WarningSummary {
                player_number: b_warn_0.player_number,
                color: Color::Black,
            })
        );
        assert_eq!(
            warn_edit.get_item(Color::White, 0),
            Ok(WarningSummary {
                player_number: w_warn_0.player_number,
                color: Color::White,
            })
        );
        assert_eq!(
            warn_edit.items.black,
            vec![EditableItem::Original(b_origin(0), b_warn_0.clone())]
        );
        assert_eq!(
            warn_edit.items.white,
            vec![EditableItem::Original(w_origin(0), w_warn_0.clone())]
        );

        warn_edit.delete_item(Color::Black, 0).unwrap();
        warn_edit.delete_item(Color::White, 0).unwrap();

        assert_eq!(
            warn_edit.get_item(Color::Black, 0),
            Ok(WarningSummary {
                player_number: b_warn_0.player_number,
                color: Color::Black,
            })
        );
        assert_eq!(
            warn_edit.get_item(Color::White, 0),
            Ok(WarningSummary {
                player_number: w_warn_0.player_number,
                color: Color::White,
            })
        );
        assert_eq!(
            warn_edit.items.black,
            vec![EditableItem::Deleted(b_origin(0), b_warn_0.clone())]
        );
        assert_eq!(
            warn_edit.items.white,
            vec![EditableItem::Deleted(w_origin(0), w_warn_0.clone())]
        );

        // The original ones should be re-deletable without any changes
        warn_edit.delete_item(Color::Black, 0).unwrap();
        warn_edit.delete_item(Color::White, 0).unwrap();

        assert_eq!(
            warn_edit.items.black,
            vec![EditableItem::Deleted(b_origin(0), b_warn_0.clone())]
        );
        assert_eq!(
            warn_edit.items.white,
            vec![EditableItem::Deleted(w_origin(0), w_warn_0.clone())]
        );

        warn_edit
            .items
            .black
            .push(EditableItem::Edited(b_origin(1), b_warn_1.clone()));
        warn_edit
            .items
            .black
            .push(EditableItem::New((), Some(9), Infraction::Unknown));
        warn_edit
            .items
            .white
            .push(EditableItem::Edited(w_origin(1), w_warn_1.clone()));
        warn_edit
            .items
            .white
            .push(EditableItem::New((), Some(3), Infraction::Unknown));

        warn_edit.delete_item(Color::Black, 1).unwrap();
        warn_edit.delete_item(Color::Black, 2).unwrap();
        warn_edit.delete_item(Color::White, 1).unwrap();
        warn_edit.delete_item(Color::White, 2).unwrap();

        assert_eq!(
            warn_edit.items.black,
            vec![
                EditableItem::Deleted(b_origin(0), b_warn_0.clone()),
                EditableItem::Deleted(b_origin(1), b_warn_1.clone())
            ]
        );
        assert_eq!(
            warn_edit.items.white,
            vec![
                EditableItem::Deleted(w_origin(0), w_warn_0.clone()),
                EditableItem::Deleted(w_origin(1), w_warn_1.clone())
            ]
        );
        assert_eq!(
            warn_edit.delete_item(Color::Black, 2),
            Err(PenaltyEditorError::InvalidIndex("Black".to_string(), 2))
        );
        assert_eq!(
            warn_edit.delete_item(Color::White, 2),
            Err(PenaltyEditorError::InvalidIndex("White".to_string(), 2))
        );

        warn_edit.items.black.remove(1);
        warn_edit.items.white.remove(1);

        now += Duration::from_secs(20);
        warn_edit.apply_changes(now).unwrap();

        assert_eq!(
            tm.lock().unwrap().get_warnings(),
            &BlackWhiteBundle {
                black: vec![],
                white: vec![],
            }
        );
    }

    #[test]
    fn test_edit_warning() {
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let mut now = Instant::now();
        let mut tm = TournamentManager::new(config);
        tm.start_play_now(now).unwrap();

        now += Duration::from_secs(5);

        let b_warn_0 = InfractionDetails {
            player_number: Some(7),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::Unknown,
        };

        let b_warn_0_ed = InfractionDetails {
            player_number: Some(2),
            ..b_warn_0
        };

        let w_warn_0 = InfractionDetails {
            player_number: Some(4),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::DelayOfGame,
        };

        let w_warn_0_ed = InfractionDetails {
            player_number: Some(3),
            ..w_warn_0
        };

        let b_warn_1 = InfractionDetails {
            player_number: Some(13),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::FalseStart,
        };

        let b_warn_1_ed = InfractionDetails {
            player_number: Some(5),
            ..b_warn_1
        };

        let w_warn_1 = InfractionDetails {
            player_number: Some(6),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::FreeArm,
        };

        let w_warn_1_ed = InfractionDetails {
            player_number: Some(9),
            ..w_warn_1
        };

        let b_warn_2 = InfractionDetails {
            player_number: Some(1),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::GrabbingTheBarrier,
        };

        let b_warn_2_ed = InfractionDetails {
            player_number: Some(8),
            ..b_warn_2
        };

        let w_warn_2 = InfractionDetails {
            player_number: Some(2),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::IllegalAdvancement,
        };

        let w_warn_2_ed = InfractionDetails {
            player_number: Some(1),
            ..w_warn_2
        };

        tm.add_warning(
            Color::Black,
            b_warn_0.player_number,
            b_warn_0.infraction,
            now,
        )
        .unwrap();
        tm.add_warning(
            Color::White,
            w_warn_0.player_number,
            w_warn_0.infraction,
            now,
        )
        .unwrap();
        tm.add_warning(
            Color::Black,
            b_warn_1.player_number,
            b_warn_1.infraction,
            now,
        )
        .unwrap();
        tm.add_warning(
            Color::White,
            w_warn_1.player_number,
            w_warn_1.infraction,
            now,
        )
        .unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut warn_edit = ListEditor::new(tm.clone());

        assert_eq!(
            warn_edit.edit_item(
                Color::Black,
                0,
                Color::Black,
                Some(2),
                (),
                Infraction::Unknown
            ),
            Err(PenaltyEditorError::NotInSession)
        );

        warn_edit.start_session().unwrap();

        // Edit Original without color change
        warn_edit
            .edit_item(
                Color::Black,
                0,
                Color::Black,
                b_warn_0_ed.player_number,
                (),
                b_warn_0_ed.infraction,
            )
            .unwrap();
        warn_edit
            .edit_item(
                Color::White,
                0,
                Color::White,
                w_warn_0_ed.player_number,
                (),
                w_warn_0_ed.infraction,
            )
            .unwrap();

        // Edit Original with color change
        warn_edit
            .edit_item(
                Color::Black,
                1,
                Color::White,
                b_warn_1_ed.player_number,
                (),
                b_warn_1_ed.infraction,
            )
            .unwrap();
        warn_edit
            .edit_item(
                Color::White,
                1,
                Color::Black,
                w_warn_1_ed.player_number,
                (),
                w_warn_1_ed.infraction,
            )
            .unwrap();

        assert_eq!(
            warn_edit.items.black,
            vec![
                EditableItem::Edited(b_origin(0), b_warn_0_ed.clone()),
                EditableItem::Edited(w_origin(1), w_warn_1_ed.clone())
            ]
        );
        assert_eq!(
            warn_edit.items.white,
            vec![
                EditableItem::Edited(w_origin(0), w_warn_0_ed.clone()),
                EditableItem::Edited(b_origin(1), b_warn_1_ed.clone())
            ]
        );

        // Edit Edited
        warn_edit
            .edit_item(
                Color::Black,
                1,
                Color::Black,
                w_warn_1_ed.player_number,
                (),
                w_warn_1_ed.infraction,
            )
            .unwrap();
        warn_edit
            .edit_item(
                Color::White,
                1,
                Color::White,
                b_warn_1_ed.player_number,
                (),
                b_warn_1_ed.infraction,
            )
            .unwrap();

        assert_eq!(
            warn_edit.items.black,
            vec![
                EditableItem::Edited(b_origin(0), b_warn_0_ed.clone()),
                EditableItem::Edited(w_origin(1), w_warn_1_ed.clone())
            ]
        );
        assert_eq!(
            warn_edit.items.white,
            vec![
                EditableItem::Edited(w_origin(0), w_warn_0_ed.clone()),
                EditableItem::Edited(b_origin(1), b_warn_1_ed.clone())
            ]
        );

        // Edit Deleted and New
        warn_edit
            .items
            .black
            .push(EditableItem::Deleted(b_origin(2), b_warn_2));
        warn_edit
            .items
            .white
            .push(EditableItem::Deleted(w_origin(2), w_warn_2));

        warn_edit
            .items
            .black
            .push(EditableItem::New((), Some(15), Infraction::Unknown));
        warn_edit
            .items
            .white
            .push(EditableItem::New((), Some(3), Infraction::Unknown));

        warn_edit
            .edit_item(
                Color::Black,
                2,
                Color::Black,
                b_warn_2_ed.player_number,
                (),
                b_warn_2_ed.infraction,
            )
            .unwrap();
        warn_edit
            .edit_item(
                Color::White,
                2,
                Color::White,
                w_warn_2_ed.player_number,
                (),
                w_warn_2_ed.infraction,
            )
            .unwrap();

        warn_edit
            .edit_item(
                Color::Black,
                3,
                Color::Black,
                Some(14),
                (),
                Infraction::IllegalSubstitution,
            )
            .unwrap();
        warn_edit
            .edit_item(
                Color::White,
                3,
                Color::White,
                Some(3),
                (),
                Infraction::IllegallyStoppingThePuck,
            )
            .unwrap();

        assert_eq!(
            warn_edit.items.black,
            vec![
                EditableItem::Edited(b_origin(0), b_warn_0_ed.clone()),
                EditableItem::Edited(w_origin(1), w_warn_1_ed.clone()),
                EditableItem::Edited(b_origin(2), b_warn_2_ed),
                EditableItem::New((), Some(14), Infraction::IllegalSubstitution)
            ]
        );
        assert_eq!(
            warn_edit.items.white,
            vec![
                EditableItem::Edited(w_origin(0), w_warn_0_ed.clone()),
                EditableItem::Edited(b_origin(1), b_warn_1_ed.clone()),
                EditableItem::Edited(w_origin(2), w_warn_2_ed),
                EditableItem::New((), Some(3), Infraction::IllegallyStoppingThePuck)
            ]
        );

        // Test applying changes
        warn_edit.items.black.remove(3);
        warn_edit.items.black.remove(2);
        warn_edit.items.white.remove(3);
        warn_edit.items.white.remove(2);

        now += Duration::from_secs(20);
        warn_edit.apply_changes(now).unwrap();
        assert_eq!(
            tm.lock().unwrap().get_warnings(),
            &BlackWhiteBundle {
                black: vec![b_warn_0_ed.clone(), w_warn_1_ed.clone()],
                white: vec![w_warn_0_ed.clone(), b_warn_1_ed.clone()],
            }
        );

        // Test applying changes with a warning that has no player number
        warn_edit.start_session().unwrap();

        warn_edit
            .items
            .black
            .push(EditableItem::New((), None, Infraction::IllegalSubstitution));
        warn_edit.items.white.push(EditableItem::New(
            (),
            None,
            Infraction::IllegallyStoppingThePuck,
        ));

        now += Duration::from_secs(20);
        warn_edit.apply_changes(now).unwrap();
        assert_eq!(
            tm.lock().unwrap().get_warnings(),
            &BlackWhiteBundle {
                black: vec![
                    b_warn_0_ed.clone(),
                    w_warn_1_ed.clone(),
                    InfractionDetails {
                        player_number: None,
                        start_period: GamePeriod::FirstHalf,
                        start_time: Duration::from_secs(855),
                        start_instant: now,
                        infraction: Infraction::IllegalSubstitution,
                    }
                ],
                white: vec![
                    w_warn_0_ed,
                    b_warn_1_ed,
                    InfractionDetails {
                        player_number: None,
                        start_period: GamePeriod::FirstHalf,
                        start_time: Duration::from_secs(855),
                        start_instant: now,
                        infraction: Infraction::IllegallyStoppingThePuck,
                    }
                ],
            }
        );
    }

    #[test]
    fn test_add_foul() {
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let now = Instant::now();
        let apply_time = now + Duration::from_secs(20);
        let mut tm = TournamentManager::new(config);
        tm.start_play_now(now).unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut foul_edit = ListEditor::<InfractionDetails, Option<Color>>::new(tm.clone());

        let b_foul = InfractionDetails {
            player_number: Some(3),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(880),
            start_instant: apply_time,
            infraction: Infraction::Unknown,
        };

        let e_foul = InfractionDetails {
            player_number: None,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(880),
            start_instant: apply_time,
            infraction: Infraction::IllegalSubstitution,
        };

        let w_foul = InfractionDetails {
            player_number: Some(13),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(880),
            start_instant: apply_time,
            infraction: Infraction::DelayOfGame,
        };

        assert_eq!(
            foul_edit.add_item(Some(Color::Black), Some(4), (), Infraction::Unknown),
            Err(PenaltyEditorError::NotInSession)
        );

        foul_edit.start_session().unwrap();
        foul_edit
            .add_item(
                Some(Color::Black),
                b_foul.player_number,
                (),
                b_foul.infraction,
            )
            .unwrap();

        assert_eq!(
            foul_edit.get_item(Some(Color::Black), 0),
            Ok(FoulSummary {
                player_number: b_foul.player_number,
                color: Some(Color::Black),
            })
        );

        foul_edit
            .add_item(None, e_foul.player_number, (), e_foul.infraction)
            .unwrap();

        assert_eq!(
            foul_edit.get_item(None, 0),
            Ok(FoulSummary {
                player_number: e_foul.player_number,
                color: None,
            })
        );

        foul_edit
            .add_item(
                Some(Color::White),
                w_foul.player_number,
                (),
                w_foul.infraction,
            )
            .unwrap();

        assert_eq!(
            foul_edit.get_item(Some(Color::White), 0),
            Ok(FoulSummary {
                player_number: w_foul.player_number,
                color: Some(Color::White),
            })
        );

        foul_edit.apply_changes(apply_time).unwrap();
        assert_eq!(
            tm.lock().unwrap().get_fouls(),
            &OptColorBundle {
                black: vec![b_foul],
                equal: vec![e_foul],
                white: vec![w_foul],
            }
        );
    }

    #[test]
    fn test_delete_foul() {
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let mut now = Instant::now();
        let mut tm = TournamentManager::new(config);
        tm.start_play_now(now).unwrap();

        now += Duration::from_secs(5);

        let b_foul_0 = InfractionDetails {
            player_number: Some(7),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::Unknown,
        };

        let e_foul_0 = InfractionDetails {
            player_number: None,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::IllegalSubstitution,
        };

        let w_foul_0 = InfractionDetails {
            player_number: Some(4),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::DelayOfGame,
        };

        let b_foul_1 = InfractionDetails {
            player_number: Some(13),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::FalseStart,
        };

        let e_foul_1 = InfractionDetails {
            player_number: None,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::IllegalAdvancement,
        };

        let w_foul_1 = InfractionDetails {
            player_number: Some(6),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::FreeArm,
        };

        tm.add_foul(
            Some(Color::Black),
            b_foul_0.player_number,
            b_foul_0.infraction,
            now,
        )
        .unwrap();
        tm.add_foul(None, e_foul_0.player_number, e_foul_0.infraction, now)
            .unwrap();
        tm.add_foul(
            Some(Color::White),
            w_foul_0.player_number,
            w_foul_0.infraction,
            now,
        )
        .unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut foul_edit = ListEditor::new(tm.clone());

        assert_eq!(
            foul_edit.delete_item(Some(Color::Black), 0),
            Err(PenaltyEditorError::NotInSession)
        );

        foul_edit.start_session().unwrap();

        assert_eq!(
            foul_edit.get_item(Some(Color::Black), 0),
            Ok(FoulSummary {
                player_number: b_foul_0.player_number,
                color: Some(Color::Black),
            })
        );
        assert_eq!(
            foul_edit.get_item(None, 0),
            Ok(FoulSummary {
                player_number: e_foul_0.player_number,
                color: None,
            })
        );
        assert_eq!(
            foul_edit.get_item(Some(Color::White), 0),
            Ok(FoulSummary {
                player_number: w_foul_0.player_number,
                color: Some(Color::White),
            })
        );
        assert_eq!(
            foul_edit.items.black,
            vec![EditableItem::Original(b_o_origin(0), b_foul_0.clone())]
        );
        assert_eq!(
            foul_edit.items.equal,
            vec![EditableItem::Original(e_o_origin(0), e_foul_0.clone())]
        );
        assert_eq!(
            foul_edit.items.white,
            vec![EditableItem::Original(w_o_origin(0), w_foul_0.clone())]
        );

        foul_edit.delete_item(Some(Color::Black), 0).unwrap();
        foul_edit.delete_item(None, 0).unwrap();
        foul_edit.delete_item(Some(Color::White), 0).unwrap();

        assert_eq!(
            foul_edit.get_item(Some(Color::Black), 0),
            Ok(FoulSummary {
                player_number: b_foul_0.player_number,
                color: Some(Color::Black),
            })
        );
        assert_eq!(
            foul_edit.get_item(None, 0),
            Ok(FoulSummary {
                player_number: e_foul_0.player_number,
                color: None,
            })
        );
        assert_eq!(
            foul_edit.get_item(Some(Color::White), 0),
            Ok(FoulSummary {
                player_number: w_foul_0.player_number,
                color: Some(Color::White),
            })
        );
        assert_eq!(
            foul_edit.items.black,
            vec![EditableItem::Deleted(b_o_origin(0), b_foul_0.clone())]
        );
        assert_eq!(
            foul_edit.items.equal,
            vec![EditableItem::Deleted(e_o_origin(0), e_foul_0.clone())]
        );
        assert_eq!(
            foul_edit.items.white,
            vec![EditableItem::Deleted(w_o_origin(0), w_foul_0.clone())]
        );

        // The original ones should be re-deletable without any changes
        foul_edit.delete_item(Some(Color::Black), 0).unwrap();
        foul_edit.delete_item(None, 0).unwrap();
        foul_edit.delete_item(Some(Color::White), 0).unwrap();

        assert_eq!(
            foul_edit.items.black,
            vec![EditableItem::Deleted(b_o_origin(0), b_foul_0.clone())]
        );
        assert_eq!(
            foul_edit.items.equal,
            vec![EditableItem::Deleted(e_o_origin(0), e_foul_0.clone())]
        );
        assert_eq!(
            foul_edit.items.white,
            vec![EditableItem::Deleted(w_o_origin(0), w_foul_0.clone())]
        );

        foul_edit
            .items
            .black
            .push(EditableItem::Edited(b_o_origin(1), b_foul_1.clone()));
        foul_edit
            .items
            .black
            .push(EditableItem::New((), Some(9), Infraction::Unknown));
        foul_edit
            .items
            .equal
            .push(EditableItem::Edited(e_o_origin(1), e_foul_1.clone()));
        foul_edit
            .items
            .equal
            .push(EditableItem::New((), Some(3), Infraction::Unknown));
        foul_edit
            .items
            .white
            .push(EditableItem::Edited(w_o_origin(1), w_foul_1.clone()));
        foul_edit
            .items
            .white
            .push(EditableItem::New((), Some(5), Infraction::Unknown));

        foul_edit.delete_item(Some(Color::Black), 1).unwrap();
        foul_edit.delete_item(Some(Color::Black), 2).unwrap();
        foul_edit.delete_item(None, 1).unwrap();
        foul_edit.delete_item(None, 2).unwrap();
        foul_edit.delete_item(Some(Color::White), 1).unwrap();
        foul_edit.delete_item(Some(Color::White), 2).unwrap();

        assert_eq!(
            foul_edit.items.black,
            vec![
                EditableItem::Deleted(b_o_origin(0), b_foul_0.clone()),
                EditableItem::Deleted(b_o_origin(1), b_foul_1.clone())
            ]
        );
        assert_eq!(
            foul_edit.items.equal,
            vec![
                EditableItem::Deleted(e_o_origin(0), e_foul_0.clone()),
                EditableItem::Deleted(e_o_origin(1), e_foul_1.clone())
            ]
        );
        assert_eq!(
            foul_edit.items.white,
            vec![
                EditableItem::Deleted(w_o_origin(0), w_foul_0.clone()),
                EditableItem::Deleted(w_o_origin(1), w_foul_1.clone())
            ]
        );
        assert_eq!(
            foul_edit.delete_item(Some(Color::Black), 2),
            Err(PenaltyEditorError::InvalidIndex(
                "Some(Black)".to_string(),
                2
            ))
        );
        assert_eq!(
            foul_edit.delete_item(None, 2),
            Err(PenaltyEditorError::InvalidIndex("None".to_string(), 2))
        );
        assert_eq!(
            foul_edit.delete_item(Some(Color::White), 2),
            Err(PenaltyEditorError::InvalidIndex(
                "Some(White)".to_string(),
                2
            ))
        );

        foul_edit.items.black.remove(1);
        foul_edit.items.equal.remove(1);
        foul_edit.items.white.remove(1);

        now += Duration::from_secs(20);
        foul_edit.apply_changes(now).unwrap();

        assert_eq!(
            tm.lock().unwrap().get_fouls(),
            &OptColorBundle {
                black: vec![],
                equal: vec![],
                white: vec![],
            }
        );
    }

    #[test]
    fn test_edit_foul() {
        let config = GameConfig {
            half_play_duration: Duration::from_secs(900),
            ..Default::default()
        };

        let mut now = Instant::now();
        let mut tm = TournamentManager::new(config);
        tm.start_play_now(now).unwrap();

        now += Duration::from_secs(5);

        let b_foul_0 = InfractionDetails {
            player_number: Some(7),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::Unknown,
        };

        let b_foul_0_ed = InfractionDetails {
            player_number: Some(2),
            ..b_foul_0
        };

        let e_foul_0 = InfractionDetails {
            player_number: None,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::IllegalSubstitution,
        };

        let e_foul_0_ed = InfractionDetails {
            player_number: None,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::DelayOfGame,
        };

        let w_foul_0 = InfractionDetails {
            player_number: Some(4),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::DelayOfGame,
        };

        let w_foul_0_ed = InfractionDetails {
            player_number: Some(3),
            ..w_foul_0
        };

        let b_foul_1 = InfractionDetails {
            player_number: Some(13),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::FalseStart,
        };

        let b_foul_1_ed = InfractionDetails {
            player_number: Some(5),
            ..b_foul_1
        };

        let e_foul_1 = InfractionDetails {
            player_number: None,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::IllegalAdvancement,
        };

        let e_foul_1_ed = InfractionDetails {
            player_number: None,
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::FreeArm,
        };

        let w_foul_1 = InfractionDetails {
            player_number: Some(6),
            start_period: GamePeriod::FirstHalf,
            start_time: Duration::from_secs(895),
            start_instant: now,
            infraction: Infraction::FreeArm,
        };

        let w_foul_1_ed = InfractionDetails {
            player_number: Some(9),
            ..w_foul_1
        };

        tm.add_foul(
            Some(Color::Black),
            b_foul_0.player_number,
            b_foul_0.infraction,
            now,
        )
        .unwrap();
        tm.add_foul(None, e_foul_0.player_number, e_foul_0.infraction, now)
            .unwrap();
        tm.add_foul(
            Some(Color::White),
            w_foul_0.player_number,
            w_foul_0.infraction,
            now,
        )
        .unwrap();
        tm.add_foul(
            Some(Color::Black),
            b_foul_1.player_number,
            b_foul_1.infraction,
            now,
        )
        .unwrap();
        tm.add_foul(None, e_foul_1.player_number, e_foul_1.infraction, now)
            .unwrap();
        tm.add_foul(
            Some(Color::White),
            w_foul_1.player_number,
            w_foul_1.infraction,
            now,
        )
        .unwrap();

        let tm = Arc::new(Mutex::new(tm));
        let mut foul_edit = ListEditor::new(tm.clone());

        assert_eq!(
            foul_edit.edit_item(
                Some(Color::Black),
                0,
                Some(Color::Black),
                Some(2),
                (),
                Infraction::Unknown
            ),
            Err(PenaltyEditorError::NotInSession)
        );

        foul_edit.start_session().unwrap();

        // Edit Original without color change
        foul_edit
            .edit_item(
                Some(Color::Black),
                0,
                Some(Color::Black),
                b_foul_0_ed.player_number,
                (),
                b_foul_0_ed.infraction,
            )
            .unwrap();
        foul_edit
            .edit_item(
                None,
                0,
                None,
                e_foul_0_ed.player_number,
                (),
                e_foul_0_ed.infraction,
            )
            .unwrap();
        foul_edit
            .edit_item(
                Some(Color::White),
                0,
                Some(Color::White),
                w_foul_0_ed.player_number,
                (),
                w_foul_0_ed.infraction,
            )
            .unwrap();

        // Edit Original with color change
        foul_edit
            .edit_item(
                Some(Color::Black),
                1,
                None,
                b_foul_1_ed.player_number,
                (),
                b_foul_1_ed.infraction,
            )
            .unwrap();
        foul_edit
            .edit_item(
                None,
                1,
                Some(Color::White),
                e_foul_1_ed.player_number,
                (),
                e_foul_1_ed.infraction,
            )
            .unwrap();
        foul_edit
            .edit_item(
                Some(Color::White),
                1,
                Some(Color::Black),
                w_foul_1_ed.player_number,
                (),
                w_foul_1_ed.infraction,
            )
            .unwrap();

        assert_eq!(
            foul_edit.items.black,
            vec![
                EditableItem::Edited(b_o_origin(0), b_foul_0_ed.clone()),
                EditableItem::Edited(w_o_origin(1), w_foul_1_ed.clone())
            ]
        );
        assert_eq!(
            foul_edit.items.equal,
            vec![
                EditableItem::Edited(e_o_origin(0), e_foul_0_ed.clone()),
                EditableItem::Edited(b_o_origin(1), b_foul_1_ed.clone())
            ]
        );
        assert_eq!(
            foul_edit.items.white,
            vec![
                EditableItem::Edited(w_o_origin(0), w_foul_0_ed.clone()),
                EditableItem::Edited(e_o_origin(1), e_foul_1_ed.clone())
            ]
        );

        // Edit Edited
        foul_edit
            .edit_item(
                Some(Color::Black),
                1,
                Some(Color::Black),
                w_foul_1_ed.player_number,
                (),
                w_foul_1_ed.infraction,
            )
            .unwrap();
        foul_edit
            .edit_item(
                None,
                1,
                None,
                b_foul_1_ed.player_number,
                (),
                b_foul_1_ed.infraction,
            )
            .unwrap();
        foul_edit
            .edit_item(
                Some(Color::White),
                1,
                Some(Color::White),
                e_foul_1_ed.player_number,
                (),
                e_foul_1_ed.infraction,
            )
            .unwrap();

        assert_eq!(
            foul_edit.items.black,
            vec![
                EditableItem::Edited(b_o_origin(0), b_foul_0_ed.clone()),
                EditableItem::Edited(w_o_origin(1), w_foul_1_ed.clone())
            ]
        );
        assert_eq!(
            foul_edit.items.equal,
            vec![
                EditableItem::Edited(e_o_origin(0), e_foul_0_ed.clone()),
                EditableItem::Edited(b_o_origin(1), b_foul_1_ed.clone())
            ]
        );
        assert_eq!(
            foul_edit.items.white,
            vec![
                EditableItem::Edited(w_o_origin(0), w_foul_0_ed.clone()),
                EditableItem::Edited(e_o_origin(1), e_foul_1_ed.clone())
            ]
        );

        // Edit Deleted and New
        foul_edit
            .items
            .black
            .push(EditableItem::Deleted(b_o_origin(2), b_foul_0));
        foul_edit
            .items
            .equal
            .push(EditableItem::Deleted(e_o_origin(2), e_foul_0));
        foul_edit
            .items
            .white
            .push(EditableItem::Deleted(w_o_origin(2), w_foul_0));

        foul_edit
            .items
            .black
            .push(EditableItem::New((), Some(15), Infraction::Unknown));
        foul_edit
            .items
            .equal
            .push(EditableItem::New((), None, Infraction::IllegalSubstitution));
        foul_edit
            .items
            .white
            .push(EditableItem::New((), Some(3), Infraction::Unknown));

        foul_edit
            .edit_item(
                Some(Color::Black),
                2,
                Some(Color::Black),
                b_foul_0_ed.player_number,
                (),
                b_foul_0_ed.infraction,
            )
            .unwrap();
        foul_edit
            .edit_item(
                None,
                2,
                None,
                e_foul_0_ed.player_number,
                (),
                e_foul_0_ed.infraction,
            )
            .unwrap();
        foul_edit
            .edit_item(
                Some(Color::White),
                2,
                Some(Color::White),
                w_foul_0_ed.player_number,
                (),
                w_foul_0_ed.infraction,
            )
            .unwrap();

        foul_edit
            .edit_item(
                Some(Color::Black),
                3,
                Some(Color::Black),
                Some(14),
                (),
                Infraction::IllegalSubstitution,
            )
            .unwrap();
        foul_edit
            .edit_item(
                None,
                3,
                None,
                None,
                (),
                Infraction::IllegallyStoppingThePuck,
            )
            .unwrap();
        foul_edit
            .edit_item(
                Some(Color::White),
                3,
                Some(Color::White),
                Some(3),
                (),
                Infraction::IllegallyStoppingThePuck,
            )
            .unwrap();

        assert_eq!(
            foul_edit.items.black,
            vec![
                EditableItem::Edited(b_o_origin(0), b_foul_0_ed.clone()),
                EditableItem::Edited(w_o_origin(1), w_foul_1_ed.clone()),
                EditableItem::Edited(b_o_origin(2), b_foul_0_ed.clone()),
                EditableItem::New((), Some(14), Infraction::IllegalSubstitution)
            ]
        );
        assert_eq!(
            foul_edit.items.equal,
            vec![
                EditableItem::Edited(e_o_origin(0), e_foul_0_ed.clone()),
                EditableItem::Edited(b_o_origin(1), b_foul_1_ed.clone()),
                EditableItem::Edited(e_o_origin(2), e_foul_0_ed.clone()),
                EditableItem::New((), None, Infraction::IllegallyStoppingThePuck)
            ]
        );
        assert_eq!(
            foul_edit.items.white,
            vec![
                EditableItem::Edited(w_o_origin(0), w_foul_0_ed.clone()),
                EditableItem::Edited(e_o_origin(1), e_foul_1_ed.clone()),
                EditableItem::Edited(w_o_origin(2), w_foul_0_ed.clone()),
                EditableItem::New((), Some(3), Infraction::IllegallyStoppingThePuck)
            ]
        );

        // Test applying changes
        foul_edit.items.black.remove(3);
        foul_edit.items.black.remove(2);
        foul_edit.items.equal.remove(3);
        foul_edit.items.equal.remove(2);
        foul_edit.items.white.remove(3);
        foul_edit.items.white.remove(2);

        now += Duration::from_secs(20);
        foul_edit.apply_changes(now).unwrap();

        assert_eq!(
            tm.lock().unwrap().get_fouls(),
            &OptColorBundle {
                black: vec![b_foul_0_ed.clone(), w_foul_1_ed.clone()],
                equal: vec![e_foul_0_ed.clone(), b_foul_1_ed.clone()],
                white: vec![w_foul_0_ed.clone(), e_foul_1_ed.clone()],
            }
        );

        // Test applying changes with a foul that has no player number
        foul_edit.start_session().unwrap();

        foul_edit
            .items
            .black
            .push(EditableItem::New((), None, Infraction::IllegalSubstitution));
        foul_edit
            .items
            .equal
            .push(EditableItem::New((), None, Infraction::IllegalSubstitution));
        foul_edit
            .items
            .white
            .push(EditableItem::New((), None, Infraction::IllegalSubstitution));

        now += Duration::from_secs(20);
        foul_edit.apply_changes(now).unwrap();

        assert_eq!(
            tm.lock().unwrap().get_fouls(),
            &OptColorBundle {
                black: vec![
                    b_foul_0_ed.clone(),
                    w_foul_1_ed.clone(),
                    InfractionDetails {
                        player_number: None,
                        start_period: GamePeriod::FirstHalf,
                        start_time: Duration::from_secs(855),
                        start_instant: now,
                        infraction: Infraction::IllegalSubstitution,
                    }
                ],
                equal: vec![
                    e_foul_0_ed.clone(),
                    b_foul_1_ed.clone(),
                    InfractionDetails {
                        player_number: None,
                        start_period: GamePeriod::FirstHalf,
                        start_time: Duration::from_secs(855),
                        start_instant: now,
                        infraction: Infraction::IllegalSubstitution,
                    }
                ],
                white: vec![
                    w_foul_0_ed.clone(),
                    e_foul_1_ed.clone(),
                    InfractionDetails {
                        player_number: None,
                        start_period: GamePeriod::FirstHalf,
                        start_time: Duration::from_secs(855),
                        start_instant: now,
                        infraction: Infraction::IllegalSubstitution,
                    }
                ],
            }
        );
    }
}
