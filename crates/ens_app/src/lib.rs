//! This crate is about everything concerning the highest-level, application layer of an Ens
//! application.

mod app;
mod main_schedule;
mod plugin;
mod plugin_group;
mod schedule_runner;

mod task_pool_plugin;

pub use app::*;
pub use ens_derive::DynamicPlugin;
pub use main_schedule::*;
pub use plugin::*;
pub use plugin_group::*;
pub use schedule_runner::*;
pub use task_pool_plugin::*;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        app::App,
        main_schedule::{
            Main, PostStartup, PostUpdate, PreStartup, PreUpdate, Startup, StateTransition, Update,
        },
        task_pool_plugin::TaskPoolPlugin,
        DynamicPlugin, Plugin, PluginGroup,
    };
}
