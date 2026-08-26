//! Turns the bridge's live picture of the game into the fixed-shape JSON tables vMix polls.
//!
//! Every function here is pure: no network, no files, no clock reads. Each takes [`state::Display`]
//! (and, where the refbox itself has nothing to offer -- team names, court, start time, roster
//! names -- already-resolved data derived from [`portal::Directory`]) and returns
//! `Vec<BTreeMap<String, String>>`, the shape vMix's JSON data source wants: one array element per
//! row, one object per row. `BTreeMap` (not `HashMap`) so a table's columns always serialize in a
//! stable, alphabetical order -- convenient when eyeballing a raw response, though vMix itself
//! matches columns by name (or position, as a fallback), never by their order in the object.
//!
//! [`state::Display`]: crate::state::Display
//! [`portal::Directory`]: crate::portal::Directory
//!
//! # Column names are a published contract
//!
//! The exact shapes are recorded in `docs/superpowers/specs/2026-08-26-vmix-integration-steps.md`.
//! Every column name here must match that document, and vice versa: once an operator or a third
//! party has bound a vMix title field to a column by name, renaming it silently breaks every title
//! built against it (see that document's gotcha G1). Two things about the shapes below were added
//! deliberately, beyond what that document first sketched, and are recorded there too:
//!
//! - `/scorebug` carries `leftTeam`/`leftScore`/`rightTeam`/`rightScore` alongside the existing
//!   `black*`/`white*` columns, so an operator whose camera has the two teams reversed from their
//!   kit colours (the "side of pool" setting -- see [`scorebug`]'s doc) can bind to a column that
//!   is never the wrong team, without displacing the black/white columns anyone else already
//!   bound to.
//! - `/scorebug` also carries `blackFouls`/`whiteFouls`/`blackWarnings`/`whiteWarnings`/
//!   `equalFouls`: the true, untruncated total recorded for each team (plus the independent
//!   both-at-fault count), so a title can show a running count even though the `/fouls` and
//!   `/warnings` tables themselves may not carry every entry (see "Row counts" below).
//!
//! # The `connected` column
//!
//! **Every table carries a `connected` column, on every row** -- `"true"` or `"false"`, following
//! the "bare values" rule below. It reflects whether the bridge's connection to the refbox is
//! alive right now, judged entirely by the connection itself ([`crate::feed::Connection`]) --
//! never by how long it has been since a message arrived, because the refbox goes completely
//! silent whenever the clock is stopped and a silence-based rule would blank the graphic every
//! time the referee stops the clock (spec §4.6, §5.4). The flag is added to every table, not only
//! `/scorebug`, because a title can bind to any one of them directly (a penalties title binds to
//! `/penalties`, not `/scorebug`) and needs the flag on the source it actually reads.
//!
//! **When disconnected, every other value in every row is also blanked to `""`** -- the row keeps
//! its shape (every column key still present, exactly as a normal padding row already does) but
//! nothing except `connected` itself is a value the refbox actually sent. This is the backstop
//! for a title that was never wired to `connected`: a careless one shows nothing meaningful
//! instead of stale numbers; a careful one, bound to `connected`, vanishes completely. Every
//! public function in this module ends by calling [`finish_table`], which is what applies this
//! rule -- see its doc for the exact mechanics.
//!
//! # Fixed length, blank padding
//!
//! vMix binds a title field to an explicit row number (the companion document's gotcha G2), so a
//! table whose length varies leaves a title bound to a row that used to exist reading stale or
//! missing data. `/scorebug` and `/nextgame` are always exactly one row. `/penalties` is always
//! exactly [`PENALTY_ROWS`] rows. `/fouls` and `/warnings` are always at least [`MIN_EVENT_ROWS`]
//! rows -- see "Row counts" below for why those two, alone, are allowed to grow past it. Every
//! padding row has every column present with an empty string, never an absent key -- an absent key
//! and an empty value are different things to a JSON consumer, and only the latter is safe for a
//! title bound to that column to read.
//!
//! # Row counts
//!
//! Penalties are culled by the refbox the moment they finish being served, so the active set is
//! naturally bounded and cannot realistically approach [`PENALTY_ROWS`] -- ten, fixed, no growth.
//!
//! Fouls and warnings are **never** culled -- they accumulate for the whole game -- and more than
//! ten in a single half is not rare. A fixed table sized for that would send mostly-empty rows on
//! every poll of every quiet game (a vMix title realistically binds within the first few rows and
//! never to row 87), so instead `/fouls` and `/warnings` start at [`MIN_EVENT_ROWS`] (blank-padded
//! below it), grow one row per entry above it, and stop growing at [`MAX_EVENT_ROWS`] -- beyond
//! that the newest entries are kept and the oldest are dropped. **This makes their row count
//! variable, which is itself part of the published contract**: a title should only ever bind
//! within the first ten rows, which are guaranteed to exist regardless of how the game has gone;
//! beyond ten, a row exists only if there is an entry for it.
//!
//! # "Most recent first" is a best effort, not exact, across teams
//!
//! Fouls and warnings are ordered most-recent-first, so a title bound to row 1 shows what just
//! happened and truncation at [`MAX_EVENT_ROWS`] drops the oldest, never the newest. Within one
//! team's own list, "most recent" is exact: the refbox always appends, so the last entry in a
//! team's list is always that team's newest. **Across teams it is only ever an approximation**,
//! because [`InfractionSnapshot`] carries no timestamp -- nothing in the feed says whether a given
//! black foul happened before or after a given white one, and a foul or warning can even be
//! reassigned from one team's list to the other's mid-game (`refbox/src/tournament_manager/mod.rs`,
//! `edit_warning`/`edit_foul`, lines 884 and 916 -- both `remove` then `push`), which would put it
//! at the *front* of its new list however long ago it actually happened. [`event_rows`] resolves
//! this by round-robin: newest of team A, newest of team B, (newest of team C, for fouls' `equal`
//! bucket), then second-newest of
//! each in the same order, and so on. This guarantees both teams' latest activity surfaces within
//! the first few rows -- rather than, say, one team's fifteen fouls filling every row a title binds
//! to before the other team's single very recent one ever appears -- but it is not a true
//! chronological merge. Flagged here, and in the report for this task, as a real limitation of what
//! the refbox's feed can support, not an oversight.
//!
//! # Bare values, no baked-in labels
//!
//! Every value is a string, including numbers -- vMix title fields are text, and this avoids any
//! question of how a native JSON number would render. A duration-shaped value (the game clock, a
//! penalty's or timeout's remaining time) is served twice: once display-ready (`"3:47"`, `"TD"`)
//! and once as a plain number of seconds (`"227"`), because title systems cannot do arithmetic on
//! the display-ready form. `court` and `start_time` are served bare (`"2"`, `"09:30"`), never as
//! `"COURT: 2"` or `"START: 09:55"` -- the overlay bakes those labels in
//! (`overlay/src/network.rs:335,340`) because it draws its own picture; vMix titles add their own
//! label through the data source's Format setting, so a baked-in label here could never be removed
//! by an operator. A start time is a wall-clock instant, not a countdown, so unlike the durations
//! above it is served only in its display-ready `HH:MM` form -- there is no meaningful "seconds"
//! companion for it here, and the companion document does not ask for one.
//!
//! # Cap numbers with no name never become a placeholder
//!
//! The refbox reports cap numbers only; names come entirely from the roster lookups the caller
//! passes in as a [`Rosters`]. A cap number absent from the roster (nothing fetched yet, or a
//! genuinely unrostered player) renders with the number present and the name column empty --
//! never `"None"`, `"null"`, or `"Unknown"`.

