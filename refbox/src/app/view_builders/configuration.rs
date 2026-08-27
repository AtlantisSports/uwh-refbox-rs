use super::fit_text::fit_text;
use super::{ViewData, fl, message::*, shared_elements::*, theme::*};
use crate::app::PageEntrySnapshot;
use crate::app::languages::Language;
use crate::config::{CustomSite, GameSource, Level, Mode, RemoteSource};
use crate::portal_manager::PortalIndicatorState;
use crate::sim_frame::FrontDisplayLayout;
use crate::sound_controller::*;
use collect_array::CollectArrayResult;
use iced::{
    Alignment, Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{
        Image, button, column, container, horizontal_space, image, row, svg, svg::Svg, text,
        text_input, vertical_space,
    },
};
use matrix_drawing::transmitted_data::Brightness;
use std::collections::BTreeMap;
use tokio::time::Duration;
use uwh_common::{
    config::Game as GameConfig,
    game_snapshot::{GamePeriod, GameSnapshot},
    uwhportal::schedule::{Event, EventId, GameNumber, Schedule},
};

impl EditableSettings {
    /// Whether games come from a remote site at all, as opposed to being
    /// entered by hand. Most callers only need this question; only the few
    /// that must tell the official Portal from a third-party site match on
    /// `source` directly.
    pub fn uses_remote(&self) -> bool {
        !matches!(self.source, GameSource::Manual)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(in super::super) struct EditableSettings {
    pub config: GameConfig,
    pub game_number: GameNumber,
    pub white_on_right: bool,
    pub brightness: Brightness,
    pub front_display_layout: FrontDisplayLayout,
    pub source: GameSource,
    /// Which remote to return to when leaving MANUAL. A sticky preference
    /// rather than a page setting: deliberately not part of the Cancel/revert
    /// snapshot, so it does not count as a visible edit.
    pub remembered_remote: RemoteSource,
    /// The custom site as staged by the editor. Unlike `remembered_remote` this
    /// IS snapshotted, so Cancel discards a half-typed URL.
    pub custom_site: CustomSite,
    pub uwhportal_token_valid: Option<bool>,
    pub current_event_id: Option<EventId>,
    pub current_court: Option<String>,
    pub schedule: Option<Schedule>,
    pub sound: SoundSettings,
    pub mode: Mode,
    pub hide_time: bool,
    pub collect_scorer_cap_num: bool,
    pub track_fouls_and_warnings: bool,
    pub show_behind_schedule_time: bool,
    pub confirm_score: bool,
    pub audible_countdown: bool,
    pub pending_language: Option<Language>,
    pub original_language: Option<Language>,
    /// Staged copy of `config.beep_test.levels` used by the BeepTest
    /// Edit Levels sub-page. `Some(_)` only while that page is open;
    /// `None` everywhere else.
    pub beep_test_levels: Option<Vec<Level>>,
    /// Index into `beep_test_levels` of the currently-selected level
    /// in the BeepTest Edit Levels sub-page. Defaults to 0 on entry.
    pub selected_level: usize,
}

impl EditableSettings {
    /// Returns `true` when portal mode is engaged but the configuration is not
    /// yet committable: event/court/schedule still missing, the chosen game number
    /// isn't in the schedule, or the chosen game's court doesn't match the
    /// currently-selected court.
    ///
    /// Both `apply_game_options` (gating the actual commit) and the Game page's
    /// action row in `make_event_config_page` (disabling Apply when nothing is
    /// committable) rely on this predicate, so they stay in sync.
    pub(in super::super) fn uwhportal_incomplete(&self) -> bool {
        if !self.uses_remote() {
            return false;
        }
        if self.current_event_id.is_none()
            || self.current_court.is_none()
            || self.schedule.is_none()
        {
            return true;
        }
        // Safety: guarded by the is_none() check on lines 55-58 above; reachable only when both schedule and current_court are Some.
        match self.schedule.as_ref().unwrap().games.get(&self.game_number) {
            Some(g) => g.court != *self.current_court.as_ref().unwrap(),
            None => true,
        }
    }

    /// Record an event-picker selection. Sets the new event id and clears any
    /// court / game-number / schedule that was filtered by the previous event so
    /// the user re-picks against the new event's data.
    pub(in super::super) fn select_event(&mut self, id: EventId) {
        self.current_event_id = Some(id);
        self.current_court = None;
        self.game_number = String::new();
        self.schedule = None;
    }

    /// Record a court-picker selection. Sets the new court and clears the
    /// game number so the user re-picks from the new court's filtered list.
    pub(in super::super) fn select_court(&mut self, court: String) {
        self.current_court = Some(court);
        self.game_number = String::new();
    }

    /// Start the remote pickers from a blank slate when manual games are
    /// switched off, per ADR 017 (Portal Data Lifecycle). The saved credential
    /// itself is untouched — only the pickers and the validity cache reset.
    ///
    /// The indicator resets to FAILED rather than `None`. `None` renders as
    /// "CHECKING…", which promises a check that cannot happen: this same reset
    /// clears the event id, and a token is only ever verified against an event.
    /// The portal path recovers once the operator picks one from the event
    /// list; a custom site has no such list — its event is adopted only when
    /// the site is applied — so the row would sit on "CHECKING…" permanently,
    /// looking like a hang. FAILED is the resting state the other two seeding
    /// sites already use when there is no event to check against.
    pub(in super::super) fn clear_for_remote_switch(&mut self) {
        self.current_event_id = None;
        self.current_court = None;
        self.schedule = None;
        self.game_number = String::new();
        self.uwhportal_token_valid = Some(false);
    }

    /// Whether a freshly-arrived schedule should auto-select its court for the
    /// current edit session. True only when the schedule is for the event the
    /// operator currently has selected, that event has exactly one court, and no
    /// court is chosen yet. The event-id check stops a late schedule from a
    /// previously-selected event from filling the court for a different event
    /// (it mirrors the event-id guard on the schedule-store in RecvSchedule).
    pub(in super::super) fn should_adopt_auto_court(
        &self,
        schedule_event_id: &EventId,
        court_count: usize,
    ) -> bool {
        court_count == 1
            && self.current_court.is_none()
            && self.current_event_id.as_ref() == Some(schedule_event_id)
    }
}

pub(in super::super) trait Cyclable
where
    Self: Sized,
{
    fn next(&self) -> Self;

    fn cycle(&mut self) {
        *self = self.next();
    }
}

impl Cyclable for Option<BuzzerSound> {
    fn next(&self) -> Self {
        match self {
            Some(BuzzerSound::Buzz) => Some(BuzzerSound::Whoop),
            Some(BuzzerSound::Whoop) => Some(BuzzerSound::Crazy),
            Some(BuzzerSound::Crazy) => Some(BuzzerSound::DeDeDu),
            Some(BuzzerSound::DeDeDu) => Some(BuzzerSound::TwoTone),
            Some(BuzzerSound::TwoTone) => Some(BuzzerSound::Airhorn),
            Some(BuzzerSound::Airhorn) => Some(BuzzerSound::Pipes),
            Some(BuzzerSound::Pipes) => Some(BuzzerSound::Klaxon),
            Some(BuzzerSound::Klaxon) => Some(BuzzerSound::Pip),
            Some(BuzzerSound::Pip) => Some(BuzzerSound::Pulse),
            Some(BuzzerSound::Pulse) => Some(BuzzerSound::Siren),
            Some(BuzzerSound::Siren) => Some(BuzzerSound::Trill),
            Some(BuzzerSound::Trill) => None,
            None => Some(BuzzerSound::Buzz),
        }
    }
}

impl Cyclable for Volume {
    fn next(&self) -> Self {
        match self {
            Self::Off => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Max,
            Self::Max => Self::Off,
        }
    }
}

impl Cyclable for Mode {
    fn next(&self) -> Self {
        match self {
            Self::Hockey6V6 => Self::Hockey3V3,
            Self::Hockey3V3 => Self::Rugby,
            Self::Rugby => Self::BeepTest,
            Self::BeepTest => Self::Hockey6V6,
        }
    }
}

impl Cyclable for Brightness {
    fn next(&self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Outdoor,
            Self::Outdoor => Self::Low,
        }
    }
}

impl Cyclable for FrontDisplayLayout {
    fn next(&self) -> Self {
        // Call the inherent `FrontDisplayLayout::next` explicitly. Writing
        // `self.next()` here would resolve back to this trait method and
        // recurse forever.
        FrontDisplayLayout::next(*self)
    }
}

pub(in super::super) fn page_has_changes(
    page: ConfigPage,
    edited: &EditableSettings,
    snapshot: Option<&PageEntrySnapshot>,
) -> bool {
    let Some(snapshot) = snapshot else {
        return false;
    };
    match (page, snapshot) {
        (
            ConfigPage::Game,
            PageEntrySnapshot::Game {
                config,
                game_number,
                source,
                current_event_id,
                current_court,
                schedule,
            },
        ) => {
            edited.config != *config
                || edited.game_number != *game_number
                || edited.source != *source
                || edited.current_event_id != *current_event_id
                || edited.current_court != *current_court
                || edited.schedule != *schedule
        }
        (
            ConfigPage::App,
            PageEntrySnapshot::App {
                source,
                current_event_id,
                current_court,
                schedule,
                mode,
                collect_scorer_cap_num,
                track_fouls_and_warnings,
                show_behind_schedule_time,
                confirm_score,
                hide_time,
                audible_countdown,
            },
        ) => {
            edited.source != *source
                || edited.current_event_id != *current_event_id
                || edited.current_court != *current_court
                || edited.schedule != *schedule
                || edited.mode != *mode
                || edited.collect_scorer_cap_num != *collect_scorer_cap_num
                || edited.track_fouls_and_warnings != *track_fouls_and_warnings
                || edited.show_behind_schedule_time != *show_behind_schedule_time
                || edited.confirm_score != *confirm_score
                || edited.hide_time != *hide_time
                || edited.audible_countdown != *audible_countdown
        }
        (
            ConfigPage::Display,
            PageEntrySnapshot::Display {
                white_on_right,
                brightness,
                front_display_layout,
            },
        ) => {
            edited.white_on_right != *white_on_right
                || edited.brightness != *brightness
                || edited.front_display_layout != *front_display_layout
        }
        (ConfigPage::Sound, PageEntrySnapshot::Sound { sound }) => edited.sound != *sound,
        (ConfigPage::Remotes(_, _), PageEntrySnapshot::Remotes { remotes }) => {
            edited.sound.remotes != *remotes
        }
        (
            ConfigPage::Language,
            PageEntrySnapshot::Language {
                original_language,
                pending_language,
            },
        ) => {
            edited.original_language != *original_language
                || edited.pending_language != *pending_language
        }
        (ConfigPage::Buzzer, PageEntrySnapshot::Buzzer { buzzer_sound }) => {
            edited.sound.buzzer_sound != *buzzer_sound
        }
        (ConfigPage::CustomSite(_), PageEntrySnapshot::CustomSite { custom_site }) => {
            edited.custom_site != *custom_site
        }
        _ => false,
    }
}

pub(in super::super) fn build_game_config_edit_page<'a>(
    data: ViewData<'_, '_>,
    settings: &EditableSettings,
    events: Option<&BTreeMap<EventId, Event>>,
    page: ConfigPage,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
    show_power_button: bool,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        has_led_panel,
        committed_site_url,
        ..
    } = data;

    // Param order convention: per-branch additions appended in chronological order
    // — page_entry_snapshot (Unit 3) then portal_indicator (Unit 7) then has_led_panel
    //   (open-new-display gate).
    match page {
        ConfigPage::Main => make_main_config_page(
            snapshot,
            settings,
            mode,
            clock_running,
            portal_indicator,
            show_power_button,
        ),
        ConfigPage::Game => make_event_config_page(
            committed_site_url,
            snapshot,
            settings,
            events,
            mode,
            clock_running,
            page_entry_snapshot,
            portal_indicator,
        ),
        ConfigPage::Sound => make_sound_config_page(
            snapshot,
            settings,
            mode,
            clock_running,
            page_entry_snapshot,
            portal_indicator,
        ),
        ConfigPage::Display => make_display_config_page(
            snapshot,
            settings,
            mode,
            clock_running,
            page_entry_snapshot,
            portal_indicator,
            has_led_panel,
        ),
        ConfigPage::App => make_app_config_page(
            mode,
            snapshot,
            settings,
            clock_running,
            page_entry_snapshot,
            portal_indicator,
        ),
        ConfigPage::User => {
            make_user_config_page(snapshot, settings, mode, clock_running, portal_indicator)
        }
        ConfigPage::Remotes(index, listening) => make_remote_config_page(
            snapshot,
            settings,
            index,
            listening,
            mode,
            clock_running,
            page_entry_snapshot,
            portal_indicator,
        ),
        ConfigPage::Language => make_language_select_page(
            snapshot,
            settings,
            mode,
            clock_running,
            page_entry_snapshot,
            portal_indicator,
        ),
        ConfigPage::Buzzer => make_buzzer_select_page(
            snapshot,
            settings,
            mode,
            clock_running,
            page_entry_snapshot,
            portal_indicator,
        ),
        ConfigPage::CustomSite(show_invalid) => make_custom_site_page(
            snapshot,
            settings,
            show_invalid,
            mode,
            clock_running,
            page_entry_snapshot,
            portal_indicator,
        ),
    }
}

