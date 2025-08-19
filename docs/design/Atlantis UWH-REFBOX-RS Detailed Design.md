# Atlantis UWH-REFBOX-RS Detailed Design

Author: Augment Agent (based on GPT‑5)
Date: 2025-08-18
Last Updated: 2025-08-18

## 1. Purpose and Goals
This document provides a comprehensive design overview of the UWH RefBox Rust workspace, including recent enhancements and automated testing strategy. It covers:
- **Dynamic Font Sizing System**: Intelligent font scaling for UI elements with long text content
- **Referee Information Display**: Enhanced table layout for referee assignments and roles
- **Automated Testing Strategy**: Comprehensive testing approach for UI, integration, and functionality
- **CI/CD Pipeline Integration**: Continuous integration and deployment configuration
- **Developer Workflow**: Tools and processes for maintaining code quality
- **Internationalization Support**: Multi-language layout consistency and validation

## 2. Recent Enhancements

### 2.1 Dynamic Font Sizing System
**Implementation**: `refbox/src/app/dynamic_font_sizing.rs`

The dynamic font sizing system automatically adjusts font sizes to accommodate long text content while maintaining readability and layout consistency.

**Key Features**:
- **Group-based Font Sizing**: All referee information cells use consistent font sizes
- **Real-time Text Measurement**: Uses Roboto-Medium.ttf font for accurate width calculations
- **Minimum Readability**: Enforces minimum font size (12px) to maintain usability
- **State Management**: Resets font sizes on game state changes

**Target Cells**:
- Chief Ref: 176px available width
- Timer: 176px available width
- Water Ref 1-3: 176px available width each
- Team Names (Last/Next Game): 196px available width

**Algorithm**:
1. Measure text width at default size (18px)
2. If text exceeds available width, use binary search to find optimal size
3. Apply group-based sizing (all cells use the smallest required size)
4. Enforce minimum font size for readability

### 2.2 Referee Information Display
**Implementation**: `refbox/src/app/view_builders/main_view.rs`

Enhanced table layout displays referee assignments with proper spacing and font sizing.

**Referee Roles Supported**:
- **Chief Ref**: Primary referee for the game
- **Timer**: Official timekeeper
- **Water Ref 1-3**: In-water referees (up to 3 positions)

**Layout Features**:
- Fixed-width labels (120px) for consistent alignment
- Dynamic font sizing for long referee names
- Compact vertical spacing to fit all roles on screen
- Integration with UWH Portal for referee data

### 2.3 Current State Assessment
- **Workspace Structure**: Multiple crates (refbox, uwh-common, overlay, etc.)
- **Testing Coverage**: Unit tests for config, tournament manager, sound controller, uwh-common
- **CI Pipeline**: GitHub Actions runs `cargo test --all` across platforms
- **Recent Additions**: Dynamic font sizing tests, referee information validation
- **Remaining Gaps**: Visual regression testing, comprehensive i18n validation

## 3. Target Architecture Overview
Introduce a new workspace member `integration-tests` dedicated to tests that span crates and UI/layout logic. Keep existing unit tests in-place within each crate. Run tests locally and in CI.

High-level layers:
- Unit tests: localized logic checks (e.g., label width match expression) within or alongside crates
- Integration tests: cross-module behaviors, app flows
- UI/layout structural tests: assert table structure and length/width decisions (not pixel rendering)
- Internationalization tests: build app or components under en-US, es, fr and verify rules
- Visual regression-style: snapshot-like structural expectations, avoiding brittle pixel diffs

## 4. Directory Structure (proposed)
- integration-tests/
  - Cargo.toml
  - src/
    - lib.rs
    - ui_tests/
      - mod.rs
      - view_builders/
        - main_view_tests.rs
        - layout_tests.rs
        - label_width_tests.rs
      - internationalization/
        - language_tests.rs
        - layout_consistency_tests.rs
    - integration/
      - tournament_manager_tests.rs
      - config_tests.rs
    - visual_regression/
      - snapshot_tests.rs
      - layout_validation.rs
    - utils/
      - mock_data.rs
      - test_helpers.rs
      - layout_assertions.rs
  - tests/
    - ui_integration.rs
    - full_app_tests.rs

## 5. Testing Focus Areas and Examples

### 5.1 Dynamic Font Sizing Tests
**Location**: `refbox/src/app/dynamic_font_sizing.rs`

**Test Data** (matching specification requirements):
```rust
// Team names
("Australia", "New Zealand")        // Last Game
("Nederlands", "South Africa")      // Next Game

// Referee names
("Russell Owen Camilo La Torre")    // Chief Ref
("Norfatin Aainaa Binti Hashim")   // Timer
("Tuan San Jonathan Chan")         // Water Ref 1
("Muhammad Danish Haikal Mohd Fadel") // Water Ref 2
("A very long person name")        // Water Ref 3
```

