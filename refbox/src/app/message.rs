use super::fl;
use crate::tournament_manager::penalty::PenaltyKind;
use tokio::time::Duration;
use uwh_common::{
    color::Color as GameColor,
    game_snapshot::{GameSnapshot, Infraction},
    uwhportal::schedule::{Event, EventId, Schedule, TeamList},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Init,
    NewSnapshot(GameSnapshot),
    EditTime,
    ChangeTime {
        increase: bool,
        secs: u64,
        timeout: bool,
    },
    TimeEditComplete {
        canceled: bool,
    },
    StartPlayNow,
    EditScores,
    AddNewScore(GameColor),
    ChangeScore {
        color: GameColor,
        increase: bool,
    },
    ScoreEditComplete {
        canceled: bool,
    },
    PenaltyOverview,
    WarningOverview,
    FoulOverview,
    Scroll {
        which: ScrollOption,
        up: bool,
    },
    PenaltyOverviewComplete {
        canceled: bool,
    },
    WarningOverviewComplete {
        canceled: bool,
    },
    FoulOverviewComplete {
        canceled: bool,
    },
    ChangeKind(PenaltyKind),
    ChangeInfraction(Infraction),
    PenaltyEditComplete {
        canceled: bool,
        deleted: bool,
    },
    WarningEditComplete {
        canceled: bool,
        deleted: bool,
        ret_to_overview: bool,
    },
    FoulEditComplete {
        canceled: bool,
        deleted: bool,
        ret_to_overview: bool,
    },
    KeypadPage(KeypadPage),
    KeypadButtonPress(KeypadButton),
    ChangeColor(Option<GameColor>),
    AddScoreComplete {
        canceled: bool,
    },
    ShowGameDetails,
    ShowWarnings,
    EditGameConfig,
    ChangeConfigPage(ConfigPage),
    ConfigEditComplete {
        canceled: bool,
    },
    EditParameter(LengthParameter),
    SelectParameter(ListableParameter),
    ParameterEditComplete {
        canceled: bool,
    },
    ParameterSelected(ListableParameter, String),
    ToggleBoolParameter(BoolGameParameter),
    CycleParameter(CyclingParameter),
    TextParameterChanged(TextParameter, String),
    ApplyAuthChanges,
    RequestRemoteId,
    GotRemoteId(u32),
    DeleteRemote(usize),
    ConfirmationSelected(ConfirmationOption),
    TeamTimeout(GameColor, bool),
    RefTimeout(bool),
    PenaltyShot(bool),
    EndTimeout,
    ConfirmScores(GameSnapshot),
    ScoreConfirmation {
        correct: bool,
    },
    RecvEventList(Vec<Event>),
    RecvTeamsList(EventId, TeamList),
    RecvSchedule(EventId, Schedule),
    StopClock,
    StartClock,
    NoAction, // TODO: Remove once UI is functional
}

