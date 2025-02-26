#![cfg_attr(not(feature = "std"), no_std)]

pub mod color;

pub mod game_snapshot;

pub mod bundles;

#[cfg(feature = "std")]
pub mod config;

#[cfg(feature = "std")]
pub mod uwhportal;

pub mod drawing_support {
    pub const MAX_STRINGABLE_SECS: u16 = 5999;
    pub const MAX_LONG_STRINGABLE_SECS: u32 = 5_999_999;
    pub const MAX_SHORT_STRINGABLE_SECS: u8 = 99;
}
