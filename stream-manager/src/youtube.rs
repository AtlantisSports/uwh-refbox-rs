//! The few YouTube Data API calls stream-manager needs, with a running count of the daily
//! allowance ("quota units") they use. Every call is also added to the allowance ledger file
//! (`quota.rs`), which counts this program's use for the whole day, across restarts.
//!
//! Costs follow Google's published quota table: list calls cost 1 unit, and creating,
//! changing, binding or deleting costs 50 units.

use crate::{BoxError, app::App, google_auth::GoogleAuth, quota};
use log::warn;
use reqwest::{
    Method,
    header::{CONTENT_LENGTH, CONTENT_TYPE},
};
use serde_json::{Value, json};
use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};
use time::OffsetDateTime;
use tokio::sync::MappedMutexGuard;

const API: &str = "https://www.googleapis.com/youtube/v3";
const LIST_COST: u32 = 1;
const WRITE_COST: u32 = 50;

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub id: String,
    pub title: String,
    /// `active` while vMix is sending to this stream key.
    pub stream_status: String,
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct BroadcastInfo {
    pub life_cycle: String,
    pub live_chat_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BroadcastSpec {
    pub title: String,
    pub description: String,
    /// RFC 3339, e.g. `2026-08-01T09:30:00+10:00`.
    pub scheduled_start: String,
    pub privacy: String,
}

pub struct YouTube {
    auth: GoogleAuth,
    http: reqwest::Client,
    /// Units used through this connection since it was opened.
    pub units_used: u32,
    /// The allowance ledger file every call is added to (`None` counts nowhere else).
    ledger: Option<PathBuf>,
}

/// Hands out the YouTube connection for one step of a longer job (one game of Prepare or of a
/// title check). Taking it per step instead of for the whole job lets a court's switch, which
/// needs the same connection, go ahead between two steps.
pub enum YouTubeAccess<'a> {
    /// The caller already holds the connection (a switch, or the terminal) and keeps it.
    Held(&'a mut YouTube),
    /// Taken from the program afresh for each step.
    Shared(&'a App),
}

/// The connection for one step; released when dropped.
pub enum YouTubeStep<'a> {
    Held(&'a mut YouTube),
    Shared(MappedMutexGuard<'a, YouTube>),
}

impl YouTubeAccess<'_> {
    pub async fn youtube(&mut self) -> Result<YouTubeStep<'_>, BoxError> {
        match self {
            YouTubeAccess::Held(yt) => Ok(YouTubeStep::Held(yt)),
            YouTubeAccess::Shared(app) => Ok(YouTubeStep::Shared(app.youtube().await?)),
        }
    }
}

impl Deref for YouTubeStep<'_> {
    type Target = YouTube;

    fn deref(&self) -> &YouTube {
        match self {
            YouTubeStep::Held(yt) => yt,
            YouTubeStep::Shared(guard) => guard,
        }
    }
}

impl DerefMut for YouTubeStep<'_> {
    fn deref_mut(&mut self) -> &mut YouTube {
        match self {
            YouTubeStep::Held(yt) => yt,
            YouTubeStep::Shared(guard) => guard,
        }
    }
}

impl YouTube {
    pub fn new(auth: GoogleAuth, ledger: Option<PathBuf>) -> Result<Self, BoxError> {
        Ok(Self {
            auth,
            http: crate::http_client()?,
            units_used: 0,
            ledger,
        })
    }

    async fn call(
        &mut self,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<Value>,
        cost: u32,
    ) -> Result<Value, BoxError> {
        let token = self.auth.access_token().await?;
        let method_has_body = method == Method::POST || method == Method::PUT;
        let mut request = self
            .http
            .request(method, format!("{API}/{path}"))
            .bearer_auth(token)
            .query(query);
        if let Some(body) = body {
            request = request
                .header(CONTENT_TYPE, "application/json")
                .body(serde_json::to_vec(&body)?);
        } else if method_has_body {
            // Google rejects a POST/PUT that doesn't state its (empty) length.
            request = request.header(CONTENT_LENGTH, "0").body(Vec::new());
        }
        let response = request.send().await?;
        // Google charges for the call whether or not it succeeds.
        self.units_used += cost;
        if let Some(ledger) = &self.ledger {
            if let Err(e) = quota::record_to_file(ledger, cost, OffsetDateTime::now_utc()) {
                warn!("Couldn't save the YouTube allowance count: {e}");
            }
        }
        let status = response.status();
        let text = response.text().await?;
        let value = if text.trim().is_empty() {
            Some(Value::Null)
        } else {
            serde_json::from_str(&text).ok()
        };
        match value {
            Some(value) if status.is_success() => Ok(value),
            Some(value) => {
                let reason = value["error"]["errors"][0]["reason"]
                    .as_str()
                    .unwrap_or("unknown");
                let message = value["error"]["message"].as_str().unwrap_or(&text);
                Err(format!("YouTube refused {path} ({status}, {reason}): {message}").into())
            }
            // Not JSON, e.g. an HTML error page from Google's front end.
            None => {
                let snippet: String = text.chars().take(300).collect();
                Err(
                    format!("Unexpected reply from YouTube for {path} ({status}): {snippet}")
                        .into(),
                )
            }
        }
    }

