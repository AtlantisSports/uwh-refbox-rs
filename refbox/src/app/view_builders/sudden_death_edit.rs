use super::configuration::MIN_PERIOD_LENGTH;
use super::fit_text::fit_text;
use super::*;
use iced::{
    Alignment, Length, Theme,
    alignment::Horizontal,
    widget::{
        button::{Status, Style},
        column, horizontal_space, row, vertical_space,
    },
};
use std::time::Duration;
use uwh_common::config::Game as GameConfig;

type StyleFn = fn(&Theme, Status) -> Style;

/// The editor's starting value when no maximum is stored. It is only where the
/// time editor opens: nothing is saved until the operator picks SET and APPLY.
const DEFAULT_SUDDEN_DEATH_MAX: Duration = Duration::from_secs(300);

/// The SUDDEN DEATH page's staged choices. They reach the Game page's edit on
/// APPLY and are dropped on CANCEL, like a length editor's Duration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuddenDeathEdit {
    pub allowed: bool,
    pub max_set: bool,
    pub length: Duration,
}

/// The page's opening state, from the config it edits (the Game page's staged
/// edit when there is one). Reads the stored maximum as it is, zero included:
/// this page edits the stored value, and `sudden_death_limit` alone decides
/// what a stored zero means for play.
pub(in super::super) fn sudden_death_edit_seed(config: &GameConfig) -> SuddenDeathEdit {
    SuddenDeathEdit {
        allowed: config.sudden_death_allowed,
        max_set: config.sudden_death_max.is_some(),
        length: config.sudden_death_max.unwrap_or(DEFAULT_SUDDEN_DEATH_MAX),
    }
}

/// Whether the operator has changed anything. The length counts only under
/// SET -- under NONE it is not saved -- and only in whole seconds, which is
/// all the editor shows.
pub(in super::super) fn sudden_death_edit_has_changes(
    edit: SuddenDeathEdit,
    seed: SuddenDeathEdit,
) -> bool {
    edit.allowed != seed.allowed
        || edit.max_set != seed.max_set
        || (edit.max_set && edit.length.as_secs() != seed.length.as_secs())
}

/// Whether the maximum on screen is one APPLY refuses: a SET 0:00 while sudden
/// death is allowed. Under NO the row is greyed and not in play.
fn sudden_death_edit_refused(edit: SuddenDeathEdit) -> bool {
    edit.allowed && edit.max_set && edit.length < MIN_PERIOD_LENGTH
}

/// Whether APPLY is offered: something changed, and the maximum is not refused.
pub(in super::super) fn sudden_death_edit_apply_enabled(
    edit: SuddenDeathEdit,
    seed: SuddenDeathEdit,
) -> bool {
    sudden_death_edit_has_changes(edit, seed) && !sudden_death_edit_refused(edit)
}

/// The page's APPLY: write its choices into the Game page's staged config.
/// Never into `Config` -- the Game page's own APPLY saves them.
pub(in super::super) fn sudden_death_edit_commit(edit: SuddenDeathEdit, config: &mut GameConfig) {
    config.sudden_death_allowed = edit.allowed;
    config.sudden_death_max = edit.max_set.then_some(edit.length);
}

/// One two-way choice row, styled like the team-timeout page: the chosen
/// button is blue, the other grey. When `enabled` is false neither button
/// presses, but the chosen one still shows, so a greyed row keeps its stored
/// choice visible.
fn make_choice_row<'a>(
    label: String,
    (first, second): (String, String),
    second_chosen: bool,
    messages: (Message, Message),
    enabled: bool,
) -> Element<'a, Message> {
    let choice = |label: String, chosen: bool, message: Message| {
        let style: StyleFn = if chosen {
            blue_selected_button
        } else {
            light_gray_button
        };
        let b = make_chrome_button(label).style(style).width(Length::Fill);
        if !enabled {
            b
        } else if chosen {
            b.on_press(Message::NoAction)
        } else {
            b.on_press(message)
        }
    };
    row![
        row![
            fit_text(label)
                .size(SMALL_PLUS_TEXT)
                .align_x(Horizontal::Left)
                .width(Length::Fill)
                .height(Length::Shrink),
        ]
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fixed(MIN_BUTTON_SIZE)),
        choice(first, !second_chosen, messages.0),
        choice(second, second_chosen, messages.1),
    ]
    .spacing(SPACING)
    .into()
}

