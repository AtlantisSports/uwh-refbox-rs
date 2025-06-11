use derivative::Derivative;
use serde::{Deserialize, Serialize};

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum Color {
    #[derivative(Default)]
    Black,
    White,
}

impl Color {
    pub fn other(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }
}

impl core::fmt::Display for Color {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match *self {
            Self::Black => write!(f, "Black"),
            Self::White => write!(f, "White"),
        }
    }
}
