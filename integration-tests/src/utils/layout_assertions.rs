//! Assertions for layout properties (fixed vs proportional widths, etc.)

use iced_core::Length;

pub const GAME_LABEL_WIDTH: f32 = 100.0;
pub const REF_LABEL_WIDTH: f32 = 120.0;

pub fn determine_label_width(label: &str, label_portion: u16) -> Length {
    match label {
        "Last Game" | "Next Game" | "" => Length::Fixed(GAME_LABEL_WIDTH),
        "Chief Ref" | "Timer" | "Water Ref 1" | "Water Ref 2" | "Water Ref 3" => {
            Length::Fixed(REF_LABEL_WIDTH)
        }
        _ => Length::FillPortion(label_portion),
    }
}

#[derive(Debug, Clone)]
pub struct MockRow {
    pub left_label: String,
    pub left_value: String,
    pub center_label: Option<String>,
    pub center_value: Option<String>,
}

pub fn assert_mock_layout_structure(rows: &[MockRow]) {
    // Count 4-col rows (center_label Some) vs 2-col rows
    let four_col = rows.iter().filter(|r| r.center_label.is_some()).count();
    let two_col = rows.iter().filter(|r| r.center_label.is_none()).count();
    assert!(
        four_col >= 3,
        "expected at least 3 four-column rows, got {four_col}"
    );
    assert!(
        two_col >= 5,
        "expected at least 5 two-column rows, got {two_col}"
    );
}
