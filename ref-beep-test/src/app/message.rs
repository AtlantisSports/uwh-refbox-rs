use super::super::snapshot::BeepTestSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Reset,
    Start,
    Stop,
    NewSnapshot(BeepTestSnapshot),
    NoAction, // TODO: Remove once UI is functional
}

impl Message {
    pub fn is_repeatable(&self) -> bool {
        match self {
            Self::NoAction => true,
            Self::Start | Self::Reset | Self::Stop | Self::NewSnapshot(_) => false,
        }
    }
}
