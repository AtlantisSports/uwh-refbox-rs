use arrayref::array_ref;
use serde_derive::{Deserialize, Serialize};
use uwh_common::game_snapshot::{DecodingError, EncodingError, GameSnapshotNoHeap};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TransmittedData {
    pub white_on_right: bool,
    pub snapshot: GameSnapshotNoHeap,
}

impl TransmittedData {
    pub const ENCODED_LEN: usize = GameSnapshotNoHeap::ENCODED_LEN + 1;

    pub fn encode(&self) -> Result<[u8; Self::ENCODED_LEN], EncodingError> {
        let mut val = [0u8; Self::ENCODED_LEN];
        val[0] = self.white_on_right as u8;
        val[1..].copy_from_slice(&self.snapshot.encode()?);
        Ok(val)
    }

    pub fn decode(bytes: &[u8; Self::ENCODED_LEN]) -> Result<Self, DecodingError> {
        Ok(Self {
            white_on_right: bytes[0] != 0,
            snapshot: GameSnapshotNoHeap::decode(array_ref![
                bytes,
                1,
                GameSnapshotNoHeap::ENCODED_LEN
            ])?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use arrayvec::ArrayVec;
    use uwh_common::game_snapshot::{GamePeriod, PenaltySnapshot, PenaltyTime, TimeoutSnapshot};

    #[test]
    fn test_serialize_and_desereialize() -> Result<(), Box<dyn std::error::Error>> {
        let state = GameSnapshotNoHeap {
            current_period: GamePeriod::BetweenGames,
            secs_in_period: 0,
            timeout: TimeoutSnapshot::None,
            b_score: 0,
            w_score: 0,
            b_penalties: ArrayVec::new(),
            w_penalties: ArrayVec::new(),
        };

        let mut data = TransmittedData {
            white_on_right: true,
            snapshot: state,
        };

        let test_data = |data: &mut TransmittedData| -> Result<(), Box<dyn std::error::Error>> {
            let serialization = data.encode()?;
            let mut recreated = TransmittedData::decode(array_ref![
                serialization,
                0,
                TransmittedData::ENCODED_LEN
            ])?;
            assert_eq!(data, &mut recreated);
            Ok(())
        };

        test_data(&mut data)?;

        data.white_on_right = false;
        data.snapshot.current_period = GamePeriod::FirstHalf;
        data.snapshot.secs_in_period = 345;
        data.snapshot.timeout = TimeoutSnapshot::Black(16);
        data.snapshot.b_score = 2;
        data.snapshot.w_score = 5;
        data.snapshot.b_penalties.push(PenaltySnapshot {
            player_number: 1,
            time: PenaltyTime::Seconds(48),
        });
        data.snapshot.w_penalties.push(PenaltySnapshot {
            player_number: 12,
            time: PenaltyTime::Seconds(96),
        });

        test_data(&mut data)?;

        Ok(())
    }
}