use std::collections::{BTreeMap, HashMap};

use time::{OffsetDateTime, format_description::BorrowedFormatItem, macros::format_description};
use uwh_common::{
    bundles::BlackWhiteBundle,
    color::Color,
    game_snapshot::{InfractionSnapshot, PenaltySnapshot, PenaltyTime, TimeoutSnapshot},
    uwhportal::schedule::FORMAT as ISO8601_FORMAT,
};

use crate::{portal::TeamNames, state::Display};

/// Every penalty table is exactly this many rows, blank-padded. Penalties are culled by the
/// refbox the moment they finish being served, so the active set is naturally bounded and cannot
/// realistically approach this many -- see the module doc's "Row counts" section for the fuller
/// reasoning, and why fouls and warnings are handled differently.
const PENALTY_ROWS: usize = 10;

/// The floor every `/fouls` and `/warnings` table is padded up to, and the point above which they
/// start growing one row per entry instead of staying fixed. See the module doc's "Row counts"
/// section.
const MIN_EVENT_ROWS: usize = 10;

/// The ceiling `/fouls` and `/warnings` stop growing at. Beyond this many entries, the newest
/// [`MAX_EVENT_ROWS`] are kept and the oldest are dropped -- `/scorebug`'s per-team counts report
/// the true, untruncated total separately, so truncation here is never invisible. See the module
/// doc's "Row counts" section.
const MAX_EVENT_ROWS: usize = 100;

/// The columns every row of `/penalties` has, including the blank-padding rows.
const PENALTY_COLUMNS: &[&str] = &[
    "team",
    "number",
    "player",
    "time",
    "timeSeconds",
    "infraction",
];

/// The columns every row of `/fouls` and `/warnings` has, including the blank-padding rows.
const EVENT_COLUMNS: &[&str] = &["team", "number", "player", "infraction"];

/// `[hour]:[minute]` -- how a raw ISO 8601 start time is rendered for display, matching the
/// overlay's own rendering (`overlay/src/network.rs:16,339-341`).
const DISPLAY_TIME_FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[hour]:[minute]");

/// One team's roster, resolved ahead of time by whatever owns a [`crate::portal::Directory`] --
/// keyed by cap number, same as the refbox itself reports players. A cap number with no entry
/// here renders with an empty name, never a placeholder; see the module doc.
pub type Roster = HashMap<u8, String>;

/// Both teams' rosters, resolved ahead of time for whichever teams are currently on the black and
/// white sides of the active game. This module never resolves a cap number to a name itself --
/// see the module doc for why `crate::portal::Directory`'s own lookup can't be called from here
/// directly (it's keyed by the portal's `TeamId`, which nothing in the refbox's own feed carries),
/// so the caller resolves both rosters first and hands the result in.
pub type Rosters = BlackWhiteBundle<Roster>;

/// The single-row `/scorebug` table: the live score, clock, period, and active timeout (if any),
/// plus per-team foul and warning counts, the independent both-at-fault foul count, and a
/// side-of-pool-aware left/right pairing.
///
/// `names` supplies the team names for the game currently being played (`display.snapshot`'s own
/// `game_number()`), resolved by the caller from [`crate::portal::Directory::names_for`] --
/// `None` if nothing has been resolved yet, in which case both name columns are empty rather than
/// a placeholder.
///
/// `white_on_right` is the operator's side-of-pool setting: whether the white team is drawn on the
/// physical right (matching the refbox's own `white_on_right` hardware setting,
/// `refbox/src/app/update_sender.rs:536-545`, which the live feed itself never carries). It only
/// affects `leftTeam`/`leftScore`/`rightTeam`/`rightScore` -- `blackTeam`/`whiteTeam`/`blackScore`/
/// `whiteScore` are unaffected, since a team's kit colour never changes mid-game.
///
/// `connected` is passed straight through to [`finish_table`] -- see that function's doc and the
/// module doc's "The `connected` column" section.
pub fn scorebug(
    display: &Display,
    names: Option<&TeamNames>,
    white_on_right: bool,
    connected: bool,
) -> Vec<BTreeMap<String, String>> {
    let snapshot = &display.snapshot;

    let black_team = names.and_then(|n| n.dark.clone()).unwrap_or_default();
    let white_team = names.and_then(|n| n.light.clone()).unwrap_or_default();
    let black_score = snapshot.scores.black.to_string();
    let white_score = snapshot.scores.white.to_string();

    let (timeout_text, timeout_secs) = match snapshot.timeout {
        Some(timeout) => (
            timeout_label(timeout).to_string(),
            Some(u32::from(timeout_seconds(timeout))),
        ),
        None => (String::new(), None),
    };

    let (left_team, left_score, right_team, right_score) = if white_on_right {
        (
            black_team.clone(),
            black_score.clone(),
            white_team.clone(),
            white_score.clone(),
        )
    } else {
        (
            white_team.clone(),
            white_score.clone(),
            black_team.clone(),
            black_score.clone(),
        )
    };

    let mut row = BTreeMap::new();
    row.insert("blackTeam".to_string(), black_team);
    row.insert("blackScore".to_string(), black_score);
    row.insert("whiteTeam".to_string(), white_team);
    row.insert("whiteScore".to_string(), white_score);
    row.insert("clock".to_string(), clock_string(snapshot.secs_in_period));
    row.insert(
        "clockSeconds".to_string(),
        snapshot.secs_in_period.to_string(),
    );
    row.insert("period".to_string(), snapshot.current_period.to_string());
    row.insert("timeout".to_string(), timeout_text);
    row.insert(
        "timeoutClock".to_string(),
        timeout_secs.map(clock_string).unwrap_or_default(),
    );
    row.insert(
        "timeoutClockSeconds".to_string(),
        timeout_secs
            .map(|secs| secs.to_string())
            .unwrap_or_default(),
    );
    row.insert("leftTeam".to_string(), left_team);
    row.insert("leftScore".to_string(), left_score);
    row.insert("rightTeam".to_string(), right_team);
    row.insert("rightScore".to_string(), right_score);
    row.insert(
        "blackFouls".to_string(),
        snapshot.fouls.black.len().to_string(),
    );
    row.insert(
        "whiteFouls".to_string(),
        snapshot.fouls.white.len().to_string(),
    );
    row.insert(
        "blackWarnings".to_string(),
        snapshot.warnings.black.len().to_string(),
    );
    row.insert(
        "whiteWarnings".to_string(),
        snapshot.warnings.white.len().to_string(),
    );
    // The both-at-fault ("equal") foul count, independent of `blackFouls`/`whiteFouls` -- a foul
    // recorded as equal fault is never counted as either team's own. Like those two, this is the
    // true, untruncated total, not limited to whatever `/fouls` itself carries (see that table's
    // doc and the module doc's "Row counts" section).
    row.insert(
        "equalFouls".to_string(),
        snapshot.fouls.equal.len().to_string(),
    );

    finish_table(vec![row], connected)
}