**Key Test Cases**:
- `test_font_resizing_with_specification_test_data()`: Validates all specified names
- `test_all_referee_rows_visible_with_long_names()`: Ensures all referee rows remain visible
- `test_font_sizing_maintains_readability()`: Enforces minimum readable font sizes
- `test_font_sizing_reset_on_state_changes()`: Validates state management
- `test_specification_team_names_font_sizing()`: Team name specific validation

### 5.2 Referee Information Row Tests
**Location**: `integration-tests/src/ui_tests/view_builders/main_view_tests.rs`

**Test Coverage**:
- `test_referee_information_rows_structure()`: Validates table row structure
- `test_referee_information_values_display()`: Tests value display and formatting
- `test_table_row_visibility_and_layout()`: Ensures all rows are visible
- `test_font_sizing_consistency_across_referee_cells()`: Group-based font sizing

### 5.3 Label Width and Layout (foundation)
- Constants:
  - `GAME_LABEL_WIDTH = 100.0`
  - `REF_LABEL_WIDTH = 120.0`
- Single `match` expression determines width for known labels; default uses `FillPortion`
- Unit tests: verify mappings for Last/Next Game, Chief Ref, Timer, Water Ref 1-3, and default path

### 5.4 Internationalization (i18n)
**Location**: `integration-tests/src/ui_tests/internationalization/`

**Test Coverage**:
- Instantiate app/components under `en-US`, `es`, `fr`
- Verify that fixed-width rules still apply regardless of translated label text
- Validate dynamic font sizing works with translated referee role labels
- Ensure layout consistency across different language text lengths
- Test that referee information labels translate correctly while maintaining layout

### 5.3 Layout Structure Tests
- Validate number and type of rows (4-column vs 2-column)
- Assert fixed widths are used where expected; proportional widths elsewhere

### 5.4 Integration Tests
- TournamentManager clock start/stop behaviors
- Config persistence round-trips through UI interactions (as feasible)

### 5.5 Visual Regression-Style
- Snapshot expectations on structural properties (counts, label types, width kinds) — not pixels

## 6. CI/CD Integration

### 6.1 Current Pipeline Status
**File**: `.github/workflows/rust.yml`

**Current Configuration**:
- Runs `cargo test --all` across multiple platforms (Windows, macOS, Linux)
- Builds all workspace crates including refbox, uwh-common, overlay
- Validates compilation and basic functionality

### 6.2 Enhanced Pipeline Features
**Planned Enhancements**:
- Build and test the `integration-tests` crate
- Run dynamic font sizing tests with specification test data
- Validate referee information display tests
- Execute i18n-focused tests (serialized with `--test-threads=1` if needed)
- Maintain backward compatibility (keep existing jobs green)

### 6.3 Test Categories in CI
1. **Unit Tests**: Individual crate functionality
2. **Integration Tests**: Cross-crate interactions
3. **UI/Layout Tests**: Table structure and font sizing validation
4. **Font Sizing Tests**: Dynamic sizing with long names
5. **Internationalization Tests**: Multi-language layout consistency

## 7. Implementation Status

### 7.1 Completed Features ✅
- **Dynamic Font Sizing System**: Fully implemented with comprehensive test coverage
- **Referee Information Display**: All 5 referee roles (Chief Ref, Timer, Water Ref 1-3) supported
- **Font Size Optimization**: Reduced from 19px to 18px for better space utilization
- **Group-based Font Sizing**: Consistent font sizes across all referee information cells
- **Test Coverage**: Specification-compliant test data for all referee names and team names
- **State Management**: Font sizing resets properly on game state changes

### 7.2 Current Configuration
- **Default Font Size**: 18px (SMALL_TEXT)
- **Minimum Font Size**: 12px (MIN_FONT_SIZE)
- **Available Widths**:
  - Referee cells: 176px
  - Team name cells: 196px
- **Font**: Roboto-Medium.ttf for accurate text measurement

### 7.3 Test Data Validation ✅
All test cases use the exact specification data:
- **Team Names**: Australia, New Zealand, Nederlands, South Africa
- **Referee Names**: Russell Owen Camilo La Torre, Norfatin Aainaa Binti Hashim, etc.
- **Font Sizing**: Validates that all referee rows remain visible with long names
- **Layout**: Ensures Water Ref 3 is no longer cut off from display

## 8. Timer System Functional Requirements

### 8.1 Core Timer Architecture

#### 8.1.1 Multi-Clock System
- **Game Clock**: Primary countdown timer for game periods
  - Supports countdown mode for regular periods (First Half, Second Half, Overtime)
  - Supports count-up mode for Sudden Death periods
  - Precision: Nanosecond accuracy using `tokio::time::Instant`
  - Range: 0 to 65,535 seconds (18+ hours)

