//! StS2 macro dashboard UI. The binary is a thin wrapper; the library form
//! lets `examples/frame_bench.rs` drive the same UI headlessly.

pub use spire_platform as platform;

pub mod actions;
pub mod app;
pub mod bench;
pub mod chrome;
pub mod draft;
pub mod monitor;
pub mod style;
pub mod views;
pub mod widgets;
