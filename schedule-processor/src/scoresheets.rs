use std::collections::HashMap;
use std::{
    fmt, fs,
    path::{Path, PathBuf},
    process::Command,
};
use time::{
    Duration as TimeDur, OffsetDateTime, format_description::FormatItem, macros::format_description,
};
use uwh_common::uwhportal::UwhPortalClient;
use uwh_common::uwhportal::schedule::{
    DateRange, Event, EventId, Game, ScheduledTeam, TeamList, TimingRule,
};

#[derive(Debug)]
struct AuthRequiredError;
impl fmt::Display for AuthRequiredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "authentication required for schedule")
    }
}
impl std::error::Error for AuthRequiredError {}

// Output style for scoresheets
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetStyle {
    Detailed,
    Simple,
}

// Simple container for user inputs
#[derive(Clone, Debug)]
pub struct RenderInputs {
    pub left_logo: Option<PathBuf>,
    pub right_logo: Option<PathBuf>,
    pub output_dir: PathBuf,
    pub style: SheetStyle,
    pub prefer_portal_officials: bool,
}

pub async fn generate_scoresheets_for_event(
    portal_client: &mut UwhPortalClient,
    event: &Event,
    inputs: RenderInputs,
    csv_schedule: Option<&uwh_common::uwhportal::schedule::Schedule>,
    ref_csv_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let schedule = if portal_client.has_token() {
        // Prefer privileged schedule when we can; it includes referee assignments
        portal_client
            .get_event_schedule_privileged(&event.id)
            .await?
    } else {
        match portal_client.get_event_schedule_public(&event.id).await {
            Ok(s) => s,
            Err(_) => return Err(Box::new(AuthRequiredError)),
        }
    };
    let teams = portal_client.get_event_teams(&event.id).await?;

    fs::create_dir_all(&inputs.output_dir)?;

    // Copy logos into output dir for stable relative paths
    let left_logo_rel = copy_logo(&inputs.output_dir, inputs.left_logo.as_deref(), "left")?;
    let right_logo_rel = copy_logo(&inputs.output_dir, inputs.right_logo.as_deref(), "right")?;

    // Load optional referee overrides from a CSV mapping (Game # -> names)
    let ref_overrides: HashMap<String, OfficialNames> = if let Some(p) = ref_csv_path {
        match parse_referee_csv(p) {
            Ok(map) => map,
            Err(e) => {
                log::warn!(
                    "Failed to parse referee schedule CSV ({}): {}",
                    p.display(),
                    e
                );
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    };

    let mut name_cache: HashMap<String, String> = HashMap::new();
    // Pre-fill official name cache: try public /referees first (no auth), then merge /participants (auth)
    match portal_client
        .get_event_referee_name_map_from_referees(&event.id)
        .await
    {
        Ok(map) => {
            let count = map.len();
            for (uid, name) in map {
                name_cache.entry(uid).or_insert(name);
            }
            log::debug!("Prefilled {} official names from /referees", count);
        }
        Err(e) => {
            log::warn!("Could not prefill officials from /referees: {}", e);
        }
    }
    match portal_client.get_event_referee_name_map(&event.id).await {
        Ok(map) => {
            let mut added = 0usize;
            for (uid, name) in map {
                if name_cache.insert(uid.clone(), name).is_none() {
                    added += 1;
                }
            }
            log::debug!("Merged {} official names from /participants", added);
        }
        Err(e) => {
            log::debug!(
                "Participants map not available (unauthenticated or forbidden): {}",
                e
            );
        }
    }

    // For combined single-PDF generation
    let mut combined_pages: String = String::new();
    let mut combined_css: Option<String> = None;

    for (num, game) in schedule.games.iter() {
        let (white_suffix, white_name) = placeholder_suffix_and_name(&game.light, &teams);
        let (black_suffix, black_name) = placeholder_suffix_and_name(&game.dark, &teams);

        // Decide official names based on preference: portal display names vs CSV
        let officials = if inputs.prefer_portal_officials {
            let resolved =
                resolve_officials(&*portal_client, &event.id, game, &mut name_cache).await;
            // Fallback to CSV for this game if portal provides nothing
            if resolved.chief.is_empty()
                && resolved.water1.is_empty()
                && resolved.water2.is_empty()
                && resolved.water3.is_empty()
                && resolved.ts_keeper.is_empty()
                && resolved.ts_helper.is_empty()
            {
                ref_overrides.get(num).cloned().unwrap_or(resolved)
            } else {
                resolved
            }
        } else if let Some(o) = ref_overrides.get(num) {
            o.clone()
        } else {
            resolve_officials(&*portal_client, &event.id, game, &mut name_cache).await
        };

        let tr = find_timing_rule(game, csv_schedule, &schedule)?;

        let html = match inputs.style {
            SheetStyle::Detailed => render_html(
                event,
                num,
                game,
                csv_schedule.or(Some(&schedule)),
                tr,
                &white_suffix,
                &white_name,
                &black_suffix,
                &black_name,
                &officials,
                left_logo_rel.as_deref(),
                right_logo_rel.as_deref(),
            ),
            SheetStyle::Simple => render_html_simple(
                event,
                num,
                game,
                csv_schedule.or(Some(&schedule)),
                tr,
                &white_suffix,
                &white_name,
                &black_suffix,
                &black_name,
                &officials,
            ),
        };

        let html_path = inputs
            .output_dir
            .join(format!("game-{}.html", sanitize(num)));
        fs::write(&html_path, html.as_bytes())?;

        // Capture CSS from the first page and append this page fragment for combined output
        if combined_css.is_none() {
            if let (Some(s), Some(e)) = (html.find("<style>"), html.find("</style>")) {
                let css_inner = &html[s + "<style>".len()..e];
                combined_css = Some(css_inner.to_string());
            }
        }
        if let Some(s) = html.find("<div class='page'>") {
            // Prefer slicing up to the closing </div> immediately before </body>,
            // so we include the full page wrapper regardless of how many nested divs are inside.
            if let Some(body_idx) = html.rfind("</body>") {
                if let Some(d) = html[..body_idx].rfind("</div>") {
                    let frag = &html[s..d + "</div>".len()];
                    combined_pages.push_str(frag);
                } else {
                    log::warn!(
                        "Could not find closing </div> before </body> for game {}",
                        num
                    );
                }
            } else if let Some(e) = html.rfind("</div></div>") {
                // Fallback for pages that end with two closing divs right before </body>
                let frag = &html[s..e + "</div></div>".len()];
                combined_pages.push_str(frag);
            } else {
                log::warn!("Could not extract page fragment for game {}", num);
            }
        } else {
            log::warn!("Page wrapper not found for game {}", num);
        }
    }

    // Build combined HTML and export a single PDF via Chrome headless
    if !combined_pages.is_empty() {
        let css = combined_css.unwrap_or_default();
        let all_html = format!(
            r#"<!doctype html><html><head><meta charset='utf-8'/><style>{css}</style></head><body>{pages}</body></html>"#,
            css = css,
            pages = combined_pages
        );
        let all_html_path = inputs.output_dir.join("scoresheets-all.html");
        fs::write(&all_html_path, all_html.as_bytes())?;
        let all_pdf_path = inputs.output_dir.join("scoresheets-all.pdf");

        // Convert to file:/// URL for Chrome without introducing Windows \\?\ prefix
        let html_abs = if all_html_path.is_absolute() {
            all_html_path.clone()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(&all_html_path)
        };
        let html_url = format!("file:///{}", html_abs.to_string_lossy().replace('\\', "/"));

        // Hardwire Chrome: prefer standard install paths, then PATH 'chrome'
        let mut candidates: Vec<String> = vec![
            r#"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"#.into(),
            r#"C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe"#.into(),
        ];
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let mut p = std::path::PathBuf::from(&local);
            p.push("Google\\Chrome\\Application\\chrome.exe");
            candidates.push(p.to_string_lossy().into_owned());
        }
        candidates.push("chrome".into());
        let try_arg_sets: [&[&str]; 4] = [
            &[
                "--headless=new",
                "--disable-gpu",
                "--allow-file-access-from-files",
                "--virtual-time-budget=8000",
                "--no-sandbox",
                "--disable-extensions",
                "--disable-dev-shm-usage",
            ],
            &[
                "--headless",
                "--disable-gpu",
                "--allow-file-access-from-files",
                "--virtual-time-budget=8000",
                "--no-sandbox",
                "--disable-extensions",
                "--disable-dev-shm-usage",
            ],
            &[
                "--headless=new",
                "--disable-gpu",
                "--allow-file-access-from-files",
                "--run-all-compositor-stages-before-draw",
                "--no-sandbox",
            ],
            &[
                "--headless",
                "--disable-gpu",
                "--allow-file-access-from-files",
                "--run-all-compositor-stages-before-draw",
                "--no-sandbox",
            ],
        ];
        let mut printed = false;
        'outer: for c in &candidates {
            // If 'c' looks like a path, ensure it exists before trying
            let is_path_like = c.contains('\\') || c.contains('/') || c.contains(':');
            if is_path_like && !std::path::Path::new(c).exists() {
                log::warn!("Chrome not found at: {}", c);
                continue;
            }
            for args in &try_arg_sets {
                let mut cmd = Command::new(c);
                let print_arg = format!("--print-to-pdf={}", all_pdf_path.display());
                cmd.args(*args)
                    .arg("--no-first-run")
                    .arg("--no-default-browser-check")
                    .arg(print_arg)
                    .arg(&html_url);
                log::info!(
                    "Attempting to print via '{}' url={} pdf={}",
                    c,
                    &html_url,
                    all_pdf_path.display()
                );
                match cmd.output() {
                    Ok(out) if out.status.success() => {
                        log::info!("Wrote combined PDF to {}", all_pdf_path.display());
                        printed = true;
                        break 'outer;
                    }
                    Ok(out) => {
                        let so = String::from_utf8_lossy(&out.stdout);
                        let se = String::from_utf8_lossy(&out.stderr);
                        log::error!(
                            "Chrome run failed (status {}). stdout: {} stderr: {}",
                            out.status,
                            so,
                            se
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to run Chrome '{}': {}", c, e);
                    }
                }
            }
        }

        if !printed {
            log::error!(
                "Could not find or run a browser to produce combined PDF. Saved combined HTML at {}",
                all_html_path.display()
            );
        }
    }

    Ok(())
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn copy_logo(
    output_dir: &Path,
    src: Option<&Path>,
    stem: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if let Some(src) = src {
        let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("png");
        let dst = output_dir.join(format!("{}.{ext}", stem));
        fs::copy(src, &dst)?;
        return Ok(Some(
            dst.file_name().unwrap().to_string_lossy().into_owned(),
        ));
    }
    Ok(None)
}

fn placeholder_suffix_and_name(team: &ScheduledTeam, teams: &TeamList) -> (String, String) {
    if let Some(id) = team.assigned() {
        let name = teams
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.partial().to_string());
        // Treat any assigned team whose name contains "Seed" (case-insensitive) as a placeholder suffix
        if name.to_ascii_lowercase().contains("seed") {
            (format!(" ({})", name), String::new())
        } else {
            (String::new(), name)
        }
    } else if let Some(r) = team.result_of() {
        match r {
            uwh_common::uwhportal::schedule::ResultOf::Winner { game_number } => {
                (format!(" (Winner {})", game_number), String::new())
            }
            uwh_common::uwhportal::schedule::ResultOf::Loser { game_number } => {
                (format!(" (Loser {})", game_number), String::new())
            }
        }
    } else if let Some(s) = team.seeded_by() {
        (format!(" (Seed {} {})", s.number, s.group), String::new())
    } else if let Some(p) = team.pending() {
        let up = p.to_ascii_uppercase();
        if up.contains("SEED")
            || up.starts_with("W_")
            || up.starts_with("L_")
            || up.contains("WINNER")
            || up.contains("LOSER")
        {
            (format!(" ({})", p), String::new())
        } else {
            // Treat as a free-text team name if not a recognized placeholder pattern
            (String::new(), p.to_string())
        }
    } else {
        (String::new(), String::new())
    }
}

fn derive_category(rule_name: &str) -> &'static str {
    let up = rule_name.to_ascii_uppercase();
    if up.contains("RR") {
        "Round Robin"
    } else if up.contains("XO") {
        "Crossover"
    } else if up.contains("PO") {
        "Playoff"
    } else if up.contains("MD") {
        "Medal Game"
    } else {
        ""
    }
}