fn make_main_config_page<'a>(
    snapshot: &GameSnapshot,
    _settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
    portal_indicator: Option<PortalIndicatorState>,
    show_power_button: bool,
) -> Element<'a, Message> {
    let row_top = row![
        make_tile_button(fl!("game-options"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::Game)),
        make_tile_button(fl!("app-options"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::App)),
    ]
    .spacing(SPACING)
    .height(Length::Fill);

    let row_bottom = row![
        make_tile_button(fl!("user-options"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::User)),
        make_tile_button(fl!("language"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::Language)),
    ]
    .spacing(SPACING)
    .height(Length::Fill);

    // Icon-only blue power button opposite Back (right third), shown only on the
    // Pi (or with --force-power-controls). Sizing mirrors make_chrome_button so
    // it lines up with the Back button; matches the icon-button pattern in
    // shared_elements.
    let power_slot: Element<_> = if show_power_button {
        button(
            container(
                Svg::new(svg::Handle::from_memory(
                    &include_bytes!("../../../resources/power.svg")[..],
                ))
                .style(white_svg)
                .width(Length::Fixed(44.0))
                .height(Length::Fixed(44.0)),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(transparent_container),
        )
        .padding(PADDING)
        .height(Length::Fixed(MIN_BUTTON_SIZE))
        .width(Length::Fill)
        .style(blue_button)
        .on_press(Message::OpenPowerPage)
        .into()
    } else {
        horizontal_space().into()
    };

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        row_top,
        row_bottom,
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![
            make_back_button(Message::ConfigEditComplete),
            horizontal_space(),
            power_slot,
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

fn make_back_button<'a>(destination: Message) -> Element<'a, Message> {
    make_chrome_button(fl!("back"))
        .style(red_button)
        .on_press(destination)
        .into()
}

fn make_cancel_apply_footer<'a>(
    page: ConfigPage,
    edited: &EditableSettings,
    snapshot: Option<&PageEntrySnapshot>,
    game_in_progress: bool,
) -> Element<'a, Message> {
    // Apply is enabled when there are pending changes. The pages that use this
    // footer (App, Display, Sound, Remotes) have no committability gate; the
    // Game page has one but builds its own action row rather than using this.
    let has_changes = page_has_changes(page, edited, snapshot);
    let apply_enabled = has_changes;

    let cancel = make_chrome_button(cancel_or_back_label(has_changes))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::CancelConfigPage(page));

    let apply = make_chrome_button(fl!("apply"))
        .style(green_button)
        .width(Length::Fill);
    let apply = if apply_enabled {
        apply.on_press(Message::ApplyConfigPage(page))
    } else {
        apply
    };

    if page == ConfigPage::App {
        // Blue "Check Version" button opens the self-update page. Disabled
        // (no on_press → greyed) while a game is in progress so an operator
        // can't trigger a restart mid-game.
        let check = make_chrome_button(fl!("check-version"))
            .style(blue_button)
            .width(Length::Fill);
        let check = if game_in_progress {
            check
        } else {
            check.on_press(Message::OpenUpdatesPage)
        };
        row![cancel, check, apply].spacing(SPACING).into()
    } else {
        row![cancel, horizontal_space(), apply]
            .spacing(SPACING)
            .into()
    }
}

fn make_user_config_page<'a>(
    snapshot: &GameSnapshot,
    _settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
    portal_indicator: Option<PortalIndicatorState>,
) -> Element<'a, Message> {
    let tiles = row![
        make_tile_button(fl!("display-options"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::Display)),
        make_tile_button(fl!("sound-options"))
            .style(light_gray_button)
            .on_press(Message::ChangeConfigPage(ConfigPage::Sound)),
    ]
    .spacing(SPACING)
    .height(Length::Fill);

    let view_mode_label = match display_mode() {
        DisplayMode::Light => fl!("display-mode-light"),
        DisplayMode::Dark => fl!("display-mode-dark"),
        DisplayMode::HighContrast => fl!("display-mode-high-contrast"),
    };
    let view_mode_button = make_value_button(
        fl!("view-mode"),
        view_mode_label,
        (false, true),
        Some(Message::CycleDisplayMode),
    );

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        tiles,
        row![view_mode_button, horizontal_space()]
            .spacing(SPACING)
            .height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![horizontal_space()].height(Length::Fill),
        row![
            make_back_button(Message::ChangeConfigPage(ConfigPage::Main)),
            horizontal_space(),
            horizontal_space(),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// Whether the configured Game Block leaves enough time for the game plus
/// breaks and team timeouts. Drives the red/yellow validation styling.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum GameBlockValidity {
    Ok,
    Tight,
    TooShort,
}

fn game_block_validity(cfg: &GameConfig) -> GameBlockValidity {
    if cfg.game_block < cfg.game_block_minimum() {
        GameBlockValidity::TooShort
    } else if cfg.game_block_buffer() <= cfg.team_timeout_allotment() {
        // `<=`: a Game Block that exactly covers the game, break, and full timeout
        // allotment is "barely sufficient" (no slack to recover if running behind),
        // so it warns yellow rather than reading as comfortably Ok.
        GameBlockValidity::Tight
    } else {
        GameBlockValidity::Ok
    }
}

// View builder takes app-state slices; grouping into a context struct is a separate refactor across all view_builders. Filed as a Findings-Backlog item in AUDIT-PLAN.md (Unit 3, 2026-05-13).
#[allow(clippy::too_many_arguments)]
/// `committed_site_url` is the custom site address as committed — see
/// `ViewData::committed_site_url` for why the SITE row shows that rather than
/// the address currently typed into the editor.
fn make_event_config_page<'a>(
    committed_site_url: &str,
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    events: Option<&BTreeMap<EventId, Event>>,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
    portal_indicator: Option<PortalIndicatorState>,
) -> Element<'a, Message> {
    let EditableSettings {
        config,
        game_number,
        source,
        remembered_remote,
        current_event_id,
        current_court,
        schedule,
        ..
    } = settings;

    let uses_remote = *source != GameSource::Manual;

    // A rejected access token greys out the schedule-derived pickers. The site
    // is normally the only thing enforcing the token, so against a site that
    // hands its schedule to an unauthenticated request the operator would
    // otherwise be offered courts and games we were never authorised to have.
    // EVENT is deliberately exempt: it has to be selectable before a token can
    // be obtained at all. `None` (still checking) does not grey anything —
    // only an outright rejection does.
    let token_rejected = settings.uwhportal_token_valid == Some(false);

    // Game-number picker — placed in the centre cell of the action row
    // (Cancel | Game | Apply) in both portal modes per ADR-009 Task 14 layout.
    let game_btn_msg = if uses_remote {
        if !token_rejected
            && current_event_id.is_some()
            && current_court.is_some()
            && schedule.is_some()
        {
            Some(Message::SelectParameter(ListableParameter::Game))
        } else {
            None
        }
    } else {
        Some(Message::KeypadPage(KeypadPage::GameNumber))
    };

    let mut game_large_text = true;
    let game_label = if uses_remote {
        if let (Some(_), Some(cur_court)) = (current_event_id, current_court) {
            if let Some(schedule) = schedule {
                match schedule.games.get(game_number) {
                    Some(game) if game.court == *cur_court => game.number.to_string(),
                    _ => {
                        game_large_text = false;
                        fl!("none-selected")
                    }
                }
            } else {
                fl!("loading")
            }
        } else {
            String::new()
        }
    } else {
        game_number.to_string()
    };

    // MANUAL GAMES — row 1 left cell in both modes. Two states, but set
    // explicitly rather than toggled: turning it off lands on whichever remote
    // was last applied, which a toggle could not express.
    let manual_games_btn = make_value_button(
        fl!("manual-games"),
        bool_string(!uses_remote),
        (false, true),
        Some(Message::SelectGameSource(if uses_remote {
            GameSource::Manual
        } else {
            match remembered_remote {
                RemoteSource::Portal => GameSource::Portal,
                RemoteSource::Custom => GameSource::Custom,
            }
        })),
    );

    // Column layout: page_content fills available height between the top
    // game-time button and the bottom timeout ribbon. Data rows take Fill
    // height so they each absorb an equal share of the leftover vertical
    // space, giving uniform inter-row gaps with the action row sitting just
    // above the timeout ribbon. Action row stays at MIN_BUTTON_SIZE so the
    // Cancel/Game/Apply chrome reads at a consistent size across pages.
    let mut col = column![make_game_time_button(
        snapshot,
        false,
        false,
        mode,
        clock_running,
        portal_indicator,
        None,
    )]
    .spacing(SPACING)
    .height(Length::Fill);

    if uses_remote {
        // Portal mode ON: row 1 = Manual Games + the portal button (labelled for
        // the current mode — UWH, UWR) + Custom; rows 2–4 are full-width
        // single-button rows — Event (or Custom Site, under CUSTOM), then Token,
        // then Court.
        let event_label = if let Some(events) = events {
            if let Some(event_id) = current_event_id {
                match events.get(event_id) {
                    Some(t) => t.name.clone(),
                    None => fl!("none-selected"),
                }
            } else {
                fl!("none-selected")
            }
        } else {
            fl!("loading")
        };

        let event_btn_msg = if events.is_some() {
            Some(Message::SelectParameter(ListableParameter::Event))
        } else {
            None
        };

        let pool_label = if let Some(event) = events
            .as_ref()
            .and_then(|events| events.get(current_event_id.as_ref()?))
        {
            if event.courts.is_some() {
                if let Some(court) = current_court {
                    court.clone()
                } else {
                    fl!("none-selected")
                }
            } else {
                fl!("loading")
            }
        } else {
            String::new()
        };

        // The court picker is tappable only when the event has more than one
        // court. A single-court event auto-selects that court (see RecvSchedule
        // and ParameterSelected::Event), so there is nothing to choose: the tile
        // is greyed (no on_press) while still showing the court via pool_label.
        // A rejected token greys it for the separate reason above.
        let pool_btn_msg = if token_rejected {
            None
        } else {
            events
                .as_ref()
                .and_then(|tourns| tourns.get(current_event_id.as_ref()?)?.courts.as_ref())
                .filter(|courts| courts.len() > 1)
                .map(|_| Message::SelectParameter(ListableParameter::Court))
        };

        let auth_container = |auth| {
            let txt = match auth {
                Some(true) => "OK",
                Some(false) => "FAILED",
                None => "CHECKING...",
            };
            let style = match auth {
                Some(true) => green_container,
                Some(false) => red_container,
                None => gray_container,
            };
            container(txt)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(style)
        };

        let uwhportal_auth_text = text(fl!("access-token"))
            .size(MEDIUM_TEXT)
            .align_y(Vertical::Center)
            .align_x(Horizontal::Right)
            .width(Length::Fill)
            .height(Length::Fill);

        let auth_state_message = if settings.current_event_id.is_some() {
            Some(Message::KeypadPage(KeypadPage::PortalLogin(0, false)))
        } else {
            None
        };

        let auth_state_button = button(
            row![
                uwhportal_auth_text,
                auth_container(settings.uwhportal_token_valid),
            ]
            .padding(PADDING)
            .spacing(SPACING)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .padding(0)
        .style(light_gray_button)
        .on_press_maybe(auth_state_message);

        // These two fill row 1's second and third cells, beside Manual Games. The
        // active one is marked with the existing selected-button style rather
        // than a new treatment. The labels are `fit_text`, as the tiles beside
        // them are: Italian's PERSONALIZZATO is one 14-character word with
        // nowhere to wrap, so shrinking it is the only way to show it whole.
        // (Not every tile on this page is one yet -- the ACCESS TOKEN row
        // below is still a plain `text`.) The width this row leaves the label,
        // and the size Italian settles at inside it, are pinned by
        // `the_italian_source_label_shrinks_but_stays_clear_of_the_floor`.
        let portal_source_btn = button(
            fit_text(fl!("source-portal", portal = portal_name_for_mode(mode))).size(MEDIUM_TEXT),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(if settings.source == GameSource::Portal {
            light_gray_selected_button
        } else {
            light_gray_button
        })
        .on_press(Message::SelectGameSource(GameSource::Portal));

        let custom_source_btn = button(fit_text(fl!("source-custom")).size(MEDIUM_TEXT))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(if settings.source == GameSource::Custom {
                light_gray_selected_button
            } else {
                light_gray_button
            })
            .on_press(Message::SelectGameSource(GameSource::Custom));

        col = col
            .push(
                row![manual_games_btn, portal_source_btn, custom_source_btn]
                    .spacing(SPACING)
                    .height(Length::Fill),
            )
            // Under CUSTOM the event is named inside the URL, so there is
            // nothing to pick: the SITE row takes the EVENT row's slot and the
            // page stays at four rows either way.
            .push(if *source == GameSource::Custom {
                // The committed address, never the one being typed: an address
                // that has not been applied is not the address in use, and
                // showing it here would hide exactly that difference.
                // "None provided" rather than the "None selected" the rows below
                // use: those offer a list refbox fetched, while an address is
                // typed in, so "selected" describes the wrong action.
                let shown = if committed_site_url.is_empty() {
                    fl!("none-provided")
                } else {
                    committed_site_url.to_string()
                };
                make_long_value_button(
                    fl!("custom-site"),
                    shown,
                    Some(Message::ChangeConfigPage(ConfigPage::CustomSite(false))),
                )
            } else {
                make_value_button(fl!("event"), event_label, (true, true), event_btn_msg)
            })
            .push(auth_state_button)
            .push(make_value_button(
                fl!("court"),
                pool_label,
                (true, true),
                pool_btn_msg,
            ));
    } else {
        // Portal mode OFF: 4 data rows × 3 cells each.
        col = col
            .push(
                row![
                    manual_games_btn,
                    make_value_button(
                        fl!("overtime-allowed"),
                        bool_string(config.overtime_allowed),
                        (false, true),
                        Some(Message::ToggleBoolParameter(
                            BoolGameParameter::OvertimeAllowed,
                        )),
                    ),
                    make_value_button(
                        fl!("sudden-death-allowed"),
                        bool_string(config.sudden_death_allowed),
                        (false, true),
                        Some(Message::ToggleBoolParameter(
                            BoolGameParameter::SuddenDeathAllowed,
                        )),
                    )
                ]
                .spacing(SPACING)
                .height(Length::Fill),
            )
            .push(
                row![
                    make_value_button(
                        if config.single_half {
                            fl!("game-length")
                        } else {
                            fl!("half-length-full")
                        },
                        time_string(config.half_play_duration),
                        (false, true),
                        Some(Message::EditParameter(LengthParameter::Half)),
                    )
                    .style(length_button_style(config, LengthParameter::Half)),
                    make_value_button(
                        fl!("pre-ot-break-length"),
                        time_string(config.pre_overtime_break),
                        (false, true),
                        if config.overtime_allowed {
                            Some(Message::EditParameter(LengthParameter::PreOvertime))
                        } else {
                            None
                        },
                    )
                    .style(length_button_style(config, LengthParameter::PreOvertime)),
                    make_value_button(
                        fl!("pre-sd-break-length"),
                        time_string(config.pre_sudden_death_duration),
                        (false, true),
                        if config.sudden_death_allowed {
                            Some(Message::EditParameter(LengthParameter::PreSuddenDeath))
                        } else {
                            None
                        },
                    )
                    .style(length_button_style(config, LengthParameter::PreSuddenDeath))
                ]
                .spacing(SPACING)
                .height(Length::Fill),
            )
            .push(
                row![
                    make_value_button(
                        fl!("half-time-length"),
                        time_string(config.half_time_duration),
                        (false, true),
                        if !config.single_half {
                            Some(Message::EditParameter(LengthParameter::HalfTime))
                        } else {
                            None
                        },
                    )
                    .style(length_button_style(config, LengthParameter::HalfTime)),
                    make_value_button(
                        fl!("ot-half-length"),
                        time_string(config.ot_half_play_duration),
                        (false, true),
                        if config.overtime_allowed {
                            Some(Message::EditParameter(LengthParameter::OvertimeHalf))
                        } else {
                            None
                        },
                    )
                    .style(length_button_style(config, LengthParameter::OvertimeHalf)),
                    make_value_button(
                        fl!("minimum-brk-btwn-games"),
                        time_string(config.minimum_break),
                        (false, true),
                        Some(Message::EditParameter(LengthParameter::MinimumBetweenGame)),
                    )
                    .style(length_button_style(
                        config,
                        LengthParameter::MinimumBetweenGame
                    ))
                ]
                .spacing(SPACING)
                .height(Length::Fill),
            )
            .push(
                row![
                    make_value_button(
                        fl!("team-timeouts-label"),
                        // Compact value, matching the "0 / 1/HALF / 1/GAME"
                        // representation used on the Main and Game Info pages.
                        if config.num_team_timeouts_allowed == 0 {
                            "0".to_string()
                        } else if config.timeouts_counted_per_half {
                            format!("{}/{}", config.num_team_timeouts_allowed, fl!("half"))
                        } else {
                            format!("{}/{}", config.num_team_timeouts_allowed, fl!("game"))
                        },
                        (false, true),
                        Some(Message::KeypadPage(KeypadPage::TeamTimeouts(
                            config.team_timeout_duration,
                            config.timeouts_counted_per_half,
                        ))),
                    ),
                    make_value_button(
                        fl!("ot-half-time-length"),
                        time_string(config.ot_half_time_duration),
                        (false, true),
                        if config.overtime_allowed {
                            Some(Message::EditParameter(LengthParameter::OvertimeHalfTime))
                        } else {
                            None
                        },
                    )
                    .style(length_button_style(
                        config,
                        LengthParameter::OvertimeHalfTime
                    )),
                    make_value_button(
                        fl!("game-block-full"),
                        time_string(config.game_block),
                        (false, true),
                        Some(Message::EditParameter(LengthParameter::GameBlock)),
                    )
                    .style(match game_block_validity(config) {
                        GameBlockValidity::TooShort => red_button,
                        GameBlockValidity::Tight => yellow_button,
                        GameBlockValidity::Ok => light_gray_button,
                    })
                ]
                .spacing(SPACING)
                .height(Length::Fill),
            );
    }

    // Action row: Cancel | Game-number picker | Apply.
    // Apply is blocked when the portal state is incomplete, so a click on Apply
    // can't reach a wasteful "fix something and try again" dialog.
    let apply_blocked = settings.uwhportal_incomplete();
    // A red (too-short) Game Block is invalid, so APPLY must be disabled until it
    // is widened. Only gate this in portal-OFF mode — that is the only mode that
    // renders the Game Block button, so the disabled APPLY always has a visible
    // red button explaining it. Yellow ("tight") is a caution, not invalid, and
    // does not block.
    let game_block_too_short =
        !uses_remote && matches!(game_block_validity(config), GameBlockValidity::TooShort);
    let has_changes = page_has_changes(ConfigPage::Game, settings, page_entry_snapshot);
    let apply_enabled = page_apply_enabled(
        has_changes,
        apply_blocked,
        game_block_too_short,
        config,
        uses_remote,
    );

    let cancel_btn = make_chrome_button(cancel_or_back_label(has_changes))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::CancelConfigPage(ConfigPage::Game));

    // Footer furniture, not a tile. A filling child here would not collapse — it
    // would make the whole Cancel/Apply row claim a share of the page and grow,
    // at the body's expense.
    let game_picker_btn = make_value_chrome_button(
        fl!("game-select"),
        game_label,
        (false, game_large_text),
        game_btn_msg,
    );

    let apply_btn = make_chrome_button(fl!("apply"))
        .style(green_button)
        .width(Length::Fill);
    let apply_btn = if apply_enabled {
        apply_btn.on_press(Message::ApplyConfigPage(ConfigPage::Game))
    } else {
        apply_btn
    };

    col = col.push(row![cancel_btn, game_picker_btn, apply_btn].spacing(SPACING));

    col.into()
}

