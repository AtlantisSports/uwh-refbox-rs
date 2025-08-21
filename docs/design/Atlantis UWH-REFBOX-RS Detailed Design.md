# Atlantis UWH-REFBOX-RS Detailed Design

Author: Augment Agent (based on GPT‑5)
Date: 2025-08-18
Last Updated: 2025-08-18

## 1. Purpose and Goals
This document provides a comprehensive design overview of the UWH RefBox Rust workspace, including recent enhancements and automated testing strategy. It covers:
- **Application Architecture**: Complete system design including UI components, timer systems, and data management
- **Testing Strategy**: Comprehensive testing approach for UI, integration, and functionality validation
- **CI/CD Pipeline Integration**: Continuous integration and deployment configuration
- **Developer Workflow**: Tools and processes for maintaining code quality
- **Internationalization Support**: Multi-language layout consistency and validation
- **Performance Requirements**: System performance specifications and optimization strategies

## 2. How to Run the Application

### 2.1 Basic Usage

**From the refbox directory:**
```powershell
cd refbox
cargo run
```

**Or from the project root:**
```powershell
cargo run --bin refbox
```

### 2.2 Command Line Options

The app supports extensive configuration through command-line arguments:

#### Essential Options
```powershell
# Basic run
cargo run

# Run with help to see all options
cargo run -- --help

# Run in fullscreen mode
cargo run -- --fullscreen

# Run without the simulator GUI
cargo run -- --no-simulate

# Increase verbosity for debugging
cargo run -- -v    # or -vv for more verbose
```

#### Network Configuration
```powershell
# Set custom ports
cargo run -- --binary-port 8001 --json-port 8000

# Run with serial port connection
cargo run -- --serial-port COM3 --baud-rate 115200

# Allow HTTP connections to UWH Portal (disable HTTPS requirement)
cargo run -- --allow-http

# List all events from UWH Portal, including past ones
cargo run -- --all-events
```

#### Display and Simulator Options
```powershell
# Set pixel scale for simulator (default: 4)
cargo run -- --scale 6

# Set spacing between pixels in simulator
cargo run -- --spacing 2.0

# Run simulator in sunlight mode
cargo run -- --simulate-sunlight-display
```

#### Logging Configuration
```powershell
# Set custom log directory
cargo run -- --log-location "C:\custom\log\path"

# Set maximum log file size (default: 5MB)
cargo run -- --log-max-file-size 10000000

# Set number of archived logs to keep (default: 3)
cargo run -- --num-old-logs 5
```

### 2.3 Testing Options

#### Comprehensive Test Suite
```powershell
# Run all tests across all crates (recommended)
cargo test --all

# Run all tests with output visible
cargo test --all -- --nocapture

# Run tests with specific thread count (useful for UI tests)
cargo test --all -- --test-threads=1
```

#### Individual Test Suites
```powershell
# Integration tests (UI, font sizing, i18n)
cargo test -p integration-tests

# Main refbox application tests
cargo test -p refbox

# Common utilities tests
cargo test -p uwh-common

# Overlay functionality tests
cargo test -p overlay

# LED panel simulation tests
cargo test -p led-panel-sim

# Font processing tests
cargo test -p fonts
```

#### Specialized Testing
```powershell
# Audio/beep testing utility
cargo run -p beep-test

# Run specific test by name
cargo test test_font_sizing

# Run tests matching a pattern
cargo test font_sizing

# Run tests with debug output
cargo test -- --nocapture --test-threads=1
```

### 2.4 Development Workflow

#### Building for Different Targets
```powershell
# Install cross-compilation tool
cargo install cross

# Build for Raspberry Pi 4/5
cross build --all --release --target aarch64-unknown-linux-gnu

# Build for Windows
cross build --all --release --target x86_64-pc-windows-gnu

# Build for Intel Macs
cross build --all --release --target x86_64-apple-darwin

# Build for ARM Macs (M series)
cross build --all --release --target aarch64-apple-darwin
```

