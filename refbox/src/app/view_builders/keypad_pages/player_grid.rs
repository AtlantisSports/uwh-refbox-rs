// `use super::*` is how every sibling in this directory picks up the theme
// constants, styles and `Message` — mirror `foul_add.rs`, do not re-import them.
use super::*;
use iced::{
    Length,
    widget::{column, row, vertical_space},
};

/// Columns in the player-number grid. The grid is read left to right, then
/// top to bottom, so this is fixed rather than derived from the roster.
pub(super) const GRID_COLUMNS: usize = 3;

/// Side length of one grid button: the same size the number pad's own buttons
/// use, so three of them plus two `SPACING` gaps fill the panel's content width
/// exactly (`3 * 89 + 2 * 8 = 283`). Grid and pad are therefore the same width,
/// and toggling between a team with a roster and one without cannot shift the
/// CANCEL and DONE buttons beside them.
///
/// This is not a side length in every mode, though: in Rugby
/// `GRID_BUTTON_SIZE` is a button *width* only. The height there comes from
/// `Length::Fill` (see `grid_fills_height` below), which divides up whatever
/// height the panel has rather than needing five rows to fit at a fixed size.
///
/// In the default 691px window a keypad page has `691` window height, minus
/// `16` for `main_view`'s own top-and-bottom `.padding(PADDING)`, minus `89`
/// for the timeout ribbon, minus `8` spacing, minus `89` for the game-time
/// banner, minus `8` spacing, minus `16` for the panel container's own
/// top-and-bottom `.padding(PADDING)`: `691 - 16 - 89 - 8 - 89 - 8 - 16 = 465`
/// px of usable panel content height.
pub(super) const GRID_BUTTON_SIZE: f32 = MIN_BUTTON_SIZE;

/// Whether this mode's grid stretches to fill the panel's height.
///
/// Every mode carries the panel's title row (about 70px of the 465px budget).
/// Rugby's five rows cannot keep their square 89px height under that — five
/// squares plus their gaps need 477 — so the rows share what is left and the
/// buttons come out visibly shorter than they are wide, rather than the grid
/// overflowing the panel. The hockey modes have slack even with the title, so
/// they keep square buttons and sit at the bottom of the panel.
/// Exhaustive rather than a `matches!`, to match `grid_cells` below: a new mode
/// should not silently inherit a layout nobody chose for it.
fn grid_fills_height(mode: Mode) -> bool {
    match mode {
        Mode::Rugby => true,
        Mode::Hockey6V6 | Mode::Hockey3V3 => false,
        // No grid at all, so this is never read.
        Mode::BeepTest => false,
    }
}

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

    #[test]
    fn only_rugby_stretches_its_rows() {
        assert!(grid_fills_height(Mode::Rugby));
        assert!(!grid_fills_height(Mode::Hockey6V6));
        assert!(!grid_fills_height(Mode::Hockey3V3));
        // No grid at all, so the value is never read — pinned so the exhaustive
        // match keeps an answer for every mode.
        assert!(!grid_fills_height(Mode::BeepTest));
    }
}

/// One row of three cells per grid row. A cell with a number is tappable
/// unless the panel is disabled; a leftover cell (`None`), or any cell while
/// disabled, has no `on_press`, which iced renders in `Status::Disabled` and
/// `blue_button` paints as window background with grayed text — the
/// greyed-out look, with no extra style needed.
///
/// `label` is the panel's title row, built by `make_panel_label` and identical
/// to the one the number pad carries. Every mode gets it, so no text appears or
/// vanishes when the operator toggles to a team whose grid cannot be shown.
pub(super) fn make_player_grid<'a>(
    label: Element<'a, Message>,
    numbers: &[u8],
    mode: Mode,
    selected: u32,
    enabled: bool,
) -> Element<'a, Message> {
    let fills_height = grid_fills_height(mode);

    let mut grid = column![].spacing(SPACING);
    if fills_height {
        grid = grid.height(Length::Fill);
    }

    for cells in grid_rows(numbers, mode) {
        let mut line = row![].spacing(SPACING);
        if fills_height {
            // Every row takes an equal share of the panel, so the buttons end
            // up the same height as each other rather than each keeping the
            // square height and leaving a gap at the bottom.
            line = line.height(Length::Fill);
        }
        for cell in cells {
            line = line.push(make_grid_cell(cell, selected, enabled, fills_height));
        }
        grid = grid.push(line);
    }

    if fills_height {
        // Rugby's rows already claim every pixel below the title, so there is
        // no spacer to separate the two — one `SPACING` gap is spent here
        // instead, and the rows divide what is left. A two-element column takes
        // exactly one gap, which the height budget can afford.
        return column![label, grid]
            .spacing(SPACING)
            .height(Length::Fill)
            .into();
    }

    // The hockey modes have vertical slack, so the title sits at the top of the
    // panel and the square buttons are pushed to the bottom of it.
    //
    // No `.spacing()` here: the fill spacer already supplies all the
    // separation there is slack for, so an outer spacing would only add two
    // dead gaps this column never uses (see `GRID_BUTTON_SIZE`'s doc comment
    // for the height budget that makes those gaps costly).
    column![label, vertical_space(), grid]
        .height(Length::Fill)
        .into()
}

/// Reuses `make_small_button` from `shared_elements.rs` (text centred inside
/// a filled container inside a fixed-size button), overriding its size to
/// `GRID_BUTTON_SIZE`.
fn make_grid_cell<'a>(
    cell: Option<u8>,
    selected: u32,
    enabled: bool,
    fills_height: bool,
) -> Element<'a, Message> {
    let label = match cell {
        Some(number) => number.to_string(),
        None => String::new(),
    };

    let cell_button = make_small_button(label, MEDIUM_TEXT)
        .width(Length::Fixed(GRID_BUTTON_SIZE))
        .height(if fills_height {
            Length::Fill
        } else {
            Length::Fixed(GRID_BUTTON_SIZE)
        });

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
                // tappable cell. Those pages take `PanelRole::TeamEntry`, which
                // `build_keypad_page` maps to an empty roster, so no grid is
                // built for them at all and this is currently unreachable — it
                // stays as a deliberate guard against a future disabling
                // condition that greys the panel without emptying the roster.
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