fn make_app_config_page<'a>(
    mode: Mode,
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
    portal_indicator: Option<PortalIndicatorState>,
) -> Element<'a, Message> {
    let EditableSettings {
        collect_scorer_cap_num,
        track_fouls_and_warnings,
        show_behind_schedule_time,
        confirm_score,
        hide_time,
        audible_countdown,
        ..
    } = settings;

    // A game is "in progress" for the purpose of gating the updater whenever we
    // are not in the BetweenGames period.
    let game_in_progress = snapshot.current_period != GamePeriod::BetweenGames;

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        row![
            make_value_button(
                fl!("app-mode"),
                settings.mode.to_string(),
                (false, true),
                Some(Message::CycleParameter(CyclingParameter::Mode)),
            ),
            make_value_button(
                fl!("track-cap-number-of-scorer"),
                bool_string(*collect_scorer_cap_num),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::ScorerCapNum,
                )),
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_value_button(
                fl!("track-fouls-and-warnings"),
                bool_string(*track_fouls_and_warnings),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::FoulsAndWarnings,
                )),
            ),
            make_value_button(
                fl!("confirm-score-at-game-end"),
                bool_string(*confirm_score),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::ConfirmScore,
                )),
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_value_button(
                // Internally still `hide_time`; the button shows the INVERSE:
                // YES = the final 10-second countdown IS shown on the scoreboard.
                fl!("show-countdown-for-last-10-seconds"),
                bool_string(!*hide_time),
                (false, true),
                Some(Message::ToggleBoolParameter(BoolGameParameter::HideTime)),
            ),
            make_value_button(
                fl!("audible-countdown-for-last-10-seconds"),
                bool_string(*audible_countdown),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::AudibleCountdown,
                )),
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_value_button(
                fl!("show-behind-schedule-time"),
                bool_string(*show_behind_schedule_time),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::ShowBehindScheduleTime,
                )),
            ),
            horizontal_space(),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        make_cancel_apply_footer(
            ConfigPage::App,
            settings,
            page_entry_snapshot,
            game_in_progress
        ),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// The embedded preview picture matching a staged layout + starting-side.
/// The exhaustive match means every `FrontDisplayLayout` must have a picture:
/// adding a new variant won't compile until its PNG is added here and generated
/// via `just capture-previews`.
fn layout_preview_handle(layout: FrontDisplayLayout, white_on_right: bool) -> image::Handle {
    macro_rules! preview {
        ($stem:literal) => {
            &include_bytes!(concat!(
                "../../../resources/layout-previews/",
                $stem,
                ".png"
            ))[..]
        };
    }
    let bytes: &'static [u8] = match (layout, white_on_right) {
        (FrontDisplayLayout::Default, false) => preview!("default-white-left"),
        (FrontDisplayLayout::Default, true) => preview!("default-white-right"),
        (FrontDisplayLayout::Classic, false) => preview!("classic-white-left"),
        (FrontDisplayLayout::Classic, true) => preview!("classic-white-right"),
        (FrontDisplayLayout::BigTime, false) => preview!("big-time-white-left"),
        (FrontDisplayLayout::BigTime, true) => preview!("big-time-white-right"),
        (FrontDisplayLayout::Corners, false) => preview!("corners-white-left"),
        (FrontDisplayLayout::Corners, true) => preview!("corners-white-right"),
        (FrontDisplayLayout::ScoresOnly, false) => preview!("scores-only-white-left"),
        (FrontDisplayLayout::ScoresOnly, true) => preview!("scores-only-white-right"),
    };
    image::Handle::from_bytes(bytes)
}

fn make_display_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
    portal_indicator: Option<PortalIndicatorState>,
    has_led_panel: bool,
) -> Element<'a, Message> {
    let EditableSettings {
        white_on_right,
        brightness,
        front_display_layout,
        ..
    } = settings;

    let white = container(text(fl!("light-team-name-caps")))
        .center_x(Length::FillPortion(2))
        .center_y(Length::Fill)
        .style(white_container);
    let black = container(text(fl!("dark-team-name-caps")))
        .center_x(Length::FillPortion(2))
        .center_y(Length::Fill)
        .style(black_container);

    // No `align_y` here: the row centres this node now, so it would be dead — and
    // a centre-anchored paragraph is the iced 0.13 stale-pixel pattern the button
    // helpers were rewritten to avoid.
    let center = text(fl!("starting-sides"))
        .size(MEDIUM_TEXT)
        .align_x(Horizontal::Center)
        .width(Length::FillPortion(3));

    // `white_on_right` is based on the view from the front of the panels, so for the ref's point
    // of view we need to reverse the direction
    let sides = if *white_on_right {
        // White to Ref's left
        row![white, center, black]
            .align_y(Alignment::Center)
            .padding(PADDING)
    } else {
        // White to Ref's right
        row![black, center, white]
            .align_y(Alignment::Center)
            .padding(PADDING)
    };

    let sides_btn = button(sides.width(Length::Fill).height(Length::Fill))
        .height(Length::Fill)
        .width(Length::Fill)
        .padding(0)
        .style(light_gray_button)
        .on_press(Message::ToggleBoolParameter(
            BoolGameParameter::WhiteOnRight,
        ));

    // When a real LED panel is connected the layout picker is grayed out (no
    // `on_press`) and its label is forced to DEFAULT, because the physical panel
    // always renders the Default layout. The preview follows the same effective
    // layout so it matches what the picker shows.
    let effective_layout = if has_led_panel {
        FrontDisplayLayout::Default
    } else {
        *front_display_layout
    };
    let layout_label = match effective_layout {
        FrontDisplayLayout::Default => fl!("layout-default"),
        FrontDisplayLayout::Classic => fl!("layout-classic"),
        FrontDisplayLayout::BigTime => fl!("layout-big-time"),
        FrontDisplayLayout::Corners => fl!("layout-corners"),
        FrontDisplayLayout::ScoresOnly => fl!("layout-scores-only"),
    };
    let layout_btn = make_value_button(
        fl!("front-display-layout"),
        layout_label,
        (false, true),
        if has_led_panel {
            None
        } else {
            Some(Message::CycleParameter(
                CyclingParameter::FrontDisplayLayout,
            ))
        },
    );

    // Stays fixed to match OPEN NEW DISPLAY directly above it. (The band around
    // them sets its own height, so nothing here can claim page height either
    // way — the sibling is the whole reason.)
    let brightness_btn = make_value_chrome_button(
        fl!("player-display-brightness"),
        fl!("brightness", brightness = brightness.to_string()),
        (false, true),
        if has_led_panel {
            Some(Message::CycleParameter(CyclingParameter::Brightness))
        } else {
            None
        },
    );

    // The button is grayed out (no `on_press`) when a real LED panel is connected
    // (`--serial-port`); opening a sim window then would compete with the panel.
    let open_display_btn = {
        let btn = make_chrome_button(fl!("open-new-display")).style(light_gray_button);
        if has_led_panel {
            btn
        } else {
            btn.on_press(Message::OpenNewDisplay)
        }
    };

    // Static preview of the staged layout, shown via a plain Image (NOT a live
    // canvas, which crashes the Linux/tiny-skia renderer — see design Decision D).
    let preview = container(
        Image::new(layout_preview_handle(effective_layout, *white_on_right))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill);

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        row![sides_btn].spacing(SPACING).height(Length::Fill),
        row![layout_btn, horizontal_space()]
            .spacing(SPACING)
            .height(Length::Fill),
        row![
            column![open_display_btn, brightness_btn]
                .spacing(SPACING)
                .width(Length::Fill),
            preview,
        ]
        .spacing(SPACING)
        .height(Length::FillPortion(2)),
        make_cancel_apply_footer(ConfigPage::Display, settings, page_entry_snapshot, false),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

fn make_sound_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
    portal_indicator: Option<PortalIndicatorState>,
) -> Element<'a, Message> {
    let EditableSettings { sound, .. } = settings;

    // Re-adopt the OS default output device. Laptop-only: the Pi runs Linux
    // with a fixed dedicated speaker, so the button is absent there and the
    // bottom row keeps its existing empty spacer.
    #[cfg(not(target_os = "linux"))]
    let audio_output_slot: Element<'a, Message> = make_tile_button(fl!("update-audio-output"))
        .on_press(Message::UpdateAudioOutput)
        .style(light_gray_button)
        .into();
    #[cfg(target_os = "linux")]
    let audio_output_slot: Element<'a, Message> = horizontal_space().into();

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        row![
            make_value_button(
                fl!("sound-enabled"),
                bool_string(sound.sound_enabled),
                (false, true),
                Some(Message::ToggleBoolParameter(
                    BoolGameParameter::SoundEnabled,
                )),
            ),
            make_value_button(
                fl!("buzzer-sound"),
                sound.buzzer_sound.to_string().to_uppercase(),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::ChangeConfigPage(ConfigPage::Buzzer))
                } else {
                    None
                },
            ),
            make_tile_button(fl!("manage-remotes"))
                .on_press(Message::ChangeConfigPage(ConfigPage::Remotes(0, false)),)
                .style(light_gray_button),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_value_button(
                fl!("whistle-enabled"),
                bool_string(sound.whistle_enabled),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::RefAlertEnabled,
                    ))
                } else {
                    None
                },
            ),
            make_value_button(
                fl!("above-water-volume"),
                sound.above_water_vol.to_string(),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::CycleParameter(CyclingParameter::AboveWaterVol))
                } else {
                    None
                },
            ),
            make_value_button(
                fl!("alarm-button"),
                bool_string(sound.manual_alarm_enabled),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::ManualAlarmEnabled,
                    ))
                } else {
                    None
                },
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            make_value_button(
                fl!("whistle-volume"),
                sound.whistle_vol.to_string(),
                (false, true),
                if sound.sound_enabled && sound.whistle_enabled {
                    Some(Message::CycleParameter(CyclingParameter::AlertVolume))
                } else {
                    None
                },
            ),
            make_value_button(
                fl!("underwater-volume"),
                sound.under_water_vol.to_string(),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::CycleParameter(CyclingParameter::UnderWaterVol))
                } else {
                    None
                },
            ),
            make_value_button(
                fl!("auto-sound-start-play"),
                bool_string(sound.auto_sound_start_play),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::AutoSoundStartPlay,
                    ))
                } else {
                    None
                },
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        row![
            audio_output_slot,
            horizontal_space(),
            make_value_button(
                fl!("auto-sound-stop-play"),
                bool_string(sound.auto_sound_stop_play),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::ToggleBoolParameter(
                        BoolGameParameter::AutoSoundStopPlay,
                    ))
                } else {
                    None
                },
            ),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
        make_cancel_apply_footer(ConfigPage::Sound, settings, page_entry_snapshot, false),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

