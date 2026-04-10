# schedule-processor — Crate Guide

The `schedule-processor` is a command-line tool run by the tournament organizer before each
tournament. It reads schedule data, validates it, generates scoresheets, and outputs data the
refbox can load.

---

## What This Crate Does

1. **Reads** the tournament schedule — either from a CSV export or the UWH Portal API
2. **Validates** the schedule for common errors (team conflicts, missing games, etc.)
3. **Resolves** coin tosses via the portal API
4. **Generates** scoresheet PDFs for the tournament
5. **Outputs** the schedule in a format the refbox can load

This tool is run once (or a few times) per tournament, not during games.

---

## Key Files and What They Do

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry point — argument parsing, interactive prompts, main workflow |
| `src/csv_parser.rs` | Parses the tournament schedule from CSV format |
| `src/schedule_checks.rs` | Validates the schedule for correctness |
| `src/scoresheets.rs` | Generates scoresheet PDFs |

---

## How It Works

The tool is interactive — it uses `inquire` to ask the user questions at the terminal:
- Which event/tournament to process
- Whether to allow schedule check failures
- Portal login credentials (if needed)

The portal API (via `uwh-common::uwhportal`) is used to fetch schedule data and submit coin flip
results.

---

## Data Flow

```
CSV file / Portal API
        ↓
   csv_parser.rs (parse)
        ↓
   schedule_checks.rs (validate)
        ↓
   scoresheets.rs (generate PDFs)
        ↓
   Output files for refbox
```

---

## Mock Schedules

The `Mock Schedules for testing/` directory contains CSV files used for testing the parser
without needing a live portal connection. When adding new parsing features, add a corresponding
mock CSV that exercises the new behaviour.

---

## Dependencies to Be Aware Of

- `uwh-common` — provides portal API types (`UwhPortalClient`, schedule types)
- `clap` — CLI argument parsing
- `inquire` — interactive terminal prompts
- `reqwest` — HTTP client for portal API calls (async via `tokio`)
- `rfd` — file dialog for selecting files (platform-native)
- `prettytable` — formatted terminal table output
