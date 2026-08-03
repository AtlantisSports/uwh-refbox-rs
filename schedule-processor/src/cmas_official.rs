//! The "CMAS Official" scoresheet style.
//!
//! Unlike the other styles, the whole page below the title band is a single HTML
//! table of 25 master columns, so every block aligns to shared column boundaries.
//! See `docs/superpowers/specs/2026-08-02-cmas-official-scoresheet-design.md`.

use std::path::Path;

use uwh_common::uwhportal::RosterPlayer;

/// Player rows printed per team. An official CMAS team is capped at 12 players.
pub const ROSTER_ROWS: usize = 12;

/// One printed roster line. Empty strings mean a blank row for hand-writing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SheetRosterRow {
    pub cap: String,
    pub name: String,
}

/// Turn a portal roster into exactly [`ROSTER_ROWS`] printable rows.
///
/// Players arrive already sorted by cap number. Captains are marked `(C)` and
/// vice-captains `(VC)`; if a player somehow holds both, `(C)` wins. Returns the
/// rows plus the names of any players beyond the twelfth, so the caller can warn.
pub fn roster_rows(players: &[RosterPlayer]) -> (Vec<SheetRosterRow>, Vec<String>) {
    let mut rows = vec![SheetRosterRow::default(); ROSTER_ROWS];

    for (row, player) in rows.iter_mut().zip(players.iter()) {
        row.cap = player.number.map(|n| n.to_string()).unwrap_or_default();
        row.name = if player.is_captain {
            format!("{} (C)", player.name)
        } else if player.is_vice_captain {
            format!("{} (VC)", player.name)
        } else {
            player.name.clone()
        };
    }

    let dropped = players
        .iter()
        .skip(ROSTER_ROWS)
        .map(|p| p.name.clone())
        .collect();

    (rows, dropped)
}

/// The CMAS logo, compiled into the binary so the sheet needs no setup.
const CMAS_LOGO_BYTES: &[u8] = include_bytes!("../assets/cmas-logo.png");

/// Filename the logo is written under, and referenced by, inside the output directory.
pub const CMAS_LOGO_FILENAME: &str = "cmas-logo.png";

/// Write the bundled CMAS logo into `output_dir` and return the relative filename
/// to use in the rendered HTML. Mirrors how operator-supplied logos are handled:
/// written next to the combined HTML, then removed once the PDF is produced.
pub fn write_cmas_logo(output_dir: &Path) -> Result<&'static str, std::io::Error> {
    std::fs::write(output_dir.join(CMAS_LOGO_FILENAME), CMAS_LOGO_BYTES)?;
    Ok(CMAS_LOGO_FILENAME)
}

/// Render one game's CMAS Official scoresheet page.
///
/// Returns a complete standalone HTML document. The caller extracts the `<style>`
/// block and the `<div class='page'>` fragment to build the combined PDF.
#[allow(clippy::too_many_arguments)]
pub fn render_html_cmas_official() -> String {
    // Replaced with the real grid in a later task.
    String::from(
        "<!doctype html><html><head><meta charset='utf-8'/><style></style></head>\
         <body><div class='page'></div></body></html>",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_bundled_logo_into_the_output_dir() {
        let dir = std::env::temp_dir().join("cmas-logo-test");
        // Start from a clean slate so a stale file cannot make this pass.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test setup: create temp dir");

        let rel = write_cmas_logo(&dir).expect("logo should be written");

        assert_eq!(rel, CMAS_LOGO_FILENAME);
        let written = std::fs::read(dir.join(rel)).expect("file should exist on disk");
        assert_eq!(
            written.as_slice(),
            CMAS_LOGO_BYTES,
            "written bytes must match the embedded asset"
        );
        assert_eq!(&written[1..4], b"PNG", "must be a real PNG");

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn player(number: Option<u8>, name: &str, cap: bool, vice: bool) -> RosterPlayer {
        RosterPlayer {
            number,
            name: name.to_string(),
            is_captain: cap,
            is_vice_captain: vice,
        }
    }

    #[test]
    fn always_returns_exactly_twelve_rows() {
        let (rows, dropped) = roster_rows(&[]);
        assert_eq!(rows.len(), ROSTER_ROWS);
        assert!(dropped.is_empty());
        assert!(
            rows.iter().all(|r| r.cap.is_empty() && r.name.is_empty()),
            "an empty roster must produce twelve blank rows, not a panic"
        );
    }

    #[test]
    fn fills_from_the_top_and_leaves_the_rest_blank() {
        let players = vec![
            player(Some(1), "Drake QUIEC", false, false),
            player(Some(5), "Logan DUONG", false, false),
        ];
        let (rows, dropped) = roster_rows(&players);

        assert_eq!(rows[0].cap, "1");
        assert_eq!(rows[0].name, "Drake QUIEC");
        assert_eq!(rows[1].cap, "5");
        assert_eq!(rows[1].name, "Logan DUONG");
        assert_eq!(rows[2].cap, "");
        assert_eq!(rows[2].name, "");
        assert_eq!(rows.len(), ROSTER_ROWS);
        assert!(dropped.is_empty());
    }

    #[test]
    fn marks_captain_and_vice_captain_inline() {
        let players = vec![
            player(Some(7), "Blake RIVE", true, false),
            player(Some(8), "Keith LIN", false, true),
            player(Some(9), "Levi COOK", false, false),
        ];
        let (rows, _) = roster_rows(&players);

        assert_eq!(rows[0].name, "Blake RIVE (C)");
        assert_eq!(rows[1].name, "Keith LIN (VC)");
        assert_eq!(rows[2].name, "Levi COOK");
    }

    #[test]
    fn captain_wins_if_a_player_somehow_holds_both_roles() {
        let players = vec![player(Some(4), "Ashley OOSTHUIZEN", true, true)];
        let (rows, _) = roster_rows(&players);
        assert_eq!(rows[0].name, "Ashley OOSTHUIZEN (C)");
    }

    #[test]
    fn reports_players_that_do_not_fit() {
        let players: Vec<RosterPlayer> = (1..=14)
            .map(|n| player(Some(n), &format!("Player {n}"), false, false))
            .collect();
        let (rows, dropped) = roster_rows(&players);

        assert_eq!(rows.len(), ROSTER_ROWS);
        assert_eq!(rows[11].name, "Player 12");
        assert_eq!(
            dropped,
            vec!["Player 13".to_string(), "Player 14".to_string()],
            "the operator must be told exactly who was left off"
        );
    }

    #[test]
    fn blank_cap_number_prints_an_empty_cell_not_a_zero() {
        let players = vec![player(None, "Unnumbered PLAYER", false, false)];
        let (rows, _) = roster_rows(&players);
        assert_eq!(rows[0].cap, "");
        assert_eq!(rows[0].name, "Unnumbered PLAYER");
    }
}
