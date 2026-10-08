//! The control page: a small web server on this mini PC. By default only the mini PC itself can
//! reach it; with "Allow other devices" on, the listed devices on the venue network can too.
//! Everything except the page itself needs a PIN sign-in, or the button key (Stream Deck links).
//! Until a PIN is set on the mini PC, nothing but setting it works.

use crate::{
    access::{self, Devices, SignIn},
    app::App,
    config::{Config, DEV_PORTAL_URL, LIVE_PORTAL_URL},
    google_auth,
    portal::EventPlan,
    prepare::{self, Selection},
    switcher::Command,
    youtube::YouTubeAccess,
};
use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use log::{info, warn};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    cmp::Ordering,
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    sync::{Arc, OnceLock},
};

const INDEX_HTML: &str = include_str!("../web/index.html");
const SESSION_COOKIE: &str = "sm_session";

const DEVICE_NOT_ALLOWED: &str = "This device isn't allowed. On the mini PC, add its address under \
     Settings → Allow other devices.";
const HOST_NOT_ALLOWED: &str = "This web address isn't allowed. On the mini PC, open the control \
     page at http://127.0.0.1; on an allowed device, use the mini PC's own address.";
const TOO_MANY_SIGN_INS: &str = "Too many sign-in attempts; wait a moment and try again.";
const DEVICE_SETTINGS_LOCAL_ONLY: &str =
    "\"Allow other devices\" can only be changed on the mini PC itself.";
const BUTTON_KEY_LOCAL_ONLY: &str =
    "The button key can only be seen or changed on the mini PC itself.";
const PIN_LOCAL_ONLY: &str = "The PIN can only be changed on the mini PC itself.";
const PIN_NOT_SET: &str = "Set a PIN on the mini PC first";

type AppState = Arc<App>;
type ApiResult = Result<Json<Value>, ApiError>;

/// An error shown to the operator: `{ "error": "..." }` with an HTTP status.
struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<String> for ApiError {
    fn from(message: String) -> Self {
        bad(message)
    }
}

fn fail(status: StatusCode, message: impl Into<String>) -> ApiError {
    ApiError(status, message.into())
}

fn bad(message: impl Into<String>) -> ApiError {
    fail(StatusCode::BAD_REQUEST, message)
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

/// Whether a request comes from this mini PC itself (as for setting the first PIN).
fn from_this_pc(addr: SocketAddr) -> bool {
    addr.ip().is_loopback()
}

/// Allowed if signed in with the PIN (cookie). Nothing is allowed until a PIN has been set.
fn authorize(app: &App, headers: &HeaderMap) -> Result<(), ApiError> {
    if app.config().pin.is_empty() {
        return Err(fail(StatusCode::UNAUTHORIZED, PIN_NOT_SET));
    }
    if session_cookie(headers).is_some_and(|token| app.has_session(&token)) {
        return Ok(());
    }
    Err(fail(StatusCode::UNAUTHORIZED, "PIN required"))
}

/// For the Stream Deck links: the button key (`?key=`), or anything [`authorize`] allows.
fn authorize_button(
    app: &App,
    headers: &HeaderMap,
    query: &HashMap<String, String>,
) -> Result<(), ApiError> {
    if app.config().pin.is_empty() {
        return Err(fail(StatusCode::UNAUTHORIZED, PIN_NOT_SET));
    }
    if let Some(key) = query.get("key") {
        return if access::secret_matches(key, &app.config().button_key) {
            Ok(())
        } else {
            Err(fail(
                StatusCode::UNAUTHORIZED,
                "Wrong button key: copy the link again from the Live tab on the mini PC",
            ))
        };
    }
    authorize(app, headers)
}

/// `host` (a `Host` header) without its port: `127.0.0.1:8090` → `127.0.0.1`, `[::1]:8090` →
/// `[::1]`.
fn without_port(host: &str) -> &str {
    if host.starts_with('[') {
        host.find(']').map_or(host, |end| &host[..=end])
    } else {
        host.split(':').next().unwrap_or(host)
    }
}

/// Whether the web address a request was sent to (its `Host`, port ignored) is `localhost` or a
/// bare IP address (IPv4, or IPv6 in brackets). Any other name is refused: a web page on some
/// other site whose name was made to point at this PC (DNS rebinding) always uses a name, so it
/// gets nothing. Which devices may connect at all is decided by [`Devices::allows`].
fn host_allowed(host: &str) -> bool {
    let name = without_port(host.trim()).to_ascii_lowercase();
    if name == "localhost" || name.parse::<Ipv4Addr>().is_ok() {
        return true;
    }
    name.strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .is_some_and(|inside| inside.parse::<std::net::Ipv6Addr>().is_ok())
}

/// A refusal: JSON for the API, plain text for the page.
fn refuse(path: &str, message: &'static str) -> Response {
    if path.starts_with("/api/") {
        fail(StatusCode::FORBIDDEN, message).into_response()
    } else {
        (StatusCode::FORBIDDEN, message).into_response()
    }
}

/// Answers only this mini PC itself and the allowed devices; every other device is refused,
/// whatever it asks for (the page included). Requests sent to a web address that is a name
/// other than `localhost` are refused too (see [`host_allowed`]).
async fn refuse_other_devices(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    if !app.devices.allows(addr.ip()) {
        return refuse(request.uri().path(), DEVICE_NOT_ALLOWED);
    }
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            request
                .uri()
                .authority()
                .map(|authority| authority.as_str())
        });
    if !host.is_some_and(host_allowed) {
        return refuse(request.uri().path(), HOST_NOT_ALLOWED);
    }
    next.run(request).await
}

