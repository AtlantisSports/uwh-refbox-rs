use super::fl;
use crate::{
    sound_controller::RemoteId,
    tournament_manager::{TournamentManager, penalty::PenaltyKind},
};
use std::sync::{Arc, Mutex};
use tokio::{sync::mpsc::Sender, time::Duration};
use uwh_common::{
    color::Color as GameColor,
    game_snapshot::{GameSnapshot, Infraction},
    uwhportal::{
        PortalTokenResponse,
        schedule::{Event, EventId, Schedule, TeamList},
    },
};

#[derive(Debug, Clone)]
pub enum Message {
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
    RequestPortalRefresh,
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
    RequestRemoteId,
    GotRemoteId(RemoteId),
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
    AutoConfirmScores(GameSnapshot),
    RecvEventList(Vec<Event>),
    RecvTeamsList(EventId, TeamList),
    RecvSchedule(EventId, Schedule),
    RecvPortalToken(PortalTokenResponse),
    RecvTokenValid(bool),
    StopClock,
    StartClock,
    TimeUpdaterStarted(Sender<Arc<Mutex<TournamentManager>>>),
    NoAction,
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
            | Self::RecvPortalToken(_)
            | Self::RecvTokenValid(_)
            | Self::TimeUpdaterStarted(_)
            | Self::NoAction => true,

