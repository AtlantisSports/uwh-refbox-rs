//! The control page: a small web server on this laptop, reachable from any device on the venue
//! network and from Companion. Everything except the page itself needs the PIN.

use crate::{
    app::App,
    config::{Config, DEV_PORTAL_URL, LIVE_PORTAL_URL},
    google_auth,
    prepare::{self, Selection},
    switcher::Command,
};
use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use log::{info, warn};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, hash_map::RandomState},
    hash::{BuildHasher, Hasher},
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::Arc,
    time::Duration,
};

const INDEX_HTML: &str = include_str!("../web/index.html");
const SESSION_COOKIE: &str = "sm_session";

type AppState = Arc<App>;
type ApiResult = Result<Json<Value>, ApiError>;

/// An error shown to the operator: `{ "error": "..." }` with an HTTP status.
struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

fn fail(status: StatusCode, message: impl Into<String>) -> ApiError {
    ApiError(status, message.into())
}

fn bad(message: impl Into<String>) -> ApiError {
    fail(StatusCode::BAD_REQUEST, message)
}

fn random_token() -> String {
    let a = RandomState::new().build_hasher().finish();
    let b = RandomState::new().build_hasher().finish();
    format!("{a:016x}{b:016x}")
}

fn session_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == SESSION_COOKIE)
        .map(|(_, v)| v.to_string())
}

/// Allowed if: logged in with the PIN (cookie), or the request carries the PIN (Companion), or
/// no PIN has been set yet and the request comes from this laptop itself.
fn authorize(
    app: &App,
    headers: &HeaderMap,
    query: &HashMap<String, String>,
    addr: SocketAddr,
) -> Result<(), ApiError> {
    let pin = app.config().pin;
    if pin.is_empty() {
        return if addr.ip().is_loopback() {
            Ok(())
        } else {
            Err(fail(
                StatusCode::UNAUTHORIZED,
                "Set a PIN on the laptop running Stream Manager first",
            ))
        };
    }
    let header_pin = headers.get("x-pin").and_then(|v| v.to_str().ok());
    if query.get("pin").map(String::as_str) == Some(pin.as_str())
        || header_pin == Some(pin.as_str())
    {
        return Ok(());
    }
    if session_cookie(headers).is_some_and(|token| app.has_session(&token)) {
        return Ok(());
    }
    Err(fail(StatusCode::UNAUTHORIZED, "PIN required"))
}

pub async fn serve(app: Arc<App>, open_browser: bool) -> Result<(), crate::BoxError> {
    let port = app.config().web_port;
    let router = Router::new()
        .route("/", get(|| async { Html(INDEX_HTML) }))
        .route("/api/session", get(session))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/pin", post(set_pin))
        .route("/api/status", get(status))
        .route(
            "/api/court/{court}/{action}",
            get(court_action).post(court_action),
        )
        .route("/api/settings", get(get_settings).post(save_settings))
        .route("/api/events", get(events))
        .route("/api/schedule", get(schedule))
        .route("/api/schedule/refresh", post(refresh_schedule))
        .route("/api/prepare/preview", post(prepare_preview))
        .route("/api/prepare/run", post(prepare_run))
        .route("/api/videos", get(videos).post(videos_refresh))
        .route("/api/youtube/connect", post(youtube_connect))
        .route("/api/youtube/check", post(youtube_check))
        .route("/api/cleanup", post(cleanup))
        .with_state(Arc::clone(&app));

    let listener = tokio::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))
        .await
        .map_err(|e| format!("Couldn't use port {port} for the control page (is Stream Manager already running?): {e}"))?;
    let local_url = format!("http://127.0.0.1:{port}");
    println!("\nStream Manager is running. Keep this window open; close it to stop.");
    println!("Control page on this laptop:  {local_url}");
    if let Some(ip) = lan_ip() {
        println!("From other devices / Companion: http://{ip}:{port}");
    }
    println!();
    if open_browser {
        google_auth::open_browser(&local_url);
    }
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

/// This laptop's address on the local network (no traffic is sent).
pub fn lan_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

// ----- Login / PIN -----

async fn session(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    let pin_set = !app.config().pin.is_empty();
    let authed = authorize(&app, &headers, &HashMap::new(), addr).is_ok();
    Json(json!({
        "pin_set": pin_set,
        "authed": authed,
        "is_local": addr.ip().is_loopback(),
        "lan_url": lan_ip().map(|ip| format!("http://{ip}:{}", app.config().web_port)),
    }))
}

#[derive(Deserialize)]
struct PinBody {
    pin: String,
}