impl Message {
    pub fn is_repeatable(&self) -> bool {
        match self {
            Self::NewSnapshot(_)
            | Self::ChangeTime { .. }
            | Self::ChangeScore { .. }
            | Self::Scroll { .. }
            | Self::KeypadButtonPress(_)
            | Self::ToggleBoolParameter(_)
            | Self::CycleParameter(_)
            | Self::RecvEventList(_)
            | Self::RecvTeamsList(_, _)
            | Self::RecvSchedule(_, _)
            | Self::NoAction => true,

            Self::Init
            | Self::EditTime
            | Self::TimeEditComplete { .. }
            | Self::StartPlayNow
            | Self::EditScores
            | Self::AddNewScore(_)
            | Self::ScoreEditComplete { .. }
            | Self::PenaltyOverview
            | Self::WarningOverview
            | Self::FoulOverview
            | Self::PenaltyOverviewComplete { .. }
            | Self::WarningOverviewComplete { .. }
            | Self::FoulOverviewComplete { .. }
            | Self::ChangeKind(_)
            | Self::ChangeInfraction(_)
            | Self::PenaltyEditComplete { .. }
            | Self::WarningEditComplete { .. }
            | Self::FoulEditComplete { .. }
            | Self::KeypadPage(_)
            | Self::ChangeColor(_)
            | Self::AddScoreComplete { .. }
            | Self::ShowGameDetails
            | Self::ShowWarnings
            | Self::EditGameConfig
            | Self::ChangeConfigPage(_)
            | Self::ConfigEditComplete { .. }
            | Self::EditParameter(_)
            | Self::SelectParameter(_)
            | Self::ParameterEditComplete { .. }
            | Self::ParameterSelected(_, _)
            | Self::TextParameterChanged(_, _)
            | Self::ApplyAuthChanges
            | Self::RequestRemoteId
            | Self::GotRemoteId(_)
            | Self::DeleteRemote(_)
            | Self::ConfirmationSelected(_)
            | Self::TeamTimeout(_, _)
            | Self::RefTimeout(_)
            | Self::PenaltyShot(_)
            | Self::EndTimeout
            | Self::ConfirmScores(_)
            | Self::ScoreConfirmation { .. }
            | Self::StopClock
            | Self::StartClock => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigPage {
    Main,
    Game,
    Sound,
    Display,
    App,
    Credentials,
    Remotes(usize, bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthParameter {
    Half,
    HalfTime,
    NominalBetweenGame,
    MinimumBetweenGame,
    PreOvertime,
    OvertimeHalf,
    OvertimeHalfTime,
    PreSuddenDeath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListableParameter {
    Event,
    Court,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolGameParameter {
    OvertimeAllowed,
    SuddenDeathAllowed,
    SingleHalf,
    WhiteOnRight,
    UsingUwhPortal,
    SoundEnabled,
    RefAlertEnabled,
    AutoSoundStartPlay,
    AutoSoundStopPlay,
    HideTime,
    ScorerCapNum,
    FoulsAndWarnings,
    TeamWarning,
    TimeoutsCountedPerHalf,
    ConfirmScore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CyclingParameter {
    BuzzerSound,
    RemoteBuzzerSound(usize),
    AlertVolume,
    AboveWaterVol,
    UnderWaterVol,
    Mode,
    Brightness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextParameter {
    UwhportalToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollOption {
    Black,
    White,
    Equal,
    GameParameter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeypadPage {
    AddScore(GameColor),
    Penalty(
        Option<(GameColor, usize)>,
        GameColor,
        PenaltyKind,
        Infraction,
    ),
    GameNumber,
    TeamTimeouts(Duration, bool),
    FoulAdd {
        origin: Option<(Option<GameColor>, usize)>,
        color: Option<GameColor>,
        infraction: Infraction,
        ret_to_overview: bool,
    },
    WarningAdd {
        origin: Option<(GameColor, usize)>,
        color: GameColor,
        infraction: Infraction,
        team_warning: bool,
        ret_to_overview: bool,
    },
}

impl KeypadPage {
    pub fn max_val(&self) -> u16 {
        match self {
            Self::AddScore(_)
            | Self::Penalty(_, _, _, _)
            | Self::FoulAdd { .. }
            | Self::WarningAdd { .. } => 99,
            Self::TeamTimeouts(_, _) => 999,
            Self::GameNumber => 9999,
        }
    }

    pub fn text(&self) -> String {
        match self {
            Self::AddScore(_)
            | Self::Penalty(_, _, _, _)
            | Self::FoulAdd { .. }
            | Self::WarningAdd { .. } => fl!("player-number"),
            Self::GameNumber => fl!("game-number"),
            Self::TeamTimeouts(_, true) => fl!("num-tos-per-half"),
            Self::TeamTimeouts(_, false) => fl!("num-tos-per-game"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeypadButton {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationOption {
    DiscardChanges,
    GoBack,
    EndGameAndApply,
    KeepGameAndApply,
}
