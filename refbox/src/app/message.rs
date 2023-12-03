use crate::tournament_manager::PenaltyKind;
use tokio::time::Duration;
use uwh_common::{
    game_snapshot::{Color as GameColor, GameSnapshot},
    uwhscores::{GameInfo, TournamentInfo},
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
    Scroll {
        which: ScrollOption,
        up: bool,
    },
    PenaltyOverviewComplete {
        canceled: bool,
    },
    ChangeKind(PenaltyKind),
    PenaltyEditComplete {
        canceled: bool,
        deleted: bool,
    },
    KeypadPage(KeypadPage),
    KeypadButtonPress(KeypadButton),
    ChangeColor(GameColor),
    AddScoreComplete {
        canceled: bool,
    },
    ShowGameDetails,
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
    ParameterSelected(ListableParameter, usize),
    ToggleBoolParameter(BoolGameParameter),
    CycleParameter(CyclingParameter),
    RequestRemoteId,
    GotRemoteId(u32),
    DeleteRemote(usize),
    ConfirmationSelected(ConfirmationOption),
    BlackTimeout(bool),
    WhiteTimeout(bool),
    RefTimeout(bool),
    PenaltyShot(bool),
    EndTimeout,
    ConfirmScores(GameSnapshot),
    ScoreConfirmation {
        correct: bool,
    },
    RecvTournamentList(Vec<TournamentInfo>),
    RecvTournament(TournamentInfo),
    RecvGameList(Vec<GameInfo>),
    RecvGame(GameInfo),
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
            | Self::RecvTournamentList(_)
            | Self::RecvTournament(_)
            | Self::RecvGameList(_)
            | Self::RecvGame(_)
            | Self::NoAction => true,

            Self::Init
            | Self::EditTime
            | Self::TimeEditComplete { .. }
            | Self::StartPlayNow
            | Self::EditScores
            | Self::AddNewScore(_)
            | Self::ScoreEditComplete { .. }
            | Self::PenaltyOverview
            | Self::PenaltyOverviewComplete { .. }
            | Self::ChangeKind(_)
            | Self::PenaltyEditComplete { .. }
            | Self::KeypadPage(_)
            | Self::ChangeColor(_)
            | Self::AddScoreComplete { .. }
            | Self::ShowGameDetails
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
            | Self::BlackTimeout(_)
            | Self::WhiteTimeout(_)
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
    Tournament,
    Sound,
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
    Tournament,
    Pool,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolGameParameter {
    OvertimeAllowed,
    SuddenDeathAllowed,
    WhiteOnRight,
    UsingUwhScores,
    SoundEnabled,
    RefAlertEnabled,
    AutoSoundStartPlay,
    AutoSoundStopPlay,
    HideTime,
    ScorerCapNum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CyclingParameter {
    BuzzerSound,
    RemoteBuzzerSound(usize),
    AlertVolume,
    AboveWaterVol,
    UnderWaterVol,
    Mode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollOption {
    Black,
    White,
    GameParameter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeypadPage {
    AddScore(GameColor),
    Penalty(Option<(GameColor, usize)>, GameColor, PenaltyKind),
    GameNumber,
    TeamTimeouts(Duration),
}

impl KeypadPage {
    pub fn max_val(&self) -> u16 {
        match self {
            Self::AddScore(_) | Self::Penalty(_, _, _) => 99,
            Self::GameNumber => 9999,
            Self::TeamTimeouts(_) => 999,
        }
    }

    pub fn text(&self) -> &'static str {
        match self {
            Self::AddScore(_) | Self::Penalty(_, _, _) => "PLAYER\nNUMBER:",
            Self::GameNumber => "GAME\nNUMBER:",
            Self::TeamTimeouts(_) => "NUM T/Os\nPER HALF:",
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