#### Quality Checks
```powershell
# Run clippy for linting
cargo clippy --all

# Check for security vulnerabilities
cargo audit

# Format code
cargo fmt --all

# Check formatting without changing files
cargo fmt --all -- --check
```

### 2.5 Key Features Available

- **Timer System**: Full refbox timer functionality with period management
- **Panel Simulator**: Built-in LED panel simulator (runs by default unless `--no-simulate`)
- **Network Connectivity**: TCP connections on configurable ports
- **Serial Communication**: Hardware connection via serial ports
- **UWH Portal Integration**: Tournament data synchronization
- **Multi-language Support**: Internationalization with multiple language options
- **Sound System**: Audio alerts and wireless remote support
- **Logging**: Comprehensive logging to system-appropriate directories

### 2.6 Quick Start Guide

For a basic timer session:
```powershell
cd refbox
cargo run
```

This starts both the refbox timer application and LED panel simulator. The application opens in a window where you can immediately begin using timer functionality.

For tournament integration:
```powershell
cargo run -- --allow-http --all-events
```

This enables UWH Portal connectivity and shows all available events for selection.

## 3. Target Architecture Overview
Introduce a new workspace member `integration-tests` dedicated to tests that span crates and UI/layout logic. Keep existing unit tests in-place within each crate. Run tests locally and in CI.

High-level layers:
- Unit tests: localized logic checks (e.g., label width match expression) within or alongside crates
- Integration tests: cross-module behaviors, app flows
- UI/layout structural tests: assert table structure and length/width decisions (not pixel rendering)
- Internationalization tests: build app or components under en-US, es, fr and verify rules
- Visual regression-style: snapshot-like structural expectations, avoiding brittle pixel diffs

## 4. Directory Structure

### 4.1. Workspace Overview

```
uwh-refbox-rs/                                   # Root workspace directory
├── Cargo.toml                                   # Workspace configuration
├── Cargo.lock                                   # Dependency lock file
├── Cross.toml                                   # Cross-compilation configuration
├── LICENSE.txt                                  # Project license
├── README.md                                    # Project documentation
├── uwh-refbox-rs.code-workspace                # VS Code workspace file
├── alphagen/                                    # Alpha channel generation utility
│   ├── Cargo.toml                              # Package configuration
│   └── src/                                    # Source code
├── beep-test/                                   # Audio testing utility
│   ├── Cargo.toml                              # Package configuration
│   ├── build.rs                                # Build script
│   ├── resources/                              # Audio resources
│   └── src/                                    # Source code
├── ci/                                          # Continuous integration scripts
│   └── check-msrv-present.sh                  # MSRV validation script
├── docs/                                        # Documentation
│   ├── README.md                               # Documentation index
│   ├── design/                                 # Design documents
│   └── scripts/                                # Documentation generation scripts
├── fonts/                                       # Font processing utilities
│   ├── Cargo.toml                              # Package configuration
│   ├── build.rs                                # Build script
│   ├── convert_to_raw.py                       # Font conversion script
│   ├── print_raw.py                            # Font debugging script
│   └── src/                                    # Source code
├── integration-tests/                           # Integration testing suite
│   └── [detailed structure below]              # See section 4.2
├── led-panel/                                   # Hardware LED panel support
│   ├── README.md                               # Hardware documentation
│   ├── boards/                                 # Board definitions
│   ├── builds/                                 # Build artifacts
│   ├── fusesoc.conf                            # FuseSoC configuration
│   ├── led_panel.core                          # Core definition
│   ├── requirements.txt                        # Python dependencies
│   ├── rtl/                                    # RTL source code
│   ├── synth/                                  # Synthesis scripts
│   └── tb/                                     # Testbenches
├── led-panel-sim/                              # LED panel simulator
│   ├── Cargo.toml                              # Package configuration
│   └── src/                                    # Source code
├── matrix-drawing/                              # Matrix display utilities
│   ├── Cargo.toml                              # Package configuration
│   └── src/                                    # Source code
├── overlay/                                     # Streaming overlay system
│   ├── Cargo.toml                              # Package configuration
│   ├── assets/                                 # Web assets
│   ├── src/                                    # Source code
│   └── test_server/                            # Test server utilities
├── refbox/                                      # Main referee box application
│   ├── Cargo.toml                              # Package configuration
│   ├── build.rs                                # Build script
│   ├── i18n.toml                               # Internationalization config
│   ├── resources/                              # Application resources
│   ├── src/                                    # Source code
│   ├── test_data/                              # Test data files
│   └── translations/                           # Translation files
├── schedule-processor/                          # Tournament schedule processor
│   ├── Cargo.toml                              # Package configuration
│   └── src/                                    # Source code
├── uwh-common/                                  # Common utilities library
│   ├── Cargo.toml                              # Package configuration
│   └── src/                                    # Source code
├── wireless-modes/                              # Wireless communication modes
│   ├── Cargo.toml                              # Package configuration
│   └── src/                                    # Source code
└── wireless-remote/                             # Wireless remote control
    ├── Cargo.toml                              # Package configuration
    ├── Cargo.lock                              # Dependency lock file
    ├── builds/                                 # Build artifacts
    ├── memory.x                                # Memory layout
    ├── rust-toolchain.toml                     # Rust toolchain config
    └── src/                                    # Source code
```

