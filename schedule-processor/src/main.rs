use clap::Parser;
use inquire::{Confirm, Password, PasswordDisplayMode, Select, Text};
use itertools::Itertools;
use log::{LevelFilter, error, info};
use log4rs::{
    append::console::{ConsoleAppender, Target},
    config::{Appender, Config as LogConfig, Logger, Root},
    encode::pattern::PatternEncoder,
};
use prettytable::{Cell, Row, Table};
use rfd::FileDialog;
use std::{
    collections::BTreeMap,
    fmt::{Display, Write},
    vec,
};
use uwh_common::uwhportal::{CoinFlipTeam, SetCoinFlipModel, UwhPortalClient, schedule::*};

mod csv_parser;
use csv_parser::parse_csv;

use schedule_processor::scoresheets::{
    RenderInputs, SheetStyle, generate_example_rule_sheets, generate_scoresheets_for_event,
};

mod schedule_checks;
use schedule_checks::run_schedule_checks;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Allow failures of the schedule checks
    #[clap(long, default_value = "false")]
    allow_failures: bool,

    #[clap(long, short, action(clap::ArgAction::Count))]
    /// Increase the log verbosity
    verbose: u8,
}

const APP_NAME: &str = "schedule_processor";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let log_level = match args.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    #[cfg(not(target_os = "windows"))]
    let console_target = Target::Stderr;
    #[cfg(target_os = "windows")]
    let console_target = Target::Stdout; // Windows apps don't get a stderr handle
    let console = ConsoleAppender::builder()
        .target(console_target)
        .encoder(Box::new(PatternEncoder::new("[{d} {h({l:5})} {M}] {m}{n}")))
        .build();

    // Setup the logging from all locations to use `LevelFilter::Error`
    let root = Root::builder().appender("console");
    let root = root.build(LevelFilter::Error);

    // Setup the top level logging config
    let log_config =
        LogConfig::builder().appender(Appender::builder().build("console", Box::new(console)));

    let log_config = log_config
        .logger(Logger::builder().build(APP_NAME, log_level)) // Setup the logging from this app to use `log_level`
        .logger(Logger::builder().build("uwh_common", log_level))
        .build(root)
        .unwrap();

    log4rs::init_config(log_config).unwrap();
    if args.verbose > 0 {
        log_panics::init();
    }

    // Defer CSV selection and parsing until it is actually needed by an action.
    // First, choose sport/site, connect, and select the event.
    let options = vec!["Underwater Hockey", "Underwater Rugby"];
    let sport_choice = Select::new("Select the sport for the schedule:", options)
        .prompt()
        .unwrap_or_else(|_| {
            error!("No sport selected. Exiting.");
            std::process::exit(1);
        });

    let options = vec!["Production", "Development", "Local", "Mock"];
    let site_choice = Select::new("Select the uwhportal site to connect to:", options)
        .prompt()
        .unwrap_or_else(|_| {
            error!("No site selected. Exiting.");
            std::process::exit(1);
        });

    let site_url = match (site_choice, sport_choice) {
        ("Production", "Underwater Hockey") => "https://api.uwhportal.com",
        ("Production", "Underwater Rugby") => "https://api.uwrportal.com",
        ("Development", "Underwater Hockey") => "https://api.dev.uwhportal.com",
        ("Development", "Underwater Rugby") => "https://api.dev.uwrportal.com",
        ("Local", _) => "http://localhost:9000",
        ("Mock", _) => "http://localhost:5000",
        _ => unreachable!(),
    };

    info!("Using URL: {site_url}");
    info!("Fetching event list from uwhportal...");

    let mut portal_client = UwhPortalClient::new(
        site_url,
        None,
        !matches!(site_choice, "Local" | "Mock"),
        std::time::Duration::from_secs(10),
    )?;

    let events = portal_client.get_event_list(false, false).await?;

    struct SelectableEvent(Event);

    impl Display for SelectableEvent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} (ID: {})", self.0.name, self.0.id.full())
        }
    }

    let options = events.into_iter().map(SelectableEvent).collect::<Vec<_>>();

    let event_choice = Select::new("Select the event to process:", options)
        .prompt()
        .unwrap_or_else(|_| {
            error!("No event selected. Exiting.");
            std::process::exit(1);
        });
    let event = event_choice.0;
    info!("Selected event: {} - {}", event.id.full(), event.name);

    let offset = event.date_range.start.offset();
    info!("Using timezone offset: {offset}");

    // Lazy-loaded CSV schedule state. Actions that need it will trigger loading.
    let mut schedule: Option<Schedule> = None;
    let mut unassigned_teams: Vec<String> = Vec::new();

    let mut team_map = vec![];

    'outer: loop {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum StepChoice {
            LoadSchedule,
            MapTeams,
            Upload,
            UploadDisabled,
            SaveSchedule,
            PrintSchedule,
            DumpScheduleJson,
            ResolveCoinTosses,
            GenerateScoreSheets,
            GenerateExampleSheets,
            SaveTeamMap,
            SaveTeamMapDisabled,
            PrintTeamMap,
            PrintTeamMapDisabled,
            Exit,
        }

        impl Display for StepChoice {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    StepChoice::LoadSchedule => write!(f, "Load Schedule (CSV)"),
                    StepChoice::MapTeams => write!(f, "Map Teams"),
                    StepChoice::Upload => write!(f, "Upload Schedule"),
                    StepChoice::UploadDisabled => write!(f, "U̶p̶l̶o̶a̶d̶ ̶S̶c̶h̶e̶d̶u̶l̶e̶"),
                    StepChoice::SaveSchedule => write!(f, "Save Schedule to File"),
                    StepChoice::PrintSchedule => write!(f, "Print Schedule"),
                    StepChoice::DumpScheduleJson => write!(f, "Dump Schedule JSON"),
                    StepChoice::ResolveCoinTosses => write!(f, "Resolve Coin Tosses"),
                    StepChoice::GenerateScoreSheets => write!(f, "Generate Score Sheets"),
                    StepChoice::GenerateExampleSheets => {
                        write!(f, "Generate Example Sheets (rule options)")
                    }
                    StepChoice::SaveTeamMap => write!(f, "Save Team Map to File"),
                    StepChoice::SaveTeamMapDisabled => {
                        write!(f, "S̶a̶v̶e̶ ̶T̶e̶a̶m̶ ̶M̶a̶p̶ ̶t̶o̶ ̶F̶i̶l̶e̶")
                    }
                    StepChoice::PrintTeamMap => write!(f, "Print Team Map"),
                    StepChoice::PrintTeamMapDisabled => write!(f, "P̶r̶i̶n̶t̶ ̶T̶e̶a̶m̶ ̶M̶a̶p̶"),
                    StepChoice::Exit => write!(f, "Exit"),
                }
            }
        }

        let mut choices = if team_map.is_empty() {
            vec![
                StepChoice::LoadSchedule,
                StepChoice::MapTeams,
                StepChoice::UploadDisabled,
                StepChoice::SaveSchedule,
                StepChoice::PrintSchedule,
                StepChoice::DumpScheduleJson,
                StepChoice::ResolveCoinTosses,
                StepChoice::GenerateScoreSheets,
                StepChoice::SaveTeamMapDisabled,
                StepChoice::PrintTeamMapDisabled,
                StepChoice::Exit,
            ]
        } else {
            vec![
                StepChoice::LoadSchedule,
                StepChoice::MapTeams,
                StepChoice::Upload,
                StepChoice::SaveSchedule,
                StepChoice::PrintSchedule,
                StepChoice::DumpScheduleJson,
                StepChoice::ResolveCoinTosses,
                StepChoice::GenerateScoreSheets,
                StepChoice::SaveTeamMap,
                StepChoice::PrintTeamMap,
                StepChoice::Exit,
            ]
        };

        // Mock environment only: also offer example sheet generation in the menu
        if site_choice == "Mock" {
            let pos = choices
                .iter()
                .position(|c| *c == StepChoice::Exit)
                .unwrap_or(choices.len());
            choices.insert(pos, StepChoice::GenerateExampleSheets);
        }

        let step_choice = Select::new("What would you like to do next?", choices)
            .prompt()
            .unwrap_or_else(|_| {
                error!("No step selected. Exiting.");
                std::process::exit(1);
            });

        match step_choice {
            StepChoice::LoadSchedule => {
                info!("Please select a CSV schedule to process in the file dialog.");
                let Some(path) = FileDialog::new()
                    .add_filter("CSV files", &["csv"])
                    .set_title("Select Schedule CSV File")
                    .pick_file()
                else {
                    error!("No file selected. Please try again.");
                    continue 'outer;
                };
                info!("Reading csv file: {}", path.display());
                let csv = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to read CSV file: {e}");
                        continue 'outer;
                    }
                };
                let parsed = match parse_csv(&csv, offset, event.id.clone()) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to parse CSV file: {e}");
                        continue 'outer;
                    }
                };
                info!("Running schedule checks");
                if let Err(e) = run_schedule_checks(&parsed) {
                    error!("Schedule checks failed: {e}");
                    if !args.allow_failures {
                        continue 'outer;
                    }
                }
                unassigned_teams = parsed
                    .games
                    .iter()
                    .flat_map(|(_, g)| vec![&g.light, &g.dark])
                    .filter_map(|t| t.pending().map(|name| name.to_string()))
                    .unique()
                    .collect();
                schedule = Some(parsed);
                info!(
                    "Schedule loaded: {} games, groups: {}",
                    schedule.as_ref().unwrap().games.len(),
                    schedule
                        .as_ref()
                        .unwrap()
                        .groups
                        .iter()
                        .map(|g| &g.name)
                        .join(", ")
                );
            }
            StepChoice::MapTeams => {
                if schedule.is_none() {
                    error!("No schedule loaded. Please choose 'Load Schedule (CSV)' first.");
                    continue 'outer;
                }

                let event_teams = match portal_client.get_event_teams(&event.id).await {
                    Ok(teams) => teams,
                    Err(e) => {
                        error!("Getting event teams failed. Please try again. Reason: {e}");
                        continue 'outer;
                    }
                };

                let mut event_teams: Vec<EventTeam> = event_teams
                    .into_iter()
                    .map(|(id, name)| EventTeam { id, name })
                    .collect();

                let mut unmapped_teams = unassigned_teams.clone();

                while let Some((event_team, unmapped_name)) =
                    get_best_match(&mut event_teams, &mut unmapped_teams)
                {
                    if unmapped_name == event_team.name {
                        info!("Mapping '{unmapped_name}' to '{}'", event_team.name);
                        team_map.push(MappedTeam {
                            unassigned_name: unmapped_name,
                            event_team,
                        });
                        continue;
                    }

                    let selection = Confirm::new(&format!(
                        "Map unmapped team '{unmapped_name}' to event team '{}' (ID: {})? (y/n)",
                        event_team.name,
                        event_team.id.partial()
                    ))
                    .prompt();

                    match selection {
                        Ok(true) => {
                            info!("Mapping '{unmapped_name}' to '{}'", event_team.name);
                            team_map.push(MappedTeam {
                                unassigned_name: unmapped_name,
                                event_team,
                            });
                        }
                        Ok(false) => {
                            event_teams.push(event_team);
                            match Confirm::new(&format!(
                                "Do you want to map '{unmapped_name}' to another team?",
                            ))
                            .prompt()
                            {
                                Ok(false) => continue,
                                Ok(true) => {
                                    let options = event_teams.clone();
                                    let new_team_choice =
                                        match Select::new("Select a team to map to:", options)
                                            .prompt()
                                        {
                                            Ok(team) => team,
                                            Err(_) => {
                                                error!("No team selected. Please try again.");
                                                continue 'outer;
                                            }
                                        };
                                    info!(
                                        "Mapping '{unmapped_name}' to '{}'",
                                        new_team_choice.name
                                    );
                                    team_map.push(MappedTeam {
                                        unassigned_name: unmapped_name,
                                        event_team: new_team_choice,
                                    });
                                }
                                Err(_) => {
                                    error!(
                                        "Failed to get confirmation for mapping. Please try again."
                                    );
                                    continue 'outer;
                                }
                            }
                        }
                        Err(_) => {
                            error!("Failed to get confirmation for mapping. Please try again.");
                            continue 'outer;
                        }
                    }
                }

                if team_map.is_empty() {
                    error!("No teams were mapped. Please try again.");
                    continue 'outer;
                }

                let mut table = Table::new();
                table.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
                table.set_titles(Row::new(vec![
                    Cell::new("Unassigned Team"),
                    Cell::new("Mapped Team"),
                ]));
                for mapped in &team_map {
                    table.add_row(Row::new(vec![
                        Cell::new(&mapped.unassigned_name),
                        Cell::new(&format!("{}", mapped.event_team)),
                    ]));
                }

                let mut message = "Is the following team mapping correct?\n".to_string();
                if !unmapped_teams.is_empty() {
                    writeln!(
                        &mut message,
                        "WARNING: The following teams were not mapped: {}",
                        unmapped_teams.join(", ")
                    )
                    .unwrap();
                }
                if !event_teams.is_empty() {
                    writeln!(
                        &mut message,
                        "WARNING: The following event teams were not used: {}",
                        event_teams.iter().map(|et| &et.name).join(", ")
                    )
                    .unwrap();
                }
                message.push_str(&table.to_string());
                let confirmation = Confirm::new(&message).with_default(true).prompt();
                match confirmation {
                    Ok(true) => {
                        info!("Team mapping confirmed.");
                    }
                    Ok(false) => {
                        error!("Team mapping was not confirmed. Please try again.");
                        team_map.clear();
                    }
                    Err(_) => {
                        error!("Failed to get confirmation for team mapping. Please try again.");
                        continue 'outer;
                    }
                }
            }
            StepChoice::Upload => {
                // Load schedule if not already done
                if schedule.is_none() {
                    info!("Please select a CSV schedule to process in the file dialog.");
                    let Some(path) = FileDialog::new()
                        .add_filter("CSV files", &["csv"])
                        .set_title("Select Schedule CSV File")
                        .pick_file()
                    else {
                        error!("No file selected. Please try again.");
                        continue 'outer;
                    };
                    info!("Reading csv file: {}", path.display());
                    let csv = match std::fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(e) => {
                            error!("Failed to read CSV file: {e}");
                            continue 'outer;
                        }
                    };
                    let parsed = match parse_csv(&csv, offset, event.id.clone()) {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Failed to parse CSV file: {e}");
                            continue 'outer;
                        }
                    };
                    info!("Running schedule checks");
                    if let Err(e) = run_schedule_checks(&parsed) {
                        error!("Schedule checks failed: {e}");
                        if !args.allow_failures {
                            continue 'outer;
                        }
                    }
                    unassigned_teams = parsed
                        .games
                        .iter()
                        .flat_map(|(_, g)| vec![&g.light, &g.dark])
                        .filter_map(|t| t.pending().map(|name| name.to_string()))
                        .unique()
                        .collect();
                    schedule = Some(parsed);
                }
                let sendable_schedule: SendableSchedule = schedule.as_ref().unwrap().clone().into();

                if !portal_client.has_token() {
                    #[allow(non_snake_case)]
                    let emailOrusername =
                        match Text::new("Enter your uwhportal emailOrusername:").prompt() {
                            Ok(v) => v,
                            Err(_) => {
                                error!("No emailOrusername provided. Please try again.");
                                continue 'outer;
                            }
                        };
                    let password = match Password::new("Enter your uwhportal password:")
                        .with_display_mode(PasswordDisplayMode::Masked)
                        .without_confirmation()
                        .prompt()
                    {
                        Ok(pass) => pass,
                        Err(_) => {
                            error!("No password provided. Please try again.");
                            continue 'outer;
                        }
                    };

                    let token = match portal_client
                        .login_with_email_and_password(&emailOrusername, &password)
                        .await
                    {
                        Ok(token) => token,
                        Err(e) => {
                            error!("uwhportal login failed. Please try again. Reason: {e}");
                            continue 'outer;
                        }
                    };

                    portal_client.set_token(&token);
                }

                let force = match Confirm::new(
                    "Do you want to force upload the schedule? (This will overwrite existing data)",
                )
                .with_default(false)
                .prompt()
                {
                    Ok(confirm) => confirm,
                    Err(_) => {
                        error!("No confirmation provided. Please try again.");
                        continue 'outer;
                    }
                };

                info!("Uploading schedule for event: {}", event.slug);
                match portal_client
                    .push_event_schedule(&event.slug, &sendable_schedule, force)
                    .await
                {
                    Ok(_) => {
                        info!("Schedule uploaded successfully.");
                    }
                    Err(e) => {
                        error!("Failed to upload schedule. Please try again. Reason: {e}");
                        portal_client.clear_token();
                        continue 'outer;
                    }
                }

                info!("Sending team map");
                match portal_client
                    .push_team_map(&event.slug, &sendable_team_map(&team_map))
                    .await
                {
                    Ok(_) => {
                        info!("Team map sent successfully.");
                    }
                    Err(e) => {
                        error!("Failed to send team map. Please try again. Reason: {e}");
                        portal_client.clear_token();
                        continue 'outer;
                    }
                }
            }
            StepChoice::UploadDisabled => {
                error!("Upload cannot be performed at this time. Please map teams first.");
            }
            StepChoice::SaveSchedule => {
                if schedule.is_none() {
                    error!("No schedule loaded. Choose 'Load Schedule (CSV)' first.");
                    continue 'outer;
                }
                let output_path = FileDialog::new()
                    .add_filter("JSON files", &["json"])
                    .set_title("Save Schedule as JSON")
                    .save_file();

                if let Some(path) = output_path {
                    let sendable: SendableSchedule = schedule.as_ref().unwrap().clone().into();
                    info!("Saving schedule to file: {}", path.display());
                    let output = serde_json::to_string_pretty(&sendable)?;
                    std::fs::write(path, output)?;
                } else {
                    error!("No file selected for saving. Skipping save.");
                    continue 'outer;
                }
            }
            StepChoice::PrintSchedule => {
                if schedule.is_none() {
                    error!("No schedule loaded. Choose 'Load Schedule (CSV)' first.");
                    continue 'outer;
                }
                let sendable: SendableSchedule = schedule.as_ref().unwrap().clone().into();
                let output = serde_json::to_string_pretty(&sendable)?;
                println!("{output}");
            }
            StepChoice::DumpScheduleJson => {
                // Fetch the privileged schedule JSON from the portal (requires login).
                if !portal_client.has_token() {
                    #[allow(non_snake_case)]
                    let emailOrusername =
                        match Text::new("Enter your uwhportal emailOrusername:").prompt() {
                            Ok(v) => v,
                            Err(_) => {
                                error!("No emailOrusername provided. Please try again.");
                                continue 'outer;
                            }
                        };
                    let password = match Password::new("Enter your uwhportal password:")
                        .with_display_mode(PasswordDisplayMode::Masked)
                        .without_confirmation()
                        .prompt()
                    {
                        Ok(pass) => pass,
                        Err(_) => {
                            error!("No password provided. Please try again.");
                            continue 'outer;
                        }
                    };
                    match portal_client
                        .login_with_email_and_password(&emailOrusername, &password)
                        .await
                    {
                        Ok(token) => portal_client.set_token(&token),
                        Err(e) => {
                            error!("uwhportal login failed. Please try again. Reason: {e}");
                            continue 'outer;
                        }
                    }
                }

                let body = match portal_client
                    .get_event_schedule_privileged_raw(&event.id)
                    .await
                {
                    Ok(b) => b,
                    Err(e) => {
                        error!("Failed to get privileged schedule JSON: {e}");
                        continue 'outer;
                    }
                };

                let output_path = FileDialog::new()
                    .add_filter("JSON files", &["json"])
                    .set_title("Save event schedule JSON (privileged)")
                    .save_file();

                let Some(path) = output_path else {
                    error!("No file selected for saving. Skipping.");
                    continue 'outer;
                };

                std::fs::write(&path, body)?;
                info!("Saved privileged schedule JSON to {}", path.display());
            }
            StepChoice::ResolveCoinTosses => {
                // Requires login to access privileged coin-flip endpoints.
                if !portal_client.has_token() {
                    #[allow(non_snake_case)]
                    let emailOrusername =
                        match Text::new("Enter your uwhportal emailOrusername:").prompt() {
                            Ok(v) => v,
                            Err(_) => {
                                error!("No emailOrusername provided. Please try again.");
                                continue 'outer;
                            }
                        };
                    let password = match Password::new("Enter your uwhportal password:")
                        .with_display_mode(PasswordDisplayMode::Masked)
                        .without_confirmation()
                        .prompt()
                    {
                        Ok(pass) => pass,
                        Err(_) => {
                            error!("No password provided. Please try again.");
                            continue 'outer;
                        }
                    };
                    match portal_client
                        .login_with_email_and_password(&emailOrusername, &password)
                        .await
                    {
                        Ok(token) => portal_client.set_token(&token),
                        Err(e) => {
                            error!("uwhportal login failed. Please try again. Reason: {e}");
                            continue 'outer;
                        }
                    }
                }

                let team_lookup: BTreeMap<String, String> =
                    match portal_client.get_event_teams(&event.id).await {
                        Ok(teams) => teams
                            .into_iter()
                            .map(|(id, name)| (id.full().to_string(), name))
                            .collect(),
                        Err(_) => BTreeMap::new(),
                    };

                let details = match portal_client.get_coin_flips(&event.slug).await {
                    Ok(d) => d,
                    Err(e) => {
                        error!("Failed to fetch coin tosses: {e}");
                        continue 'outer;
                    }
                };

                #[derive(Clone)]
                struct FlipOption {
                    label: String,
                    group_identifier: Option<String>,
                    coin_flip_identifier: String,
                    teams: Vec<CoinFlipTeam>,
                }
                impl Display for FlipOption {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "{}", self.label)
                    }
                }

                let team_label = |t: &CoinFlipTeam| -> String {
                    if let Some(id) = &t.team_id {
                        team_lookup
                            .get(id)
                            .cloned()
                            .unwrap_or_else(|| format!("Team {id}"))
                    } else if let Some(n) = &t.pending_assignment_name {
                        n.clone()
                    } else {
                        "<unknown>".to_string()
                    }
                };

                let mut options: Vec<FlipOption> = Vec::new();
                for g in details.groups {
                    for flip in g.coin_flips {
                        if flip.result.is_none() {
                            let teams_str = flip.tied_teams.iter().map(team_label).join(" vs ");
                            options.push(FlipOption {
                                label: format!("Group {} — {}", g.name, teams_str),
                                group_identifier: Some(g.identifier.clone()),
                                coin_flip_identifier: flip.identifier.clone(),
                                teams: flip.tied_teams,
                            });
                        }
                    }
                }
                for flip in details.games {
                    if flip.result.is_none() {
                        let teams_str = flip.tied_teams.iter().map(team_label).join(" vs ");
                        options.push(FlipOption {
                            label: format!("Tied Game — {teams_str}"),
                            group_identifier: None,
                            coin_flip_identifier: flip.identifier,
                            teams: flip.tied_teams,
                        });
                    }
                }

                if options.is_empty() {
                    error!("No pending coin tosses for this event.");
                    continue 'outer;
                }

                let selected = match Select::new("Select a coin toss to resolve:", options).prompt()
                {
                    Ok(s) => s,
                    Err(_) => {
                        error!("No selection made. Skipping.");
                        continue 'outer;
                    }
                };

                #[derive(Clone)]
                struct TeamChoice {
                    label: String,
                    ident: String,
                }
                impl Display for TeamChoice {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "{}", self.label)
                    }
                }

                let team_choices: Vec<TeamChoice> = selected
                    .teams
                    .iter()
                    .map(|t| TeamChoice {
                        label: team_label(t),
                        ident: t
                            .team_id
                            .clone()
                            .or(t.pending_assignment_name.clone())
                            .unwrap_or_default(),
                    })
                    .collect();

                if team_choices.is_empty() {
                    error!("No teams found on selected coin toss.");
                    continue 'outer;
                }

                let picked_team = match Select::new(
                    "Which team should win the coin toss?",
                    team_choices,
                )
                .prompt()
                {
                    Ok(t) => t,
                    Err(_) => {
                        error!("No team selected.");
                        continue 'outer;
                    }
                };

                let kind = match Select::new("Outcome kind:", vec!["Favor", "Against"]).prompt() {
                    Ok(k) => k.to_string(),
                    Err(_) => {
                        error!("No outcome kind selected.");
                        continue 'outer;
                    }
                };

                let model = SetCoinFlipModel {
                    group_identifier: selected.group_identifier.clone(),
                    coin_flip_identifier: selected.coin_flip_identifier.clone(),
                    team_id_or_pending_assignment_name: picked_team.ident,
                    kind,
                };

                match portal_client
                    .set_coin_flip_result(&event.slug, &model, false)
                    .await
                {
                    Ok(()) => info!("Coin toss resolved successfully."),
                    Err(e) => error!("Failed to set coin toss: {e}"),
                }
            }
            StepChoice::GenerateScoreSheets => {
                let Some(output_dir) = FileDialog::new()
                    .set_title("Select output folder for score sheets")
                    .pick_folder()
                else {
                    error!("No output folder selected. Aborting.");
                    continue 'outer;
                };

                let path_str = output_dir.to_string_lossy();
                if path_str.contains('\'')
                    || path_str.contains('"')
                    || path_str.contains('&')
                    || path_str.contains('!')
                {
                    log::warn!(
                        "Output folder path contains special characters which may prevent PDF \
                        generation. Consider a path without special characters."
                    );
                }

                let style = match Select::new(
                    "Sheet style:",
                    vec!["Detailed", "Simple", "Simple (Team Refs)", "Col_3x3"],
                )
                .prompt()
                {
                    Ok("Simple") => SheetStyle::Simple,
                    Ok("Simple (Team Refs)") => SheetStyle::SimpleTeamRefs,
                    Ok("Col_3x3") => SheetStyle::Col3x3,
                    Ok(_) => SheetStyle::Detailed,
                    Err(_) => SheetStyle::Detailed,
                };

                let (left_logo, right_logo) = if style == SheetStyle::Detailed {
                    let left = loop {
                        let p = FileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "svg"])
                            .set_title("Select sanctioning body logo (left) — optional")
                            .pick_file();
                        match p {
                            Some(p) if p.is_file() => break Some(p),
                            Some(p) if p.is_dir() => {
                                error!("Selected a folder. Please select an image file.");
                                continue;
                            }
                            _ => break None,
                        }
                    };
                    let right = loop {
                        let p = FileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "svg"])
                            .set_title("Select tournament logo (right) — optional")
                            .pick_file();
                        match p {
                            Some(p) if p.is_file() => break Some(p),
                            Some(p) if p.is_dir() => {
                                error!("Selected a folder. Please select an image file.");
                                continue;
                            }
                            _ => break None,
                        }
                    };
                    (left, right)
                } else {
                    (None, None)
                };

                let (ref_csv_path, prefer_portal_officials) = if style == SheetStyle::SimpleTeamRefs
                {
                    (None, false)
                } else {
                    let include_referees = Confirm::new("Include referee names on the scoresheet?")
                        .with_default(true)
                        .prompt()
                        .unwrap_or(true);

                    let csv_path = if include_referees {
                        FileDialog::new()
                            .add_filter("CSV files", &["csv"])
                            .set_title("Select Referee Schedule CSV (optional)")
                            .pick_file()
                    } else {
                        None
                    };

                    let prefer_officials = if csv_path.is_some() {
                        Confirm::new(
                            "Use portal display names instead of names from the Referee CSV?",
                        )
                        .with_default(true)
                        .prompt()
                        .unwrap_or(true)
                    } else {
                        false
                    };

                    (csv_path, prefer_officials)
                };

                let inputs = RenderInputs {
                    left_logo,
                    right_logo,
                    output_dir,
                    style,
                    prefer_portal_officials,
                };

                if !portal_client.has_token() && style != SheetStyle::SimpleTeamRefs {
                    if let Ok(true) =
                        Confirm::new("Use display names for officials? (requires uwhportal login)")
                            .with_default(true)
                            .prompt()
                    {
                        #[allow(non_snake_case)]
                        let emailOrusername =
                            match Text::new("Enter your uwhportal emailOrusername:").prompt() {
                                Ok(v) => v,
                                Err(_) => {
                                    error!(
                                        "No emailOrusername provided. Proceeding without login."
                                    );
                                    String::new()
                                }
                            };
                        if !emailOrusername.is_empty() {
                            let password = Password::new("Enter your uwhportal password:")
                                .with_display_mode(PasswordDisplayMode::Masked)
                                .without_confirmation()
                                .prompt()
                                .unwrap_or_default();
                            if !password.is_empty() {
                                match portal_client
                                    .login_with_email_and_password(&emailOrusername, &password)
                                    .await
                                {
                                    Ok(token) => portal_client.set_token(&token),
                                    Err(e) => error!(
                                        "uwhportal login failed. Proceeding without login. \
                                        Reason: {e}"
                                    ),
                                }
                            }
                        }
                    }
                }

                let csv_schedule_opt = schedule.as_ref();
                match generate_scoresheets_for_event(
                    &mut portal_client,
                    &event,
                    inputs.clone(),
                    csv_schedule_opt,
                    ref_csv_path.as_deref(),
                    None,
                )
                .await
                {
                    Ok(()) => info!("Score sheets generated."),
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("authentication required for schedule") {
                            info!("Public schedule not available; prompting for login...");
                            #[allow(non_snake_case)]
                            let emailOrusername =
                                match Text::new("Enter your uwhportal emailOrusername:").prompt() {
                                    Ok(v) => v,
                                    Err(_) => {
                                        error!("No emailOrusername provided.");
                                        continue 'outer;
                                    }
                                };
                            let password = match Password::new("Enter your uwhportal password:")
                                .with_display_mode(PasswordDisplayMode::Masked)
                                .without_confirmation()
                                .prompt()
                            {
                                Ok(pass) => pass,
                                Err(_) => {
                                    error!("No password provided.");
                                    continue 'outer;
                                }
                            };
                            match portal_client
                                .login_with_email_and_password(&emailOrusername, &password)
                                .await
                            {
                                Ok(token) => portal_client.set_token(&token),
                                Err(e) => {
                                    error!("Login failed: {e}");
                                    continue 'outer;
                                }
                            }
                            let csv_schedule_opt = schedule.as_ref();
                            match generate_scoresheets_for_event(
                                &mut portal_client,
                                &event,
                                inputs,
                                csv_schedule_opt,
                                ref_csv_path.as_deref(),
                                None,
                            )
                            .await
                            {
                                Ok(()) => info!("Score sheets generated."),
                                Err(e2) => error!("Failed to generate score sheets: {e2}"),
                            }
                        } else {
                            error!("Failed to generate score sheets: {e}");
                        }
                    }
                }
            }
            StepChoice::GenerateExampleSheets => {
                let Some(output_dir) = FileDialog::new()
                    .set_title("Select output folder for example sheets")
                    .pick_folder()
                else {
                    error!("No output folder selected. Aborting.");
                    continue 'outer;
                };

                let style = match Select::new("Sheet style:", vec!["Detailed", "Simple", "Col_3x3"])
                    .prompt()
                {
                    Ok("Simple") => SheetStyle::Simple,
                    Ok("Col_3x3") => SheetStyle::Col3x3,
                    Ok(_) => SheetStyle::Detailed,
                    Err(_) => SheetStyle::Detailed,
                };

                match generate_example_rule_sheets(&output_dir, style) {
                    Ok(()) => info!("Example sheets generated in {}", output_dir.display()),
                    Err(e) => error!("Failed to generate example sheets: {e}"),
                }
            }
            StepChoice::SaveTeamMap => {
                let output_path = FileDialog::new()
                    .add_filter("JSON files", &["json"])
                    .set_title("Save Team Map as JSON")
                    .save_file();

                if let Some(path) = output_path {
                    info!("Saving team map to file: {}", path.display());
                    let output = serde_json::to_string_pretty(&sendable_team_map(&team_map))?;
                    std::fs::write(path, output)?;
                } else {
                    error!("No file selected for saving team map. Skipping.");
                    continue 'outer;
                }
            }
            StepChoice::SaveTeamMapDisabled => {
                error!("Saving team map cannot be performed at this time. Please map teams first.");
            }
            StepChoice::PrintTeamMap => {
                let output = serde_json::to_string_pretty(&sendable_team_map(&team_map))?;
                println!("{output}");
            }
            StepChoice::PrintTeamMapDisabled => {
                error!(
                    "Printing team map cannot be performed at this time. Please map teams first."
                );
            }
            StepChoice::Exit => {
                info!("Exiting the application.");
                return Ok(());
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EventTeam {
    id: TeamId,
    name: String,
}

impl Display for EventTeam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (ID: {})", self.name, self.id.partial())
    }
}

struct MappedTeam {
    unassigned_name: String,
    event_team: EventTeam,
}

fn get_best_match(
    event_teams: &mut Vec<EventTeam>,
    unmapped_teams: &mut Vec<String>,
) -> Option<(EventTeam, String)> {
    let mut best_match: Option<(usize, usize)> = None;
    let mut best_score = 0.0;

    for (i, event_team) in event_teams.iter().enumerate() {
        for (j, unmapped_name) in unmapped_teams.iter().enumerate() {
            let score = strsim::normalized_levenshtein(&event_team.name, unmapped_name);
            if best_match.is_none() || score > best_score {
                best_match = Some((i, j));
                best_score = score;
            }
        }
    }

    if let Some((i, j)) = best_match {
        let team = event_teams.remove(i);
        let unmapped_name = unmapped_teams.remove(j);
        Some((team, unmapped_name))
    } else {
        None
    }
}

fn sendable_team_map(team_map: &[MappedTeam]) -> BTreeMap<&str, &str> {
    team_map
        .iter()
        .map(|team| (team.unassigned_name.as_str(), team.event_team.id.full()))
        .collect()
}