- **Timeout Clock**: Independent timer for timeout periods
  - Team timeouts, referee timeouts, penalty shots
  - Can run concurrently with stopped game clock
  - Supports both countdown and count-up modes
  - Automatic timeout type detection and display

- **Penalty Timers**: Individual penalty duration tracking
  - Multiple concurrent penalty timers per team
  - Automatic calculation of time remaining based on game state
  - Cross-period penalty time tracking
  - Support for 30-second, 1-minute, 2-minute, 4-minute, 5-minute, and Total Dismissal penalties

#### 8.1.2 Clock State Management
- **Three Clock States**:
  - `Stopped`: Clock paused with fixed time value
  - `CountingDown`: Active countdown from start time with remaining duration
  - `CountingUp`: Active count-up from start time (used in Sudden Death)

- **State Transitions**: Atomic state changes with logging
- **Thread Safety**: Mutex-protected tournament manager for concurrent access
- **State Persistence**: Clock state maintained across application restarts

### 8.2 Game Period Management

#### 8.2.1 Period Types and Durations
- **Between Games**: Variable duration based on schedule
- **First Half**: Configurable duration (default: 15 minutes)
- **Half Time**: Configurable break duration (default: 3 minutes)
- **Second Half**: Same duration as First Half
- **Pre-Overtime**: Configurable break (default: 3 minutes)
- **Overtime First Half**: Configurable duration (default: 5 minutes)
- **Overtime Half Time**: Configurable break (default: 3 minutes)
- **Overtime Second Half**: Same as Overtime First Half
- **Pre-Sudden Death**: Configurable break (default: 1 minute)
- **Sudden Death**: Count-up timer with no maximum duration

#### 8.2.2 Period Transition Logic
- **Automatic Transitions**: Seamless progression between periods when time expires
- **Manual Transitions**: Referee-controlled period advancement
- **Conditional Transitions**: Overtime and Sudden Death based on game configuration
- **Score-Based Logic**: Automatic game end when score difference determined
- **Reset Functionality**: Game reset during Between Games period

### 8.3 Timeout System

#### 8.3.1 Timeout Types
- **Team Timeouts**:
  - Configurable count per team (default: 1 per half)
  - Configurable duration (default: 60 seconds)
  - Can be counted per half or per game
  - Automatic timeout count reset at half time (if configured)

- **Referee Timeouts**:
  - Unlimited duration
  - Manual start/stop control
  - Can be initiated during any game state

- **Penalty Shots**:
  - Standard penalty shot: Count-up timer
  - Rugby penalty shot: Countdown timer (default: 45 seconds)
  - Automatic timeout state management

#### 8.3.2 Timeout State Management
- **Concurrent Operation**: Timeouts can run while game clock is stopped
- **State Preservation**: Timeout state maintained during clock operations
- **Automatic Termination**: Timeouts end automatically or manually
- **Display Integration**: Timeout information displayed on all output devices

### 8.4 Penalty Timer System

#### 8.4.1 Penalty Duration Tracking
- **Individual Penalty Tracking**: Each penalty tracked separately with:
  - Start time and period
  - Player number and team color
  - Penalty type and duration
  - Infraction type
  - Start instant for real-time calculations

- **Cross-Period Calculations**:
  - Penalties continue across period boundaries
  - Automatic adjustment for non-penalty periods (Half Time, etc.)
  - Complex time elapsed calculations across multiple periods

#### 8.4.2 Penalty Time Calculations
- **Time Remaining**: Real-time calculation of penalty time remaining
- **Time Elapsed**: Calculation of penalty time served
- **Completion Detection**: Automatic detection when penalty is fully served
- **Total Dismissal Handling**: Special handling for indefinite penalties

### 8.5 Time Synchronization and Broadcasting

#### 8.5.1 Update Mechanism
- **Subscription System**: Watch channel for clock running state changes
- **Automatic Updates**: Continuous time updates when clocks are running
- **Precision Timing**: Next update time calculation for optimal performance
- **Snapshot Generation**: Real-time game state snapshots with current time values

#### 8.5.2 Multi-Output Broadcasting
- **LED Panel Output**: Binary protocol for LED matrix displays
- **Serial Communication**: Hardware integration via serial ports
- **Network Broadcasting**: JSON and binary network protocols
- **UI Updates**: Real-time UI updates via message passing
- **Overlay System**: Live streaming overlay integration

### 8.6 Time Display and Formatting

#### 8.6.1 Display Formats
- **Standard Format**: MM:SS for most displays
- **Long Format**: MM:SS.sss with milliseconds for precise timing
- **Compact Format**: M:SS for space-constrained displays
- **Penalty Format**: Time remaining or "TD" for Total Dismissal

