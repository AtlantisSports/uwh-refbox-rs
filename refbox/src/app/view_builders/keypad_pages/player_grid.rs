// `use super::*` is how every sibling in this directory picks up the theme
// constants, styles and `Message` — mirror `foul_add.rs`, do not re-import them.
use super::*;
use iced::{
    Length,
    widget::{column, row},
};

/// Columns in the player-number grid. The grid is read left to right, then
/// top to bottom, so this is fixed rather than derived from the roster.
pub(super) const GRID_COLUMNS: usize = 3;

/// Side length of one grid button. Smaller than `MIN_BUTTON_SIZE` (89) so the
/// worst case — five rows in Rugby — fits with margin.
///
/// In the default 691px window a keypad page spends 89 on the time banner, 89
/// on the timeout ribbon, `SPACING` above and below the panel, and `PADDING`
/// inside its container top and bottom:
/// `691 - 89 - 8 - 89 - 8 - 16 = 481` px of usable panel height. Five rows at
/// `MIN_BUTTON_SIZE` need `5 * 89 + 4 * SPACING = 477` — a four-pixel margin,
/// too fine to rely on across DPI and text scaling. At 80 they need
/// `5 * 80 + 4 * SPACING = 432`, which leaves real headroom.
pub(super) const GRID_BUTTON_SIZE: f32 = 80.0;

/// Cells the grid shows for a mode: the rules maximum roster size. `BeepTest`
/// has no player attribution at all, so it has no grid.
pub(super) fn grid_cells(mode: Mode) -> usize {
    match mode {
        Mode::Hockey3V3 => 6,
        Mode::Hockey6V6 => 12,
        Mode::Rugby => 15,
        Mode::BeepTest => 0,
    }
}

/// Lay a team's cap numbers into grid rows: ascending, packed from the top
/// left, `None` for the cells left over once the roster runs out.
///
/// The portal restricts team size, so a roster longer than the mode's grid
/// should not occur; `truncate` keeps that from panicking or silently
/// reflowing the grid if it ever does.
pub(super) fn grid_rows(numbers: &[u8], mode: Mode) -> Vec<Vec<Option<u8>>> {
    let cells = grid_cells(mode);

    let mut sorted = numbers.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    sorted.truncate(cells);

    let mut flat: Vec<Option<u8>> = sorted.into_iter().map(Some).collect();
    flat.resize(cells, None);

    flat.chunks(GRID_COLUMNS).map(<[_]>::to_vec).collect()
}

/// Whether the grid can be shown for this team, or the number pad must be.
///
/// The grid needs usable numbers and a mode that has a grid at all. It also
/// has to be able to display whatever number is already entered: an entry
/// being edited may hold a number that is not on the current roster, and that
/// number must stay visible, so those fall back to the pad. `0` means nothing
/// is selected (and, on the goal page, a team goal), which the grid shows fine.
pub(super) fn show_grid(numbers: &[u8], mode: Mode, current: u32) -> bool {
    if numbers.is_empty() || grid_cells(mode) == 0 {
        return false;
    }

    match u8::try_from(current) {
        Ok(0) => true,
        Ok(n) => numbers.contains(&n),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_per_mode() {
        assert_eq!(grid_cells(Mode::Hockey3V3), 6);
        assert_eq!(grid_cells(Mode::Hockey6V6), 12);
        assert_eq!(grid_cells(Mode::Rugby), 15);
        assert_eq!(grid_cells(Mode::BeepTest), 0);
    }

    #[test]
    fn packs_ascending_left_to_right_then_down() {
        let rows = grid_rows(&[5, 1, 4, 2], Mode::Hockey3V3);
        assert_eq!(
            rows,
            vec![vec![Some(1), Some(2), Some(4)], vec![Some(5), None, None],]
        );
    }

    #[test]
    fn full_roster_leaves_no_gaps() {
        let numbers: Vec<u8> = (1..=12).collect();
        let rows = grid_rows(&numbers, Mode::Hockey6V6);
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().flatten().all(Option::is_some));
    }

    #[test]
    fn gaps_in_cap_numbers_are_packed_not_positioned() {
        // 1, 2, 4 — no hole where 3 would be.
        let rows = grid_rows(&[1, 2, 4], Mode::Hockey3V3);
        assert_eq!(rows[0], vec![Some(1), Some(2), Some(4)]);
    }

    #[test]
    fn numbers_above_the_cell_count_are_allowed() {
        // The portal caps the number of players, not the numbers they wear.
        let rows = grid_rows(&[1, 16], Mode::Rugby);
        assert_eq!(rows[0], vec![Some(1), Some(16), None]);
    }

    #[test]
    fn duplicates_collapse() {
        let rows = grid_rows(&[7, 7, 8], Mode::Hockey3V3);
        assert_eq!(rows[0], vec![Some(7), Some(8), None]);
    }

    #[test]
    fn oversized_roster_is_truncated_not_reflowed() {
        let numbers: Vec<u8> = (1..=20).collect();
        let rows = grid_rows(&numbers, Mode::Hockey6V6);
        assert_eq!(rows.len(), 4, "grid must stay at the mode's size");
        assert_eq!(rows[3], vec![Some(10), Some(11), Some(12)]);
    }

    #[test]
    fn empty_roster_means_no_grid() {
        assert!(!show_grid(&[], Mode::Hockey6V6, 0));
    }

    #[test]
    fn beep_test_has_no_grid() {
        assert!(!show_grid(&[1, 2, 3], Mode::BeepTest, 0));
    }

    #[test]
    fn roster_with_numbers_shows_grid() {
        assert!(show_grid(&[1, 2, 3], Mode::Hockey6V6, 0));
    }

    #[test]
    fn number_on_the_roster_shows_grid() {
        assert!(show_grid(&[1, 7, 12], Mode::Hockey6V6, 7));
    }

    #[test]
    fn number_off_the_roster_falls_back_to_the_pad() {
        assert!(!show_grid(&[1, 7, 12], Mode::Hockey6V6, 23));
    }
}