### 4.2. Integration Tests Directory Structure

```
integration-tests/
├── Cargo.toml                                    # Package configuration
├── src/                                          # Source code directory
│   ├── lib.rs                                   # Library root module
│   ├── ui_tests/                                # UI and layout tests
│   │   ├── mod.rs                               # UI tests module
│   │   ├── view_builders/                       # View builder tests
│   │   │   ├── mod.rs                           # View builder module
│   │   │   ├── main_view_tests.rs              # Main view structure tests
│   │   │   ├── layout_tests.rs                 # Layout validation tests
│   │   │   └── label_width_tests.rs            # Label width logic tests
│   │   └── internationalization/               # i18n tests
│   │       ├── mod.rs                           # i18n module
│   │       ├── language_tests.rs               # Language consistency tests
│   │       └── layout_consistency_tests.rs     # Cross-language layout tests
│   ├── integration/                             # Cross-module integration tests
│   │   ├── mod.rs                               # Integration module
│   │   ├── tournament_manager_tests.rs         # Tournament manager tests
│   │   └── config_tests.rs                     # Configuration tests
│   ├── visual_regression/                       # Visual regression tests
│   │   ├── mod.rs                               # Visual regression module
│   │   ├── snapshot_tests.rs                   # Structural snapshot tests
│   │   └── layout_validation.rs                # Layout validation tests
│   └── utils/                                   # Test utilities
│       ├── mod.rs                               # Utils module
│       ├── mock_data.rs                        # Mock data generators
│       ├── test_helpers.rs                     # Common test helpers
│       └── layout_assertions.rs                # Layout assertion helpers
└── tests/                                       # Integration test files
    ├── ui_integration.rs                        # UI integration tests
    └── full_app_tests.rs                        # Full application tests
```


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

## 12. UWH Portal Integration

### Overview
The refbox system provides comprehensive two-way integration with the UWH Portal, enabling real-time tournament management, live scoring, and detailed game statistics tracking.

### Authentication & Connection

#### Login Process
- **RefBox ID**: A randomly generated ID (1-999,999) that uniquely identifies the refbox
- **Access Code**: A numeric code entered by the user to link the refbox to a specific event
- **Bearer Token**: JWT token received after successful authentication for subsequent API calls

```rust
pub fn login_to_portal(
    &self,
    event_id: &EventId,
    code: u32,
) -> impl std::future::Future<Output = Result<PortalTokenResponse, Box<dyn Error>>> + use<>
{
    let url = format!(
        "{}/api/events/{}/access-keys/ref-box",
        self.base_url,
        event_id.partial()
    );

    let request = self
        .client
        .request(Method::POST, &url)
        .json(&serde_json::json!({
            "refBoxId": self.id().to_string(),
            "code": code.to_string()
        }));
```

