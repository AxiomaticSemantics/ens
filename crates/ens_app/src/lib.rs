//! This crate is about everything concerning the highest-level, application layer of an Ens
//! application.

mod app;
mod main_schedule;
mod plugin;
mod plugin_group;
mod schedule_runner;

pub use app::*;
pub use ens_derive::DynamicPlugin;
pub use main_schedule::*;
pub use plugin::*;
pub use plugin_group::*;
pub use schedule_runner::*;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        app::App,
        main_schedule::{
            //First, FixedFirst, FixedLast, FixedPostUpdate, FixedPreUpdate, FixedUpdate,
            Last,
            Main,
            PostStartup,
            PostUpdate,
            PreStartup,
            PreUpdate,
            Startup,
            StateTransition,
            Update,
        },
        DynamicPlugin, Plugin, PluginGroup,
    };
}