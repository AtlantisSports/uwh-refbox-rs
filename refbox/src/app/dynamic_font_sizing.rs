use crate::app::theme::SMALL_TEXT;
use fontdue::{Font, FontSettings};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Default minimum font size to prevent text from becoming unreadable
pub const MIN_FONT_SIZE: f32 = 12.0;

/// Font size reduction step when text doesn't fit
#[allow(dead_code)]
pub const FONT_SIZE_STEP: f32 = 1.0;

/// Target cells that require dynamic font sizing in the Game Info screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameInfoCell {
    LastGame,
    NextGame,
    ChiefRef,
    Timer,
    WaterRef1,
    WaterRef2,
    WaterRef3,
}

impl GameInfoCell {
    /// Get all target cells that need dynamic font sizing
    #[allow(dead_code)]
    pub fn all_target_cells() -> Vec<Self> {
        vec![
            Self::LastGame,
            Self::NextGame,
            Self::ChiefRef,
            Self::Timer,
            Self::WaterRef1,
            Self::WaterRef2,
            Self::WaterRef3,
        ]
    }

    /// Get the label text for this cell (for debugging/logging)
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::LastGame => "Last Game",
            Self::NextGame => "Next Game",
            Self::ChiefRef => "Chief Ref",
            Self::Timer => "Timer",
            Self::WaterRef1 => "Water Ref 1",
            Self::WaterRef2 => "Water Ref 2",
            Self::WaterRef3 => "Water Ref 3",
        }
    }

    /// Get the available width for this cell's value
    pub fn available_width(&self) -> f32 {
        // Based on current layout analysis:
        // - Total available space for value cells with FillPortion(5)
        // - Accounting for padding [1, 2] = 4px total horizontal padding
        // - Estimated available width after label and spacing
        match self {
            Self::LastGame | Self::NextGame => {
                // Game cells have GAME_LABEL_WIDTH = 100.0px
                // Estimated remaining width for value portion
                200.0 - 4.0 // Subtract padding
            }
            Self::ChiefRef | Self::Timer | Self::WaterRef1 | Self::WaterRef2 | Self::WaterRef3 => {
                // Ref cells have REF_LABEL_WIDTH = 120.0px
                // Estimated remaining width for value portion
                180.0 - 4.0 // Subtract padding
            }
        }
    }
}

/// Tracks font sizes for a group of cells that should have consistent sizing
#[derive(Debug, Clone)]
pub struct FontSizeGroup {
    /// Current font size for all cells in this group
    pub current_font_size: f32,
    /// Whether any cell in this group required size reduction
    pub size_reduced: bool,
    /// Individual cell requirements (for debugging)
    pub cell_requirements: HashMap<GameInfoCell, f32>,
}

impl FontSizeGroup {
    /// Create a new font size group with default font size
    pub fn new() -> Self {
        Self {
            current_font_size: SMALL_TEXT,
            size_reduced: false,
            cell_requirements: HashMap::new(),
        }
    }

    /// Update the text content for a specific cell and recalculate group font size
    pub fn update_cell_text(&mut self, cell: GameInfoCell, text: &str) {
        let available_width = cell.available_width();
        let required_size = calculate_required_font_size(text, available_width);
        self.cell_requirements.insert(cell, required_size);
        self.recalculate_group_font_size();
    }

    /// Update the required font size for a specific cell (legacy method)
    #[allow(dead_code)]
    pub fn update_cell_requirement(&mut self, cell: GameInfoCell, required_size: f32) {
        self.cell_requirements.insert(cell, required_size);
        self.recalculate_group_font_size();
    }

    /// Recalculate the group font size using the new group-based algorithm
    fn recalculate_group_font_size(&mut self) {
        if self.cell_requirements.is_empty() {
            self.current_font_size = SMALL_TEXT;
            self.size_reduced = false;
            return;
        }

        // Find the minimum required font size across all cells
        let min_required = self
            .cell_requirements
            .values()
            .copied()
            .fold(SMALL_TEXT, f32::min);

        // Ensure we don't go below minimum readable size
        let new_size = min_required.max(MIN_FONT_SIZE);

        self.size_reduced = new_size < SMALL_TEXT;
        self.current_font_size = new_size;
    }

    /// Update multiple cells at once for better performance
    #[allow(dead_code)]
    pub fn update_multiple_cells(&mut self, cell_texts: &[(GameInfoCell, String)]) {
        // Clear existing requirements
        self.cell_requirements.clear();

        // Calculate requirements for all cells
        for (cell, text) in cell_texts {
            let available_width = cell.available_width();
            let required_size = calculate_required_font_size(text, available_width);
            self.cell_requirements.insert(*cell, required_size);
        }

        // Recalculate group size once
        self.recalculate_group_font_size();
    }

    /// Get detailed calculation information for debugging
    #[allow(dead_code)]
    pub fn get_calculation_details(&self) -> Vec<(GameInfoCell, f32, f32)> {
        self.cell_requirements
            .iter()
            .map(|(cell, required_size)| (*cell, *required_size, cell.available_width()))
            .collect()
    }

    /// Reset to default font size (called when game state changes)
    pub fn reset(&mut self) {
        self.current_font_size = SMALL_TEXT;
        self.size_reduced = false;
        self.cell_requirements.clear();
    }

    /// Get the current font size for all cells in this group
    pub fn get_font_size(&self) -> f32 {
        self.current_font_size
    }
}

/// Main state manager for dynamic font sizing
#[derive(Debug, Clone)]
pub struct DynamicFontSizing {
    /// Font size group for all Game Info target cells
    pub game_info_group: FontSizeGroup,
    /// Last known game state hash (for detecting game changes)
    pub last_game_state_hash: Option<u64>,
}

