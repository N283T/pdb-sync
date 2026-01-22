//! Profile subcommands for managing sync profile presets.

pub mod add;
pub mod list;
pub mod show;

pub use add::run_profile_add;
pub use list::run_profile_list;
pub use show::run_profile_show;
