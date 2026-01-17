pub mod auto_select;
pub mod registry;

pub use auto_select::{print_mirror_latencies, select_best_mirror};
pub use registry::{Mirror, MirrorId};
