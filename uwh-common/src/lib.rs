#![cfg_attr(not(feature = "std"), no_std)]

pub mod game_snapshot;

#[cfg(feature = "prost")]
pub mod sendable_snapshot {
    include!(concat!(env!("OUT_DIR"), "/snapshot.rs"));
}

#[cfg(feature = "std")]
pub mod config;