/// The SUDDEN DEATH page, reached from the Game page's SUDDEN DEATH tile.
/// `config` is the config the page was seeded from, re-read to tell whether the
/// operator has changed anything.
pub(in super::super) fn build_sudden_death_editor<'a>(
    data: ViewData<'_, '_>,
    edit: SuddenDeathEdit,
    config: &GameConfig,
) -> Element<'a, Message> {
    let ViewData {
        snapshot,
        mode,
        clock,
        portal_indicator,
        ..
    } = data;

    let seed = sudden_death_edit_seed(config);
    let has_changes = sudden_death_edit_has_changes(edit, seed);
    let apply_enabled = sudden_death_edit_apply_enabled(edit, seed);
    let value_color = sudden_death_edit_refused(edit).then_some(RED);

    column![
        make_game_time_button(snapshot, false, false, mode, clock, portal_indicator, None),
        make_choice_row(
            fl!("sudden-death"),
            (fl!("no"), fl!("yes")),
            edit.allowed,
            (
                Message::SetSuddenDeathAllowed(false),
                Message::SetSuddenDeathAllowed(true),
            ),
            true,
        ),
        make_choice_row(
            fl!("maximum-time"),
            (fl!("sd-max-none"), fl!("sd-max-set")),
            edit.max_set,
            (
                Message::SetSuddenDeathMaxSet(false),
                Message::SetSuddenDeathMaxSet(true),
            ),
            edit.allowed,
        ),
        row![
            horizontal_space(),
            make_time_editor(
                fl!("maximum-time"),
                edit.length,
                false,
                value_color,
                edit.allowed && edit.max_set,
            ),
            horizontal_space(),
        ],
        vertical_space(),
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
    ]
    .spacing(SPACING)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn edit(allowed: bool, max_set: bool, secs: u64) -> SuddenDeathEdit {
        SuddenDeathEdit {
            allowed,
            max_set,
            length: Duration::from_secs(secs),
        }
    }

    #[test]
    fn seed_from_config() {
        let config = GameConfig {
            sudden_death_allowed: true,
            sudden_death_max: None,
            ..Default::default()
        };
        assert_eq!(sudden_death_edit_seed(&config), edit(true, false, 300));

        let config = GameConfig {
            sudden_death_allowed: true,
            sudden_death_max: Some(Duration::from_secs(420)),
            ..Default::default()
        };
        assert_eq!(sudden_death_edit_seed(&config), edit(true, true, 420));

        let config = GameConfig {
            sudden_death_allowed: false,
            sudden_death_max: None,
            ..Default::default()
        };
        assert_eq!(sudden_death_edit_seed(&config), edit(false, false, 300));
    }

    #[test]
    fn has_changes_ignores_length_under_none() {
        assert!(!sudden_death_edit_has_changes(
            edit(true, false, 60),
            edit(true, false, 300)
        ));
        assert!(sudden_death_edit_has_changes(
            edit(true, true, 60),
            edit(true, true, 300)
        ));
        // Sub-second differences are not changes: the editor shows whole seconds.
        assert!(!sudden_death_edit_has_changes(
            SuddenDeathEdit {
                length: Duration::from_micros(1),
                ..edit(true, true, 0)
            },
            edit(true, true, 0)
        ));
        assert!(sudden_death_edit_has_changes(
            edit(false, false, 300),
            edit(true, false, 300)
        ));
        assert!(sudden_death_edit_has_changes(
            edit(true, true, 300),
            edit(true, false, 300)
        ));
    }

    #[test]
    fn apply_refuses_set_zero() {
        let seed = edit(true, false, 300);
        assert!(!sudden_death_edit_apply_enabled(edit(true, true, 0), seed));
        // The editor's ZERO and - buttons floor at 1µs, which shows as 0:00.
        assert!(!sudden_death_edit_apply_enabled(
            SuddenDeathEdit {
                length: Duration::from_micros(1),
                ..edit(true, true, 0)
            },
            seed
        ));
        // NO greys the MAXIMUM TIME row: it is not in play, so it cannot refuse.
        assert!(sudden_death_edit_apply_enabled(edit(false, true, 0), seed));
        assert!(sudden_death_edit_apply_enabled(edit(true, true, 60), seed));
        assert!(!sudden_death_edit_apply_enabled(seed, seed));
    }

    #[test]
    fn commit_writes_none_or_some() {
        let mut config = GameConfig {
            sudden_death_allowed: true,
            sudden_death_max: Some(Duration::from_secs(60)),
            ..Default::default()
        };
        sudden_death_edit_commit(edit(true, false, 300), &mut config);
        assert!(config.sudden_death_allowed);
        assert_eq!(config.sudden_death_max, None);

        sudden_death_edit_commit(edit(true, true, 300), &mut config);
        assert!(config.sudden_death_allowed);
        assert_eq!(config.sudden_death_max, Some(Duration::from_secs(300)));

        sudden_death_edit_commit(edit(false, true, 300), &mut config);
        assert!(!config.sudden_death_allowed);
        assert_eq!(config.sudden_death_max, Some(Duration::from_secs(300)));
    }
}
