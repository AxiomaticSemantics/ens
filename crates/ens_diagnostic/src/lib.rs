// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]

//! This crate provides a straightforward solution for integrating diagnostics in the [Bevy game engine](https://bevyengine.org/).
//! It allows users to easily add diagnostic functionality to their Bevy applications, enhancing
//! their ability to monitor and optimize their game's.

mod diagnostic;
mod entity_count_diagnostics_plugin;
mod frame_time_diagnostics_plugin;
mod log_diagnostics_plugin;

pub use diagnostic::*;

pub use entity_count_diagnostics_plugin::EntityCountDiagnosticsPlugin;
pub use frame_time_diagnostics_plugin::FrameTimeDiagnosticsPlugin;
pub use log_diagnostics_plugin::LogDiagnosticsPlugin;

use bevy_app::prelude::*;

/// Adds core diagnostics resources to an App.
#[derive(Default)]
pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, _app: &mut App) {
        _app.init_resource::<DiagnosticsStore>();
    }
}

/// Default max history length for new diagnostics.
pub const DEFAULT_MAX_HISTORY_LENGTH: usize = 120;
