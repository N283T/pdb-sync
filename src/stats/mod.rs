//! Statistics collection for local PDB files.

pub mod collector;
pub mod remote;
pub mod types;

pub use collector::LocalStatsCollector;
pub use remote::RemoteStatsProvider;
pub use types::*;