// Same situation as make_event_config_page — view builder accumulates app-state slices. Context-struct refactor filed as Findings-Backlog.
#[allow(clippy::too_many_arguments)]
fn make_remote_config_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    index: usize,
    listening: bool,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
    portal_indicator: Option<PortalIndicatorState>,
) -> Element<'a, Message> {
    const REMOTES_LIST_LEN: usize = 4;

    let title = text(fl!("remotes"))
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

    let buttons: CollectArrayResult<_, REMOTES_LIST_LEN> = settings
        .sound
        .remotes
        .iter()
        .enumerate()
        .skip(index)
        .map(Some)
        .chain([None].into_iter().cycle())
        .take(REMOTES_LIST_LEN)
        .map(|rem| {
            if let Some((idx, rem_info)) = rem {
                let sound_text = if let Some(sound) = rem_info.sound {
                    sound.to_string().to_uppercase()
                } else {
                    fl!("default").to_owned()
                };
                let sound_text = fl!("sound", sound_text = sound_text);

                container(
                    row![
                        text(format!("ID: {}", rem_info.id))
                            .size(MEDIUM_TEXT)
                            .align_y(Vertical::Center)
                            .align_x(Horizontal::Center)
                            .height(Length::Fill)
                            .width(Length::Fill),
                        make_chrome_button(sound_text)
                            .on_press(Message::CycleParameter(
                                CyclingParameter::RemoteBuzzerSound(idx),
                            ))
                            .width(Length::Fixed(275.0))
                            .height(Length::Fixed(MIN_BUTTON_SIZE - (2.0 * PADDING)))
                            .style(yellow_button),
                        make_chrome_button(fl!("delete"))
                            .on_press(Message::DeleteRemote(idx))
                            .width(Length::Fixed(130.0))
                            .height(Length::Fixed(MIN_BUTTON_SIZE - (2.0 * PADDING)))
                            .style(red_button),
                    ]
                    .padding(PADDING)
                    .spacing(SPACING),
                )
                .width(Length::Fill)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .style(gray_container)
                .into()
            } else {
                container(horizontal_space())
                    .width(Length::Fill)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .style(disabled_container)
                    .into()
            }
        })
        .collect();

    let add_btn = if listening {
        make_chrome_button(fl!("waiting"))
    } else {
        make_chrome_button(fl!("add")).on_press(Message::RequestRemoteId)
    }
    .style(orange_button);

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        row![
            make_scroll_list(
                buttons.unwrap(),
                settings.sound.remotes.len(),
                index,
                title,
                ScrollOption::GameParameter,
                light_gray_container,
            )
            .height(Length::Fill)
            .width(Length::FillPortion(5)),
            column![vertical_space(), add_btn,]
                .spacing(SPACING)
                .height(Length::Fill)
                .width(Length::Fill),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
        .width(Length::Fill),
        make_cancel_apply_footer(
            ConfigPage::Remotes(index, listening),
            settings,
            page_entry_snapshot,
            false,
        ),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

/// Returns true when the parameter editor's buffer differs from the value shown
/// when it opened. Length is compared on whole seconds — the precision the mm:ss
/// editor displays — so zeroing and rebuilding to the same displayed value counts
/// as "no change" (mirrors `time_edit_has_changes` from PR #1218). For the Half
/// Length editor, flipping the 2 Halves / 1 Period choice is also a change, since
/// Apply commits `single_half` for that parameter; for all other parameters the
/// `single_half` flag is not committed and is ignored here.
fn param_edit_has_changes(
    length: Duration,
    old_length: Duration,
    param: LengthParameter,
    single_half: bool,
    old_single_half: bool,
) -> bool {
    length.as_secs() != old_length.as_secs()
        || (matches!(param, LengthParameter::Half) && single_half != old_single_half)
}

/// The shortest any constrained period may be.
///
/// A zero-length half is not a real game format, and the engine cannot survive one:
/// entering a period that carries penalties across the boundary (Second Half,
/// Overtime First/Second Half, and Sudden Death) sets that period's clock and then
/// immediately reads it back, and a zero-length countdown clock whose start instant
/// has already passed reports no time at all. `cull_penalties` in
/// `tournament_manager/mod.rs` turns that into an error, which the clock updater
/// unwraps — panicking with the game-state lock held, which poisons it and takes
/// the whole app down.
///
/// Sudden Death also culls, but is the exception, and not because it skips it: it
/// installs a counting-*up* clock, which never reports no time left.
const MIN_PERIOD_LENGTH: Duration = Duration::from_secs(1);

/// Whether the pending value is shorter than [`MIN_PERIOD_LENGTH`], for a parameter
/// where that is not a legal setting. Refused by greying out APPLY.
///
/// The rule is that **no time value in the settings may be zero** (ruling by the
/// tournament organiser, 2026-08-27, who owns this question). It rests on two
/// separate grounds, and it is worth knowing which applies where:
///
/// * The two play halves and the overtime halves genuinely crash the engine at zero,
///   as described on [`MIN_PERIOD_LENGTH`]. `Half` is constrained in both its
///   2 Halves and 1 Period forms, since both commit the same `half_play_duration`.
/// * The breaks — half-time, pre-overtime, overtime half-time, pre-sudden-death and
///   the minimum break between games — survive zero perfectly well: none of them
///   culls penalties, so the engine resolves a zero-length one in about a tenth of a
///   second. They are refused because zero is not a setting the operator should be
///   able to choose, not because anything breaks. A game with no half-time, for
///   instance, is expressed by picking 1 Period rather than 2 Halves with a zero
///   break, which is what makes that length irrelevant rather than zero.
///
/// `GameBlock` is the only length this does not constrain, and not because zero is
/// acceptable for it: `game_block_validity` already refuses any slot too short to
/// fit the game plus the minimum break, which refuses zero as a special case and
/// gives a more informative reason than this rule could.
fn param_length_too_short(param: LengthParameter, length: Duration) -> bool {
    // Exhaustive on purpose: a new `LengthParameter` must be classified here rather
    // than silently defaulting to unguarded.
    let is_constrained = match param {
        LengthParameter::Half
        | LengthParameter::HalfTime
        | LengthParameter::OvertimeHalf
        | LengthParameter::OvertimeHalfTime
        | LengthParameter::PreOvertime
        | LengthParameter::PreSuddenDeath
        | LengthParameter::MinimumBetweenGame => true,
        // Game Block is the exception, and not because zero is acceptable there: it
        // already has a stricter rule of its own (`game_block_validity`), which
        // refuses any slot too short to fit the game plus the minimum break, and so
        // refuses zero as a special case. A second overlapping rule would only
        // duplicate it, with a less informative reason.
        LengthParameter::GameBlock => false,
    };
    is_constrained && length < MIN_PERIOD_LENGTH
}

/// The colour for the value shown in the game parameter editor, or `None` for the
/// normal colour.
///
/// Red marks a value APPLY will refuse, which is what red already means on the Game
/// Block editor — so it is shown only for the lengths that are actually constrained.
/// A legal 0:00 (the minimum break, the pre-overtime / pre-sudden-death breaks, the
/// overtime half-time) stays the normal colour, because red beside a working APPLY
/// would teach the operator that red signals nothing.
///
/// The time edit screen deliberately has no equivalent: it passes `None`, and editing
/// the live game clock down to 0:00 is a legitimate thing to do.
fn param_value_color(
    game_block_validity: Option<GameBlockValidity>,
    param: LengthParameter,
    length: Duration,
) -> Option<iced::Color> {
    if param_length_too_short(param, length) {
        return Some(RED);
    }
    match game_block_validity {
        Some(GameBlockValidity::TooShort) => Some(RED),
        Some(GameBlockValidity::Tight) => Some(YELLOW),
        _ => None,
    }
}

/// Whether APPLY is offered in the game parameter editor.
///
/// Lifted out of the view so the composition itself is testable: a guard that is
/// correct but never consulted is indistinguishable from no guard at all.
fn param_apply_enabled(
    game_block_validity: Option<GameBlockValidity>,
    param: LengthParameter,
    length: Duration,
    has_changes: bool,
) -> bool {
    !matches!(game_block_validity, Some(GameBlockValidity::TooShort))
        && !param_length_too_short(param, length)
        && has_changes
}

/// Whether the Game config page's whole-page APPLY must be refused because a play
/// length already in the config is below [`MIN_PERIOD_LENGTH`].
///
/// The parameter editor's guard cannot cover this. That page's APPLY commits the
/// entire config, so an operator whose config already holds a zero-length overtime
/// half — saved by a build from before the guard existed, hand-edited, or inherited
/// from a portal "FINALS" rule, which leaves the length at zero and simply turns
/// overtime off — can re-arm the crash by flipping OVERTIME ALLOWED on and pressing
/// APPLY, without ever opening the OT HALF LENGTH editor.
///
/// Each length counts only while its own editor is reachable, so guard-active and
/// button-pressable are the same predicate and the control that clears a greyed
/// APPLY is always on screen: the overtime half only while OVERTIME ALLOWED is on
/// (its button is dead otherwise), and half-time only in 2 Halves mode (its button
/// is dead in 1 Period mode, where the value is irrelevant anyway). A dormant zero
/// behind a disabled button can never reach a period transition, and refusing APPLY
/// over one would block unrelated edits with nothing on screen to explain why.
/// CANCEL is unconditional either way.
///
/// Skipped whenever the config comes from a remote source — `uses_remote` is any
/// `GameSource` other than `Manual`, so Custom as well as Portal. There the config is
/// derived from the schedule's timing rule rather than typed in and the length grid is
/// not rendered at all, so refusing APPLY would strand the operator with no control
/// that could fix it.
fn config_has_too_short_length(config: &GameConfig, uses_remote: bool) -> bool {
    !uses_remote
        && ALL_LENGTHS
            .into_iter()
            .any(|param| config_length_refused(config, param))
}

/// Whether this committed length is a reason the Game config page's APPLY is refused,
/// which also drives the red style on its button in the length grid.
///
/// A length only counts while its own editor is reachable: half-time only in 2 Halves
/// mode and the overtime half only while OVERTIME ALLOWED is on, because otherwise the
/// button is dead, the value is not in play, and colouring it red would point the
/// operator at a control they cannot open. This is the same predicate the button's
/// `on_press` uses, which is what makes a greyed APPLY always escapable.
fn config_length_refused(config: &GameConfig, param: LengthParameter) -> bool {
    // The `&&` on each arm mirrors that button's own `on_press` condition in the
    // length grid, so a length is only ever flagged while the operator can open it.
    match param {
        LengthParameter::Half => param_length_too_short(param, config.half_play_duration),
        LengthParameter::HalfTime => {
            !config.single_half && param_length_too_short(param, config.half_time_duration)
        }
        LengthParameter::OvertimeHalf => {
            config.overtime_allowed && param_length_too_short(param, config.ot_half_play_duration)
        }
        LengthParameter::OvertimeHalfTime => {
            config.overtime_allowed && param_length_too_short(param, config.ot_half_time_duration)
        }
        LengthParameter::PreOvertime => {
            config.overtime_allowed && param_length_too_short(param, config.pre_overtime_break)
        }
        LengthParameter::PreSuddenDeath => {
            config.sudden_death_allowed
                && param_length_too_short(param, config.pre_sudden_death_duration)
        }
        LengthParameter::MinimumBetweenGame => param_length_too_short(param, config.minimum_break),
        // Game Block refuses zero through `game_block_validity` instead; see
        // `param_length_too_short`.
        LengthParameter::GameBlock => false,
    }
}

/// Every length parameter, in grid order.
///
/// Deliberately the FULL set rather than only the constrained ones:
/// `config_length_refused` already answers false for anything unconstrained, so
/// iterating everything means there is no second list of "which lengths are
/// constrained" that could drift out of step with `param_length_too_short`'s match.
///
/// Hand-maintained (there is no way to enumerate the variants without another
/// dependency); `tests::expected_constrained` matches exhaustively, so adding a
/// variant is a compile error there until it has been classified.
const ALL_LENGTHS: [LengthParameter; 8] = [
    LengthParameter::Half,
    LengthParameter::PreOvertime,
    LengthParameter::PreSuddenDeath,
    LengthParameter::HalfTime,
    LengthParameter::OvertimeHalf,
    LengthParameter::MinimumBetweenGame,
    LengthParameter::OvertimeHalfTime,
    LengthParameter::GameBlock,
];

/// The style for a length button in the Game config page's grid: red when that length
/// is refusing APPLY, mirroring what Game Block's button already does when it is too
/// short, so a greyed APPLY always has something red on the page explaining it.
fn length_button_style(
    config: &GameConfig,
    param: LengthParameter,
) -> fn(&iced::Theme, button::Status) -> button::Style {
    if config_length_refused(config, param) {
        red_button
    } else {
        light_gray_button
    }
}

/// Whether the Game config page's whole-page APPLY is offered. Lifted out of the
/// view for the same reason as [`param_apply_enabled`]: the composition is the part
/// that can silently rot, so it is the part worth a test.
fn page_apply_enabled(
    has_changes: bool,
    apply_blocked: bool,
    game_block_too_short: bool,
    config: &GameConfig,
    uses_remote: bool,
) -> bool {
    has_changes
        && !apply_blocked
        && !game_block_too_short
        && !config_has_too_short_length(config, uses_remote)
}

pub(in super::super) fn build_game_parameter_editor<'a>(
    data: ViewData<'_, '_>,
    param: LengthParameter,
    length: Duration,
    single_half: bool,
    config: &GameConfig,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    let title = match param {
        LengthParameter::Half => {
            if single_half {
                fl!("game-len")
            } else {
                fl!("half-length")
            }
        }
        LengthParameter::HalfTime => fl!("half-time-lenght"),
        LengthParameter::GameBlock => fl!("game-block"),
        LengthParameter::MinimumBetweenGame => fl!("min-break"),
        LengthParameter::PreOvertime => fl!("pre-ot-break-abreviated"),
        LengthParameter::OvertimeHalf => fl!("ot-half-len"),
        LengthParameter::OvertimeHalfTime => fl!("ot-half-tm-len"),
        LengthParameter::PreSuddenDeath => fl!("pre-sd-break"),
    };

    // Live Game Block validation: build a staged copy of the config with the
    // value currently being edited (and the staged 2-halves/1-period choice)
    // so the colour and the disabled Done button reflect the pending edit, not
    // the saved config. Only the Game Block editor validates; other parameters
    // get None (no colour, Done always enabled, no note).
    let game_block_validity = if matches!(param, LengthParameter::GameBlock) {
        let staged = GameConfig {
            game_block: length,
            single_half,
            ..config.clone()
        };
        Some(game_block_validity(&staged))
    } else {
        None
    };
    let value_color = param_value_color(game_block_validity, param, length);
    let validity_note: Option<Element<'a, Message>> = match game_block_validity {
        Some(GameBlockValidity::TooShort) => Some(
            text(fl!("game-block-too-short"))
                .size(SMALL_TEXT)
                .color(RED)
                .align_x(Horizontal::Center)
                .into(),
        ),
        Some(GameBlockValidity::Tight) => Some(
            text(fl!("game-block-tight"))
                .size(SMALL_TEXT)
                .color(YELLOW)
                .align_x(Horizontal::Center)
                .into(),
        ),
        _ => None,
    };

    // For the Half Length editor, offer a 2 Halves / 1 Period selector above the
    // time keypad. The active segment is highlighted (blue) and not pressable;
    // the inactive segment is gray and emits the SingleHalf toggle, which flips
    // the staged choice. Other length parameters have no selector.
    let format_selector: Option<Element<'a, Message>> = if matches!(param, LengthParameter::Half) {
        // Both segments stay pressable so the active one renders in the full
        // blue "selected" style (a button with no on_press is drawn disabled).
        // The active segment's press is a no-op; the inactive one toggles.
        let two_halves = {
            let b = make_chrome_button(fl!("two-halves"))
                .width(Length::Fill)
                .style(if single_half {
                    light_gray_button
                } else {
                    blue_selected_button
                });
            if single_half {
                b.on_press(Message::ToggleBoolParameter(BoolGameParameter::SingleHalf))
            } else {
                b.on_press(Message::NoAction)
            }
        };
        let one_period = {
            let b = make_chrome_button(fl!("one-period"))
                .width(Length::Fill)
                .style(if single_half {
                    blue_selected_button
                } else {
                    light_gray_button
                });
            if single_half {
                b.on_press(Message::NoAction)
            } else {
                b.on_press(Message::ToggleBoolParameter(BoolGameParameter::SingleHalf))
            }
        };
        Some(row![two_halves, one_period].spacing(SPACING).into())
    } else {
        None
    };

    let mut col = column![make_game_time_button(
        snapshot,
        false,
        false,
        mode,
        clock_running,
        portal_indicator,
        None
    )]
    .spacing(SPACING)
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill);

    if let Some(selector) = format_selector {
        col = col.push(selector);
    }

    let help_button = make_small_button("?", MEDIUM_TEXT)
        .style(blue_button)
        .on_press(Message::ShowParameterHelp);

    // Time editor stays centred between two balancing spacers; the ? button sits
    // top-right (its width matched by the fixed-width spacer on the left), and
    // align_y(Top) pins it to the top of the row.
    let editor_row = row![
        horizontal_space().width(Length::Fixed(MIN_BUTTON_SIZE)),
        horizontal_space(),
        make_time_editor(title, length, false, value_color),
        horizontal_space(),
        help_button,
    ]
    .spacing(SPACING)
    .align_y(Vertical::Top);

    col = col.push(editor_row);

    if let Some(note) = validity_note {
        col = col.push(note);
    }

    // The original value shown when the editor opened, re-derived from the seed
    // config (the in-progress edited config, or the saved config) — the same
    // source `EditParameter` used to populate the editor. Apply is enabled only
    // when the operator has actually changed something, the pending value is not
    // too short for Game Block, and not below `MIN_PERIOD_LENGTH` for any of the
    // lengths `param_length_too_short` constrains.
    let old_length = match param {
        LengthParameter::Half => config.half_play_duration,
        LengthParameter::HalfTime => config.half_time_duration,
        LengthParameter::GameBlock => config.game_block,
        LengthParameter::MinimumBetweenGame => config.minimum_break,
        LengthParameter::PreOvertime => config.pre_overtime_break,
        LengthParameter::OvertimeHalf => config.ot_half_play_duration,
        LengthParameter::OvertimeHalfTime => config.ot_half_time_duration,
        LengthParameter::PreSuddenDeath => config.pre_sudden_death_duration,
    };
    let has_changes =
        param_edit_has_changes(length, old_length, param, single_half, config.single_half);
    let apply_enabled = param_apply_enabled(game_block_validity, param, length, has_changes);

    col.push(vertical_space())
        .push(
            row![
                make_chrome_button(cancel_or_back_label(has_changes))
                    .style(red_button)
                    .width(Length::Fill)
                    .on_press(Message::ParameterEditComplete { canceled: true }),
                horizontal_space(),
                make_chrome_button(fl!("apply"))
                    .style(green_button)
                    .width(Length::Fill)
                    .on_press_maybe(
                        apply_enabled.then_some(Message::ParameterEditComplete { canceled: false }),
                    ),
            ]
            .spacing(SPACING),
        )
        .into()
}