    /// Collects every page of a list call.
    async fn list_all(
        &mut self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Vec<Value>, BoxError> {
        let mut items = Vec::new();
        let mut page_token = String::new();
        loop {
            let mut q = query.to_vec();
            q.push(("maxResults", "50"));
            if !page_token.is_empty() {
                q.push(("pageToken", &page_token));
            }
            let page = self.call(Method::GET, path, &q, None, LIST_COST).await?;
            if let Some(list) = page["items"].as_array() {
                items.extend(list.iter().cloned());
            }
            match page["nextPageToken"].as_str() {
                Some(next) => page_token = next.to_string(),
                None => return Ok(items),
            }
        }
    }

    /// The signed-in channel's name.
    pub async fn my_channel_title(&mut self) -> Result<String, BoxError> {
        let page = self
            .call(
                Method::GET,
                "channels",
                &[("part", "snippet"), ("mine", "true")],
                None,
                LIST_COST,
            )
            .await?;
        Ok(page["items"][0]["snippet"]["title"]
            .as_str()
            .ok_or("This Google account has no YouTube channel")?
            .to_string())
    }

    pub async fn list_streams(&mut self) -> Result<Vec<StreamInfo>, BoxError> {
        let items = self
            .list_all(
                "liveStreams",
                &[("part", "snippet,status"), ("mine", "true")],
            )
            .await?;
        Ok(items
            .iter()
            .filter_map(|s| {
                Some(StreamInfo {
                    id: s["id"].as_str()?.to_string(),
                    title: s["snippet"]["title"].as_str()?.to_string(),
                    stream_status: s["status"]["streamStatus"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string(),
                })
            })
            .collect())
    }

    pub async fn list_playlists(&mut self) -> Result<Vec<Playlist>, BoxError> {
        let items = self
            .list_all("playlists", &[("part", "snippet"), ("mine", "true")])
            .await?;
        Ok(items
            .iter()
            .filter_map(|p| {
                Some(Playlist {
                    id: p["id"].as_str()?.to_string(),
                    title: p["snippet"]["title"].as_str()?.to_string(),
                })
            })
            .collect())
    }

    pub async fn create_playlist(
        &mut self,
        title: &str,
        description: &str,
        privacy: &str,
    ) -> Result<String, BoxError> {
        let body = json!({
            "snippet": { "title": title, "description": description },
            "status": { "privacyStatus": privacy },
        });
        let created = self
            .call(
                Method::POST,
                "playlists",
                &[("part", "snippet,status")],
                Some(body),
                WRITE_COST,
            )
            .await?;
        id_of(&created)
    }

    pub async fn insert_broadcast(&mut self, spec: &BroadcastSpec) -> Result<String, BoxError> {
        let body = json!({
            "snippet": {
                "title": spec.title,
                "description": spec.description,
                "scheduledStartTime": spec.scheduled_start,
            },
            "status": {
                "privacyStatus": spec.privacy,
                "selfDeclaredMadeForKids": false,
            },
            "contentDetails": {
                // stream-manager decides when each video starts and ends, not YouTube.
                "enableAutoStart": false,
                "enableAutoStop": false,
                "enableDvr": true,
                "recordFromStart": true,
                // Without a preview stream, a video can go straight from ready to live.
                "monitorStream": { "enableMonitorStream": false },
            },
        });
        let created = self
            .call(
                Method::POST,
                "liveBroadcasts",
                &[("part", "snippet,status,contentDetails")],
                Some(body),
                WRITE_COST,
            )
            .await?;
        id_of(&created)
    }

    /// Updates title, description and scheduled start of an existing video.
    pub async fn update_broadcast(
        &mut self,
        id: &str,
        spec: &BroadcastSpec,
    ) -> Result<(), BoxError> {
        let body = json!({
            "id": id,
            "snippet": {
                "title": spec.title,
                "description": spec.description,
                "scheduledStartTime": spec.scheduled_start,
            },
        });
        self.call(
            Method::PUT,
            "liveBroadcasts",
            &[("part", "snippet")],
            Some(body),
            WRITE_COST,
        )
        .await?;
        Ok(())
    }

    /// Connects a video to one of the court's stream keys.
    pub async fn bind_broadcast(
        &mut self,
        broadcast_id: &str,
        stream_id: &str,
    ) -> Result<(), BoxError> {
        self.call(
            Method::POST,
            "liveBroadcasts/bind",
            &[
                ("id", broadcast_id),
                ("part", "id,contentDetails"),
                ("streamId", stream_id),
            ],
            None,
            WRITE_COST,
        )
        .await?;
        Ok(())
    }

    pub async fn add_to_playlist(
        &mut self,
        playlist_id: &str,
        video_id: &str,
    ) -> Result<(), BoxError> {
        let body = json!({
            "snippet": {
                "playlistId": playlist_id,
                "resourceId": { "kind": "youtube#video", "videoId": video_id },
            },
        });
        self.call(
            Method::POST,
            "playlistItems",
            &[("part", "snippet")],
            Some(body),
            WRITE_COST,
        )
        .await?;
        Ok(())
    }

    /// Current state of up to 50 videos: (id, title, lifecycle status, privacy, bound stream id).
    pub async fn broadcast_statuses(&mut self, ids: &[&str]) -> Result<Vec<[String; 5]>, BoxError> {
        let joined = ids.join(",");
        let page = self
            .call(
                Method::GET,
                "liveBroadcasts",
                &[
                    ("part", "snippet,status,contentDetails"),
                    ("id", &joined),
                    ("maxResults", "50"),
                ],
                None,
                LIST_COST,
            )
            .await?;
        let text = |v: &Value| v.as_str().unwrap_or("-").to_string();
        Ok(page["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|b| {
                        [
                            text(&b["id"]),
                            text(&b["snippet"]["title"]),
                            text(&b["status"]["lifeCycleStatus"]),
                            text(&b["status"]["privacyStatus"]),
                            text(&b["contentDetails"]["boundStreamId"]),
                        ]
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// One video's lifecycle status (`ready`, `testing`, `live`, `complete`, …) and its live chat
    /// id (only present while the video can receive chat).
    pub async fn broadcast_info(&mut self, id: &str) -> Result<BroadcastInfo, BoxError> {
        let page = self
            .call(
                Method::GET,
                "liveBroadcasts",
                &[("part", "snippet,status"), ("id", id)],
                None,
                LIST_COST,
            )
            .await?;
        let item = &page["items"][0];
        if item.is_null() {
            return Err(format!("Video {id} no longer exists on YouTube").into());
        }
        Ok(BroadcastInfo {
            life_cycle: item["status"]["lifeCycleStatus"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            live_chat_id: item["snippet"]["liveChatId"].as_str().map(str::to_string),
        })
    }

    /// Whether YouTube is receiving video on a stream key: `active` once vMix is sending.
    pub async fn stream_status(&mut self, stream_id: &str) -> Result<String, BoxError> {
        let page = self
            .call(
                Method::GET,
                "liveStreams",
                &[("part", "status"), ("id", stream_id)],
                None,
                LIST_COST,
            )
            .await?;
        Ok(page["items"][0]["status"]["streamStatus"]
            .as_str()
            .unwrap_or("unknown")
            .to_string())
    }

    /// Moves a video to `live` or `complete`.
    pub async fn transition(&mut self, id: &str, to: &str) -> Result<(), BoxError> {
        self.call(
            Method::POST,
            "liveBroadcasts/transition",
            &[("broadcastStatus", to), ("id", id), ("part", "status")],
            None,
            WRITE_COST,
        )
        .await?;
        Ok(())
    }

    pub async fn post_chat_message(
        &mut self,
        live_chat_id: &str,
        text: &str,
    ) -> Result<(), BoxError> {
        let body = json!({
            "snippet": {
                "liveChatId": live_chat_id,
                "type": "textMessageEvent",
                "textMessageDetails": { "messageText": text },
            },
        });
        self.call(
            Method::POST,
            "liveChat/messages",
            &[("part", "snippet")],
            Some(body),
            WRITE_COST,
        )
        .await?;
        Ok(())
    }

    pub async fn delete_broadcast(&mut self, id: &str) -> Result<(), BoxError> {
        self.call(
            Method::DELETE,
            "liveBroadcasts",
            &[("id", id)],
            None,
            WRITE_COST,
        )
        .await?;
        Ok(())
    }

    pub async fn delete_playlist(&mut self, id: &str) -> Result<(), BoxError> {
        self.call(Method::DELETE, "playlists", &[("id", id)], None, WRITE_COST)
            .await?;
        Ok(())
    }
}

fn id_of(resource: &Value) -> Result<String, BoxError> {
    Ok(resource["id"]
        .as_str()
        .ok_or("YouTube's reply had no id")?
        .to_string())
}
