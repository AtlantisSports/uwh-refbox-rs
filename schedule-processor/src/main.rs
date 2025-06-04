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
use uwh_common::uwhportal::{UwhPortalClient, schedule::*};

mod csv_parser;
use csv_parser::parse_csv;

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

const APP_NAME: &str = "processor";

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

    info!("Please select a CSV schedule to process in the file dialog.");
    let csv_path = FileDialog::new()
        .add_filter("CSV files", &["csv"])
        .set_title("Select Schedule CSV File")
        .pick_file();

    let csv_path = if let Some(path) = csv_path {
        path
    } else {
        error!("No file selected. Exiting.");
        return Err("No file selected".into());
    };

    let options = vec!["Production", "Development", "Local"];
    let site_choice = Select::new("Select the uwhportal site to connect to:", options)
        .prompt()
        .unwrap_or_else(|_| {
            error!("No site selected. Exiting.");
            std::process::exit(1);
        });

    let site_url = match site_choice {
        "Production" => "https://api.uwhportal.com",
        "Development" => "https://api.dev.uwhportal.com",
        "Local" => "http://localhost:9000",
        _ => unreachable!(),
    };

    info!("Using URL: {}", site_url);
    info!("Fetching event list from uwhportal...");

    let mut portal_client = UwhPortalClient::new(
        site_url,
        None,
        !matches!(site_choice, "Local"),
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

    info!("Reading csv file: {}", csv_path.display());
    let csv = std::fs::read_to_string(&csv_path)?;
    let schedule = parse_csv(&csv, offset, event.id.clone())?;

    let mut success_string = "Successfully parsed schedule. Details:".to_string();
    write!(
        &mut success_string,
        "\n    Number of games: {}",
        schedule.games.len()
    )
    .unwrap();
    write!(
        &mut success_string,
        "\n    Groups: {}",
        schedule.groups.iter().map(|g| &g.name).join(", ")
    )
    .unwrap();
    write!(
        &mut success_string,
        "\n    Timing rules: {}",
        schedule.timing_rules.iter().map(|tr| &tr.name).join(", ")
    )
    .unwrap();
    write!(
        &mut success_string,
        "\n    Courts: {}",
        schedule
            .games
            .iter()
            .map(|(_, g)| &g.court)
            .unique()
            .join(", ")
    )
    .unwrap();
    let unassigned_teams: Vec<_> = schedule
        .games
        .iter()
        .flat_map(|(_, g)| vec![&g.light, &g.dark])
        .filter_map(|t| t.pending().map(|name| name.to_string()))
        .unique()
        .collect();
    write!(
        &mut success_string,
        "\n    Unassigned teams ({}): \"{}\"",
        unassigned_teams.len(),
        unassigned_teams.iter().join("\", \"")
    )
    .unwrap();
    info!("{success_string}");

    info!("Running schedule checks");
    if let Err(e) = run_schedule_checks(&schedule) {
        if args.allow_failures {
            error!("Schedule checks failed: {e}");
        } else {
            return Err(e);
        }
    }

    let sendable_schedule = schedule.clone().into();

    let mut team_map = vec![];

    'outer: loop {
        // TODO: Add options to save and print team map
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum StepChoice {
            MapTeams,
            Upload,
            UploadDisabled,
            SaveSchedule,
            PrintSchedule,
            SaveTeamMap,
            SaveTeamMapDisabled,
            PrintTeamMap,
            PrintTeamMapDisabled,
            Exit,
        }

        impl Display for StepChoice {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    StepChoice::MapTeams => write!(f, "Map Teams"),
                    StepChoice::Upload => write!(f, "Upload Schedule"),
                    StepChoice::UploadDisabled => write!(f, "U̶p̶l̶o̶a̶d̶ ̶S̶c̶h̶e̶d̶u̶l̶e̶"),
                    StepChoice::SaveSchedule => write!(f, "Save Schedule to File"),
                    StepChoice::PrintSchedule => write!(f, "Print Schedule"),
                    StepChoice::SaveTeamMap => write!(f, "Save Team Map to File"),
                    StepChoice::SaveTeamMapDisabled => write!(f, "S̶a̶v̶e̶ ̶T̶e̶a̶m̶ ̶M̶a̶p̶ ̶t̶o̶ ̶F̶i̶l̶e̶"),
                    StepChoice::PrintTeamMap => write!(f, "Print Team Map"),
                    StepChoice::PrintTeamMapDisabled => write!(f, "P̶r̶i̶n̶t̶ ̶T̶e̶a̶m̶ ̶M̶a̶p̶"),
                    StepChoice::Exit => write!(f, "Exit"),
                }
            }
        }

        let choices = if team_map.is_empty() {
            vec![
                StepChoice::MapTeams,
                StepChoice::UploadDisabled,
                StepChoice::SaveSchedule,
                StepChoice::PrintSchedule,
                StepChoice::SaveTeamMapDisabled,
                StepChoice::PrintTeamMapDisabled,
                StepChoice::Exit,
            ]
        } else {
            vec![
                StepChoice::MapTeams,
                StepChoice::Upload,
                StepChoice::SaveSchedule,
                StepChoice::PrintSchedule,
                StepChoice::SaveTeamMap,
                StepChoice::PrintTeamMap,
                StepChoice::Exit,
            ]
        };

        let step_choice = Select::new("What would you like to do next?", choices)
            .prompt()
            .unwrap_or_else(|_| {
                error!("No step selected. Exiting.");
                std::process::exit(1);
            });

        match step_choice {
            StepChoice::MapTeams => {
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
                if !portal_client.has_token() {
                    let email = match Text::new("Enter your uwhportal email:").prompt() {
                        Ok(email) => email,
                        Err(_) => {
                            error!("No email provided. Please try again.");
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
                        .login_with_email_and_password(&email, &password)
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
                let output_path = FileDialog::new()
                    .add_filter("JSON files", &["json"])
                    .set_title("Save Schedule as JSON")
                    .save_file();

                if let Some(path) = output_path {
                    info!("Saving schedule to file: {}", path.display());
                    let output = serde_json::to_string_pretty(&sendable_schedule)?;
                    std::fs::write(path, output)?;
                } else {
                    error!("No file selected for saving. Skipping save.");
                    continue 'outer;
                }
            }
            StepChoice::PrintSchedule => {
                let output = serde_json::to_string_pretty(&sendable_schedule)?;
                println!("{output}");
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
                    error!("No file selected for saving team map. Skipping save.");
                    continue 'outer;
                }
            }
            StepChoice::SaveTeamMapDisabled => {
                error!("Saving team map cannot be performed at this time. Please map teams first.");
            }
            StepChoice::PrintTeamMap => {
                let output = serde_json::to_string_pretty(&sendable_team_map(&team_map))?;
                println!("{}", output);
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
