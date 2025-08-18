use crate::app::theme::SMALL_TEXT;
use std::collections::HashMap;
use fontdue::{Font, FontSettings};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Test data constants for validation as specified in requirements
pub mod test_data {
    /// Test data for "Last Game" value cell
    pub const LAST_GAME: &str = "Australia vs New Zealand";

    /// Test data for "Next Game" value cell
    pub const NEXT_GAME: &str = "Nederlands vs South Africa";

    /// Test data for "Chief Ref" value cell
    pub const CHIEF_REF: &str = "Russell Owen Camilo La Torre";

    /// Test data for "Timer" value cell
    pub const TIMER: &str = "Norfatin Aainaa Binti Hashim";

    /// Test data for "Water Ref 1" value cell
    pub const WATER_REF_1: &str = "Tuan San Jonathan Chan";

    /// Test data for "Water Ref 2" value cell
    pub const WATER_REF_2: &str = "Muhammad Danish Haikal Mohd Fadel";

    /// Test data for "Water Ref 3" value cell
    pub const WATER_REF_3: &str = "A very long person name";

    /// All test data values in order for iteration
    pub const ALL_TEST_VALUES: &[&str] = &[
        LAST_GAME,
        NEXT_GAME,
        CHIEF_REF,
        TIMER,
        WATER_REF_1,
        WATER_REF_2,
        WATER_REF_3,
    ];

    /// Get test data for a specific cell
    pub fn get_test_data(cell: super::GameInfoCell) -> &'static str {
        match cell {
            super::GameInfoCell::LastGame => LAST_GAME,
            super::GameInfoCell::NextGame => NEXT_GAME,
            super::GameInfoCell::ChiefRef => CHIEF_REF,
            super::GameInfoCell::Timer => TIMER,
            super::GameInfoCell::WaterRef1 => WATER_REF_1,
            super::GameInfoCell::WaterRef2 => WATER_REF_2,
            super::GameInfoCell::WaterRef3 => WATER_REF_3,
        }
    }
}

/// Default minimum font size to prevent text from becoming unreadable
pub const MIN_FONT_SIZE: f32 = 12.0;

/// Font size reduction step when text doesn't fit
pub const FONT_SIZE_STEP: f32 = 1.0;

/// Record of a font size change for debugging and analytics
#[derive(Debug, Clone)]
pub struct FontSizeChange {
    /// Timestamp when the change occurred
    pub timestamp: u64,
    /// Previous font size
    pub old_size: f32,
    /// New font size
    pub new_size: f32,
    /// Cell that triggered the change
    pub triggering_cell: GameInfoCell,
    /// Text content that caused the change
    pub triggering_text: String,
}

/// Performance and usage metrics for font sizing
#[derive(Debug, Clone)]
pub struct FontSizingMetrics {
    /// Total number of font size calculations performed
    pub total_calculations: u64,
    /// Number of times font size was reduced
    pub reduction_count: u64,
    /// Number of game state resets
    pub reset_count: u64,
    /// Average font size across all calculations
    pub average_font_size: f32,
    /// Minimum font size ever calculated
    pub min_font_size_used: f32,
    /// Maximum font size ever calculated
    pub max_font_size_used: f32,
}

impl FontSizingMetrics {
    /// Create new metrics with default values
    pub fn new() -> Self {
        Self {
            total_calculations: 0,
            reduction_count: 0,
            reset_count: 0,
            average_font_size: SMALL_TEXT,
            min_font_size_used: SMALL_TEXT,
            max_font_size_used: SMALL_TEXT,
        }
    }

    /// Record a new font size calculation
    pub fn record_calculation(&mut self, font_size: f32) {
        self.total_calculations += 1;

        if font_size < SMALL_TEXT {
            self.reduction_count += 1;
        }

        // Update running average
        let total = self.total_calculations as f32;
        self.average_font_size = (self.average_font_size * (total - 1.0) + font_size) / total;

        // Update min/max
        self.min_font_size_used = self.min_font_size_used.min(font_size);
        self.max_font_size_used = self.max_font_size_used.max(font_size);
    }

    /// Record a game state reset
    pub fn record_reset(&mut self) {
        self.reset_count += 1;
    }

    /// Get reduction percentage
    pub fn reduction_percentage(&self) -> f32 {
        if self.total_calculations == 0 {
            0.0
        } else {
            (self.reduction_count as f32 / self.total_calculations as f32) * 100.0
        }
    }
}

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
        let min_required = self.cell_requirements
            .values()
            .copied()
            .fold(SMALL_TEXT, f32::min);

        // Ensure we don't go below minimum readable size
        let new_size = min_required.max(MIN_FONT_SIZE);

        self.size_reduced = new_size < SMALL_TEXT;
        self.current_font_size = new_size;
    }

    /// Update multiple cells at once for better performance
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
    /// History of font size changes for debugging
    pub size_change_history: Vec<FontSizeChange>,
    /// Performance metrics
    pub metrics: FontSizingMetrics,
}

impl DynamicFontSizing {
    /// Create a new dynamic font sizing manager
    pub fn new() -> Self {
        Self {
            game_info_group: FontSizeGroup::new(),
            last_game_state_hash: None,
            size_change_history: Vec::new(),
            metrics: FontSizingMetrics::new(),
        }
    }

    /// Update font size requirement for a specific cell
    pub fn update_cell_font_size(&mut self, cell: GameInfoCell, text: &str) {
        let old_size = self.game_info_group.current_font_size;
        self.game_info_group.update_cell_text(cell, text);
        let new_size = self.game_info_group.current_font_size;

        // Record metrics
        self.metrics.record_calculation(new_size);

        // Record change if font size actually changed
        if (old_size - new_size).abs() > 0.1 {
            self.record_font_size_change(old_size, new_size, cell, text.to_string());
        }
    }

    /// Update multiple cells at once for better performance
    pub fn update_multiple_cells(&mut self, cell_texts: &[(GameInfoCell, String)]) {
        let old_size = self.game_info_group.current_font_size;
        self.game_info_group.update_multiple_cells(cell_texts);
        let new_size = self.game_info_group.current_font_size;

        // Record metrics for each cell
        for _ in cell_texts {
            self.metrics.record_calculation(new_size);
        }

        // Record change if font size actually changed
        if (old_size - new_size).abs() > 0.1 {
            // Find the most constraining cell for the change record
            let triggering_cell = cell_texts.first().map(|(cell, _)| *cell).unwrap_or(GameInfoCell::LastGame);
            let triggering_text = cell_texts.first().map(|(_, text)| text.clone()).unwrap_or_default();
            self.record_font_size_change(old_size, new_size, triggering_cell, triggering_text);
        }
    }

    /// Get the current font size for a specific cell
    pub fn get_font_size(&self, _cell: GameInfoCell) -> f32 {
        self.game_info_group.get_font_size()
    }

    /// Reset font sizes when game state changes
    pub fn reset_for_new_game(&mut self, game_state_hash: u64) {
        if self.last_game_state_hash != Some(game_state_hash) {
            let old_size = self.game_info_group.current_font_size;
            self.game_info_group.reset();
            self.last_game_state_hash = Some(game_state_hash);
            self.metrics.record_reset();

            // Record the reset as a font size change
            if (old_size - SMALL_TEXT).abs() > 0.1 {
                self.record_font_size_change(old_size, SMALL_TEXT, GameInfoCell::LastGame, "Game Reset".to_string());
            }
        }
    }

