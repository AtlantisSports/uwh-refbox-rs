//! Reads an event's schedule from uwhportal and turns it into the list of videos and playlists
//! we want on YouTube.

use crate::BoxError;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use time::{Date, OffsetDateTime};
use uwh_common::uwhportal::schedule::FORMAT;

/// YouTube rejects titles longer than this.
const MAX_TITLE_CHARS: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedGame {
    pub number: String,
    pub court: String,
    /// 1-based tournament day, counted over the dates that have games.
    pub day: usize,
    pub start: OffsetDateTime,
    pub dark: String,
    pub light: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventPlan {
    pub event_name: String,
    /// Games sorted by start time, then game number.
    pub games: Vec<PlannedGame>,
}

impl EventPlan {
    /// Games grouped by (day, court), each group in start-time order.
    pub fn playlists(&self) -> BTreeMap<(usize, String), Vec<&PlannedGame>> {
        let mut lists: BTreeMap<(usize, String), Vec<&PlannedGame>> = BTreeMap::new();
        for game in &self.games {
            lists
                .entry((game.day, game.court.clone()))
                .or_default()
                .push(game);
        }
        lists
    }

    pub fn game(&self, number: &str) -> Option<&PlannedGame> {
        self.games.iter().find(|g| g.number == number)
    }

    /// One court's games on one schedule day, in schedule order.
    pub fn court_games<'a>(
        &'a self,
        court: &str,
        day: usize,
    ) -> impl Iterator<Item = &'a PlannedGame> {
        self.games
            .iter()
            .filter(move |g| g.court == court && g.day == day)
    }
}

pub fn playlist_title(day: usize, court: &str) -> String {
    format!("Day {day} · Court {court}")
}

pub fn video_title(event_name: &str, game: &PlannedGame) -> String {
    let without_event = format!(
        "Court {} · Game {} · {} vs {}",
        game.court, game.number, game.dark, game.light
    );
    let full = format!("{event_name} · {without_event}");
    let title = if full.chars().count() <= MAX_TITLE_CHARS {
        full
    } else if without_event.chars().count() <= MAX_TITLE_CHARS {
        without_event
    } else {
        let mut cut: String = without_event.chars().take(MAX_TITLE_CHARS - 1).collect();
        cut.push('…');
        cut
    };
    // YouTube does not allow angle brackets in titles.
    title.replace(['<', '>'], "")
}

pub async fn fetch_event_plan(portal_url: &str, event_slug: &str) -> Result<EventPlan, BoxError> {
    let url = format!(
        "{}/api/events/{event_slug}/schedule",
        portal_url.trim_end_matches('/')
    );
    let response = reqwest::get(&url).await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(format!("portal returned {status} for {url}: {body}").into());
    }
    parse_event_plan(&body)
}

pub fn parse_event_plan(json: &str) -> Result<EventPlan, BoxError> {
    let raw: RawSchedule = serde_json::from_str(json)?;

    let mut games = Vec::with_capacity(raw.games.len());
    for game in &raw.games {
        let start = OffsetDateTime::parse(&game.starts_on, &FORMAT).map_err(|e| {
            format!(
                "game {}: bad start time {:?}: {e}",
                game.number, game.starts_on
            )
        })?;
        games.push((game, start));
    }

    // Day numbers come from the dates that actually have games, in each game's own time zone.
    let dates: BTreeSet<Date> = games.iter().map(|(_, start)| start.date()).collect();
    let day_of = |date: Date| dates.iter().position(|d| *d == date).map_or(0, |i| i + 1);

    let mut planned: Vec<PlannedGame> = games
        .into_iter()
        .map(|(game, start)| PlannedGame {
            number: game.number.clone(),
            court: game.court.clone().unwrap_or_default(),
            day: day_of(start.date()),
            start,
            dark: team_label(&game.dark, &raw.teams),
            light: team_label(&game.light, &raw.teams),
            description: game.description.clone(),
        })
        .collect();
    planned.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then_with(|| natural_game_order(&a.number, &b.number))
    });

    Ok(EventPlan {
        event_name: raw.event.name,
        games: planned,
    })
}

/// Orders "2" before "10" when both are plain numbers.
fn natural_game_order(a: &str, b: &str) -> std::cmp::Ordering {
    match (a.parse::<u64>(), b.parse::<u64>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        _ => a.cmp(b),
    }
}

/// The best name we can show for one side of a game: the real team name once it is known,
/// otherwise a placeholder like "Winner G52" or "Women Round Robin #4".
fn team_label(side: &RawSide, teams: &HashMap<String, RawTeam>) -> String {
    let Some(a) = &side.assignment else {
        return "TBD".to_string();
    };
    if let Some(name) = a.team_id.as_ref().and_then(|id| teams.get(id)) {
        return name.name.clone();
    }
    if let Some(name) = a.pending_assignment_name.as_ref().filter(|n| !n.is_empty()) {
        return name.clone();
    }
    if let Some(result) = &a.result_of {
        return match result.kind.as_str() {
            "Loser" => format!("Loser G{}", result.game_number),
            _ => format!("Winner G{}", result.game_number),
        };
    }
    if let Some(seed) = &a.seeded_by {
        return match &seed.group {
            Some(group) => format!("{} #{}", group.name, seed.number),
            None => format!("Seed #{}", seed.number),
        };
    }
    "TBD".to_string()
}

#[derive(Deserialize)]
struct RawSchedule {
    event: RawEvent,
    games: Vec<RawGame>,
    #[serde(default)]
    teams: HashMap<String, RawTeam>,
}

