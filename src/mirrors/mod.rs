pub mod auto_select;
pub mod registry;

pub use auto_select::select_best_mirror;
pub use registry::{Mirror, MirrorId};