/// One row of three cells per grid row. A cell with a number is tappable
/// unless the panel is disabled; a leftover cell (`None`), or any cell while
/// disabled, has no `on_press`, which iced renders in `Status::Disabled` and
/// `blue_button` paints as window background with grayed text — the
/// greyed-out look, with no extra style needed.
pub(super) fn make_player_grid<'a>(
    numbers: &[u8],
    mode: Mode,
    selected: u32,
    enabled: bool,
) -> Element<'a, Message> {
    // Centred, not left-aligned: the grid (3 * GRID_BUTTON_SIZE wide) sits
    // inside a panel box sized for the wider number pad, so without this it
    // would hug the box's left edge instead of sitting in the middle of it.
    let mut grid = column![].spacing(SPACING).align_x(iced::Alignment::Center);

    for cells in grid_rows(numbers, mode) {
        let mut line = row![].spacing(SPACING);
        for cell in cells {
            line = line.push(make_grid_cell(cell, selected, enabled));
        }
        grid = grid.push(line);
    }

    grid.into()
}

/// Reuses `make_small_button` from `shared_elements.rs` (text centred inside
/// a filled container inside a fixed-size button), overriding its size to
/// `GRID_BUTTON_SIZE`.
fn make_grid_cell<'a>(cell: Option<u8>, selected: u32, enabled: bool) -> Element<'a, Message> {
    let label = match cell {
        Some(number) => number.to_string(),
        None => String::new(),
    };

    let cell_button = make_small_button(label, MEDIUM_TEXT)
        .width(Length::Fixed(GRID_BUTTON_SIZE))
        .height(Length::Fixed(GRID_BUTTON_SIZE));

    match cell {
        Some(number) => {
            let is_selected = selected != 0 && selected == u32::from(number);
            // Tapping the selected cell again clears the selection, which on
            // the goal page returns to a team goal.
            let target = if is_selected { 0 } else { u32::from(number) };
            let cell_button = if enabled {
                cell_button.on_press(Message::SelectPlayerNumber(target))
            } else {
                // A disabled panel (team warning, equal foul) must not offer a
                // tappable cell. `panel_team` returning `None` already empties
                // the roster for these two cases, so this is currently
                // unreachable — it stays as a deliberate guard against a
                // future disabling condition being added to `enabled` alone
                // (see the doc comment on `enabled` in keypad_pages/mod.rs).
                cell_button
            };
            cell_button
                .style(if is_selected {
                    blue_selected_button
                } else {
                    blue_button
                })
                .into()
        }
        None => cell_button.style(blue_button).into(),
    }
}