pub(in super::super) fn build_parameter_help_page<'a>(
    data: ViewData<'_, '_>,
    param: LengthParameter,
    _length: Duration,
    single_half: bool,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    // Title reuses the editor's short, already-translated label; body is the
    // existing hint string. No new translation keys are introduced.
    let (title, body) = match param {
        LengthParameter::Half => (
            if single_half {
                fl!("game-len")
            } else {
                fl!("half-length")
            },
            if single_half {
                fl!("length-of-game-during-regular-play")
            } else {
                fl!("length-of-half-during-regular-play")
            },
        ),
        LengthParameter::HalfTime => (fl!("half-time-lenght"), fl!("length-of-half-time-period")),
        LengthParameter::GameBlock => (fl!("game-block"), fl!("game-block-help")),
        LengthParameter::MinimumBetweenGame => (fl!("min-break"), fl!("min-time-btwn-games")),
        LengthParameter::PreOvertime => (fl!("pre-ot-break-abreviated"), fl!("pre-sd-brk")),
        LengthParameter::OvertimeHalf => (fl!("ot-half-len"), fl!("time-during-ot")),
        LengthParameter::OvertimeHalfTime => {
            (fl!("ot-half-tm-len"), fl!("len-of-overtime-halftime"))
        }
        LengthParameter::PreSuddenDeath => (fl!("pre-sd-break"), fl!("pre-sd-len")),
    };
    let body = body.replace('\n', " ");

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        container(text(title).size(MEDIUM_TEXT)).center_x(Length::Fill),
        text(body).size(SMALL_TEXT).width(Length::Fill),
        vertical_space(),
        row![
            make_chrome_button(fl!("back"))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::CloseParameterHelp),
            horizontal_space(),
            horizontal_space(),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}

fn font_family_id(lang: Language) -> u8 {
    match lang {
        Language::Korean | Language::Japanese | Language::Mandarin => 1,
        Language::Thai => 2,
        _ => 0,
    }
}

fn make_buzzer_select_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
    portal_indicator: Option<PortalIndicatorState>,
) -> Element<'a, Message> {
    let selected = settings.sound.buzzer_sound;
    let has_changes = page_has_changes(ConfigPage::Buzzer, settings, page_entry_snapshot);

    let mut grid = column![make_game_time_button(
        snapshot,
        false,
        false,
        mode,
        clock_running,
        portal_indicator,
        None
    )]
    .spacing(SPACING)
    .height(Length::Fill);

    // 12 sounds laid out in 3 rows of 4, mirroring the Language page's
    // row-per-row grid structure. Shared with the beep-test buzzer picker so the
    // two cannot drift apart; only the message differs.
    for r in make_buzzer_grid_rows(selected, Message::SelectBuzzer) {
        grid = grid.push(r);
    }

    // One trailing filler row for vertical balance above the footer. This page
    // carries the "next game" ribbon across the top (the beep-test buzzer picker
    // does not), so a single filler here gives the same balance that picker gets
    // from its three filler rows.
    grid = grid.push(row![horizontal_space()].height(Length::Fill));

    // Footer: Cancel | TEST | Apply (Apply gated by page_has_changes).
    let cancel = make_chrome_button(fl!("cancel"))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::CancelConfigPage(ConfigPage::Buzzer));
    let test = make_chrome_button(fl!("test"))
        .style(blue_button)
        .width(Length::Fill)
        .on_press(Message::TestBuzzer);
    let apply = {
        let b = make_chrome_button(fl!("apply"))
            .style(green_button)
            .width(Length::Fill);
        if has_changes {
            b.on_press(Message::ApplyConfigPage(ConfigPage::Buzzer))
        } else {
            b
        }
    };

    grid.push(row![cancel, test, apply].spacing(SPACING)).into()
}

/// The custom site's URL editor, reached from the SITE row.
///
/// This holds the only text input in the application. The spacebar buzzer
/// handler is already gated to the main screen (`mod.rs`, with a comment saying
/// the gate exists so text inputs are unaffected), so typing a space here does
/// not sound the buzzer.
#[allow(clippy::too_many_arguments)]
fn make_custom_site_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    show_invalid: bool,
    mode: Mode,
    clock_running: bool,
    // Kept, though this page's APPLY no longer consults it: the snapshot is what
    // CANCEL reverts to, and the Cancel/Back label rollout will want it here.
    _page_entry_snapshot: Option<&PageEntrySnapshot>,
    portal_indicator: Option<PortalIndicatorState>,
) -> Element<'a, Message> {
    let mut col = column![make_game_time_button(
        snapshot,
        false,
        false,
        mode,
        clock_running,
        portal_indicator,
        None
    )]
    .spacing(SPACING)
    .height(Length::Fill);

    col = col.push(centered_text(fl!("custom-site-url-title")));

    col = col.push(
        text_input(&fl!("custom-site-placeholder"), &settings.custom_site.url)
            .on_input(Message::CustomSiteUrlChanged)
            .padding(PADDING)
            .size(MEDIUM_TEXT)
            .width(Length::Fill),
    );

    // The rejection message replaces empty space rather than appearing above the
    // footer, so the buttons never move under the operator's finger.
    //
    // This paragraph must stay anchored LEFT, and that is not a style choice.
    //
    // iced 0.13 computes a paragraph's visible bounds by applying the alignment
    // offset with the clipped width, while drawing applies it with the full
    // width. A centre-anchored paragraph is therefore clipped half a text-width
    // from where it draws: measured on screen, this sentence showed only its
    // middle ~476px of ~950px and lost both ends. It is not a transient repaint
    // — it survives a window resize — and it is not the box, which was proved
    // large enough by giving it a temporary background. Only removing
    // `align_x(Center)` fixed it. `centered_text` is avoided here for the same
    // reason plus the `align_y(Center)` + `height(Fill)` artifact it carries, so
    // the container does the vertical centring. Every other label on this page
    // is short enough to fit its own half and so never showed this.
    col = col.push(
        container(
            text(if show_invalid {
                fl!("custom-site-invalid")
            } else {
                String::new()
            })
            .width(Length::Fill),
        )
        .width(Length::Fill)
        .center_y(Length::Fill)
        .padding(PADDING),
    );

    let cancel = make_chrome_button(fl!("cancel"))
        .style(red_button)
        .width(Length::Fill)
        .on_press(Message::CancelConfigPage(ConfigPage::CustomSite(
            show_invalid,
        )));
    // Deliberately always available, unlike every other page's APPLY, which
    // greys until something is edited. Re-applying an unchanged address is how an
    // operator gets back onto their site after the event was cleared — the path
    // back from MANUAL — and greying it there left them stranded with the address
    // they wanted sitting on screen and no control that would act on it. Applying
    // an unchanged address is idempotent: it re-validates, re-commits the same
    // value, and repoints nothing, so offering it always costs nothing.
    let apply = make_chrome_button(fl!("apply"))
        .style(green_button)
        .width(Length::Fill)
        .on_press(Message::ApplyConfigPage(ConfigPage::CustomSite(
            show_invalid,
        )));

    col.push(row![cancel, apply].spacing(SPACING)).into()
}

fn make_language_select_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
    portal_indicator: Option<PortalIndicatorState>,
) -> Element<'a, Message> {
    let selected = settings.pending_language.unwrap_or(Language::English);
    let original = settings.original_language.unwrap_or(Language::English);
    let apply_enabled = page_has_changes(ConfigPage::Language, settings, page_entry_snapshot);

    // Font to apply to Cancel/Apply/Restart text so they render in the target language's script
    // regardless of the app's current default font. Without an explicit Latin arm, Turkish text
    // like "İPTAL" or "BAŞLAT" renders as tofu when the app is currently in a CJK/Thai locale.
    let selected_font: Option<iced_core::Font> = match selected {
        Language::Korean | Language::Japanese | Language::Mandarin => Some(CJK_FONT),
        Language::Thai => Some(THAI_FONT),
        _ => Some(LATIN_FONT),
    };

    // A restart is needed when switching between Latin and CJK font families.
    let needs_restart = font_family_id(original) != font_family_id(selected);

    let [lang_row_1, lang_row_2, lang_row_3, lang_row_4] = make_language_grid_rows(selected);

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None
        ),
        lang_row_1,
        lang_row_2,
        lang_row_3,
        lang_row_4,
        {
            // Use align_x(Left) + width(Shrink) + outer container centering for all
            // dynamic text in these buttons. This ensures iced's damage tracking
            // region starts from the text's left edge, so old glyph pixels are fully
            // cleared when content changes on language switch.
            let make_label = |content: &'static str, font: Option<iced_core::Font>| {
                let t = text(content)
                    .align_x(Horizontal::Left)
                    .align_y(Vertical::Top)
                    .width(Length::Shrink);
                let t: iced::widget::Text<'a, _, _> =
                    if let Some(f) = font { t.font(f) } else { t };
                container(t).center(Length::Fill)
            };

            // `apply_enabled` here is exactly page_has_changes(ConfigPage::Language, …)
            // (the Language page has no extra Apply gate), so it doubles as the
            // has-changes signal for the Cancel/Back swap.
            let footer_label = if apply_enabled {
                selected.cancel_text()
            } else {
                selected.back_text()
            };
            let cancel_btn = button(make_label(footer_label, selected_font))
                .padding(PADDING)
                .height(Length::Fixed(MIN_BUTTON_SIZE))
                .style(red_button)
                .width(Length::Fill)
                .on_press(Message::LanguageSelectComplete { canceled: true });

            let confirm_msg =
                apply_enabled.then_some(Message::LanguageSelectComplete { canceled: false });
            let confirm_btn: Element<'a, Message> = if needs_restart {
                button(make_label(selected.restart_text(), selected_font))
                    .padding(PADDING)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .style(blue_button)
                    .width(Length::Fill)
                    .on_press_maybe(confirm_msg)
                    .into()
            } else {
                button(make_label(selected.apply_text(), selected_font))
                    .padding(PADDING)
                    .height(Length::Fixed(MIN_BUTTON_SIZE))
                    .style(green_button)
                    .width(Length::Fill)
                    .on_press_maybe(confirm_msg)
                    .into()
            };

            row![cancel_btn, horizontal_space(), confirm_btn]
        }
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