fn login_response(app: &App) -> Response {
    let token = random_token();
    app.add_session(token.clone());
    let cookie = format!("{SESSION_COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/");
    let mut response = Json(json!({ "ok": true })).into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

async fn login(State(app): State<AppState>, Json(body): Json<PinBody>) -> Response {
    let pin = app.config().pin;
    if !pin.is_empty() && body.pin.trim() == pin {
        login_response(&app)
    } else {
        // Slow down guessing.
        tokio::time::sleep(Duration::from_secs(1)).await;
        fail(StatusCode::UNAUTHORIZED, "Wrong PIN").into_response()
    }
}

async fn logout(State(app): State<AppState>, headers: HeaderMap) -> Json<Value> {
    if let Some(token) = session_cookie(&headers) {
        app.remove_session(&token);
    }
    Json(json!({ "ok": true }))
}

/// Sets the PIN the first time (only from this laptop), or changes it (when logged in).
async fn set_pin(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<PinBody>,
) -> Response {
    if let Err(e) = authorize(&app, &headers, &HashMap::new(), addr) {
        return e.into_response();
    }
    let mut config = app.config();
    config.pin = body.pin.trim().to_string();
    if config.pin.is_empty() {
        return bad("The PIN can't be empty").into_response();
    }
    if let Err(e) = app.apply_settings(config) {
        return bad(e).into_response();
    }
    info!("PIN changed");
    login_response(&app)
}

// ----- Live -----

async fn status(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> ApiResult {
    authorize(&app, &headers, &query, addr)?;
    Ok(Json(json!(app.status())))
}

async fn court_action(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    Path((court, action)): Path<(String, String)>,
) -> ApiResult {
    authorize(&app, &headers, &query, addr)?;
    let command = match action.as_str() {
        "start" => Command::StartDay,
        "end" => Command::EndDay,
        "hold" => Command::Hold,
        "release" => Command::Release,
        "hold-toggle" => {
            let holding = app
                .status()
                .courts
                .iter()
                .any(|c| c.name == court && c.hold);
            if holding {
                Command::Release
            } else {
                Command::Hold
            }
        }
        "next" => Command::SwitchNow,
        _ => {
            return Err(bad(
                "Unknown action; use start, hold, release, hold-toggle, next or end",
            ));
        }
    };
    let message = app.court_command(&court, command).map_err(bad)?;
    Ok(Json(json!({ "message": message })))
}

// ----- Settings -----

#[derive(Deserialize)]
struct SettingsBody {
    portal_url: String,
    event_slug: String,
    privacy: String,
    switch_lead_secs: u32,
    roster_start_secs: u32,
    roster_end_secs: u32,
    practice_mode: bool,
    courts: Vec<crate::config::CourtConfig>,
}

async fn get_settings(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    let mut config = app.config();
    config.pin = String::new();
    Ok(Json(json!({
        "settings": config,
        "portals": [
            { "name": "Live portal", "url": LIVE_PORTAL_URL },
            { "name": "Dev portal (testing)", "url": DEV_PORTAL_URL },
        ],
    })))
}

async fn save_settings(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<SettingsBody>,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    let current = app.config();
    let new = Config {
        portal_url: body.portal_url,
        event_slug: body.event_slug.trim().to_string(),
        privacy: body.privacy,
        switch_lead_secs: body.switch_lead_secs,
        roster_start_secs: body.roster_start_secs,
        roster_end_secs: body.roster_end_secs,
        practice_mode: body.practice_mode,
        courts: body.courts,
        ..current
    };
    app.apply_settings(new).map_err(bad)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct EventsQuery {
    portal_url: String,
}

/// Events on the chosen portal with a published schedule, newest first.
async fn events(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(query): Query<EventsQuery>,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    if query.portal_url != LIVE_PORTAL_URL && query.portal_url != DEV_PORTAL_URL {
        return Err(bad("Unknown portal"));
    }
    let client = reqwest::Client::new();
    let mut events = Vec::new();
    for filter in ["InProgressOrUpcoming", "Past"] {
        let response = client
            .get(format!("{}/api/events", query.portal_url))
            .query(&[
                ("limit", "100"),
                ("filter", filter),
                ("isSchedulePublished", "true"),
            ])
            .send()
            .await
            .map_err(|e| bad(format!("Couldn't reach the portal: {e}")))?;
        let body: Value = serde_json::from_str(&response.text().await.unwrap_or_default())
            .map_err(|e| bad(format!("Unexpected reply from the portal: {e}")))?;
        for item in body["items"].as_array().into_iter().flatten() {
            events.push(json!({
                "slug": item["slug"],
                "name": item["name"],
                "starts": item["dateRange"]["startsOn"],
                "upcoming": filter != "Past",
            }));
        }
    }
    events.sort_by(|a, b| {
        b["starts"]
            .as_str()
            .unwrap_or("")
            .cmp(a["starts"].as_str().unwrap_or(""))
    });
    Ok(Json(json!({ "events": events })))
}

// ----- Schedule and prepare -----

/// Days and courts in the loaded schedule, for the Prepare tab.
async fn schedule(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    let Some(plan) = app.plan() else {
        return Ok(Json(json!({ "loaded": false })));
    };
    let config = app.config();
    let state = prepare::load_state(&app.state_file(), &config.event_slug).unwrap_or_default();
    let playlists: Vec<Value> = plan
        .playlists()
        .into_iter()
        .map(|((day, court), games)| {
            let date = games
                .first()
                .map(|g| g.start.date().to_string())
                .unwrap_or_default();
            json!({
                "day": day,
                "date": date,
                "court": court,
                "configured": config.courts.iter().any(|c| c.name == court),
                "games": games.iter().map(|g| json!({
                    "number": g.number,
                    "start": g.start.format(time::macros::format_description!("[hour]:[minute]")).unwrap_or_default(),
                    "title": crate::portal::video_title(&plan.event_name, g),
                    "video": state.videos.get(&g.number).map(|v| &v.broadcast_id),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(Json(json!({
        "loaded": true,
        "event_name": plan.event_name,
        "playlists": playlists,
    })))
}

async fn refresh_schedule(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    app.refresh_plan().await;
    Ok(Json(json!({ "ok": app.plan().is_some() })))
}

async fn prepare_preview(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(selection): Json<Selection>,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    let plan = app
        .plan()
        .ok_or_else(|| bad("The schedule isn't loaded yet"))?;
    let config = app.config();
    let state = prepare::load_state(&app.state_file(), &config.event_slug)
        .map_err(|e| bad(e.to_string()))?;
    let mut yt = app.youtube().await.map_err(|e| bad(e.to_string()))?;
    let lookups = prepare::lookups(&mut yt)
        .await
        .map_err(|e| bad(e.to_string()))?;
    let units = yt.units_used;
    drop(yt);
    app.record_youtube(None, units);
    let work = prepare::preview(&config, &plan, &state, &lookups, &selection)
        .map_err(|e| bad(e.to_string()))?;
    Ok(Json(json!({ "work": work, "empty": work.is_empty() })))
}

async fn prepare_run(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(selection): Json<Selection>,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    let plan = app
        .plan()
        .ok_or_else(|| bad("The schedule isn't loaded yet"))?;
    app.begin_job("Create videos").map_err(bad)?;
    let job_app = Arc::clone(&app);
    tokio::spawn(async move {
        let app = job_app;
        let config = app.config();
        let state_file = app.state_file();
        let log_app = Arc::clone(&app);
        let mut log = move |line: String| log_app.job_log(line);
        let result = async {
            let mut yt = app.youtube().await?;
            let lookups = prepare::lookups(&mut yt).await?;
            let outcome = prepare::run(
                &config,
                &plan,
                &mut yt,
                &state_file,
                &lookups,
                &selection,
                &mut log,
            )
            .await;
            app.record_youtube(None, yt.units_used);
            outcome
        }
        .await;
        app.end_job(result.err().map(|e| e.to_string()));
    });
    Ok(Json(json!({ "started": true })))
}

/// Videos recorded for this event (no YouTube call).
async fn videos(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    let config = app.config();
    let state = prepare::load_state(&app.state_file(), &config.event_slug)
        .map_err(|e| bad(e.to_string()))?;
    let list: Vec<Value> = state
        .videos
        .iter()
        .map(|(game, v)| json!({ "game": game, "id": v.broadcast_id, "title": v.title, "stream": v.bound_stream, "in_playlist": v.in_playlist }))
        .collect();
    Ok(Json(
        json!({ "videos": list, "playlists": state.playlists }),
    ))
}

/// Asks YouTube for the current state of every recorded video (about 1 unit per 50 videos).
async fn videos_refresh(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    let config = app.config();
    let state = prepare::load_state(&app.state_file(), &config.event_slug)
        .map_err(|e| bad(e.to_string()))?;
    let ids: Vec<&str> = state
        .videos
        .values()
        .map(|v| v.broadcast_id.as_str())
        .collect();
    let mut found = Vec::new();
    {
        let mut yt = app.youtube().await.map_err(|e| bad(e.to_string()))?;
        for chunk in ids.chunks(50) {
            found.extend(
                yt.broadcast_statuses(chunk)
                    .await
                    .map_err(|e| bad(e.to_string()))?,
            );
        }
        app.record_youtube(None, yt.units_used);
    }
    let list: Vec<Value> = state
        .videos
        .iter()
        .map(|(game, v)| {
            let live = found.iter().find(|f| f[0] == v.broadcast_id);
            json!({
                "game": game,
                "id": v.broadcast_id,
                "title": live.map_or(v.title.as_str(), |f| f[1].as_str()),
                "status": live.map_or("not found on YouTube", |f| f[2].as_str()),
                "privacy": live.map(|f| f[3].as_str()),
                "stream": v.bound_stream,
                "in_playlist": v.in_playlist,
            })
        })
        .collect();
    Ok(Json(
        json!({ "videos": list, "playlists": state.playlists }),
    ))
}

// ----- YouTube connection -----

/// Starts the Google sign-in. Must be done on the laptop itself, because Google sends the
/// browser back to an address on this computer.
async fn youtube_connect(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    if !addr.ip().is_loopback() {
        return Err(bad(
            "Connect YouTube from the laptop running Stream Manager (Google sends you back to it)",
        ));
    }
    let pending = google_auth::begin_sign_in(&app.client_file())
        .await
        .map_err(|e| bad(e.to_string()))?;
    let url = pending.url.clone();
    app.set_sign_in("Waiting for you to finish signing in with Google…");
    let job_app = Arc::clone(&app);
    tokio::spawn(async move {
        let app = job_app;
        match pending.finish(&app.token_file()).await {
            Ok(()) => {
                app.forget_youtube().await;
                let channel = match app.youtube().await {
                    Ok(mut yt) => {
                        let title = yt.my_channel_title().await.ok();
                        let units = yt.units_used;
                        drop(yt);
                        app.record_youtube(title.clone(), units);
                        title
                    }
                    Err(_) => None,
                };
                app.set_sign_in(&format!(
                    "Connected to YouTube channel: {}",
                    channel.as_deref().unwrap_or("(unknown)")
                ));
            }
            Err(e) => {
                warn!("YouTube sign-in failed: {e}");
                app.set_sign_in(&format!("Sign-in failed: {e}"));
            }
        }
    });
    Ok(Json(json!({ "url": url })))
}

async fn youtube_check(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    let config = app.config();
    let mut yt = app.youtube().await.map_err(|e| bad(e.to_string()))?;
    let channel = yt
        .my_channel_title()
        .await
        .map_err(|e| bad(e.to_string()))?;
    let streams = yt.list_streams().await.map_err(|e| bad(e.to_string()))?;
    let units = yt.units_used;
    drop(yt);
    app.record_youtube(Some(channel.clone()), units);
    let courts: Vec<Value> = config
        .courts
        .iter()
        .map(|court| match prepare::court_streams(court, &streams) {
            Ok(pair) => json!({
                "court": court.name,
                "ok": true,
                "streams": pair.iter().map(|s| json!({ "name": s.title, "sending": s.stream_status == "active" })).collect::<Vec<_>>(),
            }),
            Err(e) => json!({ "court": court.name, "ok": false, "error": e }),
        })
        .collect();
    Ok(Json(json!({
        "channel": channel,
        "courts": courts,
        "all_streams": streams.iter().map(|s| &s.title).collect::<Vec<_>>(),
    })))
}

// ----- Test tools -----

#[derive(Deserialize)]
struct CleanupBody {
    confirm: String,
}

async fn cleanup(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<CleanupBody>,
) -> ApiResult {
    authorize(&app, &headers, &HashMap::new(), addr)?;
    if body.confirm != "DELETE" {
        return Err(bad("Type DELETE to confirm"));
    }
    if app.status().courts.iter().any(|c| c.day_running) {
        return Err(bad("End the day on every court first"));
    }
    app.begin_job("Delete test videos").map_err(bad)?;
    let job_app = Arc::clone(&app);
    tokio::spawn(async move {
        let app = job_app;
        let slug = app.config().event_slug;
        let state_file = app.state_file();
        let log_app = Arc::clone(&app);
        let mut log = move |line: String| log_app.job_log(line);
        let result = async {
            let mut yt = app.youtube().await?;
            let outcome = prepare::cleanup(&mut yt, &state_file, &slug, &mut log).await;
            app.record_youtube(None, yt.units_used);
            outcome
        }
        .await;
        app.end_job(result.err().map(|e| e.to_string()));
    });
    Ok(Json(json!({ "started": true })))
}
