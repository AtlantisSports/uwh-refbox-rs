use arrayref::array_ref;
use derivative::Derivative;
#[cfg(feature = "std")]
use enum_derive_2018::{EnumDisplay, EnumFromStr};
#[cfg(feature = "std")]
use macro_attr_2018::macro_attr;
use serde_derive::{Deserialize, Serialize};
use uwh_common::game_snapshot::{DecodingError, EncodingError, GameSnapshotNoHeap};

#[cfg(feature = "std")]
macro_attr! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Derivative, EnumDisplay!, EnumFromStr!)]
    #[derivative(Default)]
    pub enum Brightness {
        #[derivative(Default)]
        Low,
        Medium,
        High,
        Outdoor,
    }
}

#[cfg(not(feature = "std"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub enum Brightness {
    #[derivative(Default)]
    Low,
    Medium,
    High,
    Outdoor,
}

impl Brightness {
    pub fn from_u8(val: u8) -> Self {
        match val & 0x03 {
            0x00 => Self::Low,
            0x01 => Self::Medium,
            0x02 => Self::High,
            0x03 => Self::Outdoor,
            _ => unreachable!(),
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Outdoor => 3,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TransmittedData {
    pub white_on_right: bool,
    pub flash: bool,
    pub beep_test: bool,
    pub brightness: Brightness,
    pub snapshot: GameSnapshotNoHeap,
}

impl TransmittedData {
    pub const ENCODED_LEN: usize = GameSnapshotNoHeap::ENCODED_LEN + 1;

    pub fn encode(&self) -> Result<[u8; Self::ENCODED_LEN], EncodingError> {
        let mut val = [0u8; Self::ENCODED_LEN];
        val[0] = (self.brightness.to_u8() << 3)
            | ((self.beep_test as u8) << 2)
            | ((self.flash as u8) << 1)
            | self.white_on_right as u8;
        val[1..].copy_from_slice(&self.snapshot.encode()?);
        Ok(val)
    }

    pub fn decode(bytes: &[u8; Self::ENCODED_LEN]) -> Result<Self, DecodingError> {
        Ok(Self {
            white_on_right: bytes[0] & 0x01 != 0,
            flash: bytes[0] & 0x02 != 0,
            beep_test: bytes[0] & 0x04 != 0,
            brightness: Brightness::from_u8(bytes[0] >> 3),
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
    use uwh_common::{
        bundles::BlackWhiteBundle,
        game_snapshot::{GamePeriod, Infraction, PenaltySnapshot, PenaltyTime, TimeoutSnapshot},
    };

    #[test]
    fn test_serialize_and_desereialize() -> Result<(), Box<dyn std::error::Error>> {
        let state = GameSnapshotNoHeap {
            current_period: GamePeriod::BetweenGames,
            secs_in_period: 0,
            timeout: None,
            scores: BlackWhiteBundle { black: 0, white: 0 },
            penalties: Default::default(),
            is_old_game: true,
        };

        let mut data = TransmittedData {
            white_on_right: true,
            flash: true,
            beep_test: true,
            brightness: Brightness::Low,
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
        data.snapshot.timeout = Some(TimeoutSnapshot::Black(16));
        data.snapshot.scores.black = 2;
        data.snapshot.scores.white = 5;
        data.snapshot.penalties.black.push(PenaltySnapshot {
            player_number: 1,
            time: PenaltyTime::Seconds(48),
            infraction: Infraction::Unknown, // infraction is not encoded, so the test will fail with any other value
        });
        data.snapshot.penalties.white.push(PenaltySnapshot {
            player_number: 12,
            time: PenaltyTime::Seconds(96),
            infraction: Infraction::Unknown, // infraction is not encoded, so the test will fail with any other value
        });

        test_data(&mut data)?;

        data.flash = false;

        test_data(&mut data)?;

        data.white_on_right = true;

        test_data(&mut data)?;

        data.brightness = Brightness::Medium;

        test_data(&mut data)?;

        data.brightness = Brightness::High;

        test_data(&mut data)?;

        data.brightness = Brightness::Outdoor;

        test_data(&mut data)?;

        Ok(())
    }
}