            Self::EditTime
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
            | Self::RequestPortalRefresh
            | Self::ShowWarnings
            | Self::EditGameConfig
            | Self::ChangeConfigPage(_)
            | Self::ConfigEditComplete { .. }
            | Self::EditParameter(_)
            | Self::SelectParameter(_)
            | Self::ParameterEditComplete { .. }
            | Self::ParameterSelected(_, _)
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
            | Self::AutoConfirmScores(_)
            | Self::StopClock
            | Self::StartClock => false,
        }
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::EditTime, Self::EditTime)
            | (Self::StartPlayNow, Self::StartPlayNow)
            | (Self::EditScores, Self::EditScores)
            | (Self::PenaltyOverview, Self::PenaltyOverview)
            | (Self::WarningOverview, Self::WarningOverview)
            | (Self::FoulOverview, Self::FoulOverview)
            | (Self::ShowGameDetails, Self::ShowGameDetails)
            | (Self::RequestPortalRefresh, Self::RequestPortalRefresh)
            | (Self::ShowWarnings, Self::ShowWarnings)
            | (Self::EditGameConfig, Self::EditGameConfig)
            | (Self::RequestRemoteId, Self::RequestRemoteId)
            | (Self::EndTimeout, Self::EndTimeout)
            | (Self::StopClock, Self::StopClock)
            | (Self::StartClock, Self::StartClock)
            | (Self::NoAction, Self::NoAction) => true,

            (Self::NewSnapshot(a), Self::NewSnapshot(b)) => a == b,
            (
                Self::ChangeTime {
                    increase: a,
                    secs: b,
                    timeout: c,
                },
                Self::ChangeTime {
                    increase: d,
                    secs: e,
                    timeout: f,
                },
            ) => a == d && b == e && c == f,
            (Self::TimeEditComplete { canceled: a }, Self::TimeEditComplete { canceled: b }) => {
                a == b
            }
            (
                Self::ChangeScore {
                    color: a,
                    increase: b,
                },
                Self::ChangeScore {
                    color: c,
                    increase: d,
                },
            ) => a == c && b == d,
            (Self::ScoreEditComplete { canceled: a }, Self::ScoreEditComplete { canceled: b }) => {
                a == b
            }
            (Self::Scroll { which: a, up: b }, Self::Scroll { which: c, up: d }) => {
                a == c && b == d
            }
            (
                Self::PenaltyOverviewComplete { canceled: a },
                Self::PenaltyOverviewComplete { canceled: b },
            ) => a == b,
            (
                Self::WarningOverviewComplete { canceled: a },
                Self::WarningOverviewComplete { canceled: b },
            ) => a == b,
            (
                Self::FoulOverviewComplete { canceled: a },
                Self::FoulOverviewComplete { canceled: b },
            ) => a == b,
            (Self::ChangeKind(a), Self::ChangeKind(b)) => a == b,
            (Self::ChangeInfraction(a), Self::ChangeInfraction(b)) => a == b,
            (
                Self::PenaltyEditComplete {
                    canceled: a,
                    deleted: b,
                },
                Self::PenaltyEditComplete {
                    canceled: c,
                    deleted: d,
                },
            ) => a == c && b == d,
            (
                Self::WarningEditComplete {
                    canceled: a,
                    deleted: b,
                    ret_to_overview: c,
                },
                Self::WarningEditComplete {
                    canceled: d,
                    deleted: e,
                    ret_to_overview: f,
                },
            ) => a == d && b == e && c == f,
            (
                Self::FoulEditComplete {
                    canceled: a,
                    deleted: b,
                    ret_to_overview: c,
                },
                Self::FoulEditComplete {
                    canceled: d,
                    deleted: e,
                    ret_to_overview: f,
                },
            ) => a == d && b == e && c == f,
            (Self::KeypadPage(a), Self::KeypadPage(b)) => a == b,
            (Self::KeypadButtonPress(a), Self::KeypadButtonPress(b)) => a == b,
            (Self::ChangeColor(a), Self::ChangeColor(b)) => a == b,
            (Self::AddScoreComplete { canceled: a }, Self::AddScoreComplete { canceled: b }) => {
                a == b
            }
            (Self::ConfirmScores(a), Self::ConfirmScores(b)) => a == b,
            (Self::ScoreConfirmation { correct: a }, Self::ScoreConfirmation { correct: b }) => {
                a == b
            }
            (Self::AutoConfirmScores(a), Self::AutoConfirmScores(b)) => a == b,
            (Self::EditParameter(a), Self::EditParameter(b)) => a == b,
            (Self::SelectParameter(a), Self::SelectParameter(b)) => a == b,
            (
                Self::ParameterEditComplete { canceled: a },
                Self::ParameterEditComplete { canceled: b },
            ) => a == b,
            (Self::ParameterSelected(a, b), Self::ParameterSelected(c, d)) => a == c && b == d,
            (Self::ToggleBoolParameter(a), Self::ToggleBoolParameter(b)) => a == b,
            (Self::CycleParameter(a), Self::CycleParameter(b)) => a == b,
            (Self::GotRemoteId(a), Self::GotRemoteId(b)) => a == b,
            (Self::DeleteRemote(a), Self::DeleteRemote(b)) => a == b,
            (Self::ConfirmationSelected(a), Self::ConfirmationSelected(b)) => a == b,
            (Self::TeamTimeout(a, b), Self::TeamTimeout(c, d)) => a == c && b == d,
            (Self::RefTimeout(a), Self::RefTimeout(b)) => a == b,
            (Self::PenaltyShot(a), Self::PenaltyShot(b)) => a == b,
            (Self::TimeUpdaterStarted(a), Self::TimeUpdaterStarted(b)) => a.same_channel(b),
            (Self::RecvEventList(a), Self::RecvEventList(b)) => a == b,
            (Self::RecvTeamsList(a, b), Self::RecvTeamsList(c, d)) => a == c && b == d,
            (Self::RecvSchedule(a, b), Self::RecvSchedule(c, d)) => a == c && b == d,
            (Self::RecvPortalToken(a), Self::RecvPortalToken(b)) => a == b,
            (Self::RecvTokenValid(a), Self::RecvTokenValid(b)) => a == b,

            (Self::NewSnapshot(_), _)
            | (Self::EditTime, _)
            | (Self::ChangeTime { .. }, _)
            | (Self::TimeEditComplete { .. }, _)
            | (Self::StartPlayNow, _)
            | (Self::EditScores, _)
            | (Self::AddNewScore(_), _)
            | (Self::ChangeScore { .. }, _)
            | (Self::ScoreEditComplete { .. }, _)
            | (Self::PenaltyOverview, _)
            | (Self::WarningOverview, _)
            | (Self::FoulOverview, _)
            | (Self::Scroll { .. }, _)
            | (Self::PenaltyOverviewComplete { .. }, _)
            | (Self::WarningOverviewComplete { .. }, _)
            | (Self::FoulOverviewComplete { .. }, _)
            | (Self::ChangeKind(_), _)
            | (Self::ChangeInfraction(_), _)
            | (Self::PenaltyEditComplete { .. }, _)
            | (Self::WarningEditComplete { .. }, _)
            | (Self::FoulEditComplete { .. }, _)
            | (Self::KeypadPage(_), _)
            | (Self::KeypadButtonPress(_), _)
            | (Self::ChangeColor(_), _)
            | (Self::AddScoreComplete { .. }, _)
            | (Self::ShowGameDetails, _)
            | (Self::RequestPortalRefresh, _)
            | (Self::ShowWarnings, _)
            | (Self::EditGameConfig, _)
            | (Self::ChangeConfigPage(_), _)
            | (Self::ConfigEditComplete { .. }, _)
            | (Self::EditParameter(_), _)
            | (Self::SelectParameter(_), _)
            | (Self::ParameterEditComplete { .. }, _)
            | (Self::ParameterSelected(_, _), _)
            | (Self::ToggleBoolParameter(_), _)
            | (Self::CycleParameter(_), _)
            | (Self::RequestRemoteId, _)
            | (Self::GotRemoteId(_), _)
            | (Self::DeleteRemote(_), _)
            | (Self::ConfirmationSelected(_), _)
            | (Self::TeamTimeout(_, _), _)
            | (Self::RefTimeout(_), _)
            | (Self::PenaltyShot(_), _)
            | (Self::EndTimeout, _)
            | (Self::ConfirmScores(_), _)
            | (Self::ScoreConfirmation { .. }, _)
            | (Self::AutoConfirmScores(_), _)
            | (Self::RecvEventList(_), _)
            | (Self::RecvTeamsList(_, _), _)
            | (Self::RecvSchedule(_, _), _)
            | (Self::RecvPortalToken(_), _)
            | (Self::RecvTokenValid(_), _)
            | (Self::StopClock, _)
            | (Self::StartClock, _)
            | (Self::TimeUpdaterStarted(_), _)
            | (Self::NoAction, _) => false,
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
    Language,
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
    PortalLogin(u32, bool),
}

impl KeypadPage {
    pub fn max_val(&self) -> u32 {
        match self {
            Self::AddScore(_)
            | Self::Penalty(_, _, _, _)
            | Self::FoulAdd { .. }
            | Self::WarningAdd { .. } => 99,
            Self::TeamTimeouts(_, _) => 999,
            Self::GameNumber => 9999,
            Self::PortalLogin(_, _) => 999_999,
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
            Self::PortalLogin(_, _) => fl!("portal-login-code"),
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
