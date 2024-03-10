use bevy_app::{Plugin, PluginGroup, PluginGroupBuilder};

/// This plugin group will add all the default plugins for a *Bevy* application:
/// * [`TaskPoolPlugin`](crate::core::TaskPoolPlugin)
/// * [`TypeRegistrationPlugin`](crate::core::TypeRegistrationPlugin)
/// * [`FrameCountPlugin`](crate::core::FrameCountPlugin)
/// * [`TimePlugin`](crate::time::TimePlugin)
/// * [`HierarchyPlugin`](crate::hierarchy::HierarchyPlugin)
/// * [`DiagnosticsPlugin`](crate::diagnostic::DiagnosticsPlugin)
///
/// [`DefaultPlugins`] obeys *Cargo* *feature* flags. Users may exert control over this plugin group
/// by disabling `default-features` in their `Cargo.toml` and enabling only those features
/// that they wish to use.
///
/// [`DefaultPlugins`] contains all the plugins typically required to build
/// a *Bevy* application which includes a *window* and presentation components.
/// For *headless* cases – without a *window* or presentation, see [`MinimalPlugins`].
pub struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();
        group = group
            .add(bevy_core::TaskPoolPlugin::default())
            //.add(bevy_core::TypeRegistrationPlugin)
            //.add(bevy_core::FrameCountPlugin)
            .add(bevy_time::TimePlugin)
            .add(bevy_hierarchy::HierarchyPlugin);
        //.add(bevy_diagnostic::DiagnosticsPlugin);

        group
    }
}

/// This plugin group will add the minimal plugins for a *Bevy* application:
/// * [`TaskPoolPlugin`](crate::core::TaskPoolPlugin)
/// * [`TypeRegistrationPlugin`](crate::core::TypeRegistrationPlugin)
/// * [`FrameCountPlugin`](crate::core::FrameCountPlugin)
/// * [`TimePlugin`](crate::time::TimePlugin)
/// * [`ScheduleRunnerPlugin`](crate::app::ScheduleRunnerPlugin)
///
/// This group of plugins is intended for use for minimal, *headless* programs –
/// see the [*Bevy* *headless* example](https://github.com/bevyengine/bevy/blob/main/examples/app/headless.rs)
/// – and includes a [schedule runner (`ScheduleRunnerPlugin`)](crate::app::ScheduleRunnerPlugin)
/// to provide functionality that would otherwise be driven by a windowed application's
/// *event loop* or *message loop*.
///
/// Windowed applications that wish to use a reduced set of plugins should consider the
/// [`DefaultPlugins`] plugin group which can be controlled with *Cargo* *feature* flags.
pub struct MinimalPlugins;

impl PluginGroup for MinimalPlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();
        group = group
            .add(bevy_core::TaskPoolPlugin::default())
            //.add(bevy_core::TypeRegistrationPlugin)
            //.add(bevy_core::FrameCountPlugin)
            .add(bevy_time::TimePlugin)
            .add(bevy_app::ScheduleRunnerPlugin::default());

        group
    }
}
