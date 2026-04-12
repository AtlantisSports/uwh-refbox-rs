# Scoresheet Style System

This document defines how scoresheet styles work and how to create a new one.
It is the reference for both existing styles and anyone building a custom style.

---

## What a Scoresheet Is

A scoresheet is the paper form (or PDF) used to record the outcome of a game. At a
tournament, the scorer/timekeeper fills it in during the game. After the game, it is
submitted to the tournament organizer as the official record.

Different tournaments and regions use different layouts — some prefer large text and
simple boxes, others want a detailed grid that captures every foul and penalty. The
style system supports all of these from a single data pipeline.

---

## The Data Pipeline

All styles receive exactly the same data. The pipeline is:

```
UWH Portal API
      ↓
Schedule (games, teams, court assignments, times)
      +
Referee names (chief ref, water refs, timer/scorer)
      +
Player rosters (both teams)
      +
Logos (tournament left logo, right logo)
      ↓
GameRenderContext  ← one per game
      ↓
Style render function  ← outputs HTML
      ↓
Combined HTML → PDF (via Chrome/Chromium)
```

The `GameRenderContext` is the clean hand-off point. Everything upstream of it is
shared infrastructure. Everything downstream is owned by the style.

---

## GameRenderContext — What Every Style Gets

Each game is rendered with a `GameRenderContext` that contains:

| Field | What it is |
|-------|-----------|
| `game_number` | The game number (e.g., `"G01"`) |
| `pool` | The court/pool name (e.g., `"Pool A"`) |
| `start_time` | Formatted start time (e.g., `"09:30"`) |
| `dark_team` | Dark team name (or ID if name unavailable) |
| `light_team` | Light team name (or ID if name unavailable) |
| `dark_roster` | List of dark team players (name + cap number), if fetched |
| `light_roster` | List of light team players (name + cap number), if fetched |
| `chief_ref` | Chief referee name, or `None` if not available |
| `water_refs` | Up to three water referee names (may be empty) |
| `timer` | Timer/scorer name, or `None` if not available |
| `left_logo` | Optional tournament logo (base64-encoded for HTML embedding) |
| `right_logo` | Optional second tournament logo |
| `event_name` | Tournament name (e.g., `"2026 Australian Nationals"`) |
| `event_dates` | Tournament dates formatted as a string |

**Referee roles come from the portal API as English identifiers** (`Chief`, `Water1`,
`Water2`, `Water3`, `TimeOrScoreKeeper`). The context translates these into named fields.
**How those fields are labelled in the scoresheet is entirely up to the style** — a
Spanish-language style would display "Juez Central", a French style "Arbitre Principal", etc.

---

## Page Size and Orientation

**Standard page size: A4** (210 mm × 297 mm)

A4 is the international standard and works across all tournament regions. It is declared
in the style's CSS, not in shared code.

Each style declares its own orientation:
- `@page { size: A4 landscape; }` for wide layouts
- `@page { size: A4 portrait; }` for tall layouts

There is no code-level orientation setting — it is purely a CSS decision owned by the style.

---

## Current Styles

| Variant | Orientation | Description |
|---------|-------------|-------------|
| `Detailed` | Landscape | Full grid with foul/penalty tracking rows |
| `Simple` | Portrait | Clean single-page layout, minimal detail |
| `SimpleTeamRefs` | Portrait | Same as Simple but uses team-assigned referee positions instead of portal names |
| `Col3x3` | Landscape | Three-column layout for 3-a-side underwater hockey |

---

## How to Create a New Style

### Step 1 — Define the variant

Open `schedule-processor/src/scoresheets.rs` and add your variant to the `SheetStyle` enum:

```rust
pub enum SheetStyle {
    Detailed,
    Simple,
    SimpleTeamRefs,
    Col3x3,
    MyNewStyle,   // ← add here
}
```

### Step 2 — Wire up the Display string

In the same file, add a match arm to the `Display` implementation and the `VARIANTS` list.
This is the name shown in the terminal menu when the user picks a style.

### Step 3 — Write the render function

Create a function with this signature:

```rust
fn render_my_new_style(ctx: &GameRenderContext) -> String {
    // Returns a complete HTML string for one game's scoresheet page
}
```

The function receives a `GameRenderContext` (all fields described above) and returns an
HTML string. That HTML should:

- Include a `<style>` block declaring page size and orientation:
  ```css
  @page { size: A4 landscape; margin: 8mm; }
  body { font-family: Arial, sans-serif; }
  ```
- Be entirely self-contained (no external CSS or JS files — the PDF renderer has no network)
- Embed logos using the base64 data provided in `ctx.left_logo` / `ctx.right_logo`
- Label referee roles in whatever language the style targets

### Step 4 — Register the render function

In the `render_game_html` dispatch function, add a match arm for your new variant:

```rust
SheetStyle::MyNewStyle => render_my_new_style(ctx),
```

### Step 5 — Test it

Run the integration test to confirm it generates output:

```
cargo test -p schedule-processor scoresheet
```

Then run the tool against a test event to confirm the PDF looks correct.

---

## Labels and Language

The style owns all display text. The data layer is always English, but the scoresheet
text can be in any language.

**Examples of style-owned labels:**
- `"Chief Ref"` / `"Juez Central"` / `"Arbitre Principal"` — all valid for the chief referee field
- `"Dark Team"` / `"Equipo Oscuro"` — team colour labels
- `"Game No."` / `"Nº de Juego"` — game number label
- `"Pool"` / `"Piscina"` — court label

There is no automatic translation. If you want a French-language scoresheet, write the
labels in French in the render function.

---

## What Each Style Must NOT Do

- Reach out to the network (all data is pre-fetched and passed in via `GameRenderContext`)
- Read files from disk
- Use `unwrap()` on optional fields without a fallback — some fields may be `None`
  (e.g., referee names when the portal has no assignments yet)
- Assume a specific number of water referees — the `water_refs` list may have 0–3 entries

---

## Roster Data

Player rosters are available to all styles via `ctx.dark_roster` and `ctx.light_roster`.
Each entry is a `(name, cap_number)` pair. Rosters may be empty if the portal has no
player data for that team.

Not all styles need to display rosters. Styles that do not need roster data simply ignore
the fields — they are still fetched once and passed in, so there is no extra cost to
having them available.

---

## Logos

`ctx.left_logo` and `ctx.right_logo` are `Option<String>`. When `Some`, the string is a
base64-encoded data URI ready to drop into an `<img>` tag:

```html
{% if let Some(logo) = ctx.left_logo %}
<img src="{{ logo }}" style="height: 40mm;" />
{% endif %}
```

If `None`, no logo was provided — the style should handle this gracefully (skip the
`<img>` tag entirely, don't show a broken image).
