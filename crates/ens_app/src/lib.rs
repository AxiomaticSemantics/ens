//! This crate is about everything concerning the highest-level, application layer of an Ens
//! application.

mod app;
mod main_schedule;
mod plugin;
mod plugin_group;
mod schedule_runner;

#[cfg(feature = "multi-treaded")]
mod task_pool_plugin;

pub use app::*;
pub use main_schedule::*;
pub use plugin::*;
pub use plugin_group::*;
pub use schedule_runner::*;

#[cfg(feature = "multi-treaded")]
pub use task_pool_plugin::*;

#[cfg(feature = "derive")]
pub use ens_derive::DynamicPlugin;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        app::App,
        main_schedule::{Main, PostUpdate, PreUpdate, Update},
        Plugin, PluginGroup,
    };

    #[cfg(feature = "multi-threaded")]
    pub use create::task_pool_plugin::TaskPoolPlugin;

    #[cfg(feature = "startup")]
    pub use crate::main_schedule::{PostStartup, PreStartup, Startup};

    #[cfg(feature = "states")]
    pub use crate::main_schedule::StateTransition;
}
