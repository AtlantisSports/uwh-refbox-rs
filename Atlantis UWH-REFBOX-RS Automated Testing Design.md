# Atlantis UWH-REFBOX-RS Automated Testing Design

Author: Augment Agent (based on GPT‑5)
Date: 2025-08-17

## 1. Purpose and Goals
This document defines a comprehensive, incremental, and maintainable automated testing strategy for the UWH RefBox Rust workspace. It is designed to:
- Validate recent layout changes (fixed-width label logic) as a foundation
- Provide a structure for unit, integration, UI/layout structural, i18n, and visual regression-style checks
- Integrate with the existing GitHub Actions CI pipeline
- Offer developer workflow enhancements (pre-commit checks, coverage, docs)
- Maintain backward compatibility and avoid breaking existing functionality

## 2. Current State Assessment
- Workspace contains multiple crates (refbox, uwh-common, overlay, etc.)
- Some unit tests exist (config serde, tournament manager, sound controller, uwh-common)
- CI already runs `cargo test --all` across platforms
- Gaps: UI/view testing, i18n validation, layout structure assertions, visual regression, developer workflow hooks

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
### 5.1 Recent Label Width Changes (foundation)
- Constants:
  - `GAME_LABEL_WIDTH = 100.0`
  - `REF_LABEL_WIDTH = 120.0`
- Single `match` expression determines width for known labels; default uses `FillPortion`
- Unit tests: verify mappings for Last/Next Game, Chief Ref, Timer, Water Ref 1-3, and default path

### 5.2 Internationalization (i18n)
- Instantiate app/components under `en-US`, `es`, `fr`
- Verify that fixed-width rules still apply regardless of translated label text

### 5.3 Layout Structure Tests
- Validate number and type of rows (4-column vs 2-column)
- Assert fixed widths are used where expected; proportional widths elsewhere

### 5.4 Integration Tests
- TournamentManager clock start/stop behaviors
- Config persistence round-trips through UI interactions (as feasible)

### 5.5 Visual Regression-Style
- Snapshot expectations on structural properties (counts, label types, width kinds) — not pixels

## 6. CI/CD Integration
- Extend `.github/workflows/rust.yml` to:
  - Build and test the new `integration-tests` crate
  - Optionally run i18n-focused tests serialized (`--test-threads=1` if needed)
  - Keep jobs green; do not add breaking checks initially

## 7. Developer Workflow Enhancements
- Optional pre-commit script to run fmt, clippy, unit and integration tests
- Documentation for enabling hooks locally
- Coverage collection on Linux job (optional, non-blocking) via grcov or tarpaulin

## 8. Acceptance Criteria
- New tests compile and pass across Windows/macOS/Linux in CI
- No breaking changes to existing crates or behavior
- Tests for the recent label width logic exist and pass in all languages
- Clear documentation and contribution guidance exists

## 9. Roadmap (Incremental Plan)
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

## 10. Risks and Mitigations
- GUI-render differences: Avoid pixel diffs; test structure and properties instead
- i18n string drift: Build-time checks (`build.rs`) already help; add tests that tolerate minor variations
- Flaky tests: Keep tests deterministic; serialize where necessary

## 11. Estimated Timeline
Each task is ~20 minutes per subtask. See task list for detailed breakdown with review gates.

Status update (in progress):
- Task 1 (Design docs): Completed (MD + HTML produced)
- Task 2 (Scaffold integration-tests crate): Completed; compiles and tests pass (placeholders + first unit)
- Task 3 (Unit tests for label widths): Completed in refbox and integration-tests
- Task 4 (i18n smoke tests): Initial key-consistency test added and passing

## 12. Code Snippets (illustrative)
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