/// The single-row `/nextgame` table: the upcoming game's team names, court, and start time.
///
/// `names` is resolved entirely by the caller -- this function doesn't decide which game counts
/// as "next" (that's a question about `display.snapshot.next_game_number()` versus
/// `game_number()`, which depends on the live period in a way this module has no other reason to
/// know about; see the report for this task). Pass `None` when the caller has decided there's
/// nothing to show yet, and every column comes back empty rather than a placeholder -- the same
/// happens for any individual field `names` itself doesn't have.
///
/// `connected` is passed straight through to [`finish_table`] -- see that function's doc and the
/// module doc's "The `connected` column" section.
pub fn next_game(names: Option<&TeamNames>, connected: bool) -> Vec<BTreeMap<String, String>> {
    let mut row = BTreeMap::new();
    row.insert(
        "blackTeam".to_string(),
        names.and_then(|n| n.dark.clone()).unwrap_or_default(),
    );
    row.insert(
        "whiteTeam".to_string(),
        names.and_then(|n| n.light.clone()).unwrap_or_default(),
    );
    row.insert(
        "court".to_string(),
        names.and_then(|n| n.court.clone()).unwrap_or_default(),
    );
    row.insert(
        "startTime".to_string(),
        names
            .and_then(|n| n.start_time.as_deref())
            .map(start_time_string)
            .unwrap_or_default(),
    );

    finish_table(vec![row], connected)
}

/// The `/penalties` table: always exactly [`PENALTY_ROWS`] rows, ordered the way the overlay
/// already orders its own penalty flags (`overlay/src/flag.rs:233-280`) -- total dismissals
/// first, then timed penalties by longest remaining time, reusing [`PenaltyTime`]'s own `Ord`
/// (`TotalDismissal` sorts greatest; `Seconds` compares numerically) sorted descending. If more
/// than [`PENALTY_ROWS`] are somehow active at once, the least significant by that same ordering
/// are dropped -- in practice this never happens, since penalties are culled once served.
///
/// `connected` is passed straight through to [`finish_table`] -- see that function's doc and the
/// module doc's "The `connected` column" section.
pub fn penalties(
    display: &Display,
    rosters: &Rosters,
    connected: bool,
) -> Vec<BTreeMap<String, String>> {
    let snapshot = &display.snapshot;

    let mut entries: Vec<(Color, &PenaltySnapshot)> = snapshot
        .penalties
        .black
        .iter()
        .map(|penalty| (Color::Black, penalty))
        .chain(
            snapshot
                .penalties
                .white
                .iter()
                .map(|penalty| (Color::White, penalty)),
        )
        .collect();
    entries.sort_by_key(|(_, penalty)| std::cmp::Reverse(penalty.time));

    let mut rows: Vec<BTreeMap<String, String>> = entries
        .into_iter()
        .take(PENALTY_ROWS)
        .map(|(color, penalty)| penalty_row(color, penalty, rosters))
        .collect();

    while rows.len() < PENALTY_ROWS {
        rows.push(blank_row(PENALTY_COLUMNS));
    }
    finish_table(rows, connected)
}

/// The `/fouls` table: at least [`MIN_EVENT_ROWS`] rows, growing up to [`MAX_EVENT_ROWS`] as
/// described in the module doc's "Row counts" section. Carries all three of the refbox's foul
/// buckets -- black, white, and `equal` (the both-at-fault case) -- as `team: "EQUAL"` rows; the
/// `equal` bucket is never dropped.
///
/// `connected` is passed straight through to [`finish_table`] -- see that function's doc and the
/// module doc's "The `connected` column" section.
pub fn fouls(
    display: &Display,
    rosters: &Rosters,
    connected: bool,
) -> Vec<BTreeMap<String, String>> {
    let bundle = &display.snapshot.fouls;
    let buckets: [(Option<Color>, &[InfractionSnapshot]); 3] = [
        (Some(Color::Black), bundle.black.as_slice()),
        (Some(Color::White), bundle.white.as_slice()),
        (None, bundle.equal.as_slice()),
    ];
    finish_table(event_rows(&buckets, rosters), connected)
}

/// The `/warnings` table: at least [`MIN_EVENT_ROWS`] rows, growing up to [`MAX_EVENT_ROWS`] as
/// described in the module doc's "Row counts" section. Warnings have no `equal` (both-at-fault)
/// bucket -- only `GameSnapshot`'s `fouls` does.
///
/// `connected` is passed straight through to [`finish_table`] -- see that function's doc and the
/// module doc's "The `connected` column" section.
pub fn warnings(
    display: &Display,
    rosters: &Rosters,
    connected: bool,
) -> Vec<BTreeMap<String, String>> {
    let bundle = &display.snapshot.warnings;
    let buckets: [(Option<Color>, &[InfractionSnapshot]); 2] = [
        (Some(Color::Black), bundle.black.as_slice()),
        (Some(Color::White), bundle.white.as_slice()),
    ];
    finish_table(event_rows(&buckets, rosters), connected)
}

/// Shared row-building logic for `/fouls` and `/warnings`: merges `buckets` (each a team-labelled
/// list, oldest-first exactly as the refbox reports it) into one most-recent-first list by
/// round-robin -- newest of the first bucket, newest of the second, (newest of the third, if
/// present), then each bucket's second-newest, and so on, skipping any bucket once it runs out --
/// caps at [`MAX_EVENT_ROWS`] (dropping the entries that round-robin placed last, which are the
/// oldest), and pads with blank rows up to [`MIN_EVENT_ROWS`]. See the module doc's "'Most recent
/// first' is a best effort" section for why round-robin, not a true merge, is the best this can
/// do without per-entry timestamps.
fn event_rows(
    buckets: &[(Option<Color>, &[InfractionSnapshot])],
    rosters: &Rosters,
) -> Vec<BTreeMap<String, String>> {
    let mut iters: Vec<_> = buckets
        .iter()
        .map(|(color, entries)| (*color, entries.iter().rev()))
        .collect();

    let mut merged: Vec<(Option<Color>, &InfractionSnapshot)> = Vec::new();
    loop {
        let mut any_produced = false;
        for (color, iter) in &mut iters {
            if let Some(entry) = iter.next() {
                merged.push((*color, entry));
                any_produced = true;
            }
        }
        if !any_produced {
            break;
        }
    }
    merged.truncate(MAX_EVENT_ROWS);

    let mut rows: Vec<BTreeMap<String, String>> = merged
        .into_iter()
        .map(|(color, entry)| infraction_row(color, entry, rosters))
        .collect();

    while rows.len() < MIN_EVENT_ROWS {
        rows.push(blank_row(EVENT_COLUMNS));
    }
    rows
}