/// Whether the browser says the request comes from another web page (another site, or another
/// port on this one). Browsers send `Sec-Fetch-Site` with every request, including an `<img>` or
/// a script's request on someone else's page; the control page's own requests say `same-origin`
/// (or `none` when typed in), and Companion and curl don't send it at all.
fn from_another_page(headers: &HeaderMap) -> bool {
    headers
        .get("sec-fetch-site")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|site| {
            site.eq_ignore_ascii_case("cross-site") || site.eq_ignore_ascii_case("same-site")
        })
}

/// Refuses every API request made from another web page, before any handler (including login
/// and the no-PIN case on this mini PC) sees it.
async fn refuse_other_pages(request: Request, next: Next) -> Response {
    if from_another_page(request.headers()) {
        return fail(
            StatusCode::FORBIDDEN,
            "Requests from other web pages are refused",
        )
        .into_response();
    }
    next.run(request).await
}

/// Until a PIN is set, refuses every API request except what setting the first PIN needs:
/// `/api/session`, and `/api/pin` (which only this mini PC itself may use). The page itself is
/// not behind this.
async fn refuse_until_pin_set(
    State(app): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if app.config().pin.is_empty() && path != "/api/session" && path != "/api/pin" {
        return fail(StatusCode::UNAUTHORIZED, PIN_NOT_SET).into_response();
    }
    next.run(request).await
}

fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/api/session", get(session))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/pin", post(set_pin))
        .route("/api/button-key", post(make_new_button_key))
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
        // Only the API: the page itself opens before a PIN is set, so it can set one.
        .route_layer(middleware::from_fn_with_state(
            Arc::clone(&app),
            refuse_until_pin_set,
        ))
        // Only the API: the page itself may be opened from a link anywhere. Checked first.
        .route_layer(middleware::from_fn(refuse_other_pages))
        .route("/", get(|| async { Html(INDEX_HTML) }))
        // Every route, the page included.
        .layer(middleware::from_fn_with_state(
            Arc::clone(&app),
            refuse_other_devices,
        ))
        .with_state(app)
}

/// Where the control page listens: this mini PC only, or the whole network while other devices
/// are allowed.
fn bind_ip(devices: &Devices) -> Ipv4Addr {
    if devices.allow_others {
        Ipv4Addr::UNSPECIFIED
    } else {
        Ipv4Addr::LOCALHOST
    }
}

