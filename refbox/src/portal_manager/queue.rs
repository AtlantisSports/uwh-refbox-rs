//! On-disk persistence for the portal retry queue.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::ItemId;

/// Top-level envelope for `portal_queue.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueFile {
    pub version: u32,
    pub items: Vec<QueuedItem>,
}

impl QueueFile {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn empty() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            items: Vec::new(),
        }
    }
}

/// Per-game submission record persisted on disk.
///
/// All queued items are implicitly "pending" — i.e. awaiting retry.
/// There is no per-item state enum because the portal client cannot
/// distinguish 409 Conflict, 401 Unauthorised, 5xx or network failure
/// from each other (see the amendment in ADR 011). Stuck-ness is
/// derived from `queued_at` (see `is_item_stuck` in Task 8), and
/// token problems are tracked globally on the `PortalManager` via a
/// separate `verify_token` probe.
///
/// Datetime fields use `time::OffsetDateTime` with serde's
/// RFC 3339 representation (the `time` crate's `serde-human-readable`
/// feature is already enabled workspace-wide).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueuedItem {
    #[serde(flatten)]
    pub id: ItemId,
    pub black_score: u8,
    pub white_score: u8,
    pub stats: String,
    #[serde(with = "time::serde::rfc3339")]
    pub queued_at: OffsetDateTime,
    pub attempts: u32,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_attempt_at: Option<OffsetDateTime>,
    /// When true, the next submit sends `force=true` so the portal
    /// overwrites any existing server-side value. Set by the operator
    /// via the FORCE THIS GAME RESULT button on the attention action
    /// page (see Task 15).
    pub force: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn round_trips_empty_queue() {
        let q = QueueFile::empty();
        let s = serde_json::to_string(&q).unwrap();
        let back: QueueFile = serde_json::from_str(&s).unwrap();
        assert_eq!(q, back);
        assert_eq!(back.version, 1);
        assert!(back.items.is_empty());
    }

    #[test]
    fn round_trips_queue_with_items() {
        let q = QueueFile {
            version: 1,
            items: vec![QueuedItem {
                id: ItemId {
                    event_id: "2026-spring".into(),
                    game_number: "G27".into(),
                },
                black_score: 3,
                white_score: 2,
                stats: "{\"stub\":true}".into(),
                queued_at: datetime!(2026-04-19 14:22:03 UTC),
                attempts: 2,
                last_attempt_at: Some(datetime!(2026-04-19 14:23:15 UTC)),
                force: false,
            }],
        };
        let s = serde_json::to_string_pretty(&q).unwrap();
        let back: QueueFile = serde_json::from_str(&s).unwrap();
        assert_eq!(q, back);
    }
}