/// Renders one penalty as a `/penalties` row. `time`/`timeSeconds` follow the module doc's "Bare
/// values" rule: a total dismissal has no countdown, so it renders as `"TD"` with `timeSeconds`
/// left empty -- never `"0"`, which would read as a penalty about to expire rather than one with
/// no expiry at all.
fn penalty_row(
    color: Color,
    penalty: &PenaltySnapshot,
    rosters: &Rosters,
) -> BTreeMap<String, String> {
    let (time, time_secs) = match penalty.time {
        PenaltyTime::TotalDismissal => ("TD".to_string(), String::new()),
        PenaltyTime::Seconds(secs) => (clock_string(u32::from(secs)), secs.to_string()),
    };

    let mut row = BTreeMap::new();
    row.insert("team".to_string(), color_code(color).to_string());
    row.insert("number".to_string(), penalty.player_number.to_string());
    row.insert(
        "player".to_string(),
        rosters[color]
            .get(&penalty.player_number)
            .cloned()
            .unwrap_or_default(),
    );
    row.insert("time".to_string(), time);
    row.insert("timeSeconds".to_string(), time_secs);
    row.insert(
        "infraction".to_string(),
        penalty.infraction.short_name().to_string(),
    );
    row
}

/// Renders one foul or warning as an `/fouls` or `/warnings` row. `color` is `None` only for
/// `/fouls`' `equal` (both-at-fault) bucket, which has no single team's roster to check a cap
/// number against -- its `player` column is always empty, even when a cap number is present.
fn infraction_row(
    color: Option<Color>,
    entry: &InfractionSnapshot,
    rosters: &Rosters,
) -> BTreeMap<String, String> {
    let team = match color {
        Some(color) => color_code(color).to_string(),
        None => "EQUAL".to_string(),
    };
    let player = match (color, entry.player_number) {
        (Some(color), Some(number)) => rosters[color].get(&number).cloned().unwrap_or_default(),
        _ => String::new(),
    };

    let mut row = BTreeMap::new();
    row.insert("team".to_string(), team);
    row.insert(
        "number".to_string(),
        entry
            .player_number
            .map(|number| number.to_string())
            .unwrap_or_default(),
    );
    row.insert("player".to_string(), player);
    row.insert(
        "infraction".to_string(),
        entry.infraction.short_name().to_string(),
    );
    row
}

/// A row with every one of `columns` present and set to `""` -- what a `/penalties`, `/fouls`, or
/// `/warnings` slot with nothing in it renders as. Never an absent key: an absent key and an empty
/// value are different things to a JSON consumer, and only the latter is safe for a title bound to
/// that column to read.
fn blank_row(columns: &[&str]) -> BTreeMap<String, String> {
    columns
        .iter()
        .map(|&column| (column.to_string(), String::new()))
        .collect()
}

/// Adds the `connected` column to every row of `rows`, and -- when `connected` is `false` --
/// clears every other value in every row first, to `""`. See the module doc's "The `connected`
/// column" section: this is what every one of this module's public table-building functions ends
/// with, so the rule is applied identically everywhere rather than reimplemented per table.
///
/// `connected` is always `"true"` or `"false"` (via `bool::to_string`), matching the "bare
/// values, no baked-in labels" rule the rest of this module already follows for every other
/// column.
///
/// The row's shape -- its set of column keys, and how many rows there are -- is preserved either
/// way; only values change. A row that was already blank (a `/penalties`, `/fouls`, or
/// `/warnings` padding row from [`blank_row`]) is unaffected by the clearing step, since its
/// other columns were already `""`.
fn finish_table(
    mut rows: Vec<BTreeMap<String, String>>,
    connected: bool,
) -> Vec<BTreeMap<String, String>> {
    for row in &mut rows {
        if !connected {
            for value in row.values_mut() {
                value.clear();
            }
        }
        row.insert("connected".to_string(), connected.to_string());
    }
    rows
}

/// `"BLACK"` / `"WHITE"` -- the literal team identifiers used in every table's `team` column,
/// matching the overlay's own convention (`overlay/src/network.rs`, `BLACK_TEAM_NAME` /
/// `WHITE_TEAM_NAME`).
fn color_code(color: Color) -> &'static str {
    match color {
        Color::Black => "BLACK",
        Color::White => "WHITE",
    }
}

/// A duration in whole seconds, rendered `M:SS` (no leading zero on minutes, always two digits of
/// seconds) -- matching the format already used for a penalty's remaining time
/// (`refbox/src/tournament_manager/penalty.rs`, `PenaltyTimePrintable::Remaining`).
fn clock_string(total_secs: u32) -> String {
    format!("{}:{:02}", total_secs / 60, total_secs % 60)
}

/// The remaining seconds carried by any [`TimeoutSnapshot`] variant.
fn timeout_seconds(timeout: TimeoutSnapshot) -> u16 {
    match timeout {
        TimeoutSnapshot::Black(secs)
        | TimeoutSnapshot::White(secs)
        | TimeoutSnapshot::Ref(secs)
        | TimeoutSnapshot::PenaltyShot(secs) => secs,
    }
}

/// A broadcast-appropriate label for a timeout -- deliberately not `TimeoutSnapshot`'s own
/// `Display` impl (`uwh-common/src/game_snapshot.rs`). That impl was written for internal use and
/// has never actually been viewer-facing anywhere: the refbox's own screens render a timeout
/// through a Fluent translation instead
/// (`refbox/src/app/view_builders/shared_elements.rs:806`, `fl!("penalty-shot-short")` ->
/// `"PNLTY SHT"` in en-US, properly spaced in every locale), the overlay matches on the enum
/// variant rather than the string, and the only other consumer is the golden-trace debug tool.
/// Its `PenaltyShot` case renders as `"PenaltyShot"`, with no space, unlike the other three
/// variants -- harmless where nothing ever put it in front of an audience, but this bridge would
/// be the first thing that did, so it gets its own labels instead of inheriting that
/// inconsistency onto a live broadcast.
fn timeout_label(timeout: TimeoutSnapshot) -> &'static str {
    match timeout {
        TimeoutSnapshot::Black(_) => "Black Timeout",
        TimeoutSnapshot::White(_) => "White Timeout",
        TimeoutSnapshot::Ref(_) => "Ref Timeout",
        TimeoutSnapshot::PenaltyShot(_) => "Penalty Shot",
    }
}