fn div_pod_for_game(
    csv_schedule: Option<&uwh_common::uwhportal::schedule::Schedule>,
    game_number: &str,
) -> Option<(String, String, String)> {
    // Returns (div_short, div_long, pod_short)
    let mut div_short: Option<String> = None;
    let mut div_long: Option<String> = None;
    let mut pod_short: Option<String> = None;
    let sched = csv_schedule?;
    // Normalize target by stripping a leading 'G' (common notation differences)
    let target_norm = game_number.trim_start_matches(|c: char| ['G', 'g'].contains(&c));
    for g in &sched.groups {
        // Match if any group game number equals raw or normalized target (case-insensitive)
        let matches_group = g.game_numbers.iter().any(|n| {
            let n_str = n.as_str();
            let n_norm = n_str.trim_start_matches(|c: char| ['G', 'g'].contains(&c));
            n_str.eq_ignore_ascii_case(game_number) || n_norm.eq_ignore_ascii_case(target_norm)
        });
        if matches_group {
            if let Some(t) = g.group_type {
                match t {
                    uwh_common::uwhportal::schedule::GroupType::Division => {
                        if !g.short_name.is_empty() {
                            div_short = Some(g.short_name.clone());
                        }
                        if !g.name.is_empty() {
                            div_long = Some(g.name.clone());
                        }
                    }
                    uwh_common::uwhportal::schedule::GroupType::Pod => {
                        if !g.short_name.is_empty() {
                            pod_short = Some(g.short_name.clone());
                        }
                    }
                }
            }
        }
    }
    match (div_short, div_long, pod_short) {
        (Some(ds), Some(dl), Some(ps)) => Some((ds, dl, ps)),
        (Some(ds), Some(dl), None) => Some((ds, dl, String::new())),
        (None, None, Some(ps)) => Some((String::new(), String::new(), ps)),
        _ => None,
    }
}

