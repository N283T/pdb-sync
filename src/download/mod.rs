pub mod aria2c;
pub mod engine;
pub mod https;
pub mod task;

pub use aria2c::Aria2cDownloader;
pub use engine::EngineType;
pub use https::{DownloadOptions, HttpsDownloader};
pub use task::{DownloadResult, DownloadTask};