/// Renders a raw ISO 8601 timestamp, exactly as the portal returned it, as `HH:MM` in the offset
/// the timestamp itself carries -- matching the overlay's own rendering
/// (`overlay/src/network.rs:16,339-341`). Empty if `raw` doesn't parse: a malformed or otherwise
/// unusable value must never surface as `"None"` or similar, the same rule as everywhere else in
/// this module.
fn start_time_string(raw: &str) -> String {
    OffsetDateTime::parse(raw, &ISO8601_FORMAT)
        .ok()
        .and_then(|parsed| parsed.format(&DISPLAY_TIME_FORMAT).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use uwh_common::game_snapshot::{GamePeriod, GameSnapshot, Infraction};

    use super::*;

    fn display_with(snapshot: GameSnapshot) -> Display {
        Display { snapshot }
    }

    fn base_snapshot() -> GameSnapshot {
        GameSnapshot {
            current_period: GamePeriod::SecondHalf,
            secs_in_period: 227,
            scores: BlackWhiteBundle { black: 3, white: 2 },
            ..Default::default()
        }
    }

    fn names(
        dark: Option<&str>,
        light: Option<&str>,
        court: Option<&str>,
        start: Option<&str>,
    ) -> TeamNames {
        TeamNames {
            dark: dark.map(str::to_string),
            light: light.map(str::to_string),
            court: court.map(str::to_string),
            start_time: start.map(str::to_string),
        }
    }

    fn row0(rows: &[BTreeMap<String, String>]) -> &BTreeMap<String, String> {
        &rows[0]
    }

    fn get<'a>(row: &'a BTreeMap<String, String>, column: &str) -> &'a str {
        row.get(column)
            .unwrap_or_else(|| panic!("row is missing column {column:?}"))
    }

    // ---------------------------------------------------------------- scorebug

    #[test]
    fn scorebug_is_always_exactly_one_row() {
        let display = display_with(base_snapshot());
        let rows = scorebug(&display, None, false, true);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn scorebug_renders_scores_clock_and_period_as_bare_strings() {
        let display = display_with(base_snapshot());
        let rows = scorebug(&display, None, false, true);
        let row = row0(&rows);

        assert_eq!(get(row, "blackScore"), "3");
        assert_eq!(get(row, "whiteScore"), "2");
        assert_eq!(get(row, "clock"), "3:47");
        assert_eq!(get(row, "clockSeconds"), "227");
        assert_eq!(get(row, "period"), "Second Half");
    }

    #[test]
    fn scorebug_with_no_names_resolved_yields_empty_team_columns_not_a_placeholder() {
        let display = display_with(base_snapshot());
        let rows = scorebug(&display, None, false, true);
        let row = row0(&rows);

        assert_eq!(get(row, "blackTeam"), "");
        assert_eq!(get(row, "whiteTeam"), "");
    }

    #[test]
    fn scorebug_takes_team_names_from_the_portal_directory_dark_and_light_fields() {
        let display = display_with(base_snapshot());
        let team_names = names(Some("AUSTRALIA"), Some("CANADA"), None, None);
        let rows = scorebug(&display, Some(&team_names), false, true);
        let row = row0(&rows);

        assert_eq!(get(row, "blackTeam"), "AUSTRALIA");
        assert_eq!(get(row, "whiteTeam"), "CANADA");
    }

    #[test]
    fn scorebug_with_no_active_timeout_leaves_timeout_columns_empty() {
        let display = display_with(base_snapshot());
        let rows = scorebug(&display, None, false, true);
        let row = row0(&rows);

        assert_eq!(get(row, "timeout"), "");
        assert_eq!(get(row, "timeoutClock"), "");
        assert_eq!(get(row, "timeoutClockSeconds"), "");
    }

    #[test]
    fn scorebug_renders_an_active_timeout_display_ready_and_as_seconds() {
        let display = display_with(GameSnapshot {
            timeout: Some(TimeoutSnapshot::White(75)),
            ..base_snapshot()
        });
        let rows = scorebug(&display, None, false, true);
        let row = row0(&rows);

        assert_eq!(get(row, "timeout"), "White Timeout");
        assert_eq!(get(row, "timeoutClock"), "1:15");
        assert_eq!(get(row, "timeoutClockSeconds"), "75");
    }

    #[test]
    fn a_penalty_shot_timeout_serves_a_properly_spaced_broadcast_label() {
        // TimeoutSnapshot's own `Display` impl renders this variant as "PenaltyShot", with no
        // space -- fine for the internal/debug uses it was written for, but never before put in
        // front of a viewer. This bridge must not be the first thing that does.
        let display = display_with(GameSnapshot {
            timeout: Some(TimeoutSnapshot::PenaltyShot(20)),
            ..base_snapshot()
        });
        let rows = scorebug(&display, None, false, true);
        let row = row0(&rows);

        assert_eq!(get(row, "timeout"), "Penalty Shot");
    }

    #[test]
    fn swapping_the_side_of_pool_setting_swaps_which_team_is_in_the_left_hand_columns() {
        let display = display_with(base_snapshot());
        let team_names = names(Some("AUSTRALIA"), Some("CANADA"), None, None);

        let white_on_right = scorebug(&display, Some(&team_names), true, true);
        let row = row0(&white_on_right);
        assert_eq!(get(row, "leftTeam"), "AUSTRALIA");
        assert_eq!(get(row, "leftScore"), "3");
        assert_eq!(get(row, "rightTeam"), "CANADA");
        assert_eq!(get(row, "rightScore"), "2");

        let white_on_left = scorebug(&display, Some(&team_names), false, true);
        let row = row0(&white_on_left);
        assert_eq!(get(row, "leftTeam"), "CANADA");
        assert_eq!(get(row, "leftScore"), "2");
        assert_eq!(get(row, "rightTeam"), "AUSTRALIA");
        assert_eq!(get(row, "rightScore"), "3");
    }

    #[test]
    fn scorebug_counts_report_the_true_total_even_when_the_fouls_table_has_been_truncated() {
        let black_fouls: Vec<InfractionSnapshot> = (0..120)
            .map(|n| InfractionSnapshot {
                player_number: Some((n % 15) + 1),
                infraction: Infraction::Unknown,
            })
            .collect();
        let snapshot = GameSnapshot {
            fouls: uwh_common::bundles::OptColorBundle {
                black: black_fouls,
                equal: Vec::new(),
                white: Vec::new(),
            },
            ..base_snapshot()
        };
        let display = display_with(snapshot);

        // The count on /scorebug must be the true 120, not the 100 the /fouls table itself caps
        // out at.
        let scorebug_rows = scorebug(&display, None, false, true);
        assert_eq!(get(row0(&scorebug_rows), "blackFouls"), "120");

        let rosters = Rosters::default();
        let fouls_rows = fouls(&display, &rosters, true);
        assert_eq!(fouls_rows.len(), MAX_EVENT_ROWS);
    }

    #[test]
    fn scorebug_equal_foul_total_is_independent_of_the_per_team_totals_and_counts_beyond_truncation()
     {
        // A game with 120 equal (both-at-fault) fouls and none recorded against either team on
        // its own. If `equalFouls` were derived from -- or confused with -- `blackFouls`/
        // `whiteFouls`, or only counted whatever `/fouls` itself carries, this would catch it:
        // both per-team totals must stay "0" while `equalFouls` reports the true, untruncated
        // 120, the same way `blackFouls`/`whiteFouls` already do for their own buckets (see
        // `scorebug_counts_report_the_true_total_even_when_the_fouls_table_has_been_truncated`
        // above).
        let equal_fouls: Vec<InfractionSnapshot> = (0..120)
            .map(|n| InfractionSnapshot {
                player_number: Some((n % 15) + 1),
                infraction: Infraction::Unknown,
            })
            .collect();
        let snapshot = GameSnapshot {
            fouls: uwh_common::bundles::OptColorBundle {
                black: Vec::new(),
                equal: equal_fouls,
                white: Vec::new(),
            },
            ..base_snapshot()
        };
        let display = display_with(snapshot);
        let rows = scorebug(&display, None, false, true);
        let row = row0(&rows);

        assert_eq!(get(row, "equalFouls"), "120");
        assert_eq!(get(row, "blackFouls"), "0");
        assert_eq!(get(row, "whiteFouls"), "0");
    }

    // ---------------------------------------------------------------- the `connected` column

    #[test]
    fn every_scorebug_row_carries_connected_true_when_connected() {
        let display = display_with(base_snapshot());
        let rows = scorebug(&display, None, false, true);
        assert_eq!(get(row0(&rows), "connected"), "true");
    }

    #[test]
    fn scorebug_blanks_every_value_but_connected_when_disconnected() {
        let display = display_with(base_snapshot());
        let team_names = names(Some("AUSTRALIA"), Some("CANADA"), None, None);
        let rows = scorebug(&display, Some(&team_names), false, false);
        let row = row0(&rows);

        assert_eq!(get(row, "connected"), "false");
        for column in row.keys() {
            if column != "connected" {
                assert_eq!(
                    get(row, column),
                    "",
                    "column {column:?} should be blank while disconnected"
                );
            }
        }
    }

    #[test]
    fn next_game_blanks_every_value_but_connected_when_disconnected() {
        let team_names = names(Some("SYDNEY KINGS A"), Some("BRISBANE A"), Some("2"), None);
        let rows = next_game(Some(&team_names), false);
        let row = row0(&rows);

        assert_eq!(get(row, "connected"), "false");
        assert_eq!(get(row, "blackTeam"), "");
        assert_eq!(get(row, "whiteTeam"), "");
        assert_eq!(get(row, "court"), "");
    }

    #[test]
    fn penalties_blanks_every_row_but_keeps_the_connected_column_when_disconnected() {
        let snapshot = GameSnapshot {
            penalties: BlackWhiteBundle {
                black: vec![penalty(7, PenaltyTime::Seconds(102))],
                white: Vec::new(),
            },
            ..base_snapshot()
        };
        let display = display_with(snapshot);
        let rows = penalties(&display, &Rosters::default(), false);

        assert_eq!(rows.len(), PENALTY_ROWS);
        for row in &rows {
            assert_eq!(get(row, "connected"), "false");
            for column in PENALTY_COLUMNS {
                assert_eq!(
                    get(row, column),
                    "",
                    "column {column:?} should be blank while disconnected, including on the row \
                     that had a real penalty when connected"
                );
            }
        }
    }

    #[test]
    fn fouls_blanks_every_row_but_keeps_the_connected_column_when_disconnected() {
        let black = vec![infraction(Some(3))];
        let display = display_with(fouls_snapshot(black, Vec::new(), Vec::new()));
        let rows = fouls(&display, &Rosters::default(), false);

        assert_eq!(rows.len(), MIN_EVENT_ROWS);
        for row in &rows {
            assert_eq!(get(row, "connected"), "false");
            for column in EVENT_COLUMNS {
                assert_eq!(get(row, column), "");
            }
        }
    }

    #[test]
    fn warnings_blanks_every_row_but_keeps_the_connected_column_when_disconnected() {
        let black = vec![infraction(Some(6))];
        let display = display_with(warnings_snapshot(black, Vec::new()));
        let rows = warnings(&display, &Rosters::default(), false);

        assert_eq!(rows.len(), MIN_EVENT_ROWS);
        for row in &rows {
            assert_eq!(get(row, "connected"), "false");
            for column in EVENT_COLUMNS {
                assert_eq!(get(row, column), "");
            }
        }
    }

    #[test]
    fn a_blank_padding_row_still_carries_the_connected_column_when_connected() {
        // `blank_row` (used for padding) never sets `connected` itself -- `finish_table` adds it
        // afterward, uniformly, to every row including padding ones. If that step only touched
        // populated rows, a title bound to a padding row's `connected` column would read a
        // missing key instead of "true".
        let display = display_with(base_snapshot()); // no active penalties at all
        let rows = penalties(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), PENALTY_ROWS);
        for row in &rows {
            assert_eq!(get(row, "connected"), "true");
            assert_eq!(
                get(row, "number"),
                "",
                "sanity check: this row is genuinely blank"
            );
        }
    }

    // ---------------------------------------------------------------- next_game

    #[test]
    fn next_game_with_nothing_resolved_yields_every_column_empty() {
        let rows = next_game(None, true);
        let row = row0(&rows);

        assert_eq!(get(row, "blackTeam"), "");
        assert_eq!(get(row, "whiteTeam"), "");
        assert_eq!(get(row, "court"), "");
        assert_eq!(get(row, "startTime"), "");
    }

    #[test]
    fn next_game_renders_a_raw_iso8601_start_time_as_hh_mm_and_serves_bare_court() {
        let team_names = names(
            Some("SYDNEY KINGS A"),
            Some("BRISBANE A"),
            Some("2"),
            Some("2026-08-01T09:30:00+10:00"),
        );
        let rows = next_game(Some(&team_names), true);
        let row = row0(&rows);

        assert_eq!(get(row, "blackTeam"), "SYDNEY KINGS A");
        assert_eq!(get(row, "whiteTeam"), "BRISBANE A");
        // Bare value: no "COURT: " prefix baked in.
        assert_eq!(get(row, "court"), "2");
        // Bare, display-ready HH:MM: no "START: " prefix baked in, and the +10:00 offset is
        // resolved into the printed hour rather than ignored.
        assert_eq!(get(row, "startTime"), "09:30");
    }

    #[test]
    fn next_game_with_no_court_or_start_time_yields_empty_strings_not_a_placeholder() {
        let team_names = names(Some("SYDNEY KINGS A"), Some("BRISBANE A"), None, None);
        let rows = next_game(Some(&team_names), true);
        let row = row0(&rows);

        assert_eq!(get(row, "court"), "");
        assert_eq!(get(row, "startTime"), "");
    }

    #[test]
    fn next_game_with_an_unparseable_start_time_yields_an_empty_string_not_a_panic() {
        let team_names = names(None, None, None, Some("not a real timestamp"));
        let rows = next_game(Some(&team_names), true);
        assert_eq!(get(row0(&rows), "startTime"), "");
    }

    // ---------------------------------------------------------------- penalties

    fn penalty(player_number: u8, time: PenaltyTime) -> PenaltySnapshot {
        PenaltySnapshot {
            player_number,
            time,
            infraction: Infraction::StickInfringement,
        }
    }

    #[test]
    fn no_penalties_still_returns_the_full_row_count_every_value_empty() {
        let display = display_with(base_snapshot());
        let rows = penalties(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), PENALTY_ROWS);
        for row in &rows {
            for column in PENALTY_COLUMNS {
                assert_eq!(get(row, column), "", "column {column:?} should be blank");
            }
        }
    }

    #[test]
    fn two_penalties_populate_rows_one_and_two_and_leave_the_rest_blank() {
        let snapshot = GameSnapshot {
            penalties: BlackWhiteBundle {
                black: vec![penalty(7, PenaltyTime::Seconds(102))],
                white: vec![penalty(3, PenaltyTime::TotalDismissal)],
            },
            ..base_snapshot()
        };
        let display = display_with(snapshot);
        let rows = penalties(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), PENALTY_ROWS);
        assert_ne!(get(&rows[0], "number"), "");
        assert_ne!(get(&rows[1], "number"), "");
        for row in &rows[2..] {
            for column in PENALTY_COLUMNS {
                assert_eq!(get(row, column), "");
            }
        }
    }

    #[test]
    fn a_total_dismissal_renders_td_with_an_empty_seconds_column_not_zero() {
        let snapshot = GameSnapshot {
            penalties: BlackWhiteBundle {
                black: vec![penalty(9, PenaltyTime::TotalDismissal)],
                white: Vec::new(),
            },
            ..base_snapshot()
        };
        let display = display_with(snapshot);
        let rows = penalties(&display, &Rosters::default(), true);
        let row = &rows[0];

        assert_eq!(get(row, "time"), "TD");
        assert_eq!(get(row, "timeSeconds"), "");
    }

    #[test]
    fn a_cap_number_with_no_roster_entry_renders_the_number_with_an_empty_name() {
        let snapshot = GameSnapshot {
            penalties: BlackWhiteBundle {
                black: vec![penalty(42, PenaltyTime::Seconds(30))],
                white: Vec::new(),
            },
            ..base_snapshot()
        };
        let display = display_with(snapshot);
        let rows = penalties(&display, &Rosters::default(), true);
        let row = &rows[0];

        assert_eq!(get(row, "number"), "42");
        assert_eq!(get(row, "player"), "");
    }

    #[test]
    fn a_cap_number_with_a_roster_entry_resolves_to_its_name() {
        let mut black_roster = Roster::new();
        black_roster.insert(7, "SMITH".to_string());
        let rosters = Rosters {
            black: black_roster,
            white: Roster::new(),
        };
        let snapshot = GameSnapshot {
            penalties: BlackWhiteBundle {
                black: vec![penalty(7, PenaltyTime::Seconds(102))],
                white: Vec::new(),
            },
            ..base_snapshot()
        };
        let display = display_with(snapshot);
        let rows = penalties(&display, &rosters, true);
        let row = &rows[0];

        assert_eq!(get(row, "team"), "BLACK");
        assert_eq!(get(row, "player"), "SMITH");
        assert_eq!(get(row, "time"), "1:42");
        assert_eq!(get(row, "timeSeconds"), "102");
    }

    #[test]
    fn penalties_are_ordered_dismissals_first_then_longest_remaining() {
        let snapshot = GameSnapshot {
            penalties: BlackWhiteBundle {
                black: vec![
                    penalty(1, PenaltyTime::Seconds(30)),
                    penalty(2, PenaltyTime::TotalDismissal),
                ],
                white: vec![penalty(3, PenaltyTime::Seconds(90))],
            },
            ..base_snapshot()
        };
        let display = display_with(snapshot);
        let rows = penalties(&display, &Rosters::default(), true);

        // TD (cap 2) first, then the 90s penalty (cap 3), then the 30s penalty (cap 1).
        assert_eq!(get(&rows[0], "number"), "2");
        assert_eq!(get(&rows[0], "time"), "TD");
        assert_eq!(get(&rows[1], "number"), "3");
        assert_eq!(get(&rows[1], "time"), "1:30");
        assert_eq!(get(&rows[2], "number"), "1");
        assert_eq!(get(&rows[2], "time"), "0:30");
    }

    #[test]
    fn penalties_stays_fixed_at_ten_regardless_of_how_many_are_active() {
        let black: Vec<PenaltySnapshot> = (1..=12)
            .map(|n| penalty(n, PenaltyTime::Seconds(u16::from(n) * 10)))
            .collect();
        let snapshot = GameSnapshot {
            penalties: BlackWhiteBundle {
                black,
                white: Vec::new(),
            },
            ..base_snapshot()
        };
        let display = display_with(snapshot);
        let rows = penalties(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), PENALTY_ROWS);
        // The longest-remaining ten (cap numbers 3-12) survive; the two shortest (1, 2) are the
        // least significant by the module's own ordering and are dropped.
        let surviving: Vec<&str> = rows.iter().map(|row| get(row, "number")).collect();
        assert!(!surviving.contains(&"1"));
        assert!(!surviving.contains(&"2"));
        assert!(surviving.contains(&"12"));
    }

    // ---------------------------------------------------------------- fouls / warnings shared shape

    fn infraction(player_number: Option<u8>) -> InfractionSnapshot {
        InfractionSnapshot {
            player_number,
            infraction: Infraction::DelayOfGame,
        }
    }

    fn fouls_snapshot(
        black: Vec<InfractionSnapshot>,
        white: Vec<InfractionSnapshot>,
        equal: Vec<InfractionSnapshot>,
    ) -> GameSnapshot {
        GameSnapshot {
            fouls: uwh_common::bundles::OptColorBundle {
                black,
                equal,
                white,
            },
            ..base_snapshot()
        }
    }

    #[test]
    fn fouls_with_no_entries_returns_exactly_ten_blank_rows() {
        let display = display_with(fouls_snapshot(Vec::new(), Vec::new(), Vec::new()));
        let rows = fouls(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), MIN_EVENT_ROWS);
        for row in &rows {
            for column in EVENT_COLUMNS {
                assert_eq!(get(row, column), "");
            }
        }
    }

    #[test]
    fn fouls_with_three_entries_returns_ten_rows_three_populated_seven_blank() {
        let black = vec![
            infraction(Some(1)),
            infraction(Some(2)),
            infraction(Some(3)),
        ];
        let display = display_with(fouls_snapshot(black, Vec::new(), Vec::new()));
        let rows = fouls(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), MIN_EVENT_ROWS);
        let populated = rows
            .iter()
            .filter(|row| !get(row, "number").is_empty())
            .count();
        assert_eq!(populated, 3);
    }

    #[test]
    fn fouls_with_twenty_five_entries_returns_exactly_twenty_five_rows_none_blank() {
        let black: Vec<InfractionSnapshot> = (1..=25).map(|n| infraction(Some(n))).collect();
        let display = display_with(fouls_snapshot(black, Vec::new(), Vec::new()));
        let rows = fouls(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), 25);
        for row in &rows {
            assert_ne!(get(row, "number"), "");
        }
    }

    #[test]
    fn fouls_beyond_one_hundred_entries_keeps_the_newest_one_hundred() {
        // Cap numbers double as a sequence marker: 1 is oldest (pushed first), 150 is newest.
        let black: Vec<InfractionSnapshot> = (1..=150u8).map(|n| infraction(Some(n))).collect();
        let display = display_with(fouls_snapshot(black, Vec::new(), Vec::new()));
        let rows = fouls(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), MAX_EVENT_ROWS);
        // Newest-first: row 0 is the very last one pushed (150), row 99 is the oldest survivor
        // (51) -- entries 1-50 were dropped.
        assert_eq!(get(&rows[0], "number"), "150");
        assert_eq!(get(&rows[99], "number"), "51");
        let surviving: std::collections::HashSet<&str> =
            rows.iter().map(|row| get(row, "number")).collect();
        assert!(
            !surviving.contains("50"),
            "entry 50 should have been dropped as too old"
        );
        assert!(
            surviving.contains("51"),
            "entry 51 should be the oldest survivor"
        );
    }

    #[test]
    fn the_equal_foul_bucket_appears_in_the_fouls_table_and_is_not_dropped() {
        let equal = vec![infraction(Some(5))];
        let display = display_with(fouls_snapshot(Vec::new(), Vec::new(), equal));
        let rows = fouls(&display, &Rosters::default(), true);

        let equal_row = rows
            .iter()
            .find(|row| get(row, "team") == "EQUAL")
            .expect("the equal bucket's entry should be present as a row");
        assert_eq!(get(equal_row, "number"), "5");
    }

    #[test]
    fn a_foul_with_no_player_number_renders_an_empty_number_and_player_not_a_placeholder() {
        let black = vec![infraction(None)];
        let display = display_with(fouls_snapshot(black, Vec::new(), Vec::new()));
        let rows = fouls(&display, &Rosters::default(), true);
        let populated = rows
            .iter()
            .find(|row| get(row, "team") == "BLACK")
            .expect("the black entry should be present");

        assert_eq!(get(populated, "number"), "");
        assert_eq!(get(populated, "player"), "");
    }

    #[test]
    fn fouls_interleave_across_teams_round_robin_newest_first() {
        // Black has two entries (oldest to newest: 1, 2); white has one (3, the most recent
        // overall by push order). Round-robin visits black's newest, then white's newest, before
        // moving on to black's next-newest -- so the order is 2 (black), 3 (white), 1 (black).
        let black = vec![infraction(Some(1)), infraction(Some(2))];
        let white = vec![infraction(Some(3))];
        let display = display_with(fouls_snapshot(black, white, Vec::new()));
        let rows = fouls(&display, &Rosters::default(), true);

        let order: Vec<&str> = rows
            .iter()
            .filter(|row| !get(row, "number").is_empty())
            .map(|row| get(row, "number"))
            .collect();
        assert_eq!(order, vec!["2", "3", "1"]);
    }

    #[test]
    fn fouls_with_both_teams_heavily_populated_interleaves_before_truncating() {
        // The compound case a real long, foul-heavy game actually produces: both teams well past
        // ten entries each, and the combined total well over MAX_EVENT_ROWS. Cap numbers double
        // as a push-order/recency marker, using disjoint ranges per team (black 1-80, white
        // 101-180) so surviving/dropped membership can be asserted per team without ambiguity.
        // Both buckets are the same length (80), so round-robin alternates evenly: black's
        // newest, white's newest, black's next-newest, white's next-newest, and so on -- 160
        // entries total, truncated to the newest 100.
        let black: Vec<InfractionSnapshot> = (1..=80u8).map(|n| infraction(Some(n))).collect();
        let white: Vec<InfractionSnapshot> = (101..=180u8).map(|n| infraction(Some(n))).collect();
        let display = display_with(fouls_snapshot(black, white, Vec::new()));
        let rows = fouls(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), MAX_EVENT_ROWS);

        // Row-by-row: round-robin alternates black/white, newest first, so the first four rows
        // are black's newest, white's newest, black's second-newest, white's second-newest -- and
        // the last two rows (99, 100) are the *last* survivors round-robin admitted before the
        // 100-row cutoff.
        assert_eq!(get(&rows[0], "team"), "BLACK");
        assert_eq!(get(&rows[0], "number"), "80");
        assert_eq!(get(&rows[1], "team"), "WHITE");
        assert_eq!(get(&rows[1], "number"), "180");
        assert_eq!(get(&rows[2], "team"), "BLACK");
        assert_eq!(get(&rows[2], "number"), "79");
        assert_eq!(get(&rows[3], "team"), "WHITE");
        assert_eq!(get(&rows[3], "number"), "179");
        assert_eq!(get(&rows[98], "team"), "BLACK");
        assert_eq!(get(&rows[98], "number"), "31");
        assert_eq!(get(&rows[99], "team"), "WHITE");
        assert_eq!(get(&rows[99], "number"), "131");

        // Both teams are represented in the surviving 100, in exactly equal proportion -- proves
        // truncation didn't let one team's bucket crowd the other out even under heavy load.
        let black_survivors: std::collections::HashSet<&str> = rows
            .iter()
            .filter(|row| get(row, "team") == "BLACK")
            .map(|row| get(row, "number"))
            .collect();
        let white_survivors: std::collections::HashSet<&str> = rows
            .iter()
            .filter(|row| get(row, "team") == "WHITE")
            .map(|row| get(row, "number"))
            .collect();
        assert_eq!(black_survivors.len(), 50);
        assert_eq!(white_survivors.len(), 50);

        // The survivors on each side are exactly that team's newest 50 -- not, say, its oldest 50
        // or an arbitrary 50.
        assert!(
            black_survivors.contains("80"),
            "black's newest must survive"
        );
        assert!(
            black_survivors.contains("31"),
            "the 50th-newest black entry is the cutoff"
        );
        assert!(
            !black_survivors.contains("30"),
            "the 51st-newest black entry should have been dropped as too old"
        );
        assert!(
            white_survivors.contains("180"),
            "white's newest must survive"
        );
        assert!(
            white_survivors.contains("131"),
            "the 50th-newest white entry is the cutoff"
        );
        assert!(
            !white_survivors.contains("130"),
            "the 51st-newest white entry should have been dropped as too old"
        );
    }

    // ---------------------------------------------------------------- warnings

    fn warnings_snapshot(
        black: Vec<InfractionSnapshot>,
        white: Vec<InfractionSnapshot>,
    ) -> GameSnapshot {
        GameSnapshot {
            warnings: BlackWhiteBundle { black, white },
            ..base_snapshot()
        }
    }

    #[test]
    fn warnings_with_no_entries_returns_exactly_ten_blank_rows() {
        let display = display_with(warnings_snapshot(Vec::new(), Vec::new()));
        let rows = warnings(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), MIN_EVENT_ROWS);
        for row in &rows {
            for column in EVENT_COLUMNS {
                assert_eq!(get(row, column), "");
            }
        }
    }

    #[test]
    fn warnings_with_twenty_five_entries_returns_exactly_twenty_five_rows_none_blank() {
        let black: Vec<InfractionSnapshot> = (1..=25).map(|n| infraction(Some(n))).collect();
        let display = display_with(warnings_snapshot(black, Vec::new()));
        let rows = warnings(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), 25);
        for row in &rows {
            assert_ne!(get(row, "number"), "");
        }
    }

    #[test]
    fn warnings_beyond_one_hundred_entries_keeps_the_newest_one_hundred() {
        let black: Vec<InfractionSnapshot> = (1..=130u8).map(|n| infraction(Some(n))).collect();
        let display = display_with(warnings_snapshot(black, Vec::new()));
        let rows = warnings(&display, &Rosters::default(), true);

        assert_eq!(rows.len(), MAX_EVENT_ROWS);
        assert_eq!(get(&rows[0], "number"), "130");
        assert_eq!(get(&rows[99], "number"), "31");
    }

    #[test]
    fn a_warning_cap_number_resolves_against_its_teams_roster() {
        let mut white_roster = Roster::new();
        white_roster.insert(11, "NGUYEN".to_string());
        let rosters = Rosters {
            black: Roster::new(),
            white: white_roster,
        };
        let display = display_with(warnings_snapshot(Vec::new(), vec![infraction(Some(11))]));
        let rows = warnings(&display, &rosters, true);
        let row = rows
            .iter()
            .find(|row| get(row, "team") == "WHITE")
            .expect("the white entry should be present");

        assert_eq!(get(row, "player"), "NGUYEN");
    }
}