fn find_timing_rule<'a>(
    game: &Game,
    csv_sched: Option<&'a uwh_common::uwhportal::schedule::Schedule>,
    portal_sched: &'a uwh_common::uwhportal::schedule::Schedule,
) -> Result<&'a uwh_common::uwhportal::schedule::TimingRule, Box<dyn std::error::Error>> {
    // Prefer CSV timing rules (last value wins) when available
    if let Some(cs) = csv_sched {
        if let Some(tr) = cs.get_game_timing(&game.number) {
            return Ok(tr);
        }
        if let Some(tr) = cs
            .timing_rules
            .iter()
            .find(|tr| tr.name == game.timing_rule)
        {
            return Ok(tr);
        }
    }
    if let Some(tr) = portal_sched.get_game_timing(&game.number) {
        return Ok(tr);
    }
    if let Some(tr) = portal_sched
        .timing_rules
        .iter()
        .find(|tr| tr.name == game.timing_rule)
    {
        return Ok(tr);
    }
    Err(format!(
        "Missing timing rule '{}'. Please ensure CSV has complete timing rules for this name.",
        game.timing_rule
    )
    .into())
}

#[derive(Default, Clone)]
struct OfficialNames {
    chief: String,
    water1: String,
    water2: String,
    water3: String,
    ts_keeper: String,
    ts_helper: String,
}

async fn resolve_officials(
    portal_client: &UwhPortalClient,
    event_id: &EventId,
    game: &Game,
    cache: &mut HashMap<String, String>,
) -> OfficialNames {
    let mut names = OfficialNames::default();
    if let Some(list) = &game.referee_assignments {
        let mut fetched_for_game = false;
        for a in list {
            let display = if let Some(n) = cache.get(&a.user_id) {
                n.clone()
            } else {
                // On first miss for this game, try to fetch all game refs in one call (public endpoint)
                if !fetched_for_game {
                    if let Ok(per_game) = portal_client
                        .get_game_referee_name_map(event_id, &game.number)
                        .await
                    {
                        for (uid, name) in per_game {
                            cache.entry(uid).or_insert(name);
                        }
                        log::debug!(
                            "Filled official names from /admin/events/game-referees for game {}",
                            game.number
                        );
                    }
                    fetched_for_game = true;
                }
                if let Some(n) = cache.get(&a.user_id) {
                    n.clone()
                } else {
                    match portal_client.get_user_display_name(&a.user_id).await {
                        Ok(n) => {
                            cache.insert(a.user_id.clone(), n.clone());
                            n
                        }
                        Err(_) => a
                            .user_id
                            .split('/')
                            .next_back()
                            .unwrap_or(&a.user_id)
                            .to_string(),
                    }
                }
            };

            match a.role.as_str() {
                "Chief" => names.chief = display,
                "Water1" => names.water1 = display,
                "Water2" => names.water2 = display,
                "Water3" => names.water3 = display,
                "TimeOrScoreKeeper" => names.ts_keeper = display,
                "TimeOrScoreHelper" => names.ts_helper = display,

                other => {
                    let low = other.to_ascii_lowercase();
                    if low.contains("helper") {
                        names.ts_helper = display;
                    }
                }
            }
        }
    }
    names
}

fn parse_referee_csv(
    path: &Path,
) -> Result<HashMap<String, OfficialNames>, Box<dyn std::error::Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(path)?;
    let headers = rdr.headers()?.clone();
    let find_idx = |name: &str| -> Result<usize, String> {
        headers
            .iter()
            .position(|h| h.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("Missing '{}' header in referee CSV", name))
    };
    let idx_game = find_idx("Game #")?;
    let idx_w1 = find_idx("Water 1")?;
    let idx_w2 = find_idx("Water 2")?;
    let idx_w3 = find_idx("Water 3")?;
    let idx_chief = find_idx("Chief")?;
    let idx_keeper = find_idx("T/S Keeper")?;
    let idx_helper = find_idx("T/S Helper")?;

    let mut map: HashMap<String, OfficialNames> = HashMap::new();
    for result in rdr.records() {
        let rec = result?;
        let game = rec.get(idx_game).unwrap_or("").trim().to_string();
        if game.is_empty() {
            continue;
        }
        let get = |i: usize| rec.get(i).unwrap_or("").trim().to_string();
        let names = OfficialNames {
            chief: get(idx_chief),
            water1: get(idx_w1),
            water2: get(idx_w2),
            water3: get(idx_w3),
            ts_keeper: get(idx_keeper),
            ts_helper: get(idx_helper),
        };
        map.insert(game, names);
    }
    Ok(map)
}