    /// Record a font size change in the history
    fn record_font_size_change(&mut self, old_size: f32, new_size: f32, triggering_cell: GameInfoCell, triggering_text: String) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let change = FontSizeChange {
            timestamp,
            old_size,
            new_size,
            triggering_cell,
            triggering_text,
        };

        self.size_change_history.push(change);

        // Keep only the last 100 changes to prevent unbounded growth
        if self.size_change_history.len() > 100 {
            self.size_change_history.remove(0);
        }
    }

    /// Check if any font size reduction is currently active
    pub fn has_size_reduction(&self) -> bool {
        self.game_info_group.size_reduced
    }

    /// Get detailed information about current font sizing decisions
    pub fn get_sizing_info(&self) -> FontSizingInfo {
        FontSizingInfo {
            current_font_size: self.game_info_group.current_font_size,
            is_reduced: self.game_info_group.size_reduced,
            cell_details: self.game_info_group.get_calculation_details(),
        }
    }

    /// Force recalculation of all font sizes
    pub fn recalculate_all(&mut self) {
        // This will trigger recalculation based on current cell requirements
        let current_requirements: Vec<_> = self.game_info_group.cell_requirements
            .iter()
            .map(|(cell, _)| (*cell, String::new())) // We don't have the original text, so use empty
            .collect();

        if !current_requirements.is_empty() {
            // Note: This is a limitation - we don't store original text
            // In practice, this method should be called after updating cell texts
            self.game_info_group.recalculate_group_font_size();
        }
    }

    /// Get the font size change history
    pub fn get_change_history(&self) -> &[FontSizeChange] {
        &self.size_change_history
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> &FontSizingMetrics {
        &self.metrics
    }

    /// Clear the change history (useful for testing or memory management)
    pub fn clear_history(&mut self) {
        self.size_change_history.clear();
    }

    /// Force reset all font sizes to default immediately
    pub fn force_reset(&mut self) {
        let old_size = self.game_info_group.current_font_size;
        self.game_info_group.reset();
        self.metrics.record_reset();

        // Record the reset as a font size change
        if (old_size - SMALL_TEXT).abs() > 0.1 {
            self.record_font_size_change(old_size, SMALL_TEXT, GameInfoCell::LastGame, "Force Reset".to_string());
        }

        // Clear the game state hash to ensure next change is detected
        self.last_game_state_hash = None;
    }

    /// Reset metrics and history (useful for testing)
    pub fn reset_all_state(&mut self) {
        self.game_info_group.reset();
        self.size_change_history.clear();
        self.metrics = FontSizingMetrics::new();
        self.last_game_state_hash = None;
    }

    /// Check if font sizes are currently at default
    pub fn is_at_default_size(&self) -> bool {
        (self.game_info_group.current_font_size - SMALL_TEXT).abs() < 0.1
    }

    /// Get a summary of recent font sizing activity
    pub fn get_activity_summary(&self) -> FontSizingActivitySummary {
        let recent_changes = self.size_change_history
            .iter()
            .rev()
            .take(10)
            .collect::<Vec<_>>();

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let recent_activity_count = self.size_change_history
            .iter()
            .filter(|change| current_time - change.timestamp < 300) // Last 5 minutes
            .count();

        FontSizingActivitySummary {
            current_font_size: self.game_info_group.current_font_size,
            is_reduced: self.game_info_group.size_reduced,
            total_changes: self.size_change_history.len(),
            recent_changes: recent_changes.into_iter().cloned().collect(),
            recent_activity_count,
            metrics: self.metrics.clone(),
        }
    }
}

/// Summary of font sizing activity for monitoring and debugging
#[derive(Debug, Clone)]
pub struct FontSizingActivitySummary {
    /// Current font size being used
    pub current_font_size: f32,
    /// Whether font size is currently reduced
    pub is_reduced: bool,
    /// Total number of font size changes recorded
    pub total_changes: usize,
    /// Most recent font size changes (up to 10)
    pub recent_changes: Vec<FontSizeChange>,
    /// Number of changes in the last 5 minutes
    pub recent_activity_count: usize,
    /// Performance metrics
    pub metrics: FontSizingMetrics,
}

/// Detailed information about current font sizing state
#[derive(Debug, Clone)]
pub struct FontSizingInfo {
    /// Current font size being used for all cells
    pub current_font_size: f32,
    /// Whether the font size has been reduced from default
    pub is_reduced: bool,
    /// Details for each cell: (cell, required_size, available_width)
    pub cell_details: Vec<(GameInfoCell, f32, f32)>,
}

impl FontSizingInfo {
    /// Get the cell that requires the smallest font size (most constraining)
    pub fn most_constraining_cell(&self) -> Option<(GameInfoCell, f32)> {
        self.cell_details
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(cell, required_size, _)| (*cell, *required_size))
    }

    /// Get utilization statistics
    pub fn get_utilization_stats(&self) -> (f32, f32, f32) {
        if self.cell_details.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let utilizations: Vec<f32> = self.cell_details
            .iter()
            .map(|(_, required_size, available_width)| {
                if *available_width > 0.0 {
                    (required_size / available_width * 100.0).min(100.0)
                } else {
                    0.0
                }
            })
            .collect();

        let min_util = utilizations.iter().copied().fold(f32::INFINITY, f32::min);
        let max_util = utilizations.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let avg_util = utilizations.iter().sum::<f32>() / utilizations.len() as f32;

        (min_util, max_util, avg_util)
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

    // Binary search for optimal font size
    let mut min_size = MIN_FONT_SIZE;
    let mut max_size = SMALL_TEXT;
    let mut best_size = MIN_FONT_SIZE;

    // Use binary search to find the largest font size that fits
    while max_size - min_size > 0.1 {
        let mid_size = (min_size + max_size) / 2.0;
        let width_at_mid = measure_text_width(text, mid_size);

        if width_at_mid <= available_width {
            best_size = mid_size;
            min_size = mid_size;
        } else {
            max_size = mid_size;
        }
    }

    best_size.max(MIN_FONT_SIZE)
}

/// Calculate optimal font size with step-based reduction (alternative approach)
/// Reduces font size in steps until text fits
pub fn calculate_required_font_size_stepped(text: &str, available_width: f32) -> f32 {
    if text.is_empty() {
        return SMALL_TEXT;
    }

    let mut current_size = SMALL_TEXT;

    while current_size >= MIN_FONT_SIZE {
        let width = measure_text_width(text, current_size);
        if width <= available_width {
            return current_size;
        }
        current_size -= FONT_SIZE_STEP;
    }

    MIN_FONT_SIZE
}

/// Calculate font size with margin for safety
/// Adds a small margin to ensure text definitely fits
pub fn calculate_required_font_size_with_margin(text: &str, available_width: f32, margin_percent: f32) -> f32 {
    if text.is_empty() {
        return SMALL_TEXT;
    }

    // Reduce available width by margin percentage
    let effective_width = available_width * (1.0 - margin_percent / 100.0);
    calculate_required_font_size(text, effective_width)
}

