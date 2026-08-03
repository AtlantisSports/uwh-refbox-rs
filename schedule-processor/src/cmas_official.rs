//! The "CMAS Official" scoresheet style.
//!
//! Unlike the other styles, the whole page below the title band is a single HTML
//! table of 25 master columns, so every block aligns to shared column boundaries.
//! See `docs/superpowers/specs/2026-08-02-cmas-official-scoresheet-design.md`.

use std::path::Path;

use time::{OffsetDateTime, format_description::FormatItem, macros::format_description};
use uwh_common::uwhportal::RosterPlayer;

use crate::scoresheets::html_escape;

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

const CMAS_DATE_FMT: &[FormatItem<'static>] =
    format_description!("[weekday]<br>[day]-[month repr:short]-[year]");
const CMAS_TIME_FMT: &[FormatItem<'static>] =
    format_description!("[hour repr:12 padding:none]:[minute] [period case:upper]");

/// Abbreviate a timing-rule name into the round type printed in the Division box.
pub fn round_type_abbrev(timing_rule: &str) -> &'static str {
    let up = timing_rule.to_ascii_uppercase();
    if up.contains("RR") {
        "RR"
    } else if up.contains("XO") {
        "XO"
    } else if up.contains("PO") {
        "PO"
    } else if up.contains("MD") {
        "MD"
    } else {
        ""
    }
}

/// `U19W - RR`, or whichever half is available, or empty.
pub fn division_label(div_short: &str, round_type: &str) -> String {
    match (div_short.is_empty(), round_type.is_empty()) {
        (false, false) => format!("{div_short} - {round_type}"),
        (false, true) => div_short.to_string(),
        (true, false) => round_type.to_string(),
        (true, true) => String::new(),
    }
}

/// Weekday on the first line, `05-Aug-2026` on the second. Caller supplies the
/// time already converted to the event's timezone.
///
/// The returned string embeds a literal `<br>` line break. Unlike every other
/// portal-sourced value placed into the sheet, this result must NOT be passed
/// through `html_escape` — doing so would print a literal `&lt;br&gt;` instead
/// of a line break.
pub fn format_date_two_line(dt: OffsetDateTime) -> String {
    dt.format(&CMAS_DATE_FMT).unwrap_or_default()
}

/// `7:30 AM`. Caller supplies the time already converted to the event's timezone.
pub fn format_start_time(dt: OffsetDateTime) -> String {
    dt.format(&CMAS_TIME_FMT).unwrap_or_default()
}

/// All the data needed to render one game's CMAS Official scoresheet page.
///
/// Every field is substituted into the page and, with the sole exception of
/// [`date_html`](Self::date_html), passed through [`html_escape`] first.
pub struct CmasSheetInput<'a> {
    pub event_name: &'a str,
    pub division: &'a str,
    pub game_number: &'a str,
    pub court: &'a str,
    pub black_team: &'a str,
    pub white_team: &'a str,
    /// Pre-formatted by [`format_date_two_line`]; contains a literal `<br>` and must
    /// NOT be passed through `html_escape`.
    pub date_html: &'a str,
    pub start_time: &'a str,
    pub black_roster: &'a [SheetRosterRow],
    pub white_roster: &'a [SheetRosterRow],
    pub chief: &'a str,
    pub water: [&'a str; 3],
    pub timekeeper: &'a str,
    pub timekeeper_helper: &'a str,
    pub cmas_logo_rel: Option<&'a str>,
    pub tournament_logo_rel: Option<&'a str>,
}

/// The CSS for the CMAS Official sheet, ported from the approved mockup
/// (`docs/superpowers/specs/2026-08-02-cmas-official-scoresheet-mockup.html`).
///
/// The preview-only scaffolding (`.sheetwrap`, `.sheetscale`, the fixed-pixel size
/// and border on `.cmas`) is replaced with print rules sized for A4 landscape. A
/// rule sizing `<img>` tags inside `.logobox` is added, since the mockup only ever
/// showed text placeholders there. Everything else is unchanged.
const CMAS_CSS: &str = r#"
  .cmas { background:#fff; color:#000;
          font-family: Arial, Helvetica, sans-serif; padding:24px;
          box-sizing:border-box; display:flex; flex-direction:column; gap:6px;
          overflow:hidden; }

  .cmas .titleband { display:flex; align-items:center; gap:14px; }
  .cmas .logobox { height:50px; width:50px; flex:0 0 50px; border:1px dashed #bbb;
                   display:flex; align-items:center; justify-content:center;
                   font-size:8px; color:#888; text-align:center; line-height:1.2; }
  .cmas .logobox img { max-width:100%; max-height:100%; object-fit:contain; }
  .cmas .titles { flex:1; text-align:center; }
  .cmas .titles .t1 { font-size:17px; font-weight:bold; letter-spacing:.02em; }
  .cmas .titles .t2 { font-size:14px; font-weight:bold; }

  .cmas .sheet { border-collapse:collapse; width:100%; table-layout:fixed; flex:1; height:100%; }
  .cmas td, .cmas th { border:1px solid #000; padding:1px 4px; font-size:10px;
                       vertical-align:middle; height:16px; }

  .cmas th { font-weight:bold; text-align:center; background:#dcdcdc; font-size:9px; line-height:1.05; }
  .cmas .hdrlabel { text-align:center; font-weight:bold; background:#dcdcdc; font-size:11px; }
  .cmas .hdrval { text-align:center; font-weight:bold; font-size:12px; height:21px; }
  .cmas .hdrval.two { font-size:11px; line-height:1.15; }
  .cmas .blackfill { background:#ececec; }
  .cmas th.blacktitle, .cmas .blacktitle { background:#c2c2c2; }
  .cmas .kit { font-size:10px; text-align:center; height:15px; font-weight:normal; }
  .cmas .nb { border:none; }
  .cmas .gap { border:none !important; background:transparent; }

  .cmas .divider { border-left:1px solid #000 !important; }
  .cmas th.nameh { text-align:left; padding-left:4px; }
  .cmas td.name { text-align:left; }
  .cmas td.cap { text-align:center; }
  .cmas td.lbl { font-weight:bold; text-align:left; }
  .cmas td.rlbl { font-weight:bold; text-align:right; }

  .cmas .sec { font-weight:bold; font-size:11px; text-decoration:underline; text-align:center; }
  .cmas .sec-l { font-weight:bold; font-size:11px; text-decoration:underline; text-align:left; }
  .cmas .note { font-size:8px; text-align:center; font-weight:normal; }
  .cmas .oflbl { font-weight:bold; text-align:left; }
  .cmas .ofnm { text-align:left; padding-left:12px; }
  .cmas .important { text-align:center; font-size:9px; font-style:italic; vertical-align:bottom; }

  .cmas .sigcell { vertical-align:middle; padding:6px 14px; }
  .cmas .sigbox { height:100%; display:flex; flex-direction:column; justify-content:space-around; }
  .cmas .sig .line { border-bottom:1px solid #000; height:24px; }
  .cmas .sig .cap2 { font-size:10px; font-weight:bold; text-align:center; padding-top:2px; }

  @media print { @page { size: A4 landscape; margin: 0; }
                 html, body { width: 297mm; height: 210mm; margin: 0; } }
  .cmas { width: 297mm; height: 210mm; }
  .page { break-after: page; page-break-after: always; }
"#;

/// A `.logobox` cell: a real `<img>` when a logo is available, otherwise an empty
/// box of the same fixed size (the 50px dimensions are load-bearing for page fit).
fn logobox_html(logo_rel: Option<&str>) -> String {
    match logo_rel {
        Some(rel) => format!(
            "<div class=\"logobox\"><img src=\"{}\"/></div>",
            html_escape(rel)
        ),
        None => String::from("<div class=\"logobox\"></div>"),
    }
}

/// The 12 paired roster rows (black cap/name on the left, white on the right).
/// Missing rows (a roster shorter than [`ROSTER_ROWS`]) print as blank cells
/// rather than panicking.
fn roster_rows_html(black: &[SheetRosterRow], white: &[SheetRosterRow]) -> String {
    let mut out = String::new();
    for i in 0..ROSTER_ROWS {
        let b = black.get(i).cloned().unwrap_or_default();
        let w = white.get(i).cloned().unwrap_or_default();
        let bcap = html_escape(&b.cap);
        let bname = html_escape(&b.name);
        let wcap = html_escape(&w.cap);
        let wname = html_escape(&w.name);
        out.push_str(&format!(
            "    <tr>\n      \
             <td class=\"cap blackfill\">{bcap}</td><td class=\"name blackfill fit\" colspan=\"7\">{bname}</td><td class=\"blackfill\" colspan=\"2\"></td><td class=\"blackfill\"></td><td class=\"blackfill\"></td><td class=\"blackfill\"></td>\n      \
             <td class=\"cap divider\">{wcap}</td><td class=\"name fit\" colspan=\"7\">{wname}</td><td></td><td></td><td></td><td></td>\n    \
             </tr>\n"
        ));
    }
    out
}

/// Render one game's CMAS Official scoresheet page.
///
/// Returns a complete standalone HTML document. The caller extracts the `<style>`
/// block and the `<div class='page'>` fragment to build the combined PDF.
pub fn render_html_cmas_official(input: &CmasSheetInput<'_>) -> String {
    let event_name = html_escape(input.event_name);
    let division = html_escape(input.division);
    let game_number = html_escape(input.game_number);
    let court = html_escape(input.court);
    let black_team = html_escape(input.black_team);
    let white_team = html_escape(input.white_team);
    let start_time = html_escape(input.start_time);
    let date_html = input.date_html;
    let chief = html_escape(input.chief);
    let water1 = html_escape(input.water[0]);
    let water2 = html_escape(input.water[1]);
    let water3 = html_escape(input.water[2]);
    let timekeeper = html_escape(input.timekeeper);
    let timekeeper_helper = html_escape(input.timekeeper_helper);

    let left_logobox = logobox_html(input.cmas_logo_rel);
    let right_logobox = logobox_html(input.tournament_logo_rel);
    let roster_html = roster_rows_html(input.black_roster, input.white_roster);

    format!(
        r#"<!doctype html><html><head><meta charset='utf-8'/><style>{CMAS_CSS}</style></head><body>
<div class='page'><div class="cmas">

  <div class="titleband">
    {left_logobox}
    <div class="titles">
      <div class="t1">CMAS UNDERWATER HOCKEY SCORESHEET</div>
      <div class="t2">{event_name}</div>
    </div>
    {right_logobox}
  </div>

  <table class="sheet">
    <colgroup>
      <col style="width:3%"><col style="width:4.2%"><col style="width:0.8%">
      <col style="width:3%"><col style="width:4.2%"><col style="width:0.8%">
      <col style="width:3%"><col style="width:4.5%">
      <col style="width:3%"><col style="width:3.5%">
      <col style="width:6.5%"><col style="width:6.5%"><col style="width:7%">
      <col style="width:3%"><col style="width:4.2%"><col style="width:0.8%">
      <col style="width:3%"><col style="width:4.2%"><col style="width:0.8%">
      <col style="width:3%"><col style="width:4.5%">
      <col style="width:6.5%"><col style="width:6.5%"><col style="width:6.5%"><col style="width:7%">
    </colgroup>

    <tr>
      <td class="hdrlabel" colspan="4">Division</td>
      <td class="hdrlabel" colspan="3">Game #</td>
      <td class="hdrlabel" colspan="2">Court #</td>
      <td class="hdrlabel blacktitle" colspan="4">Black</td>
      <td class="hdrlabel" colspan="8">White</td>
      <td class="hdrlabel" colspan="2">Date</td>
      <td class="hdrlabel" colspan="2">Scheduled Start Time</td>
    </tr>
    <tr>
      <td class="hdrval" colspan="4">{division}</td>
      <td class="hdrval" colspan="3">{game_number}</td>
      <td class="hdrval" colspan="2">{court}</td>
      <td class="hdrval blackfill fit" colspan="4">{black_team}</td>
      <td class="hdrval fit" colspan="8">{white_team}</td>
      <td class="hdrval two" colspan="2">{date_html}</td>
      <td class="hdrval" colspan="2">{start_time}</td>
    </tr>
    <tr>
      <td class="kit nb" colspan="9"></td>
      <td class="kit blackfill" colspan="4">Kit Checked?&nbsp;&nbsp;Y / N</td>
      <td class="kit" colspan="8">Kit Checked?&nbsp;&nbsp;Y / N</td>
      <td class="kit nb" colspan="4"></td>
    </tr>

    <tr>
      <th class="blacktitle">CAP<br>No</th><th class="nameh blacktitle" colspan="7">NAME</th>
      <th class="blacktitle" colspan="2">1st Half<br>Goals</th><th class="blacktitle">2nd Half<br>Goals</th>
      <th class="blacktitle">Extra Time<br>Goals</th><th class="blacktitle">Sum Goals</th>
      <th class="divider">CAP<br>No</th><th class="nameh" colspan="7">NAME</th>
      <th>1st Half<br>Goals</th><th>2nd Half<br>Goals</th><th>Extra Time<br>Goals</th><th>Sum Goals</th>
    </tr>
{roster_html}
    <tr>
      <td class="lbl blackfill" colspan="8">Unknown / Own Goals</td><td class="blackfill" colspan="2"></td><td class="blackfill"></td><td class="blackfill"></td><td class="blackfill"></td>
      <td class="lbl divider" colspan="8">Unknown / Own Goals</td><td></td><td></td><td></td><td></td>
    </tr>
    <tr>
      <td class="lbl blackfill" colspan="8">Penalty Goals Awarded</td><td class="blackfill" colspan="2"></td><td class="blackfill"></td><td class="blackfill"></td><td class="blackfill"></td>
      <td class="lbl divider" colspan="8">Penalty Goals Awarded</td><td></td><td></td><td></td><td></td>
    </tr>

    <tr>
      <td class="nb" colspan="8"></td>
      <td class="rlbl nb" colspan="4">Total Score</td><td class="blackfill"></td>
      <td class="nb"></td><td class="nb" colspan="7"></td>
      <td class="rlbl nb" colspan="3">Total Score</td><td></td>
    </tr>

    <tr><td class="nb" colspan="25"></td></tr>

    <tr>
      <td class="sec nb" colspan="8">Time Penalties</td>
      <td class="nb" colspan="2"></td><td class="sec nb" colspan="3">Team Time Outs</td>
      <td class="nb"></td><td class="sec-l nb" colspan="7">Referees</td>
      <td class="sigcell" colspan="4" rowspan="15">
        <div class="sigbox">
          <div class="sig"><div class="line"></div><div class="cap2">Chief Referee Signature</div></div>
          <div class="sig"><div class="line"></div><div class="cap2">White Captain Signature</div></div>
          <div class="sig"><div class="line"></div><div class="cap2">Black Captain Signature</div></div>
        </div>
      </td>
    </tr>
    <tr>
      <td class="note nb" colspan="8">Write cap num in box + (B) for Black, (W) for White</td>
      <td class="nb" colspan="2"></td><td class="nb"></td><th>1st hlf</th><th>2nd hlf</th>
      <td class="nb"></td><td class="oflbl nb" colspan="7">Chief Referee:</td>
    </tr>
    <tr>
      <th>#</th><th>Mins</th><td class="gap"></td><th>#</th><th>Mins</th><td class="gap"></td><th>#</th><th>Mins</th>
      <td class="nb" colspan="2"></td><td class="lbl blacktitle">Black</td><td class="blackfill"></td><td class="blackfill"></td>
      <td class="nb"></td><td class="ofnm nb" colspan="7">{chief}</td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="nb" colspan="2"></td><td class="lbl">White</td><td></td><td></td>
      <td class="nb"></td><td class="oflbl nb" colspan="7">Assistant Chief Referee:</td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="nb" colspan="2"></td><td class="nb" colspan="3"></td>
      <td class="nb"></td><td class="ofnm nb" colspan="7">&nbsp;</td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="nb" colspan="2"></td><td class="nb" colspan="3"></td>
      <td class="nb"></td><td class="oflbl nb" colspan="7">Water Referees:</td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="nb" colspan="2"></td><td class="oflbl nb" colspan="2">Actual Game Start Time:</td><td></td>
      <td class="nb"></td><td class="ofnm nb" colspan="7">{water1}</td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="nb" colspan="2"></td><td class="oflbl nb" colspan="2">Actual Game Finish Time:</td><td></td>
      <td class="nb"></td><td class="ofnm nb" colspan="7">{water2}</td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="nb" colspan="2"></td><td class="nb" colspan="3"></td>
      <td class="nb"></td><td class="ofnm nb" colspan="7">{water3}</td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="nb" colspan="2"></td><td class="nb" colspan="3"></td>
      <td class="nb"></td><td class="oflbl nb" colspan="7">Scorer and Time keeper:</td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="nb" colspan="2"></td><td class="nb" colspan="3"></td>
      <td class="nb"></td><td class="ofnm nb" colspan="7">{timekeeper}</td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="nb" colspan="2"></td><td class="nb" colspan="3"></td>
      <td class="nb"></td><td class="ofnm nb" colspan="7">{timekeeper_helper}</td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="nb" colspan="2"></td><td class="nb" colspan="3"></td><td class="nb"></td><td class="nb" colspan="7"></td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="nb" colspan="2"></td><td class="nb" colspan="3"></td><td class="nb"></td><td class="nb" colspan="7"></td>
    </tr>
    <tr>
      <td></td><td></td><td class="gap"></td><td></td><td></td><td class="gap"></td><td></td><td></td>
      <td class="important nb" colspan="13">Important - These are important documents and are to be handed to Tournament Director</td>
    </tr>
  </table>

</div></div></body></html>"#
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

    #[test]
    fn abbreviates_the_round_type() {
        assert_eq!(round_type_abbrev("U19W RR"), "RR");
        assert_eq!(round_type_abbrev("Elite XO"), "XO");
        assert_eq!(round_type_abbrev("po bracket"), "PO");
        assert_eq!(round_type_abbrev("MD Gold"), "MD");
        assert_eq!(round_type_abbrev("Something Else"), "");
    }

    #[test]
    fn builds_the_division_label() {
        assert_eq!(division_label("U19W", "RR"), "U19W - RR");
        // Either half missing: print what we have, never a stray dash.
        assert_eq!(division_label("U19W", ""), "U19W");
        assert_eq!(division_label("", "RR"), "RR");
        assert_eq!(division_label("", ""), "");
    }

    #[test]
    fn formats_date_on_two_lines_and_time_with_meridiem() {
        use time::macros::datetime;
        let dt = datetime!(2026-08-05 07:30:00 +00:00);
        assert_eq!(format_date_two_line(dt), "Wednesday<br>05-Aug-2026");
        assert_eq!(format_start_time(dt), "7:30 AM");
    }

    #[test]
    fn formats_afternoon_times_as_pm() {
        use time::macros::datetime;
        let dt = datetime!(2026-08-05 14:06:00 +00:00);
        assert_eq!(format_start_time(dt), "2:06 PM");
    }

    fn sample_input() -> (Vec<SheetRosterRow>, Vec<SheetRosterRow>) {
        let black = roster_rows(&[
            player(Some(7), "Blake RIVE", true, false),
            player(Some(8), "Keith LIN", false, true),
        ])
        .0;
        let white = roster_rows(&[player(Some(1), "Chloe Jade PIETERSE", false, false)]).0;
        (black, white)
    }

    fn sample_cmas_input<'a>(
        black: &'a [SheetRosterRow],
        white: &'a [SheetRosterRow],
    ) -> CmasSheetInput<'a> {
        CmasSheetInput {
            event_name: "2026 CMAS 7th World Championship",
            division: "U19W - RR",
            game_number: "2",
            court: "B",
            black_team: "NEW ZEALAND",
            white_team: "SOUTH AFRICA",
            date_html: "Wednesday<br>05-Aug-2026",
            start_time: "7:30 AM",
            black_roster: black,
            white_roster: white,
            chief: "M. ALVAREZ",
            water: ["J. SMITH", "A. CHEN", "R. PATEL"],
            timekeeper: "K. ITO",
            timekeeper_helper: "L. MOORE",
            cmas_logo_rel: Some(CMAS_LOGO_FILENAME),
            tournament_logo_rel: Some("right.png"),
        }
    }

    #[test]
    fn renders_a_page_the_pdf_pipeline_can_extract() {
        let (black, white) = sample_input();
        let input = sample_cmas_input(&black, &white);

        let html = render_html_cmas_official(&input);

        // The combined-PDF extractor depends on these exact shapes.
        assert!(
            html.contains("<div class='page'>"),
            "page wrapper must use single quotes"
        );
        assert!(html.contains("<style>") && html.contains("</style>"));
        assert!(html.trim_end().ends_with("</div></body></html>"));
        assert!(
            html.contains("size: A4 landscape"),
            "must print A4 landscape"
        );
    }

    #[test]
    fn renders_the_supplied_game_data() {
        let (black, white) = sample_input();
        let input = sample_cmas_input(&black, &white);
        let html = render_html_cmas_official(&input);

        assert!(html.contains("CMAS UNDERWATER HOCKEY SCORESHEET"));
        assert!(html.contains("2026 CMAS 7th World Championship"));
        assert!(html.contains("U19W - RR"));
        assert!(html.contains("NEW ZEALAND") && html.contains("SOUTH AFRICA"));
        assert!(
            html.contains("Wednesday<br>05-Aug-2026"),
            "date must not be escaped"
        );
        assert!(html.contains("7:30 AM"));
        assert!(html.contains("Blake RIVE (C)") && html.contains("Keith LIN (VC)"));
        assert!(html.contains("M. ALVAREZ") && html.contains("R. PATEL"));
        assert!(html.contains(CMAS_LOGO_FILENAME) && html.contains("right.png"));
    }

    #[test]
    fn escapes_team_names_but_still_prints_them() {
        let (black, white) = sample_input();
        let mut input = sample_cmas_input(&black, &white);
        input.black_team = "A & B <Club>";
        input.chief = "Q & R <Ref>";
        input.division = "U19W & <RR>";
        let html = render_html_cmas_official(&input);
        assert!(html.contains("A &amp; B &lt;Club&gt;"));
        assert!(!html.contains("A & B <Club>"));
        assert!(html.contains("Q &amp; R &lt;Ref&gt;"));
        assert!(!html.contains("Q & R <Ref>"));
        assert!(html.contains("U19W &amp; &lt;RR&gt;"));
        assert!(!html.contains("U19W & <RR>"));
    }

    #[test]
    fn has_the_expected_grid_shape() {
        let (black, white) = sample_input();
        let html = render_html_cmas_official(&sample_cmas_input(&black, &white));
        assert_eq!(html.matches("<col ").count(), 25, "25 master columns");
        assert_eq!(html.matches("<tr>").count(), 35, "35 rows");
    }

    #[test]
    fn marks_name_cells_for_shrink_to_fit() {
        let (black, white) = sample_input();
        let html = render_html_cmas_official(&sample_cmas_input(&black, &white));
        // Both team-name cells and all 24 player-name cells opt into shrink-to-fit.
        // Every such cell's class list ends in `fit` (e.g. `class="name blackfill
        // fit"`), so match on the trailing `fit"` rather than a leading `class="fit`
        // — the latter never occurs, since `fit` is always the last class token.
        assert!(
            html.matches("fit\"").count() >= 26,
            "team and player name cells must carry the fit class"
        );
    }

    #[test]
    fn team_and_player_names_get_the_fit_class() {
        // Porting rule 7: Task 8's shrink-to-fit script selects on class="fit".
        let (black, white) = sample_input();
        let html = render_html_cmas_official(&sample_cmas_input(&black, &white));

        // Team-name header cells.
        assert!(
            html.contains("class=\"hdrval blackfill fit\""),
            "black team-name header cell must carry the fit class"
        );
        assert!(
            html.contains("class=\"hdrval fit\""),
            "white team-name header cell must carry the fit class"
        );

        // Roster player-name cells (one per side, per row).
        assert!(
            html.contains("class=\"name blackfill fit\""),
            "black roster name cells must carry the fit class"
        );
        assert!(
            html.contains("class=\"name fit\""),
            "white roster name cells must carry the fit class"
        );
    }

    #[test]
    fn renders_empty_logoboxes_when_no_logo_is_supplied() {
        // Porting rule 5: the no-logo path is the one real games without a
        // tournament logo will actually exercise.
        let (black, white) = sample_input();
        let mut input = sample_cmas_input(&black, &white);
        input.cmas_logo_rel = None;
        input.tournament_logo_rel = None;
        let html = render_html_cmas_official(&input);

        assert_eq!(
            html.matches("<div class=\"logobox\"></div>").count(),
            2,
            "both logoboxes should be empty, fixed-size placeholders when no logo is supplied"
        );
        assert!(
            !html.contains("<img"),
            "no <img> tag should be emitted when no logo is supplied"
        );
    }

    // ---- A4 fit check ------------------------------------------------------
    //
    // An earlier design revision was 41px too tall and printed as two pages,
    // invisibly. This regresses the moment anyone adds a row, so it needs a
    // command rather than an eyeball: `just check-cmas-sheet`.
    //
    // This started life (Task 9's plan) as a `tests/a4_fit.rs` integration test
    // reaching a `schedule-processor` library target. Adding a `[lib]` target
    // made `sample_page_for_fit_check` (needed by that external test, but by
    // nothing else) dead code in the separate binary compilation of this same
    // file, which `cargo clippy -- -D warnings` then rejected. Per the plan's
    // documented fallback, the fit check instead lives here as an `#[ignore]`d
    // unit test — both it and its helper only exist in test builds, so there is
    // no unused-in-production code to warn about.

    /// A representative worst-case page, used by the A4 fit check.
    ///
    /// Twelve players per side with long names, so the check fails if the
    /// layout grows beyond one page for any realistic game.
    fn sample_page_for_fit_check() -> String {
        let long = |n: u8, s: &str| RosterPlayer {
            number: Some(n),
            name: s.to_string(),
            is_captain: n == 7,
            is_vice_captain: n == 8,
        };
        let players: Vec<RosterPlayer> = (1..=12)
            .map(|n| long(n, "Gesje Maria PRETORIUS-VAN REENEN"))
            .collect();
        let (black, white) = (roster_rows(&players).0, roster_rows(&players).0);

        render_html_cmas_official(&CmasSheetInput {
            event_name: "2026 CMAS 7th World Championship Underwater Hockey Age Group",
            division: "U19W - RR",
            game_number: "2",
            court: "B",
            black_team: "SeaRIA Underwater Hockey (U15)",
            white_team: "Cucut Underwater Hockey U19",
            date_html: "Wednesday<br>05-Aug-2026",
            start_time: "7:30 AM",
            black_roster: &black,
            white_roster: &white,
            chief: "M. ALVAREZ",
            water: ["J. SMITH", "A. CHEN", "R. PATEL"],
            timekeeper: "K. ITO",
            timekeeper_helper: "L. MOORE",
            cmas_logo_rel: None,
            tournament_logo_rel: None,
        })
    }

    /// Confirms the CMAS Official sheet prints on exactly one A4 landscape page.
    ///
    /// Requires Chrome/Chromium. Ignored by default; run with:
    ///   just check-cmas-sheet
    #[test]
    #[ignore = "requires a local Chrome/Chromium install"]
    fn cmas_official_sheet_is_one_a4_landscape_page() {
        use std::process::Command;

        let dir = std::env::temp_dir().join("cmas-a4-fit");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");

        let html_path = dir.join("sheet.html");
        let pdf_path = dir.join("sheet.pdf");
        std::fs::write(&html_path, sample_page_for_fit_check()).expect("write sample html");

        let browser =
            std::env::var("SCORESHEET_BROWSER").unwrap_or_else(|_| "google-chrome".to_string());
        let status = Command::new(&browser)
            .args([
                "--headless",
                "--disable-gpu",
                "--no-sandbox",
                "--no-pdf-header-footer",
            ])
            .arg(format!("--print-to-pdf={}", pdf_path.display()))
            .arg(&html_path)
            .status()
            .unwrap_or_else(|e| panic!("could not run browser '{browser}': {e}"));
        assert!(status.success(), "browser failed to print the sheet");

        let pdf = std::fs::read(&pdf_path).expect("pdf should exist");
        let pages = count_pdf_pages(&pdf);
        assert_eq!(
            pages, 1,
            "the CMAS sheet must fit one A4 landscape page; it rendered {pages}. \
             Something added height — remove a row or reduce margins."
        );
        assert!(
            contains_a4_landscape_mediabox(&pdf),
            "page must be A4 landscape (841.92 x 594.96 pt)"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn count_pdf_pages(pdf: &[u8]) -> usize {
        // Count "/Type /Page" occurrences that are not "/Type /Pages".
        let needle = b"/Type /Page";
        let mut count = 0;
        let mut i = 0;
        while let Some(pos) = pdf[i..].windows(needle.len()).position(|w| w == needle) {
            let at = i + pos;
            let next = pdf.get(at + needle.len()).copied().unwrap_or(b' ');
            if next != b's' {
                count += 1;
            }
            i = at + needle.len();
        }
        count
    }

    fn contains_a4_landscape_mediabox(pdf: &[u8]) -> bool {
        let text = String::from_utf8_lossy(pdf);
        text.contains("841.9") && text.contains("594.9")
    }
}