### Data Retrieved FROM UWH Portal

#### 1. Event List
- Event names, IDs, and slugs
- Date ranges (start/end times)
- Team lists with team IDs and names
- Court information

#### 2. Event Schedules
- Complete game schedules with timing
- Team assignments (dark/light sides)
- Game numbers and court assignments
- Timing rules (half duration, timeouts, etc.)
- Non-game entries (breaks, ceremonies)
- Group structures and standings calculations

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub number: GameNumber,
    pub dark: ScheduledTeam,
    pub light: ScheduledTeam,
    #[serde(with = "iso8601_4dig_year_no_subsecs", rename = "startsOn")]
    pub start_time: OffsetDateTime,
    pub court: String,
    #[serde(with = "item_name", rename = "timingRule")]
    pub timing_rule: String,
    pub description: Option<String>,
}
```

### Data Sent TO UWH Portal

#### 1. Game Scores
- Final scores for both teams (dark/light)
- Game number and event ID
- Force flag to override existing scores

```rust
.json(&serde_json::json!({
    "dark": {
        "value": scores.black
    },
    "light": {
        "value": scores.white
    }
}));
```

#### 2. Detailed Game Statistics
- **Goals**: Player cap number, team side, game period, time in period, timestamp
- **Penalties**: Player cap number, team side, game period, time, duration, dismissal status
- Game start/end timestamps
- All events sorted chronologically

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "$type")]
enum Event {
    #[serde(rename = "goal")]
    Goal {
        #[serde(rename = "playerCapNumber")]
        player_cap_number: u8,
        side: String,
        #[serde(rename = "gamePeriod")]
        game_period: GamePeriod,
        #[serde(rename = "periodTime")]
        period_time: f32,
        #[serde(with = "iso8601_short_year")]
        #[serde(rename = "occurredOn")]
        occurred_on: OffsetDateTime,
    },
    #[serde(rename = "penalty")]
    Penalty {
        #[serde(rename = "playerCapNumber")]
        player_cap_number: u8,
        side: String,
        #[serde(rename = "gamePeriod")]
        game_period: GamePeriod,
        #[serde(rename = "periodTime")]
        period_time: f32,
        #[serde(with = "iso8601_short_year")]
        #[serde(rename = "occurredOn")]
        occurred_on: OffsetDateTime,
        duration: Option<u64>,
        #[serde(rename = "isTotalDismissal")]
        is_total_dismissal: bool,
    },
}
```

#### 3. Schedule Management
- Complete tournament schedules can be uploaded
- Team mappings (linking unassigned names to full team IDs)
- Schedule modifications and updates

### API Endpoints Used
- `POST /api/events/{eventId}/access-keys/ref-box` - Authentication
- `GET /api/events` - List available events
- `GET /api/events/{eventId}/schedule/privileged` - Get event schedule
- `POST /api/events/{eventId}/schedule/games/{gameNumber}/scores` - Post game scores
- `POST /api/admin/events/stats` - Post detailed game statistics
- `POST /api/events/{eventSlug}/schedule` - Upload complete schedules
- `POST /api/events/{eventSlug}/schedule/map-teams` - Map team assignments

### Security & Configuration
- **HTTPS Enforcement**: Configurable requirement for secure connections
- **Bearer Token Authentication**: All authenticated requests use JWT tokens
- **Request Timeouts**: Configurable timeout settings
- **Error Handling**: Comprehensive error responses and logging

The system provides a complete two-way integration where the refbox can both consume tournament data from the portal and push back real-time game results and detailed statistics for tournament management and live scoring displays.

## 13. Risks and Mitigations
- GUI-render differences: Avoid pixel diffs; test structure and properties instead
- i18n string drift: Build-time checks (`build.rs`) already help; add tests that tolerate minor variations
- Flaky tests: Keep tests deterministic; serialize where necessary

## 14. Estimated Timeline
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