/// Calculate font size for multiple texts, returning the size that fits all
/// Used for group-based font sizing where all cells must use the same size
pub fn calculate_group_font_size(texts_and_widths: &[(String, f32)]) -> f32 {
    if texts_and_widths.is_empty() {
        return SMALL_TEXT;
    }

    let mut min_required_size = SMALL_TEXT;

    for (text, available_width) in texts_and_widths {
        let required_size = calculate_required_font_size(text, *available_width);
        min_required_size = min_required_size.min(required_size);
    }

    min_required_size.max(MIN_FONT_SIZE)
}

/// Font size calculation result with additional metadata
#[derive(Debug, Clone)]
pub struct FontSizeCalculation {
    /// The calculated font size
    pub font_size: f32,
    /// Whether the font size was reduced from default
    pub was_reduced: bool,
    /// The actual text width at the calculated font size
    pub actual_width: f32,
    /// The available width that was targeted
    pub available_width: f32,
    /// Utilization percentage of available width
    pub width_utilization: f32,
}

impl FontSizeCalculation {
    /// Create a new font size calculation result
    pub fn new(text: &str, available_width: f32) -> Self {
        let font_size = calculate_required_font_size(text, available_width);
        let actual_width = measure_text_width(text, font_size);
        let was_reduced = font_size < SMALL_TEXT;
        let width_utilization = if available_width > 0.0 {
            (actual_width / available_width * 100.0).min(100.0)
        } else {
            0.0
        };

        Self {
            font_size,
            was_reduced,
            actual_width,
            available_width,
            width_utilization,
        }
    }

    /// Check if the text fits comfortably (less than 90% width utilization)
    pub fn fits_comfortably(&self) -> bool {
        self.width_utilization < 90.0
    }