#[derive(Deserialize)]
struct RawEvent {
    name: String,
}

#[derive(Deserialize)]
struct RawTeam {
    name: String,
}

#[derive(Deserialize)]
struct RawGame {
    number: String,
    #[serde(rename = "startsOn")]
    starts_on: String,
    dark: RawSide,
    light: RawSide,
    court: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct RawSide {
    assignment: Option<RawAssignment>,
}

#[derive(Deserialize)]
struct RawAssignment {
    #[serde(rename = "teamId")]
    team_id: Option<String>,
    #[serde(rename = "pendingAssignmentName")]
    pending_assignment_name: Option<String>,
    #[serde(rename = "resultOf")]
    result_of: Option<RawResultOf>,
    #[serde(rename = "seededBy")]
    seeded_by: Option<RawSeededBy>,
}

#[derive(Deserialize)]
struct RawResultOf {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "gameNumber")]
    game_number: String,
}

#[derive(Deserialize)]
struct RawSeededBy {
    number: u32,
    group: Option<RawNamed>,
}

#[derive(Deserialize)]
struct RawNamed {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "event": { "id": "events/1-B", "name": "Test Cup" },
        "teams": {
            "teams/1-B": { "name": "Sydney Kings A" },
            "teams/2-B": { "name": "Brisbane Barracudas" }
        },
        "games": [
            {
                "number": "10", "startsOn": "2026-08-02T09:00:00+10:00", "court": "1",
                "description": "Final",
                "dark":  { "assignment": { "teamId": null, "resultOf": { "type": "Winner", "gameNumber": "3" }, "seededBy": null, "pendingAssignmentName": null } },
                "light": { "assignment": { "teamId": null, "resultOf": { "type": "Loser", "gameNumber": "2" }, "seededBy": null, "pendingAssignmentName": null } }
            },
            {
                "number": "2", "startsOn": "2026-08-01T09:30:00+10:00", "court": "2",
                "dark":  { "assignment": { "teamId": "teams/2-B", "resultOf": null, "seededBy": null, "pendingAssignmentName": null } },
                "light": { "assignment": { "teamId": null, "resultOf": null, "seededBy": { "number": 4, "group": { "name": "Women RR" } }, "pendingAssignmentName": null } }
            },
            {
                "number": "1", "startsOn": "2026-08-01T09:30:00+10:00", "court": "1",
                "dark":  { "assignment": { "teamId": "teams/1-B", "resultOf": null, "seededBy": null, "pendingAssignmentName": null } },
                "light": { "assignment": { "teamId": "teams/9-B", "resultOf": null, "seededBy": null, "pendingAssignmentName": "Gold Coast" } }
            }
        ]
    }"#;

    #[test]
    fn parses_games_in_order_with_days_and_team_names() {
        let plan = parse_event_plan(SAMPLE).unwrap();
        assert_eq!(plan.event_name, "Test Cup");
        let numbers: Vec<_> = plan.games.iter().map(|g| g.number.as_str()).collect();
        assert_eq!(numbers, ["1", "2", "10"]);

        let g1 = plan.game("1").unwrap();
        assert_eq!((g1.day, g1.court.as_str()), (1, "1"));
        assert_eq!(g1.dark, "Sydney Kings A");
        // Unknown team id falls back to the pending name.
        assert_eq!(g1.light, "Gold Coast");

        let g2 = plan.game("2").unwrap();
        assert_eq!(g2.light, "Women RR #4");

        let g10 = plan.game("10").unwrap();
        assert_eq!(g10.day, 2);
        assert_eq!(
            (g10.dark.as_str(), g10.light.as_str()),
            ("Winner G3", "Loser G2")
        );
    }

    #[test]
    fn groups_into_playlists_per_day_and_court() {
        let plan = parse_event_plan(SAMPLE).unwrap();
        let lists = plan.playlists();
        let keys: Vec<_> = lists.keys().cloned().collect();
        assert_eq!(
            keys,
            [
                (1, "1".to_string()),
                (1, "2".to_string()),
                (2, "1".to_string())
            ]
        );
        assert_eq!(playlist_title(1, "2"), "Day 1 · Court 2");
    }

    #[test]
    fn court_games_are_one_courts_games_on_one_day_in_order() {
        let plan = parse_event_plan(SAMPLE).unwrap();
        let numbers = |court: &str, day: usize| -> Vec<String> {
            plan.court_games(court, day)
                .map(|g| g.number.clone())
                .collect()
        };
        assert_eq!(numbers("1", 1), ["1"]);
        assert_eq!(numbers("2", 1), ["2"]);
        assert_eq!(numbers("1", 2), ["10"]);
        assert!(numbers("2", 2).is_empty());
    }

    #[test]
    fn titles_fit_youtube_limits() {
        let plan = parse_event_plan(SAMPLE).unwrap();
        assert_eq!(
            video_title(&plan.event_name, plan.game("1").unwrap()),
            "Test Cup · Court 1 · Game 1 · Sydney Kings A vs Gold Coast"
        );

        let long_event = "X".repeat(80);
        let title = video_title(&long_event, plan.game("1").unwrap());
        assert_eq!(title, "Court 1 · Game 1 · Sydney Kings A vs Gold Coast");

        let mut game = plan.game("1").unwrap().clone();
        game.dark = "<A>".repeat(60);
        let title = video_title("Cup", &game);
        assert!(title.chars().count() <= MAX_TITLE_CHARS);
        assert!(!title.contains('<') && !title.contains('>'));
    }
}