#[allow(clippy::too_many_arguments)]
fn render_html(
    event: &Event,
    game_number: &str,
    game: &Game,
    csv_schedule: Option<&uwh_common::uwhportal::schedule::Schedule>,
    tr: &uwh_common::uwhportal::schedule::TimingRule,
    white_suffix: &str,
    white_name: &str,
    black_suffix: &str,
    black_name: &str,
    officials: &OfficialNames,
    left_logo: Option<&str>,
    right_logo: Option<&str>,
) -> String {
    // Date/time formatting in event timezone
    let offset = event.date_range.start.offset();
    let local_dt: OffsetDateTime = game.start_time.to_offset(offset);
    // Example: Fri - Oct 17
    const DATE_FMT: &[FormatItem<'static>] =
        format_description!("[weekday repr:short] - [month repr:short] [day padding:none]");
    // Example: 10:15 AM
    const TIME_FMT: &[FormatItem<'static>] =
        format_description!("[hour repr:12]:[minute] [period case:upper]");
    let date_str = local_dt.format(&DATE_FMT).unwrap_or_default();
    let time_str = local_dt.format(&TIME_FMT).unwrap_or_default();

    // CSS ported from scoresheet-mock for pixel-match layout
    let css = r#"
      :root { --border:#888; --accent:#cc2b2b; }
      * { box-sizing:border-box; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
      body { font-family: Arial, Helvetica, sans-serif; margin:18px; color:#000; }
      .page { width: 11in; height: 8.5in; margin: 0 auto; border:1px solid #cfcfcf; padding:0.25in calc(0.25in - 10px); background:#fff; overflow:hidden; }
      @media print {
        @page { size: 11in 8.5in; margin: 0; }
        html, body { width:11in; height:8.5in; margin:0; }
        .page { width: 11in; height: 8.5in; margin: 0; padding:0.25in calc(0.25in - 10px); }
      }
      .inner { width:100%; transform: scale(0.95); transform-origin: top center; }


      .top-pad { height:48px; border:0; margin-bottom:8px; }

      .blue { display:grid; grid-template-columns: 120px 1fr 120px; gap:8px; align-items:stretch; border:1px solid var(--border); padding:6px; }
      .logo-square { border:0; aspect-ratio:1 / 1; display:flex; align-items:center; justify-content:center; text-align:center; font-size:12px; }
      .logo-square img { max-width:100%; max-height:100%; object-fit:contain; }
      .center { display:grid; grid-template-rows: auto auto auto auto; gap:4px; }
      .center .tname { text-align:center; font-weight:700; color:#000; font-size:22px; border:1px solid var(--border); padding:4px 8px; }
      .fields { display:grid; gap:5px; }
      .fields.meta { grid-template-columns: 0.69fr 0.9fr 1fr 0.75fr 0.9fr 0.9fr; }
      .fields.off1, .fields.off2 { grid-template-columns: repeat(3, 1fr); }
      .field { display:grid; grid-template-columns: auto 1fr; align-items:center; column-gap:6px; }
      .field .lbl { font-size:10px; text-align:right; line-height:1.05; white-space:nowrap; }
      .field .val { border:1px solid var(--border); height:22px; display:flex; align-items:center; justify-content:center; font-size:12px; }

      .sides { display:grid; grid-template-columns:1fr 1fr; gap:9px; margin-top:5px; }
      .block { border:0; }
      .block > * + * { margin-top:6px; }
      .block .hdr { display:none; }

      table.sheet { width:100%; border-collapse:collapse; font-size:11px; }
      table.sheet col.col-period { width:75px; }
      table.sheet col.col-score { width:30px; }
      table.sheet col.col-pgt { width:120px; }
      table.sheet.score { table-layout: fixed; }
      table.sheet.score col.col-score { width:17.4px; }
      table.sheet.score col.col-pgt { width:70px; }
      table.sheet.score col.col-sub { width:50px; }
      table.sheet.score col.col-timeouts { width:60px; }
      /* Uniform widths for faults and penalty numeric columns */
      table.sheet.faults { table-layout: fixed; }
      table.sheet.faults col.col-infra, table.sheet.faults col.col-warn { width:16.5px; }
      table.sheet.penalty { table-layout: fixed; }
      table.sheet.penalty col.col-pen { width:16.5px; }
      table.sheet.faults th:not(.tl), table.sheet.faults td:not(.tl) { padding:1px 0; }
      table.sheet.penalty th:not(.tl), table.sheet.penalty td:not(.tl) { padding:1px 0; }
      table.sheet.score th, table.sheet.score td { padding:2px 1px; }
      .sheet th, .sheet td { border:1px solid var(--border); padding:2px 2px; text-align:center; height:16px; }
      .sheet tr.final td { border-top:2px solid #666; }
      .sheet th.tl, .sheet td.tl { text-align:left; }
      .sheet th.period, .sheet td.period { white-space:nowrap; width:75px; }
      .sheet th.pgt, .sheet td.pgt { white-space:normal; }
      .sheet td.final-score { padding:1px 1px; }
      .sheet tr.sep-top td, .sheet tr.sep-top th { border-top-width:2px; }
      .sheet tr.notes-final td, .sheet tr.notes-final th { border-top-width:2px; }

      .sheet tr.score-numbers th { border-bottom:2px solid var(--border); }
      table.sheet:not(.faults):not(.penalty) tr > th:nth-child(2),
      table.sheet:not(.faults):not(.penalty) tr > td:nth-child(2) { border-left-width:2px; }
      table.sheet.score tr:not(.notes-final) > th:nth-child(16),
      table.sheet.score tr:not(.notes-final) > td:nth-child(16) { border-left-width:2px; border-right-width:2px; }
      table.sheet.score tr:not(.notes-final) > th:nth-child(17),
      table.sheet.score tr:not(.notes-final) > td:nth-child(17) { border-left-width:2px; border-right-width:2px; }
      table.sheet.score tr:not(.notes-final) > th:nth-child(18),
      table.sheet.score tr:not(.notes-final) > td:nth-child(18) { border-right-width:2px; }

      table.sheet.score tr.cap-row > th:nth-child(3),
      table.sheet.score tr.cap-row > th:nth-child(4) { border-left-width:2px; border-right-width:2px; }
      table.sheet.score tr.score-numbers > th:nth-child(14) { border-right-width:2px; }
      table.sheet.score tr.gold-goal > td:nth-child(3) { border-right-width:2px; }
      table.sheet.score tr.gold-goal > td:nth-child(4),
      table.sheet.score tr.gold-goal > td:nth-child(5) { border-left-width:2px; border-right-width:2px; }
      table.sheet.score tr.notes-final > td.final-score { border-right-width:2px; border-bottom-width:2px; }
      table.sheet.score tr.notes-final > td:nth-child(3) { border-left-width:2px; }
      table.sheet:not(.faults):not(.penalty) tr.score-numbers > th:nth-child(1) { border-left-width:2px; }
      table.sheet:not(.faults):not(.penalty) tr:not(.notes-final) > th:nth-child(16),
      table.sheet:not(.faults):not(.penalty) tr:not(.notes-final) > td:nth-child(16),
      table.sheet:not(.faults):not(.penalty) tr:not(.notes-final) > th:nth-child(17),
      table.sheet:not(.faults):not(.penalty) tr:not(.notes-final) > td:nth-child(17),
      table.sheet:not(.faults):not(.penalty) tr:not(.notes-final) > th:nth-child(18),
      table.sheet:not(.faults):not(.penalty) tr:not(.notes-final) > td:nth-child(18) { border-left-width:2px; }
      .sheet .final-score { font-weight:700; font-size:18px; text-align:left; white-space:nowrap; padding-left:6px; }
      .faults .sep-left { border-left:2px solid var(--border); }
      /* Disabled cells: white background with a small gray text 'X' centered */
      td.speckled { position: relative; background:transparent; }
      td.speckled::before {
        content: "X";
        position: absolute;
        left: 50%;
        top: 50%;
        transform: translate(-50%, -52%);
        font-size: 10px;
        line-height: 1;
        color: #9b9b9b;
        font-weight: 600;
        pointer-events: none;
      }
      .side.black table.sheet tr.cap-row th, .side.black table.sheet tr.cap-row td { background:#D3D3D3; }
      .side.black table.sheet tr.score-numbers th { background:#D3D3D3; }
      .side.black table.sheet.faults tr:nth-child(2) th { background:#D3D3D3; }
      .side.black .team-row { background:#D3D3D3; }
      .side.black td.final-score, .side.black table.sheet.score tr.notes-final > td:nth-last-child(2) { background:#D3D3D3; }
      .side.black table.sheet.score tr.notes-final > td:last-child { background:#D3D3D3; }
      .sigline { margin-top:55px; display:grid; grid-template-columns: 1fr 1fr 1fr; gap:30px; color:#333; font-size:11px; }
      .sigline .line { border-top:1px solid var(--border); text-align:center; padding-top:5px; height:32px; }
      .team-row { display:grid; grid-template-columns: auto 1fr; align-items:center; column-gap:8px; border:1px solid var(--border); padding:5px; margin-top:6px; }
      .team-label { font-weight:700; text-align:left; }
      .team-label .suffix { font-weight:400; }
      .team-name { text-align:center; }
    "#;

    // Render sections
    let score_table = score_section_with_rules(tr);
    let faults_table = faults_warnings_section();
    let penalty_table = time_penalty_section_fixed();

    // Category and division/pod
    let cat = derive_category(&game.timing_rule);
    // Per request: always display Division Long name (fallback to Division Short), even if there is a pod
    let divpod = if let Some((div_short, div_long, _pod_short)) =
        div_pod_for_game(csv_schedule, game_number)
    {
        if !div_long.is_empty() {
            div_long
        } else {
            div_short
        }
    } else {
        String::new()
    };

    // Logos inside squares
    let left_square = if let Some(src) = left_logo {
        format!("<img class='logo-in-square' src=\"{}\"/>", html_escape(src))
    } else {
        "SANCTIONING<br/>BODY LOGO".to_string()
    };
    let right_square = if let Some(src) = right_logo {
        format!("<img class='logo-in-square' src=\"{}\"/>", html_escape(src))
    } else {
        "TOURNAMENT<br/>LOGO".to_string()
    };

    // Combined side blocks
    let side_white = format!(
        "<div class='block'>\
           <div class='team-row'><div class='team-label'>WHITE TEAM<span class='suffix'>{}</span></div><div class='team-name'>{}</div></div>\
           {}{}{}\
         </div>",
        html_escape(white_suffix),
        html_escape(white_name),
        faults_table,
        penalty_table,
        score_table
    );
    let side_black = format!(
        "<div class='block'>\
           <div class='team-row'><div class='team-label'>BLACK TEAM<span class='suffix'>{}</span></div><div class='team-name'>{}</div></div>\
           {}{}{}\
         </div>",
        html_escape(black_suffix),
        html_escape(black_name),
        faults_table,
        penalty_table,
        score_table
    );

    format!(
        r#"<!doctype html><html><head><meta charset='utf-8'/>
  <title>Scoresheet G{game_number}</title>
  <style>{css}</style>
</head><body>
<div class='page'><div class='inner'>
  <div class='top-pad'></div>
  <div class='blue'>
    <div class='logo-square'>{left_square}</div>
    <div class='center'>
      <div class='tname'>{event_name}</div>
      <div class='fields meta'>
        <div class='field'><div class='lbl'>GAME #:</div><div class='val'>{game_number}</div></div>
        <div class='field'><div class='lbl'>DIV. / POD:</div><div class='val'>{divpod}</div></div>
        <div class='field'><div class='lbl'>CATEGORY:</div><div class='val'>{category}</div></div>
        <div class='field'><div class='lbl'>DATE:</div><div class='val'>{date}</div></div>
        <div class='field'><div class='lbl'>SCHEDULED<br>START TIME:</div><div class='val'>{time}</div></div>
        <div class='field'><div class='lbl'>ACTUAL<br>START TIME:</div><div class='val'></div></div>
      </div>
      <div class='fields off1'>
        <div class='field'><div class='lbl'>CHIEF REF:</div><div class='val'>{chief}</div></div>
        <div class='field'><div class='lbl'>T/S KEEPER:</div><div class='val'>{ts_keeper}</div></div>
        <div class='field'><div class='lbl'>T/S HELPER:</div><div class='val'>{ts_helper}</div></div>
      </div>
      <div class='fields off2'>
        <div class='field'><div class='lbl'>WATER REF 1:</div><div class='val'>{water1}</div></div>
        <div class='field'><div class='lbl'>WATER REF 2:</div><div class='val'>{water2}</div></div>
        <div class='field'><div class='lbl'>WATER REF 3:</div><div class='val'>{water3}</div></div>
      </div>
    </div>
    <div class='logo-square'>{right_square}</div>
  </div>
  <div class='sides'>
    <div class='side white'>{side_white}</div>
    <div class='side black'>{side_black}</div>
  </div>
  <div class='sigline'>
    <div class='line'>WHITE CAPTAIN SIGNATURE</div>
    <div class='line'>CHIEF REFEREE SIGNATURE</div>
    <div class='line'>BLACK CAPTAIN SIGNATURE</div>
  </div>
</div></div>
</body></html>
        "#,
        css = css,
        event_name = html_escape(&event.name),
        game_number = html_escape(game_number),
        divpod = html_escape(&divpod),
        category = html_escape(cat),
        date = html_escape(&date_str),
        time = html_escape(&time_str),
        chief = html_escape(&officials.chief),
        water1 = html_escape(&officials.water1),
        water2 = html_escape(&officials.water2),
        water3 = html_escape(&officials.water3),
        ts_keeper = html_escape(&officials.ts_keeper),
        ts_helper = html_escape(&officials.ts_helper),
        left_square = left_square,
        right_square = right_square,
        side_white = side_white,
        side_black = side_black,
    )
}

// Simple scoresheet layout (portrait) based on provided mockup
#[allow(clippy::too_many_arguments)]
fn render_html_simple(
    event: &Event,
    game_number: &str,
    game: &Game,
    _csv_schedule: Option<&uwh_common::uwhportal::schedule::Schedule>,
    tr: &uwh_common::uwhportal::schedule::TimingRule,
    white_suffix: &str,
    white_name: &str,
    black_suffix: &str,
    black_name: &str,
    officials: &OfficialNames,
) -> String {
    // Date/time formatting in event timezone
    let offset = event.date_range.start.offset();
    let local_dt: OffsetDateTime = game.start_time.to_offset(offset);
    const DATE_FMT: &[FormatItem<'static>] =
        format_description!("[weekday repr:short] [month repr:short] [day padding:none]");
    const TIME_FMT: &[FormatItem<'static>] =
        format_description!("[hour repr:12]:[minute] [period case:upper]");
    let date_str = local_dt.format(&DATE_FMT).unwrap_or_default();
    let white_label = if white_suffix.is_empty() {
        white_name.to_string()
    } else {
        format!("{} {}", white_name, white_suffix)
    };
    let black_label = if black_suffix.is_empty() {
        black_name.to_string()
    } else {
        format!("{} {}", black_name, black_suffix)
    };

    let time_str = local_dt.format(&TIME_FMT).unwrap_or_default();

    // Category and division/pod
    let cat = derive_category(&game.timing_rule);

    let css = r#"
      * { box-sizing:border-box; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
      body { font-family: Arial, Helvetica, sans-serif; margin:18px; color:#000; }
      .page { width: 8.5in; height: 11in; margin: 0 auto; border:1px solid #cfcfcf; padding:0.5in 0.75in; background:#fff; }
      .page { break-after: page; page-break-after: always; }
      @media print { @page { size: 8.5in 11in; margin: 0; } html, body { width:8.5in; height:11in; margin:0; } .page { padding:0.5in 0.75in; } }
      h1 { text-align:center; font-size:36px; margin:0 0 10px 0; }
      .tournament { text-align:center; font-size:22px; margin-bottom:24px; }
      .grid { display:grid; grid-template-columns: 0.6fr 0.6fr 1.4fr 1.4fr; gap:10px; margin-bottom:14px; }
      .field { display:grid; grid-template-columns:auto 1fr; gap:6px; align-items:center; }
      .lbl { font-size:12px; white-space:nowrap; }
      .val { border-bottom:1px solid #888; height:22px; text-align:center; }
      .vals3 { display:grid; grid-template-columns:1fr 1fr 1fr; gap:8px; }
      .vals3 .val { border-bottom:1px solid #888; height:22px; text-align:center; }
      .row { display:grid; grid-template-columns: 1fr 1fr; gap:10px; margin-bottom:10px; }
      .row .water-refs { grid-column: 1 / -1; }

      /* Single sheet table */
      table.sheet { width:100%; border-collapse:collapse; margin-top:6px; table-layout: fixed; }
      table.sheet th, table.sheet td { border:1px solid #888; height:36px; text-align:center; }
      table.sheet th.black, table.sheet td.black { background:#d3d3d3; }
      table.sheet td.label { font-weight:bold; background:none; }
      table.sheet td.label.black { background:#d3d3d3; }
      table.sheet td.sb { height:48px; }

      /* Disabled cells (X) */
      /* Spacing between split tables */
      table.sheet + table.sheet { margin-top:10px; }

      td.speckled { position: relative; background:transparent; }
      td.speckled::before { content: "X"; position: absolute; left: 50%; top: 50%; transform: translate(-50%, -52%); font-size: 12px; line-height: 1; color: #9b9b9b; font-weight: 600; pointer-events: none; }

      /* Penalties body rows: keep verticals, remove horizontals */
      table.sheet td.pen { border-top:none; border-bottom:none; height:28px; }
      table.sheet tr:last-child td.pen { border-bottom:1px solid #888; }
      table.sheet tr.sigrow td { border-top:none !important; height:48px; }

      /* Two-up split cells (Time | Number) */
      .split2 { display:grid; grid-template-columns:1fr 1fr; height:100%; }
      .split2 > div { border-right:1px solid #888; display:flex; align-items:center; justify-content:center; height:100%; }
      .split2 > div:last-child { border-right:none; }

      .sig { border-bottom:1px solid #888; height:14px; text-align:left; padding-top:0; }
      .bottom { margin-top:28px; display:grid; grid-template-columns:auto 1fr; align-items:end; column-gap:10px; }
      .bottom .lbl { font-size:12px; white-space:nowrap; line-height:14px; }
      .sig.short { width:40%; margin-top:0; margin-left:0; justify-self:start; }

      /* Bold center box for Final Score */
      table.sheet td.final-center { border:2px solid #000 !important; }




    "#;

    // Officials combined timer/scorer text
    let timer_scorer = if officials.ts_keeper.is_empty() && officials.ts_helper.is_empty() {
        String::new()
    } else if officials.ts_helper.is_empty() {
        officials.ts_keeper.clone()
    } else if officials.ts_keeper.is_empty() {
        officials.ts_helper.clone()
    } else {
        format!("{} / {}", officials.ts_keeper, officials.ts_helper)
    };

    // Timeouts boxes (smart like Detailed): 0 => X, 1 combined => single 'of 1', 1 per half => two 'of 1'
    let (to_cells_white, to_cells_black) = if tr.team_timeout_count == 0 {
        (
            "<td class='box speckled'></td><td class='box speckled'></td>".to_string(),
            "<td class='box speckled black'></td><td class='box speckled black'></td>".to_string(),
        )
    } else if tr.team_timeout_count == 1 && !tr.team_timeouts_counted_per_half {
        (
            "<td class='box' colspan='2'>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;of 1</td>".to_string(),
            "<td class='box black' colspan='2'>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;of 1</td>".to_string(),
        )
    } else if tr.team_timeout_count == 1 && tr.team_timeouts_counted_per_half {
        (
            "<td class='box'>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;of 1</td><td class='box'>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;of 1</td>".to_string(),
            "<td class='box black'>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;of 1</td><td class='box black'>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;of 1</td>".to_string(),
        )
    } else {
        (
            "<td class='box'></td><td class='box'></td>".to_string(),
            "<td class='box black'></td><td class='box black'></td>".to_string(),
        )
    };

    let html = format!(
        r#"<!doctype html><html><head><meta charset='utf-8'/>
  <title>Scoresheet G{game_number}</title>

  <style>{css}</style>
</head><body>
<div class='page'><div class='inner'>
  <h1>Underwater Hockey Score Sheet</h1>
  <div class='tournament'>{event_name}</div>
  <div class='grid'>
    <div class='field'><div class='lbl'>Game #:</div><div class='val'>{game_number}</div></div>
    <div class='field'><div class='lbl'>Court #:</div><div class='val'>{pool}</div></div>
    <div class='field'><div class='lbl'>Day/Time:</div><div class='val'>{date} {time}</div></div>
    <div class='field'><div class='lbl'>Game Type:</div><div class='val'>{category}</div></div>
  </div>
  <div class='row'>
    <div class='field water-refs'><div class='lbl'>Water Refs:</div><div class='vals3'><div class='val'>{wr1}</div><div class='val'>{wr2}</div><div class='val'>{wr3}</div></div></div>
  </div>
  <div class='row'>
    <div class='field'><div class='lbl'>Chief Ref:</div><div class='val'>{chief}</div></div>
    <div class='field'><div class='lbl'>Timer/Scorer:</div><div class='val'>{ts}</div></div>
  </div>

  <table class='sheet'>
    <colgroup>
      <col style='width:25%'/>
      <col style='width:25%'/>
      <col style='width:25%'/>
      <col style='width:25%'/>
    </colgroup>
    <tr>
      <th colspan='2'>White Team: {white_team}</th>
      <th class='black' colspan='2'>Black Team: {black_team}</th>
    </tr>
    <tr><td class='label' colspan='4'>Timed Penalties</td></tr>
    <tr>
      <th class='subhdr'><div class='split2'><div>Time</div><div>Number</div></div></th>
      <th>Infraction</th>
      <th class='black subhdr'><div class='split2'><div>Time</div><div>Number</div></div></th>
      <th class='black'>Infraction</th>
    </tr>
    {penalty_rows}
  </table>
  <table class='sheet'>
    <colgroup>
      <col style='width:25%'/>
      <col style='width:25%'/>
      <col style='width:25%'/>
      <col style='width:25%'/>
    </colgroup>
    <tr>
      <td class='label'>1st Half</td><td class='label'>2nd Half</td>
      <td class='label black'>1st Half</td><td class='label black'>2nd Half</td>
    </tr>
    <tr><td class='label' colspan='4'>Timeouts</td></tr>
    <tr>{to_w}{to_b}</tr>
    <tr><td class='label' colspan='4'>Scores Each Half</td></tr>
    <tr>
      <td class='box sb'></td><td class='box sb'></td>
      <td class='box black sb'></td><td class='box black sb'></td>
    </tr>
    <tr>
      <td class='label'>Final Score</td>
      <td class='final-center'></td>
      <td class='final-center black'></td>
      <td class='label black'>Final Score</td>
    </tr>
    <tr><td class='label' colspan='4'>Captain's Signatures</td></tr>
    <tr class='sigrow'><td colspan='2'></td><td class='black' colspan='2'></td></tr>
  </table>
  <div class='bottom'><div class='lbl'>Chief Ref Signature:</div><div class='sig short'></div></div></div>
</div>
</body></html>
        "#,
        css = css,
        event_name = html_escape(&event.name),
        game_number = html_escape(game_number),
        pool = html_escape(&game.court),
        date = html_escape(&date_str),
        time = html_escape(&time_str),
        category = html_escape(cat),
        white_team = html_escape(&white_label),
        black_team = html_escape(&black_label),
        wr1 = html_escape(&officials.water1),
        wr2 = html_escape(&officials.water2),
        wr3 = html_escape(&officials.water3),
        chief = html_escape(&officials.chief),
        ts = html_escape(&timer_scorer),
        to_w = to_cells_white,
        to_b = to_cells_black,
        penalty_rows = {
            // Produce a fixed number of blank rows
            let mut rows = String::new();
            for _ in 0..9 {
                rows.push_str("<tr><td class='pen'><div class='split2'><div></div><div></div></div></td><td class='pen'></td><td class='pen black'><div class='split2'><div></div><div></div></div></td><td class='pen black'></td></tr>");
            }
            rows
        },
    );

    html
}

fn empty_cells(n: usize) -> String {
    let mut s = String::new();
    for _ in 0..n {
        s.push_str("<td></td>");
    }
    s
}

fn speckled_cells(n: usize) -> String {
    let mut s = String::new();
    for _ in 0..n {
        s.push_str("<td class='speckled'></td>");
    }
    s
}

fn score_section_with_rules(tr: &uwh_common::uwhportal::schedule::TimingRule) -> String {
    let mut cols = String::new();
    for i in 1..=14 {
        cols.push_str(&format!("<th>{}</th>", i));
    }

    let blank = empty_cells(14);
    let blank_speckled = speckled_cells(14);

    // Team timeouts for 1st/2nd half
    let (to_first, to_second) = if tr.team_timeout_count == 0 {
        (
            "<td class='speckled'></td>".to_string(),
            "<td class='speckled'></td>".to_string(),
        )
    } else if tr.team_timeout_count == 1 && !tr.team_timeouts_counted_per_half {
        (
            "<td rowspan='2'>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;of 1</td>".to_string(),
            String::new(),
        )
    } else if tr.team_timeout_count == 1 && tr.team_timeouts_counted_per_half {
        (
            "<td>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;of 1</td>".to_string(),
            "<td>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;of 1</td>".to_string(),
        )
    } else {
        ("<td></td>".to_string(), "<td></td>".to_string())
    };

    // OT rows
    let (ot_cells, ot_pgt_attr, ot_sub_attr) = if !tr.overtime_allowed {
        (
            blank_speckled.as_str(),
            " class='speckled'",
            " class='speckled'",
        )
    } else {
        (blank.as_str(), "", "")
    };

    // Gold goal row

    // Gold goal row: col 1 separate; cols 2–14 merged (colspan=13) and always speckled
    let gg_col1 = if !tr.sudden_death_allowed {
        "<td class='speckled'></td>"
    } else {
        "<td></td>"
    }
    .to_string();
    let gg_merged = speckled_cells(13);
    let gg_nums = format!("{}{}", gg_col1, gg_merged);
    let (gg_pgt, gg_sub) = if !tr.sudden_death_allowed {
        (
            "<td class='speckled'></td>".to_string(),
            "<td class='speckled'></td>".to_string(),
        )
    } else {
        ("<td></td>".to_string(), "<td></td>".to_string())
    };

    format!(
        "<table class='sheet score'>\
           <colgroup><col class='col-period'/><col class='col-score' span='14'/><col class='col-pgt'/><col class='col-sub'/><col class='col-timeouts'/></colgroup>\
           <tr class='cap-row'><th class='tl period' rowspan='2'>TIME<br>PERIOD</th><th colspan='14'>SCORE COUNT</th><th class='pgt' rowspan='2'>PENALTY<br>GOAL&nbsp;TALLY</th><th rowspan='2'>SUB-TOTAL</th><th rowspan='2'>TEAM TIMEOUTS</th></tr>\
           <tr class='score-numbers'>{}</tr>\
           <tr class='sep-top'><td class='tl period'>1ST HALF</td>{}<td></td><td></td>{}</tr>\
           <tr><td class='tl period'>2ND HALF</td>{}<td></td><td></td>{}</tr>\
           <tr class='sep-top'><td class='tl period'>OT 1ST HALF</td>{}<td{}></td><td{}></td><td class='speckled'></td></tr>\
           <tr><td class='tl period'>OT 2ND HALF</td>{}<td{}></td><td{}></td><td class='speckled'></td></tr>\
           <tr class='sep-top gold-goal'><td class='tl period'>GOLD GOAL</td>{}{}{}<td class='speckled'></td></tr>\
           <tr class='notes-final'><td class='tl period'>NOTES</td><td colspan='11'></td><td class='final-score' colspan='6'>FINAL SCORE</td></tr>\
         </table>",
        cols,
        blank,
        to_first,
        blank,
        to_second,
        ot_cells,
        ot_pgt_attr,
        ot_sub_attr,
        ot_cells,
        ot_pgt_attr,
        ot_sub_attr,
        gg_nums,
        gg_pgt,
        gg_sub
    )
}

pub fn generate_example_rule_sheets(output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_dir)?;

    // Minimal example event and times
    let start = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
    let event = Event {
        id: EventId::from_partial("EXAMPLE"),
        name: "Example Event".to_string(),
        slug: "example-event".to_string(),
        date_range: DateRange {
            start,
            end: start + TimeDur::hours(8),
        },
        teams: None,
        schedule: None,
        courts: None,
    };

    // Helper to make a timing rule with the flags we want
    let make_rule =
        |name: &str, to_count: u16, per_half: bool, ot_allowed: bool, sd_allowed: bool| {
            TimingRule {
                name: name.to_string(),
                team_timeout_count: to_count,
                team_timeouts_counted_per_half: per_half,
                overtime_allowed: ot_allowed,
                sudden_death_allowed: sd_allowed,
                half_play_duration: std::time::Duration::from_secs(15 * 60),
                half_time_duration: std::time::Duration::from_secs(3 * 60),
                team_timeout_duration: std::time::Duration::from_secs(60),
                ot_half_play_duration: std::time::Duration::from_secs(5 * 60),
                ot_half_time_duration: std::time::Duration::from_secs(60),
                pre_overtime_break: std::time::Duration::from_secs(2 * 60),
                pre_sudden_death_duration: std::time::Duration::from_secs(60),
                minimum_break: std::time::Duration::from_secs(5 * 60),
            }
        };

    let examples: Vec<(&str, TimingRule)> = vec![
        (
            "ot_disallowed",
            make_rule("RR - OT OFF", 1, false, false, true),
        ),
        (
            "sudden_death_disallowed",
            make_rule("RR - SD OFF", 1, false, true, false),
        ),
        (
            "timeouts_none",
            make_rule("RR - TO 0", 0, false, true, true),
        ),
        (
            "timeouts_one_combined",
            make_rule("RR - TO 1 combined", 1, false, true, true),
        ),
        (
            "timeouts_one_per_half",
            make_rule("RR - TO 1 per half", 1, true, true, true),
        ),
        (
            "all_allowed",
            make_rule("RR - All Allowed", 1, true, true, true),
        ),
    ];

    for (i, (slug, tr)) in examples.iter().enumerate() {
        let game = Game {
            number: format!("EX{}", i + 1),
            dark: ScheduledTeam::new_pending_assignment_name("Example Black"),
            light: ScheduledTeam::new_pending_assignment_name("Example White"),
            start_time: start,
            court: "1".to_string(),
            timing_rule: tr.name.clone(),
            referee_assignments: None,
            description: Some(format!("Example: {}", slug.replace('_', " "))),
        };
        let officials = OfficialNames::default();
        let html = render_html(
            &event,
            &game.number,
            &game,
            None,
            tr,
            "",
            "Example White",
            "",
            "Example Black",
            &officials,
            None,
            None,
        );
        let path = output_dir.join(format!("scoresheet_example_{}.html", slug));
        fs::write(&path, html)?;
    }

    Ok(())
}

fn faults_warnings_section() -> String {
    let mut counts12 = String::new();
    for i in 1..=12 {
        counts12.push_str(&format!("<th>{}</th>", i));
    }
    let mut counts8 = String::new();
    for i in 1..=8 {
        if i == 1 {
            counts8.push_str(&format!("<th class='sep-left'>{}</th>", i));
        } else {
            counts8.push_str(&format!("<th>{}</th>", i));
        }
    }
    let items = [
        "STICK INFRINGEMENT",
        "ILLEGAL ADVANCEMENT",
        "OBSTR. / SCREEN / BARG.",
        "ILLEGALLY STOP",
        "FREE ARM",
        "FALSE START / BREAKING",
        "GRABBING BARRIER",
        "ILLEGAL SUBSTITUTION",
        "OUT OF BOUNDS",
        "DELAY OF GAME",
        "UNSPORTING CONDUCT",
    ];
    let mut rows = String::new();
    for it in items {
        let mut warn = String::new();
        for j in 0..8 {
            if j == 0 {
                warn.push_str("<td class='sep-left'></td>");
            } else {
                warn.push_str("<td></td>");
            }
        }
        rows.push_str(&format!(
            "<tr><td class='tl'>{}</td>{}{}</tr>",
            html_escape(it),
            empty_cells(12),
            warn
        ));
    }
    format!(
        "<table class='sheet faults'>\
           <colgroup><col class='col-fault'/><col class='col-infra' span='12'/><col class='col-warn' span='8'/></colgroup>\
           <tr class='cap-row'><th class='tl'>FAULTS AND WARNINGS</th><th colspan='12'>INFRACTION COUNT</th><th class='sep-left' colspan='8'>WARNING COUNT</th></tr>\
           <tr><th class='tl'>FAULT LIST</th>{}{}</tr>\
           {}\
         </table>",
        counts12, counts8, rows
    )
}

fn time_penalty_section_fixed() -> String {
    let mut hdr = String::new();
    for i in 1..=20 {
        hdr.push_str(&format!("<th>{}</th>", i));
    }
    format!(
        "<table class='sheet penalty'>\
           <colgroup><col class='col-label'/><col class='col-pen' span='20'/></colgroup>\
           <tr class='cap-row'><th class='tl'>TIME PENALTY &darr;&nbsp;&nbsp;&nbsp;&nbsp;COUNT &rarr;</th>{}</tr>\
           <tr><td class='tl'>1 min</td>{}</tr>\
           <tr><td class='tl'>2 min</td>{}</tr>\
           <tr><td class='tl'>5 min</td>{}</tr>\
           <tr><td class='tl'>TOTAL DISMISSAL</td>{}{}</tr>\
         </table>",
        hdr,
        empty_cells(20),
        empty_cells(20),
        empty_cells(20),
        empty_cells(10),
        speckled_cells(10)
    )
}

fn html_escape(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect(),
            '>' => "&gt;".chars().collect(),
            '"' => "&quot;".chars().collect(),
            '\'' => "&#39;".chars().collect(),
            _ => vec![c],
        })
        .collect()
}