pub async fn serve(app: Arc<App>, open_browser: bool) -> Result<(), crate::BoxError> {
    let port = app.config().web_port;
    // Worked out once, here at start-up; the page asks for it on every load.
    let this_pc = lan_ip();
    let router = router(Arc::clone(&app));

    let listener = tokio::net::TcpListener::bind((bind_ip(&app.devices), port))
        .await
        .map_err(|e| format!("Couldn't use port {port} for the control page (is Stream Manager already running?): {e}"))?;
    let local_url = format!("http://127.0.0.1:{port}");
    println!("\nStream Manager is running. Keep this window open; close it to stop.");
    println!("Control page on this mini PC:  {local_url}");
    if !app.devices.allow_others {
        println!("Other devices can't open it (Settings → Allow other devices).");
    } else if let Some(ip) = this_pc {
        println!("From the allowed devices: http://{ip}:{port}");
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

/// This mini PC's address on the local network, worked out the first time it is asked for (at
/// start-up, by [`serve`]) and kept.
pub fn lan_ip() -> Option<IpAddr> {
    static LAN_IP: OnceLock<Option<IpAddr>> = OnceLock::new();
    *LAN_IP.get_or_init(find_lan_ip)
}

/// Asks the network which address this mini PC would use to reach the internet (no traffic is
/// sent).
fn find_lan_ip() -> Option<IpAddr> {
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
    let authed = authorize(&app, &headers).is_ok();
    let lan_url = lan_ip()
        .filter(|_| app.devices.allow_others)
        .map(|ip| format!("http://{ip}:{}", app.config().web_port));
    Json(json!({
        "pin_set": pin_set,
        "authed": authed,
        "is_local": from_this_pc(addr),
        "lan_url": lan_url,
    }))
}

#[derive(Deserialize)]
struct PinBody {
    pin: String,
}

fn login_response(app: &App) -> Response {
    let token = match access::new_session_token() {
        Ok(token) => token,
        Err(e) => {
            return fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Couldn't sign in (no random numbers): {e}"),
            )
            .into_response();
        }
    };
    app.add_session(token.clone());
    let cookie = format!("{SESSION_COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/");
    let mut response = Json(json!({ "ok": true })).into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

async fn login(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<PinBody>,
) -> Response {
    match app
        .sign_in(&body.pin, &addr.ip().to_string(), from_this_pc(addr))
        .await
    {
        SignIn::Right => login_response(&app),
        SignIn::Wrong => fail(StatusCode::UNAUTHORIZED, "Wrong PIN").into_response(),
        SignIn::TooMany => fail(StatusCode::TOO_MANY_REQUESTS, TOO_MANY_SIGN_INS).into_response(),
    }
}

async fn logout(State(app): State<AppState>, headers: HeaderMap) -> Json<Value> {
    if let Some(token) = session_cookie(&headers) {
        app.remove_session(&token);
    }
    Json(json!({ "ok": true }))
}

/// Sets the PIN the first time, or changes it (when signed in). Only on this mini PC itself,
/// like the device settings and the button key. Every other session is signed out; the one that
/// changed it gets a new sign-in.
async fn set_pin(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<PinBody>,
) -> Response {
    if !from_this_pc(addr) {
        return fail(StatusCode::FORBIDDEN, PIN_LOCAL_ONLY).into_response();
    }
    if !app.config().pin.is_empty() {
        if let Err(e) = authorize(&app, &headers) {
            return e.into_response();
        }
    }
    let pin = body.pin.trim().to_string();
    if pin.is_empty() {
        return bad("The PIN can't be empty").into_response();
    }
    if let Err(e) = app.update_settings(|current| {
        Ok::<_, String>(Config {
            pin,
            ..current.clone()
        })
    }) {
        return bad(e).into_response();
    }
    app.clear_sessions();
    info!("PIN changed; every other session signed out");
    login_response(&app)
}

// ----- Live -----

async fn status(
    State(app): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> ApiResult {
    authorize_button(&app, &headers, &query)?;
    Ok(Json(json!(app.status())))
}

async fn court_action(
    State(app): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    Path((court, action)): Path<(String, String)>,
) -> ApiResult {
    authorize_button(&app, &headers, &query)?;
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
    quota_daily_limit: u32,
    quota_share_percent: u8,
    #[serde(default)]
    companion_address: String,
    courts: Vec<crate::config::CourtConfig>,
    /// Sent only by the page on this mini PC itself.
    #[serde(default)]
    allow_other_devices: Option<bool>,
    #[serde(default)]
    allowed_devices: Option<Vec<String>>,
}

/// The allowed devices' addresses as typed, blank lines skipped. IPv4 only: the control page
/// listens on IPv4, so a device listed by an IPv6 address could never reach it.
fn parse_devices(list: &[String]) -> Result<Vec<IpAddr>, String> {
    list.iter()
        .map(|text| text.trim())
        .filter(|text| !text.is_empty())
        .map(|text| match text.parse::<IpAddr>() {
            Ok(IpAddr::V4(ip)) => Ok(IpAddr::V4(ip)),
            Ok(IpAddr::V6(_)) => Err(format!(
                "\"{text}\" is an IPv6 address; use the device's IPv4 address (numbers like 192.168.1.50)"
            )),
            Err(_) => Err(format!(
                "\"{text}\" isn't a device address; use numbers like 192.168.1.50"
            )),
        })
        .collect()
}

async fn get_settings(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult {
    authorize(&app, &headers)?;
    let local = from_this_pc(addr);
    let mut settings = json!(app.config());
    if let Some(settings) = settings.as_object_mut() {
        settings.remove("pin");
        if !local {
            for hidden in ["button_key", "allow_other_devices", "allowed_devices"] {
                settings.remove(hidden);
            }
        }
    }
    Ok(Json(json!({
        "settings": settings,
        "portals": [
            { "name": "Live portal", "url": LIVE_PORTAL_URL },
            { "name": "Dev portal (testing)", "url": DEV_PORTAL_URL },
        ],
        "is_local": local,
        "restart_needed": local && app.devices_need_restart(),
        "this_pc_address": lan_ip().filter(|_| local),
    })))
}

/// The settings to save: the page's `body` on top of the settings in use (`current`, read under
/// the save's lock). The PIN and the button key are never taken from the body, so a save can't
/// bring back a key replaced meanwhile. The device settings can only be changed from this PC.
fn settings_from(body: SettingsBody, current: &Config, local: bool) -> Result<Config, ApiError> {
    let allow_other_devices = body
        .allow_other_devices
        .unwrap_or(current.allow_other_devices);
    let allowed_devices = match &body.allowed_devices {
        Some(list) => parse_devices(list).map_err(bad)?,
        None => current.allowed_devices.clone(),
    };
    if !local
        && (allow_other_devices != current.allow_other_devices
            || allowed_devices != current.allowed_devices)
    {
        return Err(fail(StatusCode::FORBIDDEN, DEVICE_SETTINGS_LOCAL_ONLY));
    }
    Ok(Config {
        portal_url: body.portal_url,
        event_slug: body.event_slug.trim().to_string(),
        privacy: body.privacy,
        switch_lead_secs: body.switch_lead_secs,
        roster_start_secs: body.roster_start_secs,
        roster_end_secs: body.roster_end_secs,
        practice_mode: body.practice_mode,
        quota_daily_limit: body.quota_daily_limit,
        quota_share_percent: body.quota_share_percent,
        companion_address: crate::companion::normalise_address(&body.companion_address),
        courts: body.courts,
        allow_other_devices,
        allowed_devices,
        ..current.clone()
    })
}

async fn save_settings(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<SettingsBody>,
) -> ApiResult {
    authorize(&app, &headers)?;
    let local = from_this_pc(addr);
    app.update_settings(|current| settings_from(body, current, local))?;
    Ok(Json(json!({
        "ok": true,
        "restart_needed": local && app.devices_need_restart(),
    })))
}

/// Replaces the Stream Deck button key; links with the old one stop working at once. Only on
/// this mini PC itself, which is the only place the key is shown.
async fn make_new_button_key(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult {
    authorize(&app, &headers)?;
    if !from_this_pc(addr) {
        return Err(fail(StatusCode::FORBIDDEN, BUTTON_KEY_LOCAL_ONLY));
    }
    let key = access::new_button_key().map_err(|e| {
        fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Couldn't make a new key (no random numbers): {e}"),
        )
    })?;
    // Only the key changes, made from the settings in use under the save's lock.
    app.update_settings(|current| {
        Ok::<_, ApiError>(Config {
            button_key: key.clone(),
            ..current.clone()
        })
    })?;
    info!("Made a new Stream Deck button key");
    Ok(Json(json!({ "button_key": key })))
}

#[derive(Deserialize)]
struct EventsQuery {
    portal_url: String,
}

/// Events on the chosen portal with a published schedule, newest first.
async fn events(
    State(app): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<EventsQuery>,
) -> ApiResult {
    authorize(&app, &headers)?;
    if query.portal_url != LIVE_PORTAL_URL && query.portal_url != DEV_PORTAL_URL {
        return Err(bad("Unknown portal"));
    }
    let client = crate::http_client().map_err(|e| bad(e.to_string()))?;
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
async fn schedule(State(app): State<AppState>, headers: HeaderMap) -> ApiResult {
    authorize(&app, &headers)?;
    let Some(plan) = app.plan() else {
        return Ok(Json(json!({ "loaded": false })));
    };
    let config = app.config();
    let state = app
        .state_file()
        .and_then(|file| prepare::load_state(&file, &config.event_slug))
        .unwrap_or_default();
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

async fn refresh_schedule(State(app): State<AppState>, headers: HeaderMap) -> ApiResult {
    authorize(&app, &headers)?;
    if app.refresh_plan().await {
        // Recovery may wait for a switch to finish, so it runs after the answer.
        let recover_app = Arc::clone(&app);
        tokio::spawn(async move { recover_app.recover_live_videos().await });
    }
    Ok(Json(json!({ "ok": app.plan().is_some() })))
}

async fn prepare_preview(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(selection): Json<Selection>,
) -> ApiResult {
    authorize(&app, &headers)?;
    let plan = app
        .plan()
        .ok_or_else(|| bad("The schedule isn't loaded yet"))?;
    let config = app.config();
    let state = app
        .state_file()
        .and_then(|file| prepare::load_state(&file, &config.event_slug))
        .map_err(|e| bad(e.to_string()))?;
    let mut yt = app.youtube().map_err(|e| bad(e.to_string()))?;
    let lookups = prepare::lookups(&mut yt)
        .await
        .map_err(|e| bad(e.to_string()))?;
    let work = prepare::preview(
        &config,
        &plan,
        &state,
        &lookups,
        &selection,
        &|court: &str| app.day_running(court),
    )
    .map_err(|e| bad(e.to_string()))?;
    Ok(Json(json!({ "work": work, "empty": work.is_empty() })))
}

async fn prepare_run(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(selection): Json<Selection>,
) -> ApiResult {
    authorize(&app, &headers)?;
    let plan = app
        .plan()
        .ok_or_else(|| bad("The schedule isn't loaded yet"))?;
    app.begin_job("Create videos").map_err(bad)?;
    let job_app = Arc::clone(&app);
    tokio::spawn(async move {
        let app = job_app;
        let config = app.config();
        let log_app = Arc::clone(&app);
        let mut log = move |line: String| log_app.job_log(line);
        let result = async {
            let state_file = app.state_file()?;
            let lookups = prepare::lookups(&mut app.youtube()?).await?;
            // Takes each court's lock one game at a time, so switches carry on meanwhile.
            prepare::run(
                &config,
                &plan,
                &mut YouTubeAccess::shared(&app),
                &state_file,
                &lookups,
                &selection,
                &|court: &str| app.day_running(court),
                &mut log,
            )
            .await
        }
        .await;
        app.end_job(result.err().map(|e| e.to_string()));
    });
    Ok(Json(json!({ "started": true })))
}

/// The recorded videos in the same order as the playlists: by day and court, then in schedule
/// order. Games no longer in the schedule come last, in number order.
fn in_schedule_order<'a>(
    plan: Option<&EventPlan>,
    state: &'a prepare::EventState,
) -> Vec<(&'a String, &'a prepare::VideoState)> {
    let mut position: HashMap<&str, (usize, String, usize)> = HashMap::new();
    let playlists = plan.map(|p| p.playlists()).unwrap_or_default();
    for ((day, court), games) in &playlists {
        for (i, game) in games.iter().enumerate() {
            position.insert(game.number.as_str(), (*day, court.clone(), i));
        }
    }
    let number = |game: &str| game.parse::<u64>().unwrap_or(u64::MAX);
    let mut list: Vec<_> = state.videos.iter().collect();
    list.sort_by(
        |(a, _), (b, _)| match (position.get(a.as_str()), position.get(b.as_str())) {
            (Some(x), Some(y)) => x.cmp(y),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => number(a).cmp(&number(b)).then_with(|| a.cmp(b)),
        },
    );
    list
}

/// Videos recorded for this event (no YouTube call).
async fn videos(State(app): State<AppState>, headers: HeaderMap) -> ApiResult {
    authorize(&app, &headers)?;
    let config = app.config();
    let state = app
        .state_file()
        .and_then(|file| prepare::load_state(&file, &config.event_slug))
        .map_err(|e| bad(e.to_string()))?;
    let plan = app.plan();
    let list: Vec<Value> = in_schedule_order(plan.as_ref(), &state)
        .into_iter()
        .map(|(game, v)| json!({ "game": game, "id": v.broadcast_id, "title": v.title, "stream": v.bound_stream, "in_playlist": v.in_playlist }))
        .collect();
    Ok(Json(
        json!({ "videos": list, "playlists": state.playlists }),
    ))
}

/// Asks YouTube for the current state of every recorded video (about 1 unit per 50 videos).
async fn videos_refresh(State(app): State<AppState>, headers: HeaderMap) -> ApiResult {
    authorize(&app, &headers)?;
    let config = app.config();
    let state = app
        .state_file()
        .and_then(|file| prepare::load_state(&file, &config.event_slug))
        .map_err(|e| bad(e.to_string()))?;
    let ids: Vec<&str> = state
        .videos
        .values()
        .map(|v| v.broadcast_id.as_str())
        .collect();
    let mut found = Vec::new();
    {
        let mut yt = app.youtube().map_err(|e| bad(e.to_string()))?;
        for chunk in ids.chunks(50) {
            found.extend(
                yt.broadcast_statuses(chunk)
                    .await
                    .map_err(|e| bad(e.to_string()))?,
            );
        }
    }
    let plan = app.plan();
    let list: Vec<Value> = in_schedule_order(plan.as_ref(), &state)
        .into_iter()
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

/// Starts the Google sign-in. Must be done on the mini PC itself, because Google sends the
/// browser back to an address on this computer.
async fn youtube_connect(
    State(app): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult {
    authorize(&app, &headers)?;
    if !addr.ip().is_loopback() {
        return Err(bad(
            "Connect YouTube from the mini PC running Stream Manager (Google sends you back to it)",
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
                app.forget_youtube();
                let channel = match app.youtube() {
                    Ok(mut yt) => {
                        let title = yt.my_channel_title().await.ok();
                        app.record_youtube(title.clone());
                        title
                    }
                    Err(_) => None,
                };
                app.set_sign_in(&format!(
                    "Connected to YouTube channel: {}",
                    channel.as_deref().unwrap_or("(unknown)")
                ));
                // A court that couldn't be recovered without YouTube is looked at now.
                app.recover_live_videos().await;
            }
            Err(e) => {
                warn!("YouTube sign-in failed: {e}");
                app.set_sign_in(&format!("Sign-in failed: {e}"));
            }
        }
    });
    Ok(Json(json!({ "url": url })))
}

async fn youtube_check(State(app): State<AppState>, headers: HeaderMap) -> ApiResult {
    authorize(&app, &headers)?;
    let config = app.config();
    let mut yt = app.youtube().map_err(|e| bad(e.to_string()))?;
    let channel = yt
        .my_channel_title()
        .await
        .map_err(|e| bad(e.to_string()))?;
    let streams = yt.list_streams().await.map_err(|e| bad(e.to_string()))?;
    app.record_youtube(Some(channel.clone()));
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
    headers: HeaderMap,
    Json(body): Json<CleanupBody>,
) -> ApiResult {
    authorize(&app, &headers)?;
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
        let log_app = Arc::clone(&app);
        let mut log = move |line: String| log_app.job_log(line);
        let result = async {
            let state_file = app.state_file()?;
            // Every court's videos are deleted, so no court may start its day meanwhile.
            let _courts = app.lock_all_courts().await;
            let mut yt = app.youtube()?;
            prepare::cleanup(&mut yt, &state_file, &slug, &mut log).await
        }
        .await;
        app.end_job(result.err().map(|e| e.to_string()));
    });
    Ok(Json(json!({ "started": true })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn requests_from_other_web_pages_are_recognised() {
        let with = |value: &str| {
            let mut headers = HeaderMap::new();
            headers.insert("sec-fetch-site", HeaderValue::from_str(value).unwrap());
            from_another_page(&headers)
        };
        assert!(with("cross-site"));
        assert!(with("same-site"));
        assert!(!with("same-origin"));
        assert!(!with("none"));
        // Companion and curl send no such header.
        assert!(!from_another_page(&HeaderMap::new()));
    }

    #[tokio::test]
    async fn the_api_refuses_other_web_pages_even_on_this_laptop_without_a_pin() {
        let dir = std::env::temp_dir().join(format!("stream-manager-web-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let app = App::new(dir.join("config.toml"), Config::default());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let service = router(app).into_make_service_with_connect_info::<SocketAddr>();
        tokio::spawn(async move { axum::serve(listener, service).await });
        let client = reqwest::Client::new();
        let send = |path: &str, site: Option<&str>| {
            let mut request = client
                .post(format!("{base}{path}"))
                .json(&json!({ "pin": "1234" }));
            if let Some(site) = site {
                request = request.header("sec-fetch-site", site);
            }
            request.send()
        };

        for site in ["cross-site", "same-site"] {
            for path in ["/api/login", "/api/pin"] {
                let response = send(path, Some(site)).await.unwrap();
                assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
                let body: Value = response.json().await.unwrap();
                assert_eq!(body["error"], "Requests from other web pages are refused");
            }
            let response = send("/api/court/1/hold", Some(site)).await.unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
        // The control page's own requests get past this check (to the "no PIN yet" one).
        let response = send("/api/court/1/hold", Some("same-origin"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        // The page itself opens from anywhere.
        let response = client
            .get(format!("{base}/"))
            .header("sec-fetch-site", "cross-site")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn the_page_and_companion_still_work_with_a_pin_and_the_key() {
        let (base, dir) = test_server("pages", None).await;
        let client = reqwest::Client::new();
        let cookie = sign_in(&client, &base).await;
        // The control page's own request, signed in.
        let response = client
            .post(format!("{base}/api/court/1/hold"))
            .header("sec-fetch-site", "same-origin")
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // Companion's (no header), with the button key.
        let response = client
            .post(format!("{base}/api/court/1/release?key={KEY}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_page_listens_on_this_pc_only_unless_other_devices_are_allowed() {
        let mut devices = Devices {
            allow_others: false,
            allowed: vec!["192.168.1.50".parse().unwrap()],
        };
        assert_eq!(bind_ip(&devices), Ipv4Addr::LOCALHOST);
        devices.allow_others = true;
        assert_eq!(bind_ip(&devices), Ipv4Addr::UNSPECIFIED);
    }

    const LISTED: [u8; 4] = [192, 168, 1, 50];
    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    /// Runs the control page on a free port with a PIN, the button key and one listed device.
    /// With `from`, every request looks as if it came from that address.
    async fn test_server(name: &str, from: Option<[u8; 4]>) -> (String, std::path::PathBuf) {
        test_server_with_pin(name, from, "1234").await
    }

    /// As [`test_server`], with `pin` as the PIN (empty: none set yet).
    async fn test_server_with_pin(
        name: &str,
        from: Option<[u8; 4]>,
        pin: &str,
    ) -> (String, std::path::PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("stream-manager-web-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let config = Config {
            pin: pin.into(),
            button_key: KEY.into(),
            allow_other_devices: true,
            allowed_devices: vec![IpAddr::from(LISTED)],
            ..Config::default()
        };
        let app = App::new(dir.join("config.toml"), config);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        match from {
            Some(ip) => {
                let service = router(app)
                    .layer(axum::extract::connect_info::MockConnectInfo(
                        SocketAddr::from((ip, 50000)),
                    ))
                    .into_make_service();
                tokio::spawn(async move { axum::serve(listener, service).await });
            }
            None => {
                let service = router(app).into_make_service_with_connect_info::<SocketAddr>();
                tokio::spawn(async move { axum::serve(listener, service).await });
            }
        }
        (base, dir)
    }

    /// Signs in with the PIN; returns the cookie to send with later requests.
    async fn sign_in(client: &reqwest::Client, base: &str) -> String {
        let response = client
            .post(format!("{base}/api/login"))
            .json(&json!({ "pin": "1234" }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
        cookie.split(';').next().unwrap().to_string()
    }

    #[tokio::test]
    async fn a_device_that_isnt_listed_gets_no_page_and_no_sign_in() {
        let (base, dir) = test_server("unlisted", Some([192, 168, 1, 51])).await;
        let client = reqwest::Client::new();
        let response = client.get(format!("{base}/")).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(response.text().await.unwrap(), DEVICE_NOT_ALLOWED);
        let response = client
            .post(format!("{base}/api/login"))
            .json(&json!({ "pin": "1234" }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"], DEVICE_NOT_ALLOWED);
        // Even with the right button key.
        let response = client
            .get(format!("{base}/api/status?key={KEY}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_listed_device_can_sign_in_but_never_sees_or_changes_device_settings_or_the_key() {
        let (base, dir) = test_server("listed", Some(LISTED)).await;
        let client = reqwest::Client::new();
        let response = client.get(format!("{base}/")).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = sign_in(&client, &base).await;

        let body: Value = client
            .get(format!("{base}/api/settings"))
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(body["is_local"], false);
        let mut settings = body["settings"].clone();
        for hidden in [
            "pin",
            "button_key",
            "allow_other_devices",
            "allowed_devices",
        ] {
            assert!(settings.get(hidden).is_none(), "{hidden} was sent");
        }
        assert!(!body.to_string().contains(KEY));

        // Saving the other settings works; changing the device settings is refused.
        let save = |settings: &Value| {
            client
                .post(format!("{base}/api/settings"))
                .header(header::COOKIE, &cookie)
                .json(settings)
                .send()
        };
        assert_eq!(save(&settings).await.unwrap().status(), StatusCode::OK);
        settings["allowed_devices"] = json!(["192.168.1.50", "192.168.1.51"]);
        let response = save(&settings).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let error: Value = response.json().await.unwrap();
        assert_eq!(error["error"], DEVICE_SETTINGS_LOCAL_ONLY);
        settings["allowed_devices"] = json!(["192.168.1.50"]);
        settings["allow_other_devices"] = json!(false);
        assert_eq!(
            save(&settings).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );

        let response = client
            .post(format!("{base}/api/button-key"))
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        // The key itself still works from a listed device (a Stream Deck elsewhere).
        let response = client
            .get(format!("{base}/api/status?key={KEY}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn stream_deck_links_need_the_button_key_and_the_pin_no_longer_works() {
        let (base, dir) = test_server("key", None).await;
        let client = reqwest::Client::new();
        let get = |query: &str| client.get(format!("{base}/api/status{query}")).send();
        assert_eq!(
            get(&format!("?key={KEY}")).await.unwrap().status(),
            StatusCode::OK
        );
        let wrong = format!("?key={}0", &KEY[..63]);
        assert_eq!(
            get(&wrong).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(get("").await.unwrap().status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            get("?key=").await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            get("?pin=1234").await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        let response = client
            .get(format!("{base}/api/status"))
            .header("x-pin", "1234")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        // A court action with the key works (court 9 doesn't exist: the key was accepted).
        let response = client
            .get(format!("{base}/api/court/9/hold?key={KEY}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response = client
            .get(format!("{base}/api/court/9/hold?pin=1234"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        // The PIN still signs in on the page, and the session works for the page's requests.
        let cookie = sign_in(&client, &base).await;
        let response = client
            .get(format!("{base}/api/status"))
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn this_pc_sees_and_changes_the_device_settings_and_the_key() {
        let (base, dir) = test_server("local", None).await;
        let client = reqwest::Client::new();
        let cookie = sign_in(&client, &base).await;
        let body: Value = client
            .get(format!("{base}/api/settings"))
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(body["is_local"], true);
        assert_eq!(body["restart_needed"], false);
        let mut settings = body["settings"].clone();
        assert_eq!(settings["button_key"], KEY);
        assert_eq!(settings["allow_other_devices"], true);
        assert_eq!(settings["allowed_devices"], json!(["192.168.1.50"]));
        assert!(settings.get("pin").is_none());

        settings["allowed_devices"] = json!(["192.168.1.50", " 192.168.1.60 ", ""]);
        let response = client
            .post(format!("{base}/api/settings"))
            .header(header::COOKIE, &cookie)
            .json(&settings)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let saved: Value = response.json().await.unwrap();
        assert_eq!(saved["restart_needed"], true);
        settings["allowed_devices"] = json!(["not an address"]);
        let response = client
            .post(format!("{base}/api/settings"))
            .header(header::COOKIE, &cookie)
            .json(&settings)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let response = client
            .post(format!("{base}/api/button-key"))
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let new: Value = response.json().await.unwrap();
        let new_key = new["button_key"].as_str().unwrap().to_string();
        assert_eq!(new_key.len(), 64);
        assert_ne!(new_key, KEY);
        let status = |key: String| client.get(format!("{base}/api/status?key={key}")).send();
        assert_eq!(
            status(KEY.to_string()).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(status(new_key).await.unwrap().status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_localhost_and_ip_addresses_are_answered() {
        for host in [
            "127.0.0.1",
            "127.0.0.1:8090",
            "localhost",
            "LocalHost:8090",
            "[::1]",
            "[::1]:8090",
            "192.168.1.7:8090",
            "192.168.1.7",
        ] {
            assert!(host_allowed(host), "{host} was refused");
        }
        for host in [
            "evil.example",
            "evil.example:8090",
            "127.0.0.1.nip.io",
            "localhost.evil.example:8090",
            "[evil.example]",
            "",
        ] {
            assert!(!host_allowed(host), "{host} was answered");
        }
    }

    #[tokio::test]
    async fn requests_sent_to_another_web_address_are_refused() {
        let (base, dir) = test_server("host", None).await;
        let client = reqwest::Client::new();
        let get = |path: &str, host: &str| {
            client
                .get(format!("{base}{path}"))
                .header(header::HOST, host)
                .send()
        };
        let response = get("/", "evil.example:8090").await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(response.text().await.unwrap(), HOST_NOT_ALLOWED);
        let response = get(&format!("/api/status?key={KEY}"), "evil.example")
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"], HOST_NOT_ALLOWED);
        for host in ["localhost:8090", "[::1]:8090", "127.0.0.1"] {
            let response = get(&format!("/api/status?key={KEY}"), host).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{host}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn changing_the_pin_signs_out_every_other_session() {
        let (base, dir) = test_server("pin-change", None).await;
        let client = reqwest::Client::new();
        let changer = sign_in(&client, &base).await;
        let other = sign_in(&client, &base).await;
        let response = client
            .post(format!("{base}/api/pin"))
            .header(header::COOKIE, &changer)
            .json(&json!({ "pin": "5678" }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
        let renewed = cookie.split(';').next().unwrap().to_string();
        let status = |cookie: &str| {
            client
                .get(format!("{base}/api/status"))
                .header(header::COOKIE, cookie)
                .send()
        };
        assert_eq!(
            status(&other).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(status(&renewed).await.unwrap().status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn the_pin_can_only_be_changed_on_this_pc() {
        let (base, dir) = test_server("pin-remote", Some(LISTED)).await;
        let client = reqwest::Client::new();
        let cookie = sign_in(&client, &base).await;
        let response = client
            .post(format!("{base}/api/pin"))
            .header(header::COOKIE, &cookie)
            .json(&json!({ "pin": "5678" }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"], PIN_LOCAL_ONLY);
        // Nothing changed: the old PIN still signs in, and the session still works.
        sign_in(&client, &base).await;
        let response = client
            .get(format!("{base}/api/status"))
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn nothing_works_until_a_pin_is_set_on_this_pc() {
        let (base, dir) = test_server_with_pin("no-pin", None, "").await;
        let client = reqwest::Client::new();
        let refused = |response: reqwest::Response| async move {
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            let body: Value = response.json().await.unwrap();
            assert_eq!(body["error"], PIN_NOT_SET);
        };
        // Even on this mini PC, and even with the button key.
        for path in [
            "/api/status".to_string(),
            format!("/api/status?key={KEY}"),
            format!("/api/court/1/hold?key={KEY}"),
            "/api/court/1/start".to_string(),
            "/api/settings".to_string(),
            "/api/schedule".to_string(),
            "/api/videos".to_string(),
        ] {
            refused(client.get(format!("{base}{path}")).send().await.unwrap()).await;
        }
        for path in [
            "/api/login",
            "/api/settings",
            "/api/prepare/preview",
            "/api/prepare/run",
            "/api/button-key",
            "/api/youtube/connect",
            "/api/cleanup",
        ] {
            let response = client
                .post(format!("{base}{path}"))
                .json(&json!({ "pin": "1234" }))
                .send()
                .await
                .unwrap();
            refused(response).await;
        }
        // The page and the sign-in check still answer.
        let response = client.get(format!("{base}/")).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let session: Value = client
            .get(format!("{base}/api/session"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(
            (&session["pin_set"], &session["authed"]),
            (&json!(false), &json!(false))
        );

        // Setting the first PIN on this mini PC signs it in.
        let response = client
            .post(format!("{base}/api/pin"))
            .json(&json!({ "pin": "1234" }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
        let cookie = cookie.split(';').next().unwrap().to_string();
        let response = client
            .get(format!("{base}/api/status"))
            .header(header::COOKIE, &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn another_device_cant_set_the_first_pin() {
        let (base, dir) = test_server_with_pin("no-pin-remote", Some(LISTED), "").await;
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{base}/api/pin"))
            .json(&json!({ "pin": "1234" }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"], PIN_LOCAL_ONLY);
        let response = client
            .get(format!("{base}/api/status?key={KEY}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_settings_save_keeps_the_key_and_pin_in_use() {
        // The page's settings as read before a new key was made (and with an old key in them).
        let before = Config {
            button_key: "old-key".into(),
            pin: "1111".into(),
            ..Config::default()
        };
        let current = Config {
            button_key: KEY.into(),
            pin: "1234".into(),
            ..Config::default()
        };
        for local in [true, false] {
            let body: SettingsBody = serde_json::from_value(json!(before)).unwrap();
            let saved = settings_from(body, &current, local).ok().unwrap();
            assert_eq!(saved.button_key, KEY);
            assert_eq!(saved.pin, "1234");
        }
    }

    #[test]
    fn allowed_devices_take_ipv4_addresses_only() {
        let list = |items: &[&str]| {
            parse_devices(&items.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        };
        assert_eq!(
            list(&["192.168.1.50", " ", "10.0.0.2 "]).unwrap(),
            vec![IpAddr::from([192, 168, 1, 50]), IpAddr::from([10, 0, 0, 2])]
        );
        for v6 in ["fe80::1", "::ffff:192.168.1.50", "::1"] {
            assert_eq!(
                list(&[v6]).unwrap_err(),
                format!(
                    "\"{v6}\" is an IPv6 address; use the device's IPv4 address (numbers like 192.168.1.50)"
                )
            );
        }
        assert!(
            list(&["tablet"])
                .unwrap_err()
                .contains("isn't a device address")
        );
    }
}