    /// Check if the text is tightly packed (more than 95% width utilization)
    pub fn is_tightly_packed(&self) -> bool {
        self.width_utilization > 95.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_size_group_creation() {
        let group = FontSizeGroup::new();
        assert_eq!(group.current_font_size, SMALL_TEXT);
        assert!(!group.size_reduced);
        assert!(group.cell_requirements.is_empty());
    }

    #[test]
    fn test_test_data_constants() {
        // Verify all test data constants are defined and non-empty
        assert!(!test_data::LAST_GAME.is_empty());
        assert!(!test_data::NEXT_GAME.is_empty());
        assert!(!test_data::CHIEF_REF.is_empty());
        assert!(!test_data::TIMER.is_empty());
        assert!(!test_data::WATER_REF_1.is_empty());
        assert!(!test_data::WATER_REF_2.is_empty());
        assert!(!test_data::WATER_REF_3.is_empty());

        // Verify ALL_TEST_VALUES contains all values
        assert_eq!(test_data::ALL_TEST_VALUES.len(), 7);
        assert_eq!(test_data::ALL_TEST_VALUES[0], test_data::LAST_GAME);
        assert_eq!(test_data::ALL_TEST_VALUES[6], test_data::WATER_REF_3);
    }

    #[test]
    fn test_get_test_data_function() {
        // Verify get_test_data returns correct values for each cell
        assert_eq!(test_data::get_test_data(GameInfoCell::LastGame), test_data::LAST_GAME);
        assert_eq!(test_data::get_test_data(GameInfoCell::NextGame), test_data::NEXT_GAME);
        assert_eq!(test_data::get_test_data(GameInfoCell::ChiefRef), test_data::CHIEF_REF);
        assert_eq!(test_data::get_test_data(GameInfoCell::Timer), test_data::TIMER);
        assert_eq!(test_data::get_test_data(GameInfoCell::WaterRef1), test_data::WATER_REF_1);
        assert_eq!(test_data::get_test_data(GameInfoCell::WaterRef2), test_data::WATER_REF_2);
        assert_eq!(test_data::get_test_data(GameInfoCell::WaterRef3), test_data::WATER_REF_3);
    }

    #[test]
    fn test_long_names_require_font_reduction() {
        // Test that the longer test names will require font size reduction
        let long_names = [
            test_data::CHIEF_REF,
            test_data::TIMER,
            test_data::WATER_REF_2,
        ];

        for name in long_names {
            let required_size = calculate_required_font_size(name, 150.0); // Typical available width
            assert!(required_size < SMALL_TEXT, "Name '{}' should require font reduction", name);
        }
    }

    #[test]
    fn test_cell_available_width() {
        assert_eq!(GameInfoCell::LastGame.available_width(), 196.0);
        assert_eq!(GameInfoCell::ChiefRef.available_width(), 176.0);
    }

    #[test]
    fn test_measure_text_width() {
        // Test basic text measurement
        let width = measure_text_width("Hello", SMALL_TEXT);
        assert!(width > 0.0);

        // Empty text should have zero width
        let empty_width = measure_text_width("", SMALL_TEXT);
        assert_eq!(empty_width, 0.0);

        // Longer text should have greater width
        let short_width = measure_text_width("Hi", SMALL_TEXT);
        let long_width = measure_text_width("Hello World", SMALL_TEXT);
        assert!(long_width > short_width);
    }

    #[test]
    fn test_calculate_required_font_size_short_text() {
        let result = calculate_required_font_size("None", 200.0);
        assert_eq!(result, SMALL_TEXT);
    }

    #[test]
    fn test_calculate_required_font_size_long_text() {
        let long_text = "A very long person name that exceeds available width";
        let result = calculate_required_font_size(long_text, 100.0);
        assert!(result < SMALL_TEXT);
        assert!(result >= MIN_FONT_SIZE);
    }

    #[test]
    fn test_calculate_required_font_size_stepped() {
        let long_text = "Muhammad Danish Haikal Mohd Fadel";
        let result = calculate_required_font_size_stepped(long_text, 150.0);
        assert!(result <= SMALL_TEXT);
        assert!(result >= MIN_FONT_SIZE);

        // Verify the text actually fits at the calculated size
        let width = measure_text_width(long_text, result);
        assert!(width <= 150.0);
    }

    #[test]
    fn test_calculate_required_font_size_with_margin() {
        let text = "Russell Owen Camilo La Torre";
        let available_width = 180.0;
        let margin_percent = 10.0;

        let result = calculate_required_font_size_with_margin(text, available_width, margin_percent);
        let width = measure_text_width(text, result);

        // Should fit within the margin-reduced width
        let effective_width = available_width * 0.9;
        assert!(width <= effective_width);
    }

    #[test]
    fn test_calculate_group_font_size() {
        let texts_and_widths = vec![
            ("Short".to_string(), 200.0),
            ("A much longer text that requires reduction".to_string(), 150.0),
            ("Medium length text".to_string(), 180.0),
        ];

        let result = calculate_group_font_size(&texts_and_widths);

        // Should be reduced due to the longest text
        assert!(result < SMALL_TEXT);
        assert!(result >= MIN_FONT_SIZE);

        // All texts should fit at this size
        for (text, available_width) in &texts_and_widths {
            let width = measure_text_width(text, result);
            assert!(width <= *available_width, "Text '{}' doesn't fit", text);
        }
    }

    #[test]
    fn test_font_size_calculation_struct() {
        let text = "Norfatin Aainaa Binti Hashim";
        let available_width = 160.0;

        let calc = FontSizeCalculation::new(text, available_width);

        assert!(calc.font_size > 0.0);
        assert!(calc.actual_width > 0.0);
        assert_eq!(calc.available_width, available_width);
        assert!(calc.width_utilization >= 0.0 && calc.width_utilization <= 100.0);

        // If font was reduced, actual width should be close to available width
        if calc.was_reduced {
            assert!(calc.width_utilization > 80.0);
        }
    }

    #[test]
    fn test_dynamic_font_sizing_reset() {
        let mut dfs = DynamicFontSizing::new();
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Very long name");

        let initial_size = dfs.get_font_size(GameInfoCell::ChiefRef);
        assert!(initial_size <= SMALL_TEXT);

        dfs.reset_for_new_game(12345);
        assert_eq!(dfs.get_font_size(GameInfoCell::ChiefRef), SMALL_TEXT);
    }

    #[test]
    fn test_group_based_font_sizing() {
        let mut group = FontSizeGroup::new();

        // Add a short text that fits at default size
        group.update_cell_text(GameInfoCell::LastGame, "Short");
        assert_eq!(group.get_font_size(), SMALL_TEXT);
        assert!(!group.size_reduced);

        // Add a long text that requires reduction
        group.update_cell_text(GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre");
        let reduced_size = group.get_font_size();
        assert!(reduced_size < SMALL_TEXT);
        assert!(group.size_reduced);

        // All cells should now use the reduced size
        assert_eq!(group.get_font_size(), reduced_size);
    }

    #[test]
    fn test_update_multiple_cells() {
        let mut dfs = DynamicFontSizing::new();

        let cell_texts = vec![
            (GameInfoCell::LastGame, "Australia vs New Zealand".to_string()),
            (GameInfoCell::NextGame, "Nederlands vs South Africa".to_string()),
            (GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre".to_string()),
            (GameInfoCell::Timer, "Norfatin Aainaa Binti Hashim".to_string()),
        ];

        dfs.update_multiple_cells(&cell_texts);

        let font_size = dfs.get_font_size(GameInfoCell::LastGame);
        assert!(font_size <= SMALL_TEXT);
        assert!(font_size >= MIN_FONT_SIZE);

        // All cells should use the same font size
        for (cell, _) in &cell_texts {
            assert_eq!(dfs.get_font_size(*cell), font_size);
        }
    }

    #[test]
    fn test_font_sizing_info() {
        let mut dfs = DynamicFontSizing::new();
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre");
        dfs.update_cell_font_size(GameInfoCell::Timer, "Short");

        let info = dfs.get_sizing_info();
        assert!(info.is_reduced);
        assert!(info.current_font_size < SMALL_TEXT);
        assert_eq!(info.cell_details.len(), 2);

        // The most constraining cell should be the one with the long name
        let (most_constraining, _) = info.most_constraining_cell().unwrap();
        assert_eq!(most_constraining, GameInfoCell::ChiefRef);

        let (min_util, max_util, avg_util) = info.get_utilization_stats();
        assert!(min_util >= 0.0 && min_util <= 100.0);
        assert!(max_util >= 0.0 && max_util <= 100.0);
        assert!(avg_util >= 0.0 && avg_util <= 100.0);
        assert!(max_util >= min_util);
    }

    #[test]
    fn test_font_size_reset_functionality() {
        let mut dfs = DynamicFontSizing::new();

        // Initially should be at default size
        assert!(dfs.is_at_default_size());
        assert_eq!(dfs.get_font_size(GameInfoCell::ChiefRef), SMALL_TEXT);

        // Make a change that reduces font size
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre");
        assert!(!dfs.is_at_default_size());
        assert!(dfs.get_font_size(GameInfoCell::ChiefRef) < SMALL_TEXT);

        // Test force reset
        dfs.force_reset();
        assert!(dfs.is_at_default_size());
        assert_eq!(dfs.get_font_size(GameInfoCell::ChiefRef), SMALL_TEXT);

        // Should have recorded the reset in history
        let history = dfs.get_change_history();
        assert!(history.len() > 0);
        let last_change = &history[history.len() - 1];
        assert_eq!(last_change.new_size, SMALL_TEXT);
        assert_eq!(last_change.triggering_text, "Force Reset");
    }

    #[test]
    fn test_game_state_reset() {
        let mut dfs = DynamicFontSizing::new();

        // Make changes
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre");
        dfs.update_cell_font_size(GameInfoCell::Timer, "Norfatin Aainaa Binti Hashim");

        assert!(!dfs.is_at_default_size());
        let metrics_before = dfs.get_metrics().clone();

        // Reset for new game
        dfs.reset_for_new_game(12345);
        assert!(dfs.is_at_default_size());

        // Metrics should show a reset
        let metrics_after = dfs.get_metrics();
        assert_eq!(metrics_after.reset_count, metrics_before.reset_count + 1);

        // Same hash should not trigger another reset
        let old_history_len = dfs.get_change_history().len();
        dfs.reset_for_new_game(12345);
        assert_eq!(dfs.get_change_history().len(), old_history_len);

        // Different hash should trigger reset
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Another long name");
        assert!(!dfs.is_at_default_size());
        dfs.reset_for_new_game(54321);
        assert!(dfs.is_at_default_size());
    }

    #[test]
    fn test_reset_all_state() {
        let mut dfs = DynamicFontSizing::new();

        // Make changes and build up state
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre");
        dfs.reset_for_new_game(12345);
        dfs.update_cell_font_size(GameInfoCell::Timer, "Another name");

        // Should have history and metrics
        assert!(dfs.get_change_history().len() > 0);
        assert!(dfs.get_metrics().total_calculations > 0);

        // Reset all state
        dfs.reset_all_state();

        // Everything should be back to initial state
        assert!(dfs.is_at_default_size());
        assert_eq!(dfs.get_change_history().len(), 0);
        assert_eq!(dfs.get_metrics().total_calculations, 0);
        assert_eq!(dfs.get_metrics().reset_count, 0);
    }

    #[test]
    fn test_font_size_mapping_for_target_cells() {
        let dfs = DynamicFontSizing::new();

        // Test that target cells return dynamic font size
        assert_eq!(dfs.get_font_size(GameInfoCell::LastGame), SMALL_TEXT);
        assert_eq!(dfs.get_font_size(GameInfoCell::NextGame), SMALL_TEXT);
        assert_eq!(dfs.get_font_size(GameInfoCell::ChiefRef), SMALL_TEXT);
        assert_eq!(dfs.get_font_size(GameInfoCell::Timer), SMALL_TEXT);
        assert_eq!(dfs.get_font_size(GameInfoCell::WaterRef1), SMALL_TEXT);
        assert_eq!(dfs.get_font_size(GameInfoCell::WaterRef2), SMALL_TEXT);
        assert_eq!(dfs.get_font_size(GameInfoCell::WaterRef3), SMALL_TEXT);

        // All target cells should return the same font size (group-based)
        let font_size = dfs.get_font_size(GameInfoCell::ChiefRef);
        for cell in [
            GameInfoCell::LastGame,
            GameInfoCell::NextGame,
            GameInfoCell::Timer,
            GameInfoCell::WaterRef1,
            GameInfoCell::WaterRef2,
            GameInfoCell::WaterRef3,
        ] {
            assert_eq!(dfs.get_font_size(cell), font_size);
        }
    }

    #[test]
    fn test_font_sizing_metrics() {
        let mut dfs = DynamicFontSizing::new();

        // Initial state
        let metrics = dfs.get_metrics();
        assert_eq!(metrics.total_calculations, 0);
        assert_eq!(metrics.reduction_count, 0);
        assert_eq!(metrics.reset_count, 0);

        // Perform some calculations
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre");
        dfs.update_cell_font_size(GameInfoCell::Timer, "Short");

        let metrics = dfs.get_metrics();
        assert_eq!(metrics.total_calculations, 2);
        assert!(metrics.reduction_count > 0); // At least one should require reduction
        assert!(metrics.average_font_size > 0.0);

        // Test reset
        dfs.reset_for_new_game(12345);
        let metrics = dfs.get_metrics();
        assert_eq!(metrics.reset_count, 1);
    }

    #[test]
    fn test_change_history() {
        let mut dfs = DynamicFontSizing::new();

        // Should start with no history
        assert_eq!(dfs.get_change_history().len(), 0);

        // Make a change that should trigger font size reduction
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre");

        // Should have recorded the change
        let history = dfs.get_change_history();
        assert!(history.len() > 0);

        let last_change = &history[history.len() - 1];
        assert_eq!(last_change.old_size, SMALL_TEXT);
        assert!(last_change.new_size < SMALL_TEXT);
        assert_eq!(last_change.triggering_cell, GameInfoCell::ChiefRef);
        assert_eq!(last_change.triggering_text, "Russell Owen Camilo La Torre");

        // Test clear history
        dfs.clear_history();
        assert_eq!(dfs.get_change_history().len(), 0);
    }

    #[test]
    fn test_activity_summary() {
        let mut dfs = DynamicFontSizing::new();

        // Make some changes
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre");
        dfs.update_cell_font_size(GameInfoCell::Timer, "Norfatin Aainaa Binti Hashim");

        let summary = dfs.get_activity_summary();
        assert!(summary.is_reduced);
        assert!(summary.current_font_size < SMALL_TEXT);
        assert!(summary.total_changes > 0);
        assert!(summary.recent_activity_count > 0);
        assert!(summary.metrics.total_calculations > 0);
    }

    #[test]
    fn test_text_measurement_accuracy() {
        // Test basic text measurement functionality
        let short_text = "Test";
        let long_text = "This is a much longer text string";

        let short_width = measure_text_width(short_text, SMALL_TEXT);
        let long_width = measure_text_width(long_text, SMALL_TEXT);

        // Longer text should have greater width
        assert!(long_width > short_width);

        // Width should be positive
        assert!(short_width > 0.0);
        assert!(long_width > 0.0);
    }

    #[test]
    fn test_font_size_affects_text_width() {
        let text = "Sample Text";

        let small_width = measure_text_width(text, 12.0);
        let large_width = measure_text_width(text, 24.0);

        // Larger font should produce wider text
        assert!(large_width > small_width);

        // Should be roughly proportional (within reasonable tolerance)
        let ratio = large_width / small_width;
        assert!(ratio > 1.5 && ratio < 2.5); // Expect roughly 2x for 2x font size
    }

    #[test]
    fn test_empty_text_measurement() {
        let width = measure_text_width("", SMALL_TEXT);
        assert_eq!(width, 0.0);
    }

    #[test]
    fn test_calculate_required_font_size_basic() {
        // Test with text that should fit at default size
        let short_text = "OK";
        let available_width = 200.0;

        let font_size = calculate_required_font_size(short_text, available_width);
        assert_eq!(font_size, SMALL_TEXT);

        // Verify it actually fits
        let actual_width = measure_text_width(short_text, font_size);
        assert!(actual_width <= available_width);
    }

    #[test]
    fn test_calculate_required_font_size_reduction() {
        // Test with text that requires reduction
        let long_text = "Russell Owen Camilo La Torre";
        let narrow_width = 100.0;

        let font_size = calculate_required_font_size(long_text, narrow_width);
        assert!(font_size < SMALL_TEXT);
        assert!(font_size >= MIN_FONT_SIZE);

        // Verify it actually fits
        let actual_width = measure_text_width(long_text, font_size);
        assert!(actual_width <= narrow_width);
    }

    #[test]
    fn test_calculate_required_font_size_minimum() {
        // Test with extremely narrow width that forces minimum font size
        let long_text = "Muhammad Danish Haikal Mohd Fadel";
        let tiny_width = 10.0;

        let font_size = calculate_required_font_size(long_text, tiny_width);
        assert_eq!(font_size, MIN_FONT_SIZE);
    }

    #[test]
    fn test_binary_search_precision() {
        // Test that binary search finds optimal font size
        let text = "Norfatin Aainaa Binti Hashim";
        let available_width = 150.0;

        let font_size = calculate_required_font_size(text, available_width);
        let actual_width = measure_text_width(text, font_size);

        // Should be close to available width (within 1 pixel)
        assert!(actual_width <= available_width);
        assert!(available_width - actual_width < 1.0);

        // A slightly larger font size should exceed the width
        if font_size < SMALL_TEXT {
            let larger_width = measure_text_width(text, font_size + 0.5);
            assert!(larger_width > available_width);
        }
    }

    #[test]
    fn test_stepped_algorithm_comparison() {
        let text = "Tuan San Jonathan Chan";
        let available_width = 140.0;

        let binary_size = calculate_required_font_size(text, available_width);
        let stepped_size = calculate_required_font_size_stepped(text, available_width);

        // Both should fit
        assert!(measure_text_width(text, binary_size) <= available_width);
        assert!(measure_text_width(text, stepped_size) <= available_width);

        // Binary search should be more optimal (equal or larger font size)
        assert!(binary_size >= stepped_size);
    }

    #[test]
    fn test_margin_based_calculation() {
        let text = "Test Text";
        let available_width = 100.0;
        let margin_percent = 10.0;

        let normal_size = calculate_required_font_size(text, available_width);
        let margin_size = calculate_required_font_size_with_margin(text, available_width, margin_percent);

        // Margin version should be more conservative (smaller or equal)
        assert!(margin_size <= normal_size);

        // Should fit within the margin-reduced width
        let effective_width = available_width * 0.9;
        let actual_width = measure_text_width(text, margin_size);
        assert!(actual_width <= effective_width);
    }

    #[test]
    fn test_group_font_size_calculation() {
        let texts_and_widths = vec![
            ("Short".to_string(), 200.0),
            ("Medium length text".to_string(), 150.0),
            ("A very long text that requires significant reduction".to_string(), 120.0),
        ];

        let group_size = calculate_group_font_size(&texts_and_widths);

        // Should be reduced due to the longest text
        assert!(group_size <= SMALL_TEXT);
        assert!(group_size >= MIN_FONT_SIZE);

        // All texts should fit at this size
        for (text, available_width) in &texts_and_widths {
            let actual_width = measure_text_width(text, group_size);
            assert!(actual_width <= *available_width, "Text '{}' doesn't fit", text);
        }
    }

    #[test]
    fn test_font_size_calculation_struct_comprehensive() {
        let text = "Russell Owen Camilo La Torre";
        let available_width = 160.0;

        let calc = FontSizeCalculation::new(text, available_width);

        // Basic properties
        assert!(calc.font_size > 0.0);
        assert!(calc.actual_width > 0.0);
        assert_eq!(calc.available_width, available_width);

        // Utilization should be reasonable
        assert!(calc.width_utilization >= 0.0 && calc.width_utilization <= 100.0);

        // If reduced, should be tightly packed
        if calc.was_reduced {
            assert!(calc.width_utilization > 80.0);
            assert!(calc.is_tightly_packed());
            assert!(!calc.fits_comfortably());
        } else {
            assert!(calc.fits_comfortably());
            assert!(!calc.is_tightly_packed());
        }

        // Actual width should match calculated width
        let measured_width = measure_text_width(text, calc.font_size);
        assert!((calc.actual_width - measured_width).abs() < 0.1);
    }

    #[test]
    fn test_measurement_with_test_data() {
        // Test with all provided test data
        for &test_text in test_data::ALL_TEST_VALUES {
            let font_size = calculate_required_font_size(test_text, 150.0);
            let actual_width = measure_text_width(test_text, font_size);

            // Should fit within the available width
            assert!(actual_width <= 150.0, "Text '{}' doesn't fit", test_text);

            // Font size should be reasonable
            assert!(font_size >= MIN_FONT_SIZE);
            assert!(font_size <= SMALL_TEXT);
        }
    }

    #[test]
    fn test_font_resizing_with_specification_test_data() {
        // Test with the exact test data specified in the requirements
        let test_cases = vec![
            // Team names
            ("Australia", GameInfoCell::LastGame),
            ("New Zealand", GameInfoCell::LastGame),
            ("Nederlands", GameInfoCell::NextGame),
            ("South Africa", GameInfoCell::NextGame),

            // Referee names
            ("Russell Owen Camilo La Torre", GameInfoCell::ChiefRef),
            ("Norfatin Aainaa Binti Hashim", GameInfoCell::Timer),
            ("Tuan San Jonathan Chan", GameInfoCell::WaterRef1),
            ("Muhammad Danish Haikal Mohd Fadel", GameInfoCell::WaterRef2),
            ("A very long person name", GameInfoCell::WaterRef3),
        ];

        let mut dfs = DynamicFontSizing::new();

        for (text, cell) in test_cases {
            // Update the font sizing with this text
            dfs.update_cell_font_size(cell, text);

            // Get the calculated font size
            let font_size = dfs.get_font_size(cell);

            // Verify font size is within acceptable bounds
            assert!(font_size >= MIN_FONT_SIZE,
                "Font size {} for '{}' is below minimum {}", font_size, text, MIN_FONT_SIZE);
            assert!(font_size <= SMALL_TEXT,
                "Font size {} for '{}' exceeds maximum {}", font_size, text, SMALL_TEXT);

            // Verify the text actually fits at the calculated size
            let available_width = cell.available_width();
            let actual_width = measure_text_width(text, font_size);
            assert!(actual_width <= available_width,
                "Text '{}' width {} exceeds available width {} at font size {}",
                text, actual_width, available_width, font_size);

            // Verify long names trigger font reduction
            if text.len() > 20 {
                assert!(font_size < SMALL_TEXT,
                    "Long text '{}' should trigger font reduction, but font size is {}",
                    text, font_size);
            }
        }
    }

    #[test]
    fn test_all_referee_rows_visible_with_long_names() {
        // Test that all referee information rows remain visible when using long names
        let mut dfs = DynamicFontSizing::new();

        // Apply all the long referee names simultaneously
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre");
        dfs.update_cell_font_size(GameInfoCell::Timer, "Norfatin Aainaa Binti Hashim");
        dfs.update_cell_font_size(GameInfoCell::WaterRef1, "Tuan San Jonathan Chan");
        dfs.update_cell_font_size(GameInfoCell::WaterRef2, "Muhammad Danish Haikal Mohd Fadel");
        dfs.update_cell_font_size(GameInfoCell::WaterRef3, "A very long person name");

        // All referee cells should use the same font size (group-based sizing)
        let chief_ref_size = dfs.get_font_size(GameInfoCell::ChiefRef);
        let timer_size = dfs.get_font_size(GameInfoCell::Timer);
        let water_ref1_size = dfs.get_font_size(GameInfoCell::WaterRef1);
        let water_ref2_size = dfs.get_font_size(GameInfoCell::WaterRef2);
        let water_ref3_size = dfs.get_font_size(GameInfoCell::WaterRef3);

        // Verify all referee cells use the same font size
        assert_eq!(chief_ref_size, timer_size);
        assert_eq!(timer_size, water_ref1_size);
        assert_eq!(water_ref1_size, water_ref2_size);
        assert_eq!(water_ref2_size, water_ref3_size);

        // Verify the font size is reduced from default
        assert!(chief_ref_size < SMALL_TEXT,
            "Font size should be reduced with long names, but is {}", chief_ref_size);

        // Verify all names fit at the calculated size
        let referee_names = vec![
            ("Russell Owen Camilo La Torre", GameInfoCell::ChiefRef),
            ("Norfatin Aainaa Binti Hashim", GameInfoCell::Timer),
            ("Tuan San Jonathan Chan", GameInfoCell::WaterRef1),
            ("Muhammad Danish Haikal Mohd Fadel", GameInfoCell::WaterRef2),
            ("A very long person name", GameInfoCell::WaterRef3),
        ];

        for (name, cell) in referee_names {
            let available_width = cell.available_width();
            let actual_width = measure_text_width(name, chief_ref_size);
            assert!(actual_width <= available_width,
                "Referee name '{}' doesn't fit: width {} > available {}",
                name, actual_width, available_width);
        }
    }

    #[test]
    fn test_font_sizing_maintains_readability() {
        // Test that font sizing maintains readability even with very long names
        let extreme_test_cases = vec![
            ("Russell Owen Camilo La Torre", GameInfoCell::ChiefRef),
            ("Muhammad Danish Haikal Mohd Fadel", GameInfoCell::WaterRef2),
        ];

        for (text, cell) in extreme_test_cases {
            let font_size = calculate_required_font_size(text, cell.available_width());

            // Verify font size never goes below minimum readable size
            assert!(font_size >= MIN_FONT_SIZE,
                "Font size {} for '{}' is below minimum readable size {}",
                font_size, text, MIN_FONT_SIZE);

            // Verify font size is reasonable for UI (not too small)
            assert!(font_size >= 12.0,
                "Font size {} for '{}' may be too small for comfortable reading",
                font_size, text);

            // Verify text measurement is accurate
            let measured_width = measure_text_width(text, font_size);
            let available_width = cell.available_width();
            assert!(measured_width <= available_width + 1.0, // Allow 1px tolerance
                "Measured width {} exceeds available width {} for '{}'",
                measured_width, available_width, text);
        }
    }

    #[test]
    fn test_font_sizing_reset_on_state_changes() {
        // Test that font sizing resets properly when game state changes
        let mut dfs = DynamicFontSizing::new();

        // Initially at default size
        assert!(dfs.is_at_default_size());
        assert_eq!(dfs.get_font_size(GameInfoCell::ChiefRef), SMALL_TEXT);

        // Apply long name that reduces font size
        dfs.update_cell_font_size(GameInfoCell::ChiefRef, "Russell Owen Camilo La Torre");
        assert!(!dfs.is_at_default_size());
        let reduced_size = dfs.get_font_size(GameInfoCell::ChiefRef);
        assert!(reduced_size < SMALL_TEXT);

        // Simulate new game state (reset)
        dfs.reset_all_state();
        assert!(dfs.is_at_default_size());
        assert_eq!(dfs.get_font_size(GameInfoCell::ChiefRef), SMALL_TEXT);

        // Apply different long name
        dfs.update_cell_font_size(GameInfoCell::Timer, "Norfatin Aainaa Binti Hashim");
        let new_reduced_size = dfs.get_font_size(GameInfoCell::Timer);
        assert!(new_reduced_size < SMALL_TEXT);

        // Verify other cells also use the reduced size (group behavior)
        assert_eq!(dfs.get_font_size(GameInfoCell::ChiefRef), new_reduced_size);
    }

    #[test]
    fn test_specification_team_names_font_sizing() {
        // Test font sizing specifically for the team names in the specification
        let team_test_cases = vec![
            // Last Game teams
            ("Australia", GameInfoCell::LastGame),
            ("New Zealand", GameInfoCell::LastGame),

            // Next Game teams
            ("Nederlands", GameInfoCell::NextGame),
            ("South Africa", GameInfoCell::NextGame),
        ];

        let mut dfs = DynamicFontSizing::new();

        for (team_name, cell) in team_test_cases {
            dfs.update_cell_font_size(cell, team_name);
            let font_size = dfs.get_font_size(cell);

            // Team names are relatively short, should not require much reduction
            assert!(font_size >= SMALL_TEXT * 0.9,
                "Team name '{}' caused excessive font reduction: {}",
                team_name, font_size);

            // Verify team name fits comfortably
            let available_width = cell.available_width();
            let actual_width = measure_text_width(team_name, font_size);
            let utilization = (actual_width / available_width) * 100.0;

            assert!(utilization <= 100.0,
                "Team name '{}' exceeds available width: {}% utilization",
                team_name, utilization);

            // Team names should fit comfortably (less than 80% width utilization)
            assert!(utilization <= 80.0,
                "Team name '{}' uses too much width: {}% utilization",
                team_name, utilization);
        }
    }

    #[test]
    fn test_edge_cases() {
        // Test with zero width
        let font_size = calculate_required_font_size("Test", 0.0);
        assert_eq!(font_size, MIN_FONT_SIZE);

        // Test with negative width
        let font_size = calculate_required_font_size("Test", -10.0);
        assert_eq!(font_size, MIN_FONT_SIZE);

        // Test with very large width
        let font_size = calculate_required_font_size("Test", 10000.0);
        assert_eq!(font_size, SMALL_TEXT);
    }

    /// CSV Test Data Structure
    #[derive(Debug, Clone)]
    struct CsvTestCase {
        cell_type: String,
        text_content: String,
        expected_font_reduction: bool,
        description: String,
    }

    /// Parse CSV test data from file
    fn load_csv_test_data() -> Vec<CsvTestCase> {
        // Try to load CSV content, fallback to embedded test data if file not found
        let csv_content = match std::fs::read_to_string("refbox/test_data/font_sizing_test_data.csv") {
            Ok(content) => content,
            Err(_) => {
                // Fallback to embedded test data
                r#"# Font Sizing Test Data - Embedded Fallback
ChiefRef,Russell Owen Camilo La Torre,true,Specification test data - Chief Ref
Timer,Norfatin Aainaa Binti Hashim,true,Specification test data - Timer
WaterRef1,Tuan San Jonathan Chan,false,Specification test data - Water Ref 1
WaterRef2,Muhammad Danish Haikal Mohd Fadel,true,Specification test data - Water Ref 2
WaterRef3,A very long person name,false,Specification test data - Water Ref 3
LastGame,Australia,false,Short team name
NextGame,New Zealand,false,Short team name
ChiefRef,John Smith,false,Short referee name
Timer,Jane Doe,false,Short referee name"#.to_string()
            }
        };
        let mut test_cases = Vec::new();

        for line in csv_content.lines() {
            let line = line.trim();
            // Skip comments and empty lines
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 4 {
                let expected_reduction = parts[2].trim().to_lowercase() == "true";
                test_cases.push(CsvTestCase {
                    cell_type: parts[0].trim().to_string(),
                    text_content: parts[1].trim().to_string(),
                    expected_font_reduction: expected_reduction,
                    description: parts[3..].join(",").trim().to_string(),
                });
            }
        }

        test_cases
    }

    /// Convert cell type string to GameInfoCell enum
    fn parse_cell_type(cell_type: &str) -> Option<GameInfoCell> {
        match cell_type {
            "LastGame" => Some(GameInfoCell::LastGame),
            "NextGame" => Some(GameInfoCell::NextGame),
            "ChiefRef" => Some(GameInfoCell::ChiefRef),
            "Timer" => Some(GameInfoCell::Timer),
            "WaterRef1" => Some(GameInfoCell::WaterRef1),
            "WaterRef2" => Some(GameInfoCell::WaterRef2),
            "WaterRef3" => Some(GameInfoCell::WaterRef3),
            _ => None,
        }
    }

    #[test]
    fn test_csv_data_loading() {
        // Test that CSV data loads correctly
        let test_cases = load_csv_test_data();
        assert!(!test_cases.is_empty(), "CSV test data should not be empty");

        // Verify we have some expected test cases
        let has_specification_data = test_cases.iter().any(|case|
            case.text_content == "Russell Owen Camilo La Torre"
        );
        assert!(has_specification_data, "Should contain specification test data");

        println!("Loaded {} test cases from CSV", test_cases.len());
        for case in &test_cases[..5] { // Print first 5 cases
            println!("  {} -> '{}' (reduction: {}) - {}",
                case.cell_type, case.text_content, case.expected_font_reduction, case.description);
        }
    }

    #[test]
    fn test_font_sizing_with_csv_data() {
        // Load test data from CSV file
        let test_cases = load_csv_test_data();
        let mut dfs = DynamicFontSizing::new();

        println!("\n=== Font Sizing Test Results ===");

        for (i, case) in test_cases.iter().enumerate() {
            if let Some(cell) = parse_cell_type(&case.cell_type) {
                // Reset for each test to get individual results
                dfs.reset_all_state();

                // Update font sizing with this text
                dfs.update_cell_font_size(cell, &case.text_content);
                let font_size = dfs.get_font_size(cell);

                // Check if font was reduced
                let was_reduced = font_size < SMALL_TEXT;

                // Verify expectations
                if case.expected_font_reduction {
                    assert!(was_reduced,
                        "Test case {}: '{}' should have triggered font reduction but didn't. Font size: {}",
                        i + 1, case.text_content, font_size);
                }

                // Verify font size is within bounds
                assert!(font_size >= MIN_FONT_SIZE,
                    "Test case {}: Font size {} is below minimum {}",
                    i + 1, font_size, MIN_FONT_SIZE);
                assert!(font_size <= SMALL_TEXT,
                    "Test case {}: Font size {} exceeds maximum {}",
                    i + 1, font_size, SMALL_TEXT);

                // Verify text fits at calculated size
                let available_width = cell.available_width();
                let actual_width = measure_text_width(&case.text_content, font_size);
                assert!(actual_width <= available_width + 1.0, // Allow 1px tolerance
                    "Test case {}: Text '{}' doesn't fit. Width: {}, Available: {}",
                    i + 1, case.text_content, actual_width, available_width);

                // Print results
                println!("Test {}: {} | '{}' | Font: {:.1}px | Reduced: {} | Fits: ✓ | {}",
                    i + 1,
                    case.cell_type,
                    case.text_content,
                    font_size,
                    if was_reduced { "YES" } else { "NO" },
                    case.description
                );
            }
        }

        println!("=== All CSV tests passed! ===\n");
    }

    #[test]
    fn test_group_font_sizing_with_csv_data() {
        // Test group-based font sizing using CSV data
        let test_cases = load_csv_test_data();
        let mut dfs = DynamicFontSizing::new();

        println!("\n=== Group Font Sizing Test ===");

        // Find referee test cases from CSV
        let referee_cases: Vec<_> = test_cases.iter()
            .filter(|case| matches!(case.cell_type.as_str(), "ChiefRef" | "Timer" | "WaterRef1" | "WaterRef2" | "WaterRef3"))
            .take(5) // Take first 5 referee cases
            .collect();

        if referee_cases.len() >= 5 {
            println!("Testing group font sizing with {} referee names:", referee_cases.len());

            // Apply all referee names simultaneously
            for case in &referee_cases {
                if let Some(cell) = parse_cell_type(&case.cell_type) {
                    dfs.update_cell_font_size(cell, &case.text_content);
                    println!("  {} -> '{}'", case.cell_type, case.text_content);
                }
            }

            // Check that all referee cells use the same font size
            let chief_ref_size = dfs.get_font_size(GameInfoCell::ChiefRef);
            let timer_size = dfs.get_font_size(GameInfoCell::Timer);
            let water_ref1_size = dfs.get_font_size(GameInfoCell::WaterRef1);
            let water_ref2_size = dfs.get_font_size(GameInfoCell::WaterRef2);
            let water_ref3_size = dfs.get_font_size(GameInfoCell::WaterRef3);

            println!("\nFont sizes after group sizing:");
            println!("  Chief Ref: {:.1}px", chief_ref_size);
            println!("  Timer: {:.1}px", timer_size);
            println!("  Water Ref 1: {:.1}px", water_ref1_size);
            println!("  Water Ref 2: {:.1}px", water_ref2_size);
            println!("  Water Ref 3: {:.1}px", water_ref3_size);

            // Verify all sizes are equal (group-based sizing)
            assert_eq!(chief_ref_size, timer_size, "Chief Ref and Timer should have same font size");
            assert_eq!(timer_size, water_ref1_size, "Timer and Water Ref 1 should have same font size");
            assert_eq!(water_ref1_size, water_ref2_size, "Water Ref 1 and 2 should have same font size");
            assert_eq!(water_ref2_size, water_ref3_size, "Water Ref 2 and 3 should have same font size");

            println!("\n✓ All referee cells use consistent font size: {:.1}px", chief_ref_size);

            // Verify all names fit at the group font size
            for case in &referee_cases {
                if let Some(cell) = parse_cell_type(&case.cell_type) {
                    let available_width = cell.available_width();
                    let actual_width = measure_text_width(&case.text_content, chief_ref_size);
                    assert!(actual_width <= available_width + 1.0,
                        "Text '{}' doesn't fit at group font size {:.1}px",
                        case.text_content, chief_ref_size);
                }
            }

            println!("✓ All referee names fit within their available widths");
        }

        println!("=== Group font sizing test passed! ===\n");
    }

    #[test]
    fn test_manual_csv_update_instructions() {
        // This test provides instructions for manually updating the CSV file
        println!("\n=== Manual CSV Testing Instructions ===");
        println!("To test with your own data:");
        println!("1. Edit the file: refbox/test_data/font_sizing_test_data.csv");
        println!("2. Add your test cases in the format:");
        println!("   cell_type,text_content,expected_font_reduction,description");
        println!("3. Valid cell_types: LastGame, NextGame, ChiefRef, Timer, WaterRef1, WaterRef2, WaterRef3");
        println!("4. Run: cargo test test_font_sizing_with_csv_data --package refbox -- --nocapture");
        println!("5. Or run: cargo test test_group_font_sizing_with_csv_data --package refbox -- --nocapture");
        println!("\nExample CSV entries:");
        println!("ChiefRef,Your Custom Referee Name,true,Custom test case");
        println!("Timer,Another Test Name,false,Another custom test");
        println!("=== End Instructions ===\n");

        // Always pass - this is just informational
        assert!(true);
    }

    #[test]
    fn test_csv_based_font_sizing_demo() {
        // Demonstrate CSV-based testing with your specification data
        println!("\n=== CSV-Based Font Sizing Demo ===");
        println!("Testing with your specification referee names:");

        let test_data = vec![
            ("ChiefRef", "Russell Owen Camilo La Torre"),
            ("Timer", "Norfatin Aainaa Binti Hashim"),
            ("WaterRef1", "Tuan San Jonathan Chan"),
            ("WaterRef2", "Muhammad Danish Haikal Mohd Fadel"),
            ("WaterRef3", "A very long person name"),
            ("LastGame", "Australia"),
            ("NextGame", "New Zealand"),
        ];

        for (cell_type, text) in test_data {
            if let Some(cell) = parse_cell_type(cell_type) {
                let mut dfs = DynamicFontSizing::new();
                dfs.update_cell_font_size(cell, text);
                let font_size = dfs.get_font_size(cell);

                let was_reduced = font_size < SMALL_TEXT;
                let available_width = cell.available_width();
                let actual_width = measure_text_width(text, font_size);

                println!("{}: '{}' -> {:.1}px ({}), fits in {:.0}px",
                    cell_type,
                    text,
                    font_size,
                    if was_reduced { "REDUCED" } else { "DEFAULT" },
                    available_width
                );

                // Verify it works
                assert!(font_size >= MIN_FONT_SIZE);
                assert!(font_size <= SMALL_TEXT);
                assert!(actual_width <= available_width + 1.0);
            }
        }

        println!("\n💡 To test with your own data:");
        println!("   1. Edit: refbox/test_data/font_sizing_test_data.csv");
        println!("   2. Add: cell_type,text_content,expected_reduction,description");
        println!("   3. Run: cargo test test_csv_based_font_sizing_demo --package refbox -- --nocapture");
        println!("=== Demo completed! ===\n");
    }
}
