use tokio::time::{Duration, Instant};
use uwh_common::game_snapshot::{GamePeriod, Infraction, InfractionSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InfractionDetails {
    pub(crate) player_number: Option<u8>,
    pub(crate) start_period: GamePeriod,
    pub(crate) start_time: Duration,
    pub(crate) start_instant: Instant,
    pub(crate) infraction: Infraction,
}

impl InfractionDetails {
    pub fn as_snapshot(&self) -> InfractionSnapshot {
        InfractionSnapshot {
            player_number: self.player_number,
            infraction: self.infraction,
        }
    }
}