#### 8.6.2 Visual Indicators
- **Period Identification**: Clear indication of current game period
- **Timeout Indicators**: Visual distinction for different timeout types
- **Clock Running State**: Visual indication when clocks are active
- **Color Coding**: Different colors for different game states (normal, overtime, sudden death)

### 8.7 Configuration and Customization

#### 8.7.1 Timing Configuration
- **Game Configuration**: All timing values configurable via TOML files
- **UWH Portal Integration**: Automatic timing rule application from tournament management
- **Runtime Adjustment**: Manual time setting during games (when clock stopped)
- **Default Values**: Sensible defaults for all timing parameters

#### 8.7.2 Behavioral Configuration
- **Timeout Counting**: Per-half vs per-game timeout counting
- **Period Enablement**: Enable/disable overtime and sudden death
- **Single Half Mode**: Support for shortened game formats
- **Break Durations**: Configurable break times between periods

### 8.8 Error Handling and Validation

#### 8.8.1 Time Validation
- **Range Checking**: All time values validated within acceptable ranges
- **Negative Time Prevention**: Protection against negative time calculations
- **Overflow Protection**: Prevention of time value overflows
- **State Consistency**: Validation of clock state consistency

#### 8.8.2 Error Recovery
- **Graceful Degradation**: System continues operation despite timing errors
- **Automatic Correction**: Self-correction of minor timing inconsistencies
- **Logging**: Comprehensive logging of all timing operations and errors
- **Manual Override**: Referee ability to manually correct timing issues

### 8.9 Performance Requirements

#### 8.9.1 Timing Accuracy
- **Precision**: Nanosecond-level internal precision
- **Display Accuracy**: Sub-second accuracy for all displayed times
- **Update Frequency**: Minimum 10Hz update rate for smooth display
- **Latency**: Maximum 100ms latency from time change to display update

#### 8.9.2 Resource Efficiency
- **CPU Usage**: Minimal CPU overhead during normal operation
- **Memory Usage**: Efficient memory usage for timing data structures
- **Network Efficiency**: Optimized network protocols for time broadcasting
- **Battery Life**: Power-efficient operation for portable devices

## 9. Developer Workflow Enhancements
- Optional pre-commit script to run fmt, clippy, unit and integration tests
- Documentation for enabling hooks locally
- Coverage collection on Linux job (optional, non-blocking) via grcov or tarpaulin

## 10. Acceptance Criteria
- New tests compile and pass across Windows/macOS/Linux in CI
- No breaking changes to existing crates or behavior
- Tests for the recent label width logic exist and pass in all languages
- Clear documentation and contribution guidance exists

## 11. Roadmap (Incremental Plan)
1) Author this design doc (MD + HTML) with diagrams and navigation
2) Create `integration-tests` crate skeleton; compile baseline
3) Add unit tests for label width logic and constants
4) Add i18n utilities and smoke tests (en, es, fr)
5) Add layout structural tests
6) Add cross-module integration tests
7) Enhance CI to run new tests
8) Add developer pre-commit checks
9) Add optional coverage reporting
10) Finalize maintenance & contribution guide

## 12. Risks and Mitigations
- GUI-render differences: Avoid pixel diffs; test structure and properties instead
- i18n string drift: Build-time checks (`build.rs`) already help; add tests that tolerate minor variations
- Flaky tests: Keep tests deterministic; serialize where necessary

## 13. Estimated Timeline
Each task is ~20 minutes per subtask. See task list for detailed breakdown with review gates.

Status update (in progress):
- Task 1 (Design docs): Completed (MD + HTML produced)
- Task 2 (Scaffold integration-tests crate): Completed; compiles and tests pass (placeholders + first unit)
- Task 3 (Unit tests for label widths): Completed in refbox and integration-tests
- Task 4 (i18n smoke tests): Initial key-consistency test added and passing

## 14. Code Snippets (illustrative)
Unit-style test for label mapping (pseudo):

```rust
#[test]
fn test_label_width_logic() {
    use iced::Length;
    let cases = vec![
        ("Last Game", Some(GAME_LABEL_WIDTH)),
        ("Next Game", Some(GAME_LABEL_WIDTH)),
        ("Chief Ref", Some(REF_LABEL_WIDTH)),
        ("Other", None),
    ];
    for (label, expected) in cases {
        let width = determine_label_width(label);
        match expected {
            Some(w) => assert!(matches!(width, Length::Fixed(x) if x == w)),
            None => assert!(matches!(width, Length::FillPortion(_))),
        }
    }
}
```

---

Appendix A: CI additions (conceptual):
- cargo test --workspace
- cargo test -p integration-tests
- cargo test -p integration-tests -- ui_tests::internationalization --test-threads=1

