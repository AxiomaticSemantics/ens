//! This module is separated into its own crate to enable simple dynamic linking for Bevy, and should not be used directly

mod default_plugins;
pub use default_plugins::*;

pub mod app {
    //! Build bevy apps, create plugins, and read events.
    pub use bevy_app::*;
}

pub mod core {
    //! Contains core plugins.
    pub use bevy_core::*;
}

#[cfg(feature = "bevy_diagnostic")]
pub mod diagnostic {
    //! Useful diagnostic plugins and types for bevy apps.
    pub use bevy_diagnostic::*;
}

pub mod ecs {
    //! Bevy's entity-component-system.
    pub use bevy_ecs::*;
}

pub mod ptr {
    //! Utilities for working with untyped pointers in a more safe way.
    pub use bevy_ptr::*;
}

pub mod tasks {
    //! Pools for async, IO, and compute tasks.
    pub use bevy_tasks::*;
}

pub mod time {
    //! Contains time utilities.
    pub use bevy_time::*;
}

#[cfg(feature = "bevy_hierarchy")]
pub mod hierarchy {
    //! Entity hierarchies and property inheritance
    pub use bevy_hierarchy::*;
}

#[cfg(feature = "bevy_utils")]
pub mod utils {
    //! Various miscellaneous utilities for easing development
    pub use bevy_utils::*;
}

#[cfg(feature = "bevy_dynamic_plugin")]
pub mod dynamic_plugin {
    //! Dynamic linking of plugins
    pub use bevy_dynamic_plugin::*;
}
