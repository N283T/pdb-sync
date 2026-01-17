pub mod https;
pub mod task;

pub use https::{DownloadOptions, HttpsDownloader};
pub use task::{DownloadResult, DownloadTask};
