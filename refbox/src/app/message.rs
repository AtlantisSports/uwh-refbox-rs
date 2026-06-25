use super::{fl, languages::Language};
use crate::{
    portal_manager::{ItemId, PortalEvent},
    sound_controller::{BuzzerSound, RemoteId},
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
    /// Spawns a new panel-simulator window (in addition to any already
    /// open). Triggered by the "Open New Display" button on the Display
    /// Options tab. See [`crate::spawn_sim_child`] for the spawn details.
    OpenNewDisplay,
    OpenPortalDetailPage,
    ClosePortalDetailPage,
    /// Emitted when the operator taps BACK on the portal attention
    /// action page. `update()` returns to the portal detail page (the
    /// list the operator came from), not all the way out to the main
    /// application screen.
    ClosePortalAttentionAction,
    PortalEvent(PortalEvent),
    PortalUiTick,
    /// Emitted when the operator taps a queued-item row on the detail
    /// page. `update()` routes stuck items to the attention action page
    /// and young pending items to a forced retry.
    PortalRowTapped(ItemId),
    /// Emitted when the operator taps FORCE THIS GAME RESULT on the
    /// attention action page. `update()` calls
    /// `PortalManager::force_submit` and returns to the detail page.
    PortalForceSubmit(ItemId),
    /// Emitted when the operator taps DISCARD THIS SUBMISSION on the
    /// attention action page. First tap arms the discard; second tap
    /// for the same item confirms and removes it from the queue.
    PortalDiscardTapped(ItemId),
    RequestPortalRefresh,
    /// Emitted when a portal schedule refresh triggered by
    /// `RequestPortalRefresh` ends without delivering a schedule (e.g. the
    /// fetch failed). Clears the Game Info REFRESH button's "Refreshing..."
    /// spinner; the success case is cleared in `RecvSchedule`.
    PortalRefreshFinished,
    /// Emitted when the operator taps RETRY ALL on the portal detail
    /// page. `update()` calls `PortalManager::retry_all`, which resets
    /// every queued game (including stuck ones) so the background task
    /// re-attempts them all on its next tick. A plain resend — never a
    /// force-overwrite.
    PortalRetryAll,
    ShowWarnings,
    ShowParameterHelp,
    CloseParameterHelp,
    EditGameConfig,
    /// Navigate directly to a specific `ConfigPage` inside the Game Options
    /// editor, bypassing the default `ConfigPage::Main` landing. Used by
    /// the game-info table tap-through (Task 7).
    EditGameConfigPage(ConfigPage),
    ChangeConfigPage(ConfigPage),
    ApplyConfigPage(ConfigPage),
    CancelConfigPage(ConfigPage),
    ConfigEditComplete,
    EditParameter(LengthParameter),
    SelectParameter(ListableParameter),
    ParameterEditComplete {
        canceled: bool,
    },
    ParameterSelected(ListableParameter, String),
    ToggleBoolParameter(BoolGameParameter),
    CycleParameter(CyclingParameter),
    /// Set the team-timeout count directly (team-timeout edit page 0/1 toggle).
    SetTeamTimeoutCount(u32),
    /// Set the team-timeout length to a preset value (team-timeout edit page).
    SetTeamTimeoutLength(Duration),
    /// Advance the on-screen display mode (Light → Dark → High Contrast → …).
    /// Commits immediately; not part of the DONE/Apply settings round-trip.
    CycleDisplayMode,
    SelectLanguage(Language),
    LanguageSelectComplete {
        canceled: bool,
    },
    /// Operator tapped a sound in the buzzer picker (stages the selection).
    SelectBuzzer(BuzzerSound),
    /// Operator pressed TEST in the buzzer picker — plays the staged sound.
    TestBuzzer,
    RequestRemoteId,
    // Constructed by the Sound-page button on non-Linux only; on Linux the Pi
    // has a fixed dedicated speaker so the button is compiled out. The variant
    // is kept ungated so the worker match and update() arm stay uniform.
    #[cfg_attr(target_os = "linux", allow(dead_code))]
    UpdateAudioOutput,
    GotRemoteId(RemoteId),
    DeleteRemote(usize),
    ConfirmationSelected(ConfirmationOption),
    TeamTimeout(GameColor, bool),
    RefTimeout(bool),
    PenaltyShot(bool),
    EndTimeout,
    CancelTimeout,
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
    /// Operator pressed Start in BeepTest mode.
    BeepTestStart,
    /// Operator pressed Stop in BeepTest mode.
    BeepTestStop,
    /// Operator pressed Reset in BeepTest mode.
    BeepTestReset,
    /// Tick from the timer subscription; advances the cadence engine.
    BeepTestTick,
    /// Operator pressed Settings on the BeepTest main view. Opens the
    /// BeepTest-specific Settings landing page.
    BeepTestOpenSettings,
    /// Operator pressed Back/Done on the BeepTest Settings landing.
    /// Returns to the BeepTest main view.
    BeepTestCloseSettings,
    /// Operator pressed Language on the BeepTest Settings landing. Seeds
    /// `edited_settings.pending_language` / `original_language` with the
    /// current language and navigates to the BeepTest-specific Language
    /// picker sub-page (no timeout ribbon, BeepTest layout).
    BeepTestEditOpenLanguage,
    /// Operator pressed Cancel on the BeepTest Language picker. Discards
    /// the staged language and returns to the BeepTest Settings landing.
    BeepTestLanguageCancel,
    /// Operator pressed Done/Restart on the BeepTest Language picker.
    /// Commits the staged language to disk; if the font family changed,
    /// the app restarts (mirroring `LanguageSelectComplete`'s restart
    /// branch). Otherwise applies the new language to the running UI and
    /// returns to the BeepTest Settings landing.
    BeepTestLanguageApply,
    /// Operator pressed Sound Settings on the BeepTest Settings landing.
    /// Seeds `edited_settings` with a clone of the current sound settings
    /// so the existing `ToggleBoolParameter` / `CycleParameter` handlers
    /// (which mutate `edited_settings`) can be reused, then navigates to
    /// the BeepTest Sound Settings sub-page.
    BeepTestEditOpenSound,
    /// Operator pressed Save on the BeepTest Sound Settings sub-page.
    /// Commits the staged sound edits to live config, persists to disk,
    /// and returns to the BeepTest Settings landing.
    BeepTestSoundSettingsSave,
    /// Operator pressed Cancel on the BeepTest Sound Settings sub-page.
    /// Discards staged sound edits and returns to the BeepTest Settings
    /// landing.
    BeepTestSoundSettingsCancel,
    /// Operator pressed Edit Levels on the BeepTest Settings landing.
    /// Seeds `edited_settings.beep_test_levels` with a clone of the
    /// current level list and `edited_settings.selected_level = 0`,
    /// then navigates to the Edit Levels sub-page.
    BeepTestEditOpenLevels,
    /// Operator tapped a column header or cell on the Edit Levels page.
    /// Sets `edited_settings.selected_level` to the tapped column's
    /// 0-based index.
    BeepTestEditSelectLevel(usize),
    /// Operator pressed `[+]` next to Count on the Edit Levels page.
    BeepTestEditCountInc,
    /// Operator pressed `[-]` next to Count on the Edit Levels page.
    BeepTestEditCountDec,
    /// Operator pressed `[+]` next to Time on the Edit Levels page.
    BeepTestEditDurationInc,
    /// Operator pressed `[-]` next to Time on the Edit Levels page.
    BeepTestEditDurationDec,
    /// Operator pressed `[+NEW]` on the Edit Levels page; appends a new
    /// level with default values and selects it.
    BeepTestEditAddLevel,
    /// Operator pressed `[REMOVE LEVEL]` on the Edit Levels page; removes
    /// the currently-selected level. No-op when only one level remains.
    BeepTestEditRemoveLevel,
    /// Operator pressed Save on the Edit Levels page. Commits the staged
    /// level list to live config, persists to disk, and returns to the
    /// BeepTest Settings landing.
    BeepTestEditLevelsSave,
    /// Operator pressed Cancel on the Edit Levels page. Discards staged
    /// level edits and returns to the BeepTest Settings landing.
    BeepTestEditLevelsCancel,
    /// Operator pressed the buzzer-sound tile on the BeepTest Sound Settings
    /// sub-page. Navigates to the full-page BeepTest Buzzer picker
    /// (`BeepTestConfigPage::Buzzer`). `edited_settings` is already seeded
    /// by `BeepTestEditOpenSound`; this handler does NOT re-seed it.
    BeepTestEditOpenBuzzer,
    /// Operator tapped a buzzer-sound cell on the BeepTest Buzzer picker.
    /// Stages the selected sound into `edited_settings.sound.buzzer_sound`.
    BeepTestSelectBuzzer(BuzzerSound),
    /// Operator pressed TEST on the BeepTest Buzzer picker. Plays the
    /// currently-staged buzzer sound without committing it.
    BeepTestTestBuzzer,
    /// Operator pressed Apply on the BeepTest Buzzer picker. Returns to the
    /// BeepTest Sound Settings sub-page with the staged sound kept; the
    /// existing `BeepTestSoundSettingsSave` path persists it.
    BeepTestBuzzerSave,
    /// Operator pressed Cancel on the BeepTest Buzzer picker. Reverts
    /// `edited_settings.sound.buzzer_sound` to the live value and returns
    /// to the BeepTest Sound Settings sub-page.
    BeepTestBuzzerCancel,
    /// Operator pressed RESTART TO APPLY on the BeepTest Settings landing
    /// after cycling the App Mode. Commits the staged mode. When the mode
    /// differs from the current mode, the app restarts (mirroring the
    /// existing mode-change exe-restart flow used by the hockey-mode
    /// PortalTenantSwitch confirmation).
    BeepTestRestartToApply,
    /// Cycle the in-memory BeepTest display layout (session-only, live-apply).
    BeepTestCycleDisplayLayout,
    AlarmPressed,
    AlarmReleased,
    SpacebarPressed,
    SpacebarReleased,
    AlarmDelayElapsed(u64),
    /// Press-down on a used-up (greyed) team timeout button — begins the
    /// 3-second hold-to-revive.
    TimeoutRevivePressed(GameColor),
    /// Release of a hold-to-revive press before it completed.
    TimeoutReviveReleased(GameColor),
    /// The 3-second revive hold elapsed for the given team (token guards stale timers).
    TimeoutReviveHoldElapsed(u64, GameColor),
    TimeUpdaterStarted(Sender<Arc<Mutex<TournamentManager>>>),
    OpenUpdatesPage,
    UpdatesCheck,
    UpdatesConfirmInstall,
    UpdatesRevert,
    UpdatesConfirmRevert,
    UpdatesCheckDone(Result<crate::updater::release::ReleaseInfo, UpdateUiError>),
    UpdatesDownloaded(Result<String, UpdateUiError>),
    UpdatesVerified(Result<(), UpdateUiError>),
    UpdatesInstalled(Result<(), UpdateUiError>),
    UpdatesBack,
    /// Fired roughly 20 seconds after startup. Receiving it proves the app
    /// launched healthily, so the update trial marker is cleared (the safety
    /// net no longer needs to auto-revert on the next boot).
    UpdaterHealthyCheck,
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
            | Self::PortalEvent(_)
            | Self::PortalUiTick
            | Self::PortalRefreshFinished
            | Self::BeepTestTick
            | Self::NoAction
            | Self::OpenNewDisplay => true,

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
            | Self::OpenPortalDetailPage
            | Self::ClosePortalDetailPage
            | Self::ClosePortalAttentionAction
            | Self::PortalRowTapped(_)
            | Self::PortalForceSubmit(_)
            | Self::PortalDiscardTapped(_)
            | Self::RequestPortalRefresh
            | Self::PortalRetryAll
            | Self::ShowWarnings
            | Self::ShowParameterHelp
            | Self::CloseParameterHelp
            | Self::EditGameConfig
            | Self::EditGameConfigPage(_)
            | Self::ChangeConfigPage(_)
            | Self::ApplyConfigPage(_)
            | Self::CancelConfigPage(_)
            | Self::ConfigEditComplete
            | Self::EditParameter(_)
            | Self::SelectParameter(_)
            | Self::ParameterEditComplete { .. }
            | Self::ParameterSelected(_, _)
            | Self::CycleDisplayMode
            | Self::SelectLanguage(_)
            | Self::LanguageSelectComplete { .. }
            | Self::SelectBuzzer(_)
            | Self::TestBuzzer
            | Self::RequestRemoteId
            | Self::UpdateAudioOutput
            | Self::GotRemoteId(_)
            | Self::DeleteRemote(_)
            | Self::ConfirmationSelected(_)
            | Self::TeamTimeout(_, _)
            | Self::RefTimeout(_)
            | Self::PenaltyShot(_)
            | Self::EndTimeout
            | Self::CancelTimeout
            | Self::ConfirmScores(_)
            | Self::ScoreConfirmation { .. }
            | Self::AutoConfirmScores(_)
            | Self::StopClock
            | Self::StartClock
            | Self::BeepTestStart
            | Self::BeepTestStop
            | Self::BeepTestReset
            | Self::BeepTestOpenSettings
            | Self::BeepTestCloseSettings
            | Self::BeepTestEditOpenLanguage
            | Self::BeepTestLanguageCancel
            | Self::BeepTestLanguageApply
            | Self::BeepTestEditOpenSound
            | Self::BeepTestSoundSettingsSave
            | Self::BeepTestSoundSettingsCancel
            | Self::BeepTestEditOpenLevels
            | Self::BeepTestEditSelectLevel(_)
            | Self::BeepTestEditCountInc
            | Self::BeepTestEditCountDec
            | Self::BeepTestEditDurationInc
            | Self::BeepTestEditDurationDec
            | Self::BeepTestEditAddLevel
            | Self::BeepTestEditRemoveLevel
            | Self::BeepTestEditLevelsSave
            | Self::BeepTestEditLevelsCancel
            | Self::BeepTestEditOpenBuzzer
            | Self::BeepTestSelectBuzzer(_)
            | Self::BeepTestTestBuzzer
            | Self::BeepTestBuzzerSave
            | Self::BeepTestBuzzerCancel
            | Self::BeepTestRestartToApply
            | Self::BeepTestCycleDisplayLayout
            | Self::AlarmPressed
            | Self::AlarmReleased
            | Self::SpacebarPressed
            | Self::SpacebarReleased
            | Self::AlarmDelayElapsed(_)
            | Self::TimeoutRevivePressed(_)
            | Self::TimeoutReviveReleased(_)
            | Self::TimeoutReviveHoldElapsed(_, _)
            | Self::OpenUpdatesPage
            | Self::UpdatesCheck
            | Self::UpdatesConfirmInstall
            | Self::UpdatesRevert
            | Self::UpdatesConfirmRevert
            | Self::UpdatesCheckDone(_)
            | Self::UpdatesDownloaded(_)
            | Self::UpdatesVerified(_)
            | Self::UpdatesInstalled(_)
            | Self::UpdatesBack
            | Self::UpdaterHealthyCheck
            | Self::SetTeamTimeoutCount(_)
            | Self::SetTeamTimeoutLength(_) => false,
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
            | (Self::OpenNewDisplay, Self::OpenNewDisplay)
            | (Self::OpenPortalDetailPage, Self::OpenPortalDetailPage)
            | (Self::ClosePortalDetailPage, Self::ClosePortalDetailPage)
            | (Self::ClosePortalAttentionAction, Self::ClosePortalAttentionAction)
            | (Self::PortalUiTick, Self::PortalUiTick)
            | (Self::RequestPortalRefresh, Self::RequestPortalRefresh)
            | (Self::PortalRefreshFinished, Self::PortalRefreshFinished)
            | (Self::PortalRetryAll, Self::PortalRetryAll)
            | (Self::ShowWarnings, Self::ShowWarnings)
            | (Self::ShowParameterHelp, Self::ShowParameterHelp)
            | (Self::CloseParameterHelp, Self::CloseParameterHelp)
            | (Self::EditGameConfig, Self::EditGameConfig)
            | (Self::ConfigEditComplete, Self::ConfigEditComplete)
            | (Self::RequestRemoteId, Self::RequestRemoteId)
            | (Self::UpdateAudioOutput, Self::UpdateAudioOutput)
            | (Self::EndTimeout, Self::EndTimeout)
            | (Self::CancelTimeout, Self::CancelTimeout)
            | (Self::StopClock, Self::StopClock)
            | (Self::StartClock, Self::StartClock)
            | (Self::BeepTestStart, Self::BeepTestStart)
            | (Self::BeepTestStop, Self::BeepTestStop)
            | (Self::BeepTestReset, Self::BeepTestReset)
            | (Self::BeepTestTick, Self::BeepTestTick)
            | (Self::BeepTestOpenSettings, Self::BeepTestOpenSettings)
            | (Self::BeepTestCloseSettings, Self::BeepTestCloseSettings)
            | (Self::BeepTestEditOpenLanguage, Self::BeepTestEditOpenLanguage)
            | (Self::BeepTestLanguageCancel, Self::BeepTestLanguageCancel)
            | (Self::BeepTestLanguageApply, Self::BeepTestLanguageApply)
            | (Self::BeepTestEditOpenSound, Self::BeepTestEditOpenSound)
            | (Self::BeepTestSoundSettingsSave, Self::BeepTestSoundSettingsSave)
            | (Self::BeepTestSoundSettingsCancel, Self::BeepTestSoundSettingsCancel)
            | (Self::BeepTestEditOpenLevels, Self::BeepTestEditOpenLevels)
            | (Self::BeepTestEditCountInc, Self::BeepTestEditCountInc)
            | (Self::BeepTestEditCountDec, Self::BeepTestEditCountDec)
            | (Self::BeepTestEditDurationInc, Self::BeepTestEditDurationInc)
            | (Self::BeepTestEditDurationDec, Self::BeepTestEditDurationDec)
            | (Self::BeepTestEditAddLevel, Self::BeepTestEditAddLevel)
            | (Self::BeepTestEditRemoveLevel, Self::BeepTestEditRemoveLevel)
            | (Self::BeepTestEditLevelsSave, Self::BeepTestEditLevelsSave)
            | (Self::BeepTestEditLevelsCancel, Self::BeepTestEditLevelsCancel)
            | (Self::BeepTestRestartToApply, Self::BeepTestRestartToApply)
            | (Self::BeepTestCycleDisplayLayout, Self::BeepTestCycleDisplayLayout)
            | (Self::CycleDisplayMode, Self::CycleDisplayMode)
            | (Self::AlarmPressed, Self::AlarmPressed)
            | (Self::AlarmReleased, Self::AlarmReleased)
            | (Self::SpacebarPressed, Self::SpacebarPressed)
            | (Self::SpacebarReleased, Self::SpacebarReleased)
            | (Self::OpenUpdatesPage, Self::OpenUpdatesPage)
            | (Self::UpdatesCheck, Self::UpdatesCheck)
            | (Self::UpdatesConfirmInstall, Self::UpdatesConfirmInstall)
            | (Self::UpdatesRevert, Self::UpdatesRevert)
            | (Self::UpdatesConfirmRevert, Self::UpdatesConfirmRevert)
            | (Self::UpdatesBack, Self::UpdatesBack)
            | (Self::UpdaterHealthyCheck, Self::UpdaterHealthyCheck)
            | (Self::NoAction, Self::NoAction) => true,
            (Self::AlarmDelayElapsed(a), Self::AlarmDelayElapsed(b)) => a == b,
            (Self::TimeoutRevivePressed(a), Self::TimeoutRevivePressed(b)) => a == b,
            (Self::TimeoutReviveReleased(a), Self::TimeoutReviveReleased(b)) => a == b,
            (Self::TimeoutReviveHoldElapsed(a, b), Self::TimeoutReviveHoldElapsed(c, d)) => {
                a == c && b == d
            }

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
            (Self::EditGameConfigPage(a), Self::EditGameConfigPage(b)) => a == b,
            (Self::ApplyConfigPage(a), Self::ApplyConfigPage(b)) => a == b,
            (Self::CancelConfigPage(a), Self::CancelConfigPage(b)) => a == b,
            (Self::SelectLanguage(a), Self::SelectLanguage(b)) => a == b,
            (
                Self::LanguageSelectComplete { canceled: a },
                Self::LanguageSelectComplete { canceled: b },
            ) => a == b,
            (Self::SelectBuzzer(a), Self::SelectBuzzer(b)) => a == b,
            (Self::BeepTestSelectBuzzer(a), Self::BeepTestSelectBuzzer(b)) => a == b,
            (Self::GotRemoteId(a), Self::GotRemoteId(b)) => a == b,
            (Self::DeleteRemote(a), Self::DeleteRemote(b)) => a == b,
            (Self::PortalRowTapped(a), Self::PortalRowTapped(b)) => a == b,
            (Self::PortalForceSubmit(a), Self::PortalForceSubmit(b)) => a == b,
            (Self::PortalDiscardTapped(a), Self::PortalDiscardTapped(b)) => a == b,
            // `PortalEvent` is an incoming stream payload that is never
            // used for message deduplication; treat two PortalEvents as
            // unequal so every event is delivered to `update()`.
            (Self::PortalEvent(_), Self::PortalEvent(_)) => false,
            (Self::ConfirmationSelected(a), Self::ConfirmationSelected(b)) => a == b,
            (Self::BeepTestEditSelectLevel(a), Self::BeepTestEditSelectLevel(b)) => a == b,
            (Self::TeamTimeout(a, b), Self::TeamTimeout(c, d)) => a == c && b == d,
            (Self::RefTimeout(a), Self::RefTimeout(b)) => a == b,
            (Self::PenaltyShot(a), Self::PenaltyShot(b)) => a == b,
            (Self::TimeUpdaterStarted(a), Self::TimeUpdaterStarted(b)) => a.same_channel(b),
            (Self::RecvEventList(a), Self::RecvEventList(b)) => a == b,
            (Self::RecvTeamsList(a, b), Self::RecvTeamsList(c, d)) => a == c && b == d,
            (Self::RecvSchedule(a, b), Self::RecvSchedule(c, d)) => a == c && b == d,
            (Self::RecvPortalToken(a), Self::RecvPortalToken(b)) => a == b,
            (Self::RecvTokenValid(a), Self::RecvTokenValid(b)) => a == b,
            (Self::SetTeamTimeoutCount(a), Self::SetTeamTimeoutCount(b)) => a == b,
            (Self::SetTeamTimeoutLength(a), Self::SetTeamTimeoutLength(b)) => a == b,
            (Self::UpdatesCheckDone(a), Self::UpdatesCheckDone(b)) => a == b,
            (Self::UpdatesDownloaded(a), Self::UpdatesDownloaded(b)) => a == b,
            (Self::UpdatesVerified(a), Self::UpdatesVerified(b)) => a == b,
            (Self::UpdatesInstalled(a), Self::UpdatesInstalled(b)) => a == b,

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
            | (Self::OpenNewDisplay, _)
            | (Self::OpenPortalDetailPage, _)
            | (Self::ClosePortalDetailPage, _)
            | (Self::ClosePortalAttentionAction, _)
            | (Self::PortalEvent(_), _)
            | (Self::PortalUiTick, _)
            | (Self::PortalRowTapped(_), _)
            | (Self::PortalForceSubmit(_), _)
            | (Self::PortalDiscardTapped(_), _)
            | (Self::RequestPortalRefresh, _)
            | (Self::PortalRefreshFinished, _)
            | (Self::PortalRetryAll, _)
            | (Self::ShowWarnings, _)
            | (Self::ShowParameterHelp, _)
            | (Self::CloseParameterHelp, _)
            | (Self::EditGameConfig, _)
            | (Self::EditGameConfigPage(_), _)
            | (Self::ChangeConfigPage(_), _)
            | (Self::ApplyConfigPage(_), _)
            | (Self::CancelConfigPage(_), _)
            | (Self::ConfigEditComplete, _)
            | (Self::EditParameter(_), _)
            | (Self::SelectParameter(_), _)
            | (Self::ParameterEditComplete { .. }, _)
            | (Self::ParameterSelected(_, _), _)
            | (Self::ToggleBoolParameter(_), _)
            | (Self::CycleParameter(_), _)
            | (Self::SelectLanguage(_), _)
            | (Self::LanguageSelectComplete { .. }, _)
            | (Self::SelectBuzzer(_), _)
            | (Self::TestBuzzer, _)
            | (Self::RequestRemoteId, _)
            | (Self::UpdateAudioOutput, _)
            | (Self::GotRemoteId(_), _)
            | (Self::DeleteRemote(_), _)
            | (Self::ConfirmationSelected(_), _)
            | (Self::TeamTimeout(_, _), _)
            | (Self::RefTimeout(_), _)
            | (Self::PenaltyShot(_), _)
            | (Self::EndTimeout, _)
            | (Self::CancelTimeout, _)
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
            | (Self::BeepTestStart, _)
            | (Self::BeepTestStop, _)
            | (Self::BeepTestReset, _)
            | (Self::BeepTestTick, _)
            | (Self::BeepTestOpenSettings, _)
            | (Self::BeepTestCloseSettings, _)
            | (Self::BeepTestEditOpenLanguage, _)
            | (Self::BeepTestLanguageCancel, _)
            | (Self::BeepTestLanguageApply, _)
            | (Self::BeepTestEditOpenSound, _)
            | (Self::BeepTestSoundSettingsSave, _)
            | (Self::BeepTestSoundSettingsCancel, _)
            | (Self::BeepTestEditOpenLevels, _)
            | (Self::BeepTestEditSelectLevel(_), _)
            | (Self::BeepTestEditCountInc, _)
            | (Self::BeepTestEditCountDec, _)
            | (Self::BeepTestEditDurationInc, _)
            | (Self::BeepTestEditDurationDec, _)
            | (Self::BeepTestEditAddLevel, _)
            | (Self::BeepTestEditRemoveLevel, _)
            | (Self::BeepTestEditLevelsSave, _)
            | (Self::BeepTestEditLevelsCancel, _)
            | (Self::BeepTestEditOpenBuzzer, _)
            | (Self::BeepTestSelectBuzzer(_), _)
            | (Self::BeepTestTestBuzzer, _)
            | (Self::BeepTestBuzzerSave, _)
            | (Self::BeepTestBuzzerCancel, _)
            | (Self::BeepTestRestartToApply, _)
            | (Self::BeepTestCycleDisplayLayout, _)
            | (Self::CycleDisplayMode, _)
            | (Self::AlarmPressed, _)
            | (Self::AlarmReleased, _)
            | (Self::SpacebarPressed, _)
            | (Self::SpacebarReleased, _)
            | (Self::AlarmDelayElapsed(_), _)
            | (Self::TimeoutRevivePressed(_), _)
            | (Self::TimeoutReviveReleased(_), _)
            | (Self::TimeoutReviveHoldElapsed(_, _), _)
            | (Self::TimeUpdaterStarted(_), _)
            | (Self::OpenUpdatesPage, _)
            | (Self::UpdatesCheck, _)
            | (Self::UpdatesConfirmInstall, _)
            | (Self::UpdatesRevert, _)
            | (Self::UpdatesConfirmRevert, _)
            | (Self::UpdatesCheckDone(_), _)
            | (Self::UpdatesDownloaded(_), _)
            | (Self::UpdatesVerified(_), _)
            | (Self::UpdatesInstalled(_), _)
            | (Self::UpdatesBack, _)
            | (Self::UpdaterHealthyCheck, _)
            | (Self::SetTeamTimeoutCount(_), _)
            | (Self::SetTeamTimeoutLength(_), _)
            | (Self::NoAction, _) => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateUiError {
    NoInternet,
    RateLimited,
    BadDownload,
    NoSpace,
    NotWritable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateUiState {
    Unknown,
    /// Idle state shown once at startup after the safety net auto-reverted a
    /// failed update. Behaves like an idle page (offers a fresh check).
    RolledBack,
    Checking,
    UpToDate,
    UpdateAvailable,
    Downloading,
    Verifying,
    Installing,
    Restarting,
    RevertConfirm,
    Error(UpdateUiError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigPage {
    Main,
    Game,
    Sound,
    Display,
    App,
    User,
    Remotes(usize, bool),
    Language,
    Buzzer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthParameter {
    Half,
    HalfTime,
    GameBlock,
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
    // Emitted by the 2 Halves / 1 Period selector in the Half Length parameter
    // editor (surfaced per the former ADR-009 Task 14 TODO). Toggles the staged
    // `single_half` choice held in AppState::ParameterEditor.
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
    ShowBehindScheduleTime,
    TeamWarning,
    TimeoutsCountedPerHalf,
    ConfirmScore,
    AudibleCountdown,
    ManualAlarmEnabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CyclingParameter {
    RemoteBuzzerSound(usize),
    AlertVolume,
    AboveWaterVol,
    UnderWaterVol,
    Mode,
    Brightness,
    FrontDisplayLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollOption {
    Black,
    White,
    Equal,
    GameParameter,
    PortalDetail,
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
    // Offered by ConfirmationKind::PortalTenantSwitch — restarts the app on the
    // new Mode/portal. Raised by apply_app_options (Task 9); handled in Task 8.
    RestartAndApply,
}