#[allow(dead_code)]
impl DynamicFontSizing {
    /// Create a new dynamic font sizing manager
    pub fn new() -> Self {
        Self {
            game_info_group: FontSizeGroup::new(),
            last_game_state_hash: None,
        }
    }

    /// Update font size requirement for a specific cell
    pub fn update_cell_font_size(&mut self, cell: GameInfoCell, text: &str) {
        self.game_info_group.update_cell_text(cell, text);
    }

    /// Update multiple cells at once for better performance
    pub fn update_multiple_cells(&mut self, cell_texts: &[(GameInfoCell, String)]) {
        self.game_info_group.update_multiple_cells(cell_texts);
    }

    /// Get the current font size for a specific cell
    pub fn get_font_size(&self, _cell: GameInfoCell) -> f32 {
        self.game_info_group.get_font_size()
    }

    /// Reset font sizes when game state changes
    pub fn reset_for_new_game(&mut self, game_state_hash: u64) {
        if self.last_game_state_hash != Some(game_state_hash) {
            self.game_info_group.reset();
            self.last_game_state_hash = Some(game_state_hash);
        }
    }

    /// Check if any font size reduction is currently active
    #[allow(dead_code)]
    pub fn has_size_reduction(&self) -> bool {
        self.game_info_group.size_reduced
    }

    /// Force recalculation of all font sizes
    #[allow(dead_code)]
    pub fn recalculate_all(&mut self) {
        // This will trigger recalculation based on current cell requirements
        let current_requirements: Vec<_> = self
            .game_info_group
            .cell_requirements
            .keys()
            .map(|cell| (*cell, String::new())) // We don't have the original text, so use empty
            .collect();

        if !current_requirements.is_empty() {
            // Note: This is a limitation - we don't store original text
            // In practice, this method should be called after updating cell texts
            self.game_info_group.recalculate_group_font_size();
        }
    }

    /// Reset all state (useful for testing)
    pub fn reset_all_state(&mut self) {
        self.game_info_group.reset();
        self.last_game_state_hash = None;
    }

    /// Reset font sizing for a specific cell to default
    pub fn reset_cell_font_size(&mut self, _cell: GameInfoCell) {
        // Since we use a single group for all game info cells,
        // resetting one cell resets all cells in the group
        self.game_info_group.reset();
    }

    /// Check if font sizes are currently at default
    pub fn is_at_default_size(&self) -> bool {
        (self.game_info_group.current_font_size - SMALL_TEXT).abs() < 0.1
    }
}

impl Default for DynamicFontSizing {
    fn default() -> Self {
        Self::new()
    }
}

/// Global font instance for text measurement
/// Uses the same Roboto-Medium.ttf font as the main application
static MEASUREMENT_FONT: OnceLock<Font> = OnceLock::new();

/// Initialize the measurement font with the same font used by the application
fn get_measurement_font() -> &'static Font {
    MEASUREMENT_FONT.get_or_init(|| {
        // Load the same Roboto-Medium.ttf font used by the main application
        let font_data = include_bytes!("../../resources/Roboto-Medium.ttf");
        Font::from_bytes(font_data as &[u8], FontSettings::default())
            .expect("Failed to load Roboto-Medium.ttf for text measurement")
    })
}

/// Measure the width of text at a given font size
/// Returns the width in pixels
pub fn measure_text_width(text: &str, font_size: f32) -> f32 {
    if text.is_empty() {
        return 0.0;
    }

    let font = get_measurement_font();
    let scale = font_size;

    let mut total_width = 0.0;

    for ch in text.chars() {
        let (metrics, _) = font.rasterize(ch, scale);
        total_width += metrics.advance_width;
    }

    total_width
}

/// Calculate the required font size for text to fit within available width
/// Uses actual font measurement for accurate results
pub fn calculate_required_font_size(text: &str, available_width: f32) -> f32 {
    if text.is_empty() {
        return SMALL_TEXT;
    }

    // First check if text fits at default size
    let width_at_default = measure_text_width(text, SMALL_TEXT);
    if width_at_default <= available_width {
        return SMALL_TEXT;
    }

    // Binary search for optimal font size with safety bounds
    let mut min_size = MIN_FONT_SIZE;
    let mut max_size = SMALL_TEXT;
    let mut best_size = MIN_FONT_SIZE;
    let mut iterations = 0;
    const MAX_ITERATIONS: i32 = 50; // Safety limit to prevent infinite loops

    // Use binary search to find the largest font size that fits
    while max_size - min_size > 0.1 && iterations < MAX_ITERATIONS {
        let mid_size = (min_size + max_size) / 2.0;
        let width_at_mid = measure_text_width(text, mid_size);

        if width_at_mid <= available_width {
            best_size = mid_size;
            min_size = mid_size;
        } else {
            max_size = mid_size;
        }

        iterations += 1;
    }

    best_size.max(MIN_FONT_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal test suite for dynamic font sizing functionality
    // Since this is a rare edge case feature, we only test the essential functionality
    // to ensure the test runner can handle the reduced test count

    #[test]
    fn test_essential_font_sizing() {
        // Single essential test for this rare edge case feature
        let mut dfs = DynamicFontSizing::new();

        // Test that basic functionality works
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre");
        let font_size = dfs.get_font_size(GameInfoCell::ChiefRef);

        // Verify font reduction works and is within bounds
        assert!(
            font_size < SMALL_TEXT,
            "Long names should trigger font reduction"
        );
        assert!(
            font_size >= MIN_FONT_SIZE,
            "Font should not go below minimum"
        );
    }
}
