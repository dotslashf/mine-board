//! ATCS Soundboard core — pure, headless library.
//!
//! Contains the database layer, models, WAV decoding, mixer math and the
//! real-time-safe clip player. This crate must not depend on Tauri or
//! PipeWire so it can be compiled and tested on any host.

pub mod audio;
pub mod db;
pub mod errors;
pub mod models;
pub mod settings;

pub use errors::{Error, Result};
pub use models::*;

pub const TARGET_SAMPLE_RATE: u32 = 48_000;
pub const TARGET_CHANNELS: u16 = 2;