pub(in super::super) fn make_updates_page<'a>(
    data: ViewData<'_, '_>,
    state: &UpdateUiState,
    backup_available: bool,
    available_version: Option<crate::updater::version::Version>,
    backup_version: Option<crate::updater::version::Version>,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock_running,
        portal_indicator,
        ..
    } = data;

    let is_progress = matches!(
        state,
        UpdateUiState::Checking
            | UpdateUiState::Downloading
            | UpdateUiState::Verifying
            | UpdateUiState::Installing
            | UpdateUiState::Restarting
    );
    let is_confirm = matches!(state, UpdateUiState::RevertConfirm);

    // 1. Time banner
    let time_banner = make_game_time_button(
        snapshot,
        false,
        false,
        mode,
        clock_running,
        portal_indicator,
        None,
    );

    // 2. Current version (left half) + primary action button (right half)
    // The Updates page is a column of one-off actions, not content tiles: every
    // button on it is furniture, so they all stay chrome and keep their size.
    // (Their labels still shift down a few pixels, like every value button —
    // see the row comment in `make_value_button`.)
    let version_element: Element<'a, Message> = make_value_chrome_button(
        fl!("updates-current-version"),
        env!("CARGO_PKG_VERSION"),
        (false, true),
        None,
    )
    .into();
    let primary_element: Element<'a, Message> = match state {
        UpdateUiState::Checking
        | UpdateUiState::Downloading
        | UpdateUiState::Verifying
        | UpdateUiState::Installing
        | UpdateUiState::Restarting => horizontal_space().into(),
        _ => make_chrome_button(fl!("updates-check-for-updates"))
            .style(yellow_button)
            .width(Length::Fill)
            .on_press(Message::UpdatesCheck)
            .into(),
    };
    let version_primary_row = row![version_element, primary_element]
        .spacing(SPACING)
        .height(Length::Fill);

    // 3. Status line
    let status_text: String = match state {
        UpdateUiState::Unknown => fl!("updates-unknown"),
        UpdateUiState::RolledBack => fl!("updates-rolled-back"),
        UpdateUiState::Checking => fl!("updates-checking"),
        UpdateUiState::UpToDate => fl!("updates-up-to-date"),
        UpdateUiState::UpdateAvailable => fl!(
            "updates-available",
            version = available_version.map(|v| v.to_string()).unwrap_or_default()
        ),
        UpdateUiState::Downloading => fl!("updates-downloading"),
        UpdateUiState::Verifying => fl!("updates-verifying"),
        UpdateUiState::Installing => fl!("updates-installing"),
        UpdateUiState::Restarting => fl!("updates-restarting"),
        UpdateUiState::RevertConfirm => fl!(
            "updates-confirm-revert",
            version = backup_version.map(|v| v.to_string()).unwrap_or_default()
        ),
        UpdateUiState::Error(UpdateUiError::NoInternet) => fl!("updates-error-no-internet"),
        UpdateUiState::Error(UpdateUiError::RateLimited) => fl!("updates-error-rate-limited"),
        UpdateUiState::Error(UpdateUiError::BadDownload) => fl!("updates-error-bad-download"),
        UpdateUiState::Error(UpdateUiError::NoSpace) => fl!("updates-error-no-space"),
        UpdateUiState::Error(UpdateUiError::NotWritable) => fl!("updates-error-not-writable"),
    };
    let status_row = row![text(status_text).size(MEDIUM_TEXT).width(Length::Fill)].spacing(SPACING);

    // 3b. Note line: explains what the bottom-right action button will do when an
    // update is available / in the revert-confirm view. Empty spacer otherwise.
    let note_text: Option<String> = match state {
        UpdateUiState::UpdateAvailable => Some(fl!("updates-install-note")),
        UpdateUiState::RevertConfirm => Some(fl!("updates-revert-note")),
        _ => None,
    };
    let note_row: Element<'a, Message> = match note_text {
        Some(t) => row![text(t).size(SMALL_TEXT).width(Length::Fill)]
            .spacing(SPACING)
            .into(),
        None => row![horizontal_space()].into(),
    };

    // 4. The "blank row": a Revert button when a backup exists and state is idle,
    // otherwise the same blank-spacer idiom the other config pages use.
    let show_revert = backup_available
        && matches!(
            state,
            UpdateUiState::Unknown | UpdateUiState::UpToDate | UpdateUiState::UpdateAvailable
        );
    let blank_or_revert_row: Element<'a, Message> = if show_revert {
        row![
            make_chrome_button(fl!(
                "updates-revert",
                version = backup_version.map(|v| v.to_string()).unwrap_or_default()
            ))
            .style(light_gray_button)
            .width(Length::Fill)
            .on_press(Message::UpdatesRevert),
        ]
        .spacing(SPACING)
        .height(Length::Fill)
        .into()
    } else {
        row![horizontal_space()].height(Length::Fill).into()
    };

    // 5. Footer: Back (idle) / Cancel (progress|confirm) / disabled Back (Restarting|Installing).
    let footer_label = if (is_progress
        && !matches!(state, UpdateUiState::Restarting | UpdateUiState::Installing))
        || is_confirm
    {
        fl!("cancel")
    } else {
        fl!("back")
    };
    let footer_btn = make_chrome_button(footer_label).style(red_button);
    let footer_btn = if matches!(state, UpdateUiState::Restarting | UpdateUiState::Installing) {
        footer_btn
    } else {
        footer_btn.on_press(Message::UpdatesBack)
    };

    // bottom-right action: Install (when an update is available) / Revert (in the
    // revert-confirm view) / nothing otherwise. Green like Apply.
    let footer_action: Element<'a, Message> = match state {
        UpdateUiState::UpdateAvailable => make_chrome_button(fl!("updates-install"))
            .style(green_button)
            .width(Length::Fill)
            .on_press(Message::UpdatesConfirmInstall)
            .into(),
        UpdateUiState::RevertConfirm => make_chrome_button(fl!("updates-do-revert"))
            .style(green_button)
            .width(Length::Fill)
            .on_press(Message::UpdatesConfirmRevert)
            .into(),
        _ => horizontal_space().into(),
    };
    let footer_row = row![footer_btn, horizontal_space(), footer_action].spacing(SPACING);

    column![
        time_banner,
        version_primary_row,
        status_row,
        note_row,
        blank_or_revert_row,
        footer_row,
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PageEntrySnapshot;
    use crate::config::Mode;
    use matrix_drawing::transmitted_data::Brightness;
    use time::macros::datetime;
    use uwh_common::uwhportal::schedule::{Game, ScheduledTeam, TeamId};

    fn make_schedule_with_one_game(event_id: EventId, game_number: &str, court: &str) -> Schedule {
        let game = Game {
            number: game_number.to_string(),
            dark: ScheduledTeam::new_team_id(TeamId::from_partial("dark")),
            light: ScheduledTeam::new_team_id(TeamId::from_partial("light")),
            start_time: datetime!(2026-01-01 0:00 UTC),
            court: court.to_string(),
            timing_rule: "RR".to_string(),
            referee_assignments: None,
            description: None,
        };
        Schedule {
            event_id,
            games: std::iter::once((game.number.clone(), game)).collect(),
            non_game_entries: vec![],
            groups: vec![],
            timing_rules: vec![],
            standings_order: None,
            final_results_order: None,
            referees_by_game_number: None,
        }
    }

    #[test]
    fn param_edit_no_change_is_false() {
        let d = Duration::from_secs(900);
        assert!(!param_edit_has_changes(
            d,
            d,
            LengthParameter::Half,
            false,
            false
        ));
    }

    #[test]
    fn param_edit_length_change_is_true() {
        assert!(param_edit_has_changes(
            Duration::from_secs(901),
            Duration::from_secs(900),
            LengthParameter::HalfTime,
            false,
            false
        ));
    }

    #[test]
    fn param_edit_single_half_toggle_on_half_is_true() {
        let d = Duration::from_secs(900);
        assert!(param_edit_has_changes(
            d,
            d,
            LengthParameter::Half,
            true,
            false
        ));
    }

    #[test]
    fn param_edit_single_half_ignored_off_half() {
        // single_half differs but the parameter is not Half, so it is not
        // committed and must not count as a change.
        let d = Duration::from_secs(900);
        assert!(!param_edit_has_changes(
            d,
            d,
            LengthParameter::GameBlock,
            true,
            false
        ));
    }

    #[test]
    fn param_edit_sub_second_matches_displayed_whole_seconds() {
        // Original 900.6s displays as "15:00" (900 whole seconds); rebuilding to
        // an exact 900.0s lands on the same displayed value, so it is not a change.
        assert!(!param_edit_has_changes(
            Duration::from_secs(900),
            Duration::from_millis(900_600),
            LengthParameter::HalfTime,
            false,
            false
        ));
    }

    #[test]
    fn zero_half_length_is_too_short() {
        // A zero-length half crashes the engine the moment the second half is
        // entered, so APPLY must refuse to commit it.
        assert!(param_length_too_short(
            LengthParameter::Half,
            Duration::ZERO
        ));
    }

    #[test]
    fn near_zero_half_length_is_too_short() {
        // The +/- buttons floor a length just above zero, which still displays as
        // 0:00 and crashes exactly like a true zero.
        assert!(param_length_too_short(
            LengthParameter::Half,
            Duration::from_micros(1)
        ));
    }

    #[test]
    fn zero_overtime_half_length_is_too_short() {
        assert!(param_length_too_short(
            LengthParameter::OvertimeHalf,
            Duration::ZERO
        ));
    }

    #[test]
    fn zero_half_time_length_is_too_short() {
        // Not a crash — half-time is not a period that culls penalties, so the engine
        // resolves a zero-length one cleanly — but not a legal setting either: a game
        // with no half-time is 1 Period, not 2 Halves with a zero break.
        assert!(param_length_too_short(
            LengthParameter::HalfTime,
            Duration::ZERO
        ));
        assert!(param_length_too_short(
            LengthParameter::HalfTime,
            Duration::from_micros(1)
        ));
    }

    #[test]
    fn one_second_half_time_length_is_allowed() {
        assert!(!param_length_too_short(
            LengthParameter::HalfTime,
            MIN_PERIOD_LENGTH
        ));
    }

    #[test]
    fn one_second_half_length_is_allowed() {
        assert!(!param_length_too_short(
            LengthParameter::Half,
            Duration::from_secs(1)
        ));
    }

    /// Whether this module's one-second rule constrains `param`, stated independently
    /// of the implementation so the tests measure against an expectation rather than
    /// against the code under test.
    ///
    /// Exhaustive on purpose: a new `LengthParameter` variant is a compile error here
    /// until someone decides whether it is a time value that may not be zero.
    fn expected_constrained(param: LengthParameter) -> bool {
        match param {
            LengthParameter::Half
            | LengthParameter::HalfTime
            | LengthParameter::OvertimeHalf
            | LengthParameter::OvertimeHalfTime
            | LengthParameter::PreOvertime
            | LengthParameter::PreSuddenDeath
            | LengthParameter::MinimumBetweenGame => true,
            LengthParameter::GameBlock => false,
        }
    }

    #[test]
    fn every_length_is_constrained_except_game_block() {
        // No time value in the settings may be zero. Game Block is the sole exception
        // to THIS rule because it has a stricter one of its own — it must fit the game
        // plus the minimum break — which already refuses zero.
        for param in ALL_LENGTHS {
            let constrained = expected_constrained(param);
            assert_eq!(
                param_length_too_short(param, Duration::ZERO),
                constrained,
                "{param:?} at 0:00"
            );
            // The value the +/- buttons actually reach, which still displays as 0:00.
            assert_eq!(
                param_length_too_short(param, Duration::from_micros(1)),
                constrained,
                "{param:?} just above zero"
            );
            assert!(
                !param_length_too_short(param, MIN_PERIOD_LENGTH),
                "{param:?} must accept the one-second minimum"
            );
        }
    }

    /// A config with the two play lengths and the overtime toggle set explicitly;
    /// every other field stays at its default.
    fn cfg_with(half: Duration, ot_half: Duration, overtime_allowed: bool) -> GameConfig {
        GameConfig {
            half_play_duration: half,
            ot_half_play_duration: ot_half,
            overtime_allowed,
            ..Default::default()
        }
    }

    #[test]
    fn apply_is_refused_for_a_too_short_length() {
        // Guards the WIRING, not the helper: deleting the too-short clause from
        // `param_apply_enabled` must fail a test, or a correct guard that is never
        // consulted is indistinguishable from no guard at all.
        assert!(!param_apply_enabled(
            None,
            LengthParameter::Half,
            Duration::ZERO,
            true
        ));
        assert!(!param_apply_enabled(
            None,
            LengthParameter::OvertimeHalf,
            Duration::ZERO,
            true
        ));
        assert!(!param_apply_enabled(
            None,
            LengthParameter::HalfTime,
            Duration::ZERO,
            true
        ));
    }

    #[test]
    fn apply_is_offered_at_the_allowed_boundary() {
        assert!(param_apply_enabled(
            None,
            LengthParameter::Half,
            MIN_PERIOD_LENGTH,
            true
        ));
        assert!(param_apply_enabled(
            None,
            LengthParameter::OvertimeHalf,
            MIN_PERIOD_LENGTH,
            true
        ));
    }

    #[test]
    fn apply_still_respects_the_pre_existing_gates() {
        let ok = Duration::from_secs(900);
        // Nothing edited yet.
        assert!(!param_apply_enabled(None, LengthParameter::Half, ok, false));
        // A too-short Game Block still blocks, as it did before this guard.
        assert!(!param_apply_enabled(
            Some(GameBlockValidity::TooShort),
            LengthParameter::GameBlock,
            ok,
            true
        ));
        // "Tight" is a caution, not invalid, and must still be committable.
        assert!(param_apply_enabled(
            Some(GameBlockValidity::Tight),
            LengthParameter::GameBlock,
            ok,
            true
        ));
    }

    #[test]
    fn page_apply_refused_for_a_zero_half_length() {
        let cfg = cfg_with(Duration::ZERO, Duration::from_secs(300), true);
        assert!(config_has_too_short_length(&cfg, false));
    }

    #[test]
    fn page_apply_refused_when_overtime_is_on_beside_a_zero_overtime_half() {
        // The hole the parameter-editor guard cannot see: the length was already
        // zero, and the operator re-arms it with the OVERTIME ALLOWED toggle
        // without ever opening the OT HALF LENGTH editor.
        let cfg = cfg_with(Duration::from_secs(900), Duration::ZERO, true);
        assert!(config_has_too_short_length(&cfg, false));
    }

    #[test]
    fn page_apply_allowed_for_a_dormant_zero_overtime_half() {
        // Overtime off: the zero can never reach a period transition, and blocking
        // APPLY over it would stop unrelated edits with nothing on screen to explain
        // why.
        let cfg = cfg_with(Duration::from_secs(900), Duration::ZERO, false);
        assert!(!config_has_too_short_length(&cfg, false));
    }

    #[test]
    fn page_apply_allowed_in_portal_mode() {
        // Portal mode does not offer the length buttons, so refusing APPLY would
        // strand the operator with no control that could fix it.
        let cfg = cfg_with(Duration::ZERO, Duration::ZERO, true);
        assert!(!config_has_too_short_length(&cfg, true));
    }

    #[test]
    fn refused_lengths_show_their_value_in_red() {
        for param in ALL_LENGTHS {
            // Game Block takes its colour from `game_block_validity` instead, so with
            // no validity passed it stays uncoloured; see the test below.
            let expected = expected_constrained(param).then_some(RED);
            assert_eq!(
                param_value_color(None, param, Duration::ZERO),
                expected,
                "{param:?} at 0:00"
            );
            // The value the +/- buttons actually reach also displays as 0:00.
            assert_eq!(
                param_value_color(None, param, Duration::from_micros(1)),
                expected,
                "{param:?} just above zero"
            );
            assert_eq!(
                param_value_color(None, param, MIN_PERIOD_LENGTH),
                None,
                "{param:?} at the minimum must not be red"
            );
        }
    }

    #[test]
    fn game_block_takes_its_colour_from_its_own_rule() {
        // Game Block is not constrained by `param_length_too_short`, so its colour
        // must come from `game_block_validity` alone — which is what the view always
        // passes for that parameter.
        assert_eq!(
            param_value_color(
                Some(GameBlockValidity::TooShort),
                LengthParameter::GameBlock,
                Duration::ZERO
            ),
            Some(RED),
            "a zero Game Block is red because it is too short to fit the game"
        );
    }

    #[test]
    fn game_block_keeps_its_own_colours() {
        let ok = Duration::from_secs(2880);
        assert_eq!(
            param_value_color(
                Some(GameBlockValidity::TooShort),
                LengthParameter::GameBlock,
                ok
            ),
            Some(RED)
        );
        assert_eq!(
            param_value_color(
                Some(GameBlockValidity::Tight),
                LengthParameter::GameBlock,
                ok
            ),
            Some(YELLOW)
        );
        assert_eq!(
            param_value_color(Some(GameBlockValidity::Ok), LengthParameter::GameBlock, ok),
            None
        );
    }

    #[test]
    fn a_refused_length_is_flagged_for_its_own_button() {
        // Half is always in play.
        let half_zero = cfg_with(Duration::ZERO, Duration::from_secs(300), true);
        assert!(config_length_refused(&half_zero, LengthParameter::Half));

        // The overtime half counts only while overtime is on — its button is dead
        // otherwise, so red would point at a control the operator cannot open.
        let ot_on = cfg_with(Duration::from_secs(600), Duration::ZERO, true);
        assert!(config_length_refused(&ot_on, LengthParameter::OvertimeHalf));
        let ot_off = cfg_with(Duration::from_secs(600), Duration::ZERO, false);
        assert!(!config_length_refused(
            &ot_off,
            LengthParameter::OvertimeHalf
        ));

        // Half-time counts only in 2 Halves mode, for the same reason.
        let two_halves = GameConfig {
            half_time_duration: Duration::ZERO,
            ..Default::default()
        };
        assert!(config_length_refused(
            &two_halves,
            LengthParameter::HalfTime
        ));
        let one_period = GameConfig {
            half_time_duration: Duration::ZERO,
            single_half: true,
            ..Default::default()
        };
        assert!(!config_length_refused(
            &one_period,
            LengthParameter::HalfTime
        ));
    }

    #[test]
    fn game_block_never_refuses_apply_through_this_rule() {
        let zeroed = GameConfig {
            game_block: Duration::ZERO,
            ..Default::default()
        };
        assert!(!config_length_refused(&zeroed, LengthParameter::GameBlock));
    }

    #[test]
    fn a_length_is_only_flagged_while_its_own_button_is_live() {
        // Each arm must match that button's `on_press` condition, or the operator can
        // face a greyed APPLY with no control able to clear it.
        let all_zero = GameConfig {
            pre_overtime_break: Duration::ZERO,
            ot_half_time_duration: Duration::ZERO,
            pre_sudden_death_duration: Duration::ZERO,
            minimum_break: Duration::ZERO,
            ..Default::default()
        };

        // Minimum break is always editable, so it always counts.
        assert!(config_length_refused(
            &all_zero,
            LengthParameter::MinimumBetweenGame
        ));

        // The overtime pair is dead while overtime is off.
        let ot_off = GameConfig {
            overtime_allowed: false,
            ..all_zero.clone()
        };
        assert!(!config_length_refused(
            &ot_off,
            LengthParameter::PreOvertime
        ));
        assert!(!config_length_refused(
            &ot_off,
            LengthParameter::OvertimeHalfTime
        ));
        let ot_on = GameConfig {
            overtime_allowed: true,
            ..all_zero.clone()
        };
        assert!(config_length_refused(&ot_on, LengthParameter::PreOvertime));
        assert!(config_length_refused(
            &ot_on,
            LengthParameter::OvertimeHalfTime
        ));

        // The pre-sudden-death break is dead while sudden death is off.
        let sd_off = GameConfig {
            sudden_death_allowed: false,
            ..all_zero.clone()
        };
        assert!(!config_length_refused(
            &sd_off,
            LengthParameter::PreSuddenDeath
        ));
        let sd_on = GameConfig {
            sudden_death_allowed: true,
            ..all_zero
        };
        assert!(config_length_refused(
            &sd_on,
            LengthParameter::PreSuddenDeath
        ));
    }

    #[test]
    fn page_apply_refused_for_a_zero_half_time_in_two_half_mode() {
        let cfg = GameConfig {
            half_time_duration: Duration::ZERO,
            ..Default::default()
        };
        assert!(
            !cfg.single_half,
            "default must be 2 Halves or this case proves nothing"
        );
        assert!(config_has_too_short_length(&cfg, false));
    }

    #[test]
    fn page_apply_allowed_for_a_zero_half_time_in_one_period_mode() {
        // 1 Period mode disables the HALF TIME LENGTH button, so refusing APPLY over
        // it would strand the operator — and the value is irrelevant in that mode.
        let cfg = GameConfig {
            half_time_duration: Duration::ZERO,
            single_half: true,
            ..Default::default()
        };
        assert!(!config_has_too_short_length(&cfg, false));
    }

    #[test]
    fn page_apply_allowed_for_a_normal_config() {
        assert!(!config_has_too_short_length(&GameConfig::default(), false));
    }

    #[test]
    fn page_apply_is_refused_for_a_too_short_play_length() {
        // Guards the WIRING of `config_has_too_short_length` into the page's APPLY,
        // the same gap `apply_is_refused_for_a_too_short_length` closes for the
        // editor.
        let bad = cfg_with(Duration::ZERO, Duration::from_secs(300), true);
        assert!(!page_apply_enabled(true, false, false, &bad, false));

        // The gates that were already there must still behave.
        let ok = GameConfig::default();
        assert!(page_apply_enabled(true, false, false, &ok, false));
        assert!(!page_apply_enabled(false, false, false, &ok, false)); // nothing edited
        assert!(!page_apply_enabled(true, true, false, &ok, false)); // portal incomplete
        assert!(!page_apply_enabled(true, false, true, &ok, false)); // game block too short
    }

    #[test]
    fn display_no_changes_when_buffer_equals_snapshot() {
        let edited = EditableSettings {
            white_on_right: false,
            brightness: Brightness::Medium,
            ..Default::default()
        };
        let snap = PageEntrySnapshot::Display {
            white_on_right: false,
            brightness: Brightness::Medium,
            front_display_layout: FrontDisplayLayout::Default,
        };
        assert!(!page_has_changes(ConfigPage::Display, &edited, Some(&snap)));
    }

    #[test]
    fn display_detects_brightness_change() {
        let edited = EditableSettings {
            white_on_right: false,
            brightness: Brightness::High,
            ..Default::default()
        };
        let snap = PageEntrySnapshot::Display {
            white_on_right: false,
            brightness: Brightness::Medium,
            front_display_layout: FrontDisplayLayout::Default,
        };
        assert!(page_has_changes(ConfigPage::Display, &edited, Some(&snap)));
    }

    #[test]
    fn display_detects_layout_change() {
        let edited = EditableSettings {
            white_on_right: false,
            brightness: Brightness::Medium,
            front_display_layout: FrontDisplayLayout::Corners,
            ..Default::default()
        };
        let snap = PageEntrySnapshot::Display {
            white_on_right: false,
            brightness: Brightness::Medium,
            front_display_layout: FrontDisplayLayout::Default,
        };
        assert!(page_has_changes(ConfigPage::Display, &edited, Some(&snap)));
    }

    #[test]
    fn page_without_snapshot_reports_no_changes() {
        let edited = EditableSettings::default();
        assert!(!page_has_changes(ConfigPage::Display, &edited, None));
    }

    #[test]
    fn app_detects_hide_time_change() {
        // hide_time moved from Display to App; dirty-check must fire on App page.
        let snap = PageEntrySnapshot::App {
            source: GameSource::Manual,
            current_event_id: None,
            current_court: None,
            schedule: None,
            mode: Mode::Hockey6V6,
            collect_scorer_cap_num: false,
            track_fouls_and_warnings: false,
            show_behind_schedule_time: false,
            confirm_score: false,
            hide_time: false,
            audible_countdown: false,
        };
        let edited = EditableSettings {
            hide_time: true,
            ..Default::default()
        };
        assert!(page_has_changes(ConfigPage::App, &edited, Some(&snap)));
    }

    #[test]
    fn app_detects_audible_countdown_change() {
        let snap = PageEntrySnapshot::App {
            source: GameSource::Manual,
            current_event_id: None,
            current_court: None,
            schedule: None,
            mode: Mode::Hockey6V6,
            collect_scorer_cap_num: false,
            track_fouls_and_warnings: false,
            show_behind_schedule_time: false,
            confirm_score: false,
            hide_time: false,
            audible_countdown: false,
        };
        let edited = EditableSettings {
            audible_countdown: true,
            ..Default::default()
        };
        assert!(page_has_changes(ConfigPage::App, &edited, Some(&snap)));
    }

    // ---------------------------------------------------------------------
    // Invariant 1: per-page snapshot capture-and-revert (B3.10, B3.33)
    //
    // The Game-slice snapshot must restore every Game-slice field on Cancel,
    // while leaving fields owned by other pages alone.
    // ---------------------------------------------------------------------

    #[test]
    fn game_snapshot_revert_restores_all_game_slice_fields() {
        let event_id = EventId::from_partial("evt-A");
        let original_config = GameConfig::default();
        let mut bumped_config = GameConfig::default();
        bumped_config.team_timeout_duration += Duration::from_secs(15);

        // Entry-time state: snapshot captures this.
        let mut edited = EditableSettings {
            config: original_config.clone(),
            game_number: "1".to_string(),
            source: GameSource::Portal,
            current_event_id: Some(event_id.clone()),
            current_court: Some("CourtA".to_string()),
            schedule: Some(make_schedule_with_one_game(event_id.clone(), "1", "CourtA")),
            ..Default::default()
        };
        let snap = PageEntrySnapshot::Game {
            config: edited.config.clone(),
            game_number: edited.game_number.clone(),
            source: edited.source,
            current_event_id: edited.current_event_id.clone(),
            current_court: edited.current_court.clone(),
            schedule: edited.schedule.clone(),
        };

        // Operator mutates every Game-slice field after entering Game Options.
        edited.config = bumped_config;
        edited.game_number = "99".to_string();
        edited.source = GameSource::Manual;
        edited.current_event_id = Some(EventId::from_partial("evt-B"));
        edited.current_court = Some("CourtB".to_string());
        edited.schedule = None;

        snap.revert_into(&mut edited);

        assert_eq!(edited.config, original_config);
        assert_eq!(edited.game_number, "1");
        assert!(edited.uses_remote());
        assert_eq!(edited.current_event_id, Some(event_id.clone()));
        assert_eq!(edited.current_court.as_deref(), Some("CourtA"));
        assert!(edited.schedule.is_some());
        assert_eq!(edited.schedule.as_ref().unwrap().event_id, event_id,);
    }

    #[test]
    fn game_snapshot_revert_leaves_other_page_slices_untouched() {
        // Entry-time Game-slice values get captured.
        let mut edited = EditableSettings {
            game_number: "1".to_string(),
            ..Default::default()
        };
        let snap = PageEntrySnapshot::Game {
            config: edited.config.clone(),
            game_number: edited.game_number.clone(),
            source: edited.source,
            current_event_id: edited.current_event_id.clone(),
            current_court: edited.current_court.clone(),
            schedule: edited.schedule.clone(),
        };

        // Operator edits non-Game-slice fields between entering and cancelling
        // Game Options: those belong to other pages and must NOT be reverted.
        edited.mode = Mode::Rugby;
        edited.confirm_score = true;
        edited.track_fouls_and_warnings = true;
        edited.collect_scorer_cap_num = true;
        edited.white_on_right = true;
        edited.brightness = Brightness::High;
        edited.hide_time = true;

        // Also mutate a Game-slice field so we can prove the Game-slice revert
        // still happened on this same call.
        edited.game_number = "99".to_string();

        snap.revert_into(&mut edited);

        // Game-slice field was reverted.
        assert_eq!(edited.game_number, "1");

        // Other-page-slice fields are untouched.
        assert_eq!(edited.mode, Mode::Rugby);
        assert!(edited.confirm_score);
        assert!(edited.track_fouls_and_warnings);
        assert!(edited.collect_scorer_cap_num);
        assert!(edited.white_on_right);
        assert_eq!(edited.brightness, Brightness::High);
        assert!(edited.hide_time);
    }

    #[test]
    fn app_snapshot_revert_restores_only_app_slice_fields() {
        // Per ADR 009 the App page owns the portal trio plus the four App-slice
        // booleans. This test mirrors Invariant 1's assertions for App.
        let original_event = EventId::from_partial("evt-A");

        let mut edited = EditableSettings {
            source: GameSource::Portal,
            current_event_id: Some(original_event.clone()),
            current_court: Some("CourtA".to_string()),
            mode: Mode::Hockey6V6,
            collect_scorer_cap_num: false,
            track_fouls_and_warnings: false,
            confirm_score: false,
            // A Game-slice field we'll mutate to prove App revert ignores it.
            game_number: "1".to_string(),
            ..Default::default()
        };
        let snap = PageEntrySnapshot::App {
            source: edited.source,
            current_event_id: edited.current_event_id.clone(),
            current_court: edited.current_court.clone(),
            schedule: edited.schedule.clone(),
            mode: edited.mode,
            collect_scorer_cap_num: edited.collect_scorer_cap_num,
            track_fouls_and_warnings: edited.track_fouls_and_warnings,
            show_behind_schedule_time: edited.show_behind_schedule_time,
            confirm_score: edited.confirm_score,
            hide_time: false,
            audible_countdown: false,
        };

        edited.source = GameSource::Manual;
        edited.current_event_id = Some(EventId::from_partial("evt-B"));
        edited.current_court = Some("CourtB".to_string());
        edited.mode = Mode::Rugby;
        edited.collect_scorer_cap_num = true;
        edited.track_fouls_and_warnings = true;
        edited.confirm_score = true;
        edited.hide_time = true;
        edited.audible_countdown = true;
        edited.game_number = "99".to_string();

        snap.revert_into(&mut edited);

        // App-slice fields restored.
        assert!(edited.uses_remote());
        assert_eq!(edited.current_event_id, Some(original_event));
        assert_eq!(edited.current_court.as_deref(), Some("CourtA"));
        assert_eq!(edited.mode, Mode::Hockey6V6);
        assert!(!edited.collect_scorer_cap_num);
        assert!(!edited.track_fouls_and_warnings);
        assert!(!edited.confirm_score);
        assert!(!edited.hide_time);
        assert!(!edited.audible_countdown);

        // Game-slice field NOT restored by the App snapshot.
        assert_eq!(edited.game_number, "99");
    }

    // ---------------------------------------------------------------------
    // Invariant 2: uwhportal_incomplete() Apply-disable predicate (B3.9, B3.37)
    //
    // The same helper backs both the Apply-button enable state in the footer
    // and the gate check at the top of apply_game_options. The two consumers
    // must stay in sync because uwhportal_incomplete() is the only source of
    // truth — these tests lock its branches.
    // ---------------------------------------------------------------------

    /// Switching manual games off must leave the ACCESS TOKEN row on FAILED,
    /// never on "CHECKING…". The same reset clears the event id, and a token is
    /// only ever verified against an event, so a `None` here promises a check
    /// that can never be made. Under a custom site nothing else resolves it —
    /// its event is adopted only when the site is applied — so the row would
    /// read "CHECKING…" for as long as the operator left it there.
    #[test]
    fn remote_switch_leaves_token_indicator_failed_not_checking() {
        let mut edited = EditableSettings {
            source: GameSource::Custom,
            current_event_id: Some(EventId::from_partial("evt-A")),
            current_court: Some("CourtA".to_string()),
            game_number: "7".to_string(),
            uwhportal_token_valid: None,
            ..Default::default()
        };

        edited.clear_for_remote_switch();

        assert_eq!(
            edited.uwhportal_token_valid,
            Some(false),
            "an indicator with no event to check against must rest on FAILED"
        );
        assert!(edited.current_event_id.is_none());
        assert!(edited.current_court.is_none());
        assert!(edited.schedule.is_none());
        assert!(edited.game_number.is_empty());
    }

    #[test]
    fn uwhportal_incomplete_false_when_portal_off() {
        let edited = EditableSettings {
            source: GameSource::Manual,
            current_event_id: None,
            current_court: None,
            schedule: None,
            ..Default::default()
        };
        assert!(!edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_true_when_event_missing() {
        let edited = EditableSettings {
            source: GameSource::Portal,
            current_event_id: None,
            current_court: Some("CourtA".to_string()),
            schedule: Some(make_schedule_with_one_game(
                EventId::from_partial("evt-A"),
                "1",
                "CourtA",
            )),
            game_number: "1".to_string(),
            ..Default::default()
        };
        assert!(edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_true_when_court_missing() {
        let event_id = EventId::from_partial("evt-A");
        let edited = EditableSettings {
            source: GameSource::Portal,
            current_event_id: Some(event_id.clone()),
            current_court: None,
            schedule: Some(make_schedule_with_one_game(event_id, "1", "CourtA")),
            game_number: "1".to_string(),
            ..Default::default()
        };
        assert!(edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_true_when_schedule_missing() {
        let edited = EditableSettings {
            source: GameSource::Portal,
            current_event_id: Some(EventId::from_partial("evt-A")),
            current_court: Some("CourtA".to_string()),
            schedule: None,
            game_number: "1".to_string(),
            ..Default::default()
        };
        assert!(edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_true_when_game_not_in_schedule() {
        let event_id = EventId::from_partial("evt-A");
        let edited = EditableSettings {
            source: GameSource::Portal,
            current_event_id: Some(event_id.clone()),
            current_court: Some("CourtA".to_string()),
            schedule: Some(make_schedule_with_one_game(event_id, "1", "CourtA")),
            game_number: "does-not-exist".to_string(),
            ..Default::default()
        };
        assert!(edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_true_when_game_court_mismatches_current_court() {
        let event_id = EventId::from_partial("evt-A");
        let edited = EditableSettings {
            source: GameSource::Portal,
            current_event_id: Some(event_id.clone()),
            current_court: Some("CourtB".to_string()),
            schedule: Some(make_schedule_with_one_game(event_id, "1", "CourtA")),
            game_number: "1".to_string(),
            ..Default::default()
        };
        assert!(edited.uwhportal_incomplete());
    }

    #[test]
    fn uwhportal_incomplete_false_when_all_present_and_matching() {
        let event_id = EventId::from_partial("evt-A");
        let edited = EditableSettings {
            source: GameSource::Portal,
            current_event_id: Some(event_id.clone()),
            current_court: Some("CourtA".to_string()),
            schedule: Some(make_schedule_with_one_game(event_id, "1", "CourtA")),
            game_number: "1".to_string(),
            ..Default::default()
        };
        assert!(!edited.uwhportal_incomplete());
    }

    // ---------------------------------------------------------------------
    // Invariant 5: a completed portal selection enables Apply (regression)
    //
    // Regression guard for the "Apply stays gray after switching USING
    // UWHPORTAL to YES and completing the picks" report. The Game-page footer
    // enables Apply only when `page_has_changes(...) && !uwhportal_incomplete()`.
    // These lock that combined decision: a fully-completed portal selection
    // (changed from a portal-off entry) enables Apply, while a still-incomplete
    // one keeps it disabled. The async flow that *populates* those fields lives
    // in App::update (not unit-testable without sockets); the existing
    // uwhportal_incomplete_* tests already cover the "schedule not locked in"
    // shape that produced the report.
    // ---------------------------------------------------------------------

    #[test]
    fn apply_enabled_after_completing_portal_selection() {
        let event_id = EventId::from_partial("evt-A");

        // Entry-time state: operator opened Game Options with the portal off.
        let entry = EditableSettings::default();
        let snapshot = PageEntrySnapshot::Game {
            config: entry.config.clone(),
            game_number: entry.game_number.clone(),
            source: entry.source,
            current_event_id: entry.current_event_id.clone(),
            current_court: entry.current_court.clone(),
            schedule: entry.schedule.clone(),
        };

        // Operator switched the portal on and completed every pick.
        let edited = EditableSettings {
            source: GameSource::Portal,
            current_event_id: Some(event_id.clone()),
            current_court: Some("CourtA".to_string()),
            schedule: Some(make_schedule_with_one_game(event_id, "1", "CourtA")),
            game_number: "1".to_string(),
            ..Default::default()
        };

        let apply_enabled = page_has_changes(ConfigPage::Game, &edited, Some(&snapshot))
            && !edited.uwhportal_incomplete();
        assert!(
            apply_enabled,
            "a completed portal selection (changed from portal-off entry) must enable Apply"
        );
    }

    #[test]
    fn apply_disabled_when_portal_selection_incomplete() {
        // Same portal-off entry snapshot.
        let entry = EditableSettings::default();
        let snapshot = PageEntrySnapshot::Game {
            config: entry.config.clone(),
            game_number: entry.game_number.clone(),
            source: entry.source,
            current_event_id: entry.current_event_id.clone(),
            current_court: entry.current_court.clone(),
            schedule: entry.schedule.clone(),
        };

        // Operator switched the portal on but has not picked event/court/game.
        let edited = EditableSettings {
            source: GameSource::Portal,
            ..Default::default()
        };

        // Toggling the portal on is itself a change...
        assert!(page_has_changes(ConfigPage::Game, &edited, Some(&snapshot)));
        let apply_enabled = page_has_changes(ConfigPage::Game, &edited, Some(&snapshot))
            && !edited.uwhportal_incomplete();
        // ...but the incomplete selection keeps Apply disabled.
        assert!(
            !apply_enabled,
            "an incomplete portal selection must keep Apply disabled"
        );
    }

    // ---------------------------------------------------------------------
    // Invariant 4: picker-driven field clearing on event/court change
    // (B3.15, B3.16)
    //
    // select_event/select_court are the helpers used by the
    // Message::ParameterSelected handler. Locking them in tests preserves the
    // documented behaviour that switching events clears court / game number /
    // schedule, and switching courts clears game number.
    // ---------------------------------------------------------------------

    #[test]
    fn select_event_sets_event_and_clears_court_game_schedule() {
        let event_id = EventId::from_partial("evt-A");
        let mut edited = EditableSettings {
            current_event_id: Some(EventId::from_partial("old-evt")),
            current_court: Some("OldCourt".to_string()),
            game_number: "42".to_string(),
            schedule: Some(make_schedule_with_one_game(
                EventId::from_partial("old-evt"),
                "42",
                "OldCourt",
            )),
            ..Default::default()
        };

        edited.select_event(event_id.clone());

        assert_eq!(edited.current_event_id, Some(event_id));
        assert_eq!(edited.current_court, None);
        assert_eq!(edited.game_number, "");
        assert!(edited.schedule.is_none());
    }

    #[test]
    fn select_court_sets_court_and_clears_game_number() {
        let event_id = EventId::from_partial("evt-A");
        let mut edited = EditableSettings {
            current_event_id: Some(event_id.clone()),
            current_court: Some("OldCourt".to_string()),
            game_number: "42".to_string(),
            schedule: Some(make_schedule_with_one_game(event_id, "42", "OldCourt")),
            ..Default::default()
        };

        edited.select_court("NewCourt".to_string());

        assert_eq!(edited.current_court.as_deref(), Some("NewCourt"));
        assert_eq!(edited.game_number, "");
        // Event id and schedule are NOT touched by a court change.
        assert!(edited.current_event_id.is_some());
        assert!(edited.schedule.is_some());
    }

    // ---------------------------------------------------------------------
    // Invariant 6: schedule-arrival auto-court adoption guard
    //
    // should_adopt_auto_court decides whether an arriving schedule auto-fills
    // the court. It must fire only for the event currently selected, when that
    // event has exactly one court and none is chosen yet — so a late schedule
    // from a previously-selected event cannot fill the court for a different one.
    // ---------------------------------------------------------------------

    #[test]
    fn auto_court_adopted_for_single_court_matching_event() {
        let event_id = EventId::from_partial("evt-A");
        let edited = EditableSettings {
            current_event_id: Some(event_id.clone()),
            current_court: None,
            ..Default::default()
        };
        assert!(edited.should_adopt_auto_court(&event_id, 1));
    }

    #[test]
    fn auto_court_rejected_for_mismatched_event() {
        // Schedule for evt-A arrives late, but the operator is now on evt-B with
        // no court chosen. The court must NOT be auto-filled from the stale event.
        let edited = EditableSettings {
            current_event_id: Some(EventId::from_partial("evt-B")),
            current_court: None,
            ..Default::default()
        };
        assert!(!edited.should_adopt_auto_court(&EventId::from_partial("evt-A"), 1));
    }

    #[test]
    fn auto_court_rejected_when_multiple_courts() {
        let event_id = EventId::from_partial("evt-A");
        let edited = EditableSettings {
            current_event_id: Some(event_id.clone()),
            current_court: None,
            ..Default::default()
        };
        assert!(!edited.should_adopt_auto_court(&event_id, 2));
    }

    #[test]
    fn auto_court_rejected_when_court_already_set() {
        let event_id = EventId::from_partial("evt-A");
        let edited = EditableSettings {
            current_event_id: Some(event_id.clone()),
            current_court: Some("CourtA".to_string()),
            ..Default::default()
        };
        assert!(!edited.should_adopt_auto_court(&event_id, 1));
    }

    // ---------------------------------------------------------------------
    // Regression: Sound Options Apply gate after returning from Manage
    // Remotes (Unit 3 audit, S3.15 manual walkthrough, 2026-05-13).
    //
    // Previously, taking the Cancel or Apply path on the Remotes sub-page
    // consumed/cleared the page entry snapshot and never re-captured it
    // for the parent Sound page. With no snapshot, page_has_changes
    // returned false even after real sound edits, so the Sound Apply
    // button stayed permanently disabled.
    //
    // The fix re-captures the parent's snapshot inside navigate_to_parent.
    // This test documents the predicate's expected behaviour at the
    // snapshot level: with a Sound snapshot present the predicate must
    // correctly detect (or not detect) edits, and with no snapshot it
    // conservatively reports no changes (which is what disables Apply —
    // the very bug the fix prevents from occurring on the Sound page).
    // ---------------------------------------------------------------------
    #[test]
    fn sound_apply_requires_snapshot_present() {
        let mut edited = EditableSettings::default();
        let snap = PageEntrySnapshot::Sound {
            sound: edited.sound.clone(),
        };

        // 1. No edits yet -> Apply must stay disabled.
        assert!(!page_has_changes(ConfigPage::Sound, &edited, Some(&snap)));

        // 2. Operator toggles a sound field -> Apply must enable.
        edited.sound.sound_enabled ^= true;
        assert!(page_has_changes(ConfigPage::Sound, &edited, Some(&snap)));

        // 3. If the snapshot is missing (the pre-fix bug condition after
        //    returning from Manage Remotes), the predicate reports no
        //    changes regardless of edits, which leaves Apply disabled.
        //    The fix ensures this branch is not reached on Sound after a
        //    sub-page navigation; this assertion documents the predicate's
        //    conservative behaviour under None.
        assert!(!page_has_changes(ConfigPage::Sound, &edited, None));
    }

    #[test]
    fn test_game_block_validity_thresholds() {
        // half 9, halftime 2, two-period => regulation 20; minimum_break 2 => minimum 22.
        // 1 timeout/team, 60s, counted per half over 2 periods => allotment = 2*2*1*60 = 240.
        let base = GameConfig {
            single_half: false,
            half_play_duration: Duration::from_secs(9),
            half_time_duration: Duration::from_secs(2),
            minimum_break: Duration::from_secs(2),
            num_team_timeouts_allowed: 1,
            team_timeout_duration: Duration::from_secs(60),
            timeouts_counted_per_half: true,
            ..Default::default()
        };
        // Below minimum (22) => TooShort.
        let too_short = GameConfig {
            game_block: Duration::from_secs(21),
            ..base.clone()
        };
        assert_eq!(game_block_validity(&too_short), GameBlockValidity::TooShort);
        // >= minimum but buffer (game_block-22) < allotment(240) => Tight.
        let tight = GameConfig {
            game_block: Duration::from_secs(100),
            ..base.clone()
        };
        assert_eq!(game_block_validity(&tight), GameBlockValidity::Tight);
        // Exactly minimum + allotment (22 + 240 = 262): barely sufficient, no slack
        // to recover if running behind => Tight (yellow), not Ok.
        let barely = GameConfig {
            game_block: Duration::from_secs(262),
            ..base.clone()
        };
        assert_eq!(game_block_validity(&barely), GameBlockValidity::Tight);
        // Above minimum + allotment (263) => Ok (green).
        let ok = GameConfig {
            game_block: Duration::from_secs(263),
            ..base.clone()
        };
        assert_eq!(game_block_validity(&ok), GameBlockValidity::Ok);
    }
}
