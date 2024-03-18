use crate::{Main, MainSchedulePlugin, Plugin, Plugins};

#[cfg(feature = "events")]
use crate::PreUpdate;

#[cfg(feature = "states")]
use crate::StateTransition;

use ens::{
    prelude::*,
    schedule::{InternedScheduleLabel, ScheduleBuildSettings, ScheduleLabel},
};

use ens_utils::{intern::Interned, label::DynEq, HashMap, HashSet};

use std::{
    fmt::Debug,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum AppError {
    #[error("duplicate plugin: `{plugin_name:?}`")]
    DuplicatePlugin { plugin_name: String },
}

#[allow(clippy::needless_doctest_main)]
/// A container of app logic and data.
///
/// Bundles together the necessary elements like [`World`] and [`Schedule`] to create
/// an ECS-based application. It also stores a pointer to a [runner function](Self::set_runner).
/// The runner is responsible for managing the application's event loop and applying the
/// [`Schedule`] to the [`World`] to drive application logic.
///
/// # Examples
///
/// Here is a simple "Hello World" Ens app:
///
/// ```
/// # use ens_app::prelude::*;
/// # use ens::prelude::*;
/// #
/// fn main() {
///    App::new()
///        .add_systems(Update, hello_world_system)
///        .run();
/// }
///
/// fn hello_world_system() {
///    println!("hello world");
/// }
/// ```
pub struct App {
    /// The main ECS [`World`] of the [`App`].
    /// This stores and provides access to all the main data of the application.
    /// The systems of the [`App`] will run using this [`World`].
    pub world: World,
    /// The [runner function](Self::set_runner) is primarily responsible for managing
    /// the application's event loop and advancing the [`Schedule`].
    /// Typically, it is not configured manually, but set by one of Bevy's built-in plugins.
    /// See [`ScheduleRunnerPlugin`](crate::schedule_runner::ScheduleRunnerPlugin).
    pub runner: Box<dyn FnOnce(App) + Send>, // Send bound is required to make App Send
    /// The schedule that systems are added to by default.
    ///
    /// The schedule that runs the main loop of schedule execution.
    ///
    /// This is initially set to [`Main`].
    pub main_schedule_label: InternedScheduleLabel,
    plugin_registry: Vec<Box<dyn Plugin>>,
    plugin_name_added: HashSet<Box<str>>,
    /// A private counter to prevent incorrect calls to `App::run()` from `Plugin::build()`
    building_plugin_depth: usize,
    plugins_state: PluginsState,
}

impl Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "App {{ ")?;
        write!(f, "}}")
    }
}

impl Default for App {
    fn default() -> Self {
        let mut app = App::empty();
        app.add_plugins(MainSchedulePlugin);

        #[cfg(feature = "events")]
        app.add_event::<AppExit>();

        app
    }
}

/// Plugins state in the application
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum PluginsState {
    /// Plugins are being added.
    Adding,
    /// All plugins already added are ready.
    Ready,
    /// Finish has been executed for all plugins added.
    Finished,
    /// Cleanup has been executed for all plugins added.
    Cleaned,
}

// Dummy plugin used to temporary hold the place in the plugin registry
struct PlaceholderPlugin;

impl Plugin for PlaceholderPlugin {
    fn build(&self, _app: &mut App) {}
}

impl App {
    /// Creates a new [`App`] with some default structure to enable core engine features.
    /// This is the preferred constructor for most use cases.
    pub fn new() -> App {
        App::default()
    }

    /// Creates a new empty [`App`] with minimal default configuration.
    ///
    /// This constructor should be used if you wish to provide custom scheduling, exit handling, cleanup, etc.
    pub fn empty() -> App {
        let mut world = World::new();
        world.init_resource::<Schedules>();
        Self {
            world,
            runner: Box::new(run_once),
            plugin_registry: Vec::default(),
            plugin_name_added: Default::default(),
            main_schedule_label: Main.intern(),
            building_plugin_depth: 0,
            plugins_state: PluginsState::Adding,
        }
    }

    /// Advances the execution of the [`Schedule`] by one cycle.
    ///
    /// The schedule run by this method is determined by the [`main_schedule_label`](App) field.
    /// By default this is [`Main`].
    ///
    /// # Panics
    ///
    /// The active schedule of the app must be set before this method is called.
    #[inline(always)]
    pub fn update(&mut self) {
        self.world.run_schedule(self.main_schedule_label);

        self.world.clear_trackers();
    }

    /// Starts the application by calling the app's [runner function](Self::set_runner).
    ///
    /// Finalizes the [`App`] configuration. For general usage, see the example on the item
    /// level documentation.
    ///
    /// # `run()` might not return
    ///
    /// Calls to [`App::run()`] will never return on iOS and Web.
    ///
    /// In simple and *headless* applications, one can expect that execution will
    /// proceed, normally, after calling [`run()`](App::run()) but this is not the case for
    /// windowed applications.
    ///
    /// Windowed apps are typically driven by an *event loop* or *message loop* and
    /// some window-manager APIs expect programs to terminate when their primary
    /// window is closed and that event loop terminates – behavior of processes that
    /// do not is often platform dependent or undocumented.
    ///
    /// By default, *Bevy* uses the `winit` crate for window creation.
    ///
    /// # Panics
    ///
    /// Panics if called from `Plugin::build()`, because it would prevent other plugins to properly build.
    pub fn run(&mut self) {
        let mut app = std::mem::replace(self, App::empty());
        if app.building_plugin_depth > 0 {
            panic!("App::run() was called from within Plugin::build(), which is not allowed.");
        }

        let runner = std::mem::replace(&mut app.runner, Box::new(run_once));
        runner(app);
    }

    /// Check the state of all plugins already added to this app. This is usually called by the
    /// event loop, but can be useful for situations where you want to use [`App::update`]
    #[inline]
    pub fn plugins_state(&self) -> PluginsState {
        match self.plugins_state {
            PluginsState::Adding => {
                for plugin in &self.plugin_registry {
                    if !plugin.ready(self) {
                        return PluginsState::Adding;
                    }
                }
                PluginsState::Ready
            }
            state => state,
        }
    }

    /// Run [`Plugin::finish`] for each plugin. This is usually called by the event loop once all
    /// plugins are ready, but can be useful for situations where you want to use [`App::update`].
    pub fn finish(&mut self) {
        // temporarily remove the plugin registry to run each plugin's setup function on app.
        let plugin_registry = std::mem::take(&mut self.plugin_registry);
        for plugin in &plugin_registry {
            plugin.finish(self);
        }
        self.plugin_registry = plugin_registry;
        self.plugins_state = PluginsState::Finished;
    }

    /// Run [`Plugin::cleanup`] for each plugin. This is usually called by the event loop after
    /// [`App::finish`], but can be useful for situations where you want to use [`App::update`].
    pub fn cleanup(&mut self) {
        // temporarily remove the plugin registry to run each plugin's setup function on app.
        let plugin_registry = std::mem::take(&mut self.plugin_registry);
        for plugin in &plugin_registry {
            plugin.cleanup(self);
        }
        self.plugin_registry = plugin_registry;
        self.plugins_state = PluginsState::Cleaned;
    }

    /// Adds a system to the given schedule in this app's [`Schedules`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use ens_app::prelude::*;
    /// # use ens::prelude::*;
    /// #
    /// # let mut app = App::new();
    /// # fn system_a() {}
    /// # fn system_b() {}
    /// # fn system_c() {}
    /// # fn should_run() -> bool { true }
    /// #
    /// app.add_systems(Update, (system_a, system_b, system_c));
    /// app.add_systems(Update, (system_a, system_b).run_if(should_run));
    /// ```
    pub fn add_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        let schedule = schedule.intern();
        let mut schedules = self.world.resource_mut::<Schedules>();

        if let Some(schedule) = schedules.get_mut(schedule) {
            schedule.add_systems(systems);
        } else {
            let mut new_schedule = Schedule::new(schedule);
            new_schedule.add_systems(systems);
            schedules.insert(new_schedule);
        }

        self
    }

    /// Configures a collection of system sets in the provided schedule, adding any sets that do not exist.
    #[track_caller]
    pub fn configure_sets(
        &mut self,
        schedule: impl ScheduleLabel,
        sets: impl IntoSystemSetConfigs,
    ) -> &mut Self {
        let schedule = schedule.intern();
        let mut schedules = self.world.resource_mut::<Schedules>();
        if let Some(schedule) = schedules.get_mut(schedule) {
            schedule.configure_sets(sets);
        } else {
            let mut new_schedule = Schedule::new(schedule);
            new_schedule.configure_sets(sets);
            schedules.insert(new_schedule);
        }
        self
    }

    /// Setup the application to manage events of type `T`.
    ///
    /// This is done by adding a [`Resource`] of type [`Events::<T>`],
    /// and inserting an [`event_update_system`] into [`First`].
    ///
    /// See [`Events`] for defining events.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ens_app::prelude::*;
    /// # use ens::prelude::*;
    /// #
    /// # #[derive(Event)]
    /// # struct MyEvent;
    /// # let mut app = App::new();
    /// #
    /// app.add_event::<MyEvent>();
    /// ```
    ///
    /// [`event_update_system`]: ens::event::event_update_system
    #[cfg(feature = "events")]
    pub fn add_event<T>(&mut self) -> &mut Self
    where
        T: Event,
    {
        if !self.world.contains_resource::<Events<T>>() {
            self.init_resource::<Events<T>>().add_systems(
                PreUpdate,
                ens::event::event_update_system::<T>
                    .in_set(ens::event::EventUpdates)
                    .run_if(ens::event::event_update_condition::<T>),
            );
        }
        self
    }

    /// Inserts a [`Resource`] to the current [`App`] and overwrites any [`Resource`] previously added of the same type.
    ///
    /// A [`Resource`] in Bevy represents globally unique data. [`Resource`]s must be added to Bevy apps
    /// before using them. This happens with [`insert_resource`](Self::insert_resource).
    ///
    /// See [`init_resource`](Self::init_resource) for [`Resource`]s that implement [`Default`] or [`FromWorld`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use ens_app::prelude::*;
    /// # use ens::prelude::*;
    /// #
    /// #[derive(Resource)]
    /// struct MyCounter {
    ///     counter: usize,
    /// }
    ///
    /// App::new()
    ///    .insert_resource(MyCounter { counter: 0 });
    /// ```
    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    /// Inserts a non-send resource to the app.
    ///
    /// You usually want to use [`insert_resource`](Self::insert_resource),
    /// but there are some special cases when a resource cannot be sent across threads.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ens_app::prelude::*;
    /// # use ens::prelude::*;
    /// #
    /// struct MyCounter {
    ///     counter: usize,
    /// }
    ///
    /// App::new()
    ///     .insert_non_send_resource(MyCounter { counter: 0 });
    /// ```
    #[cfg(feature = "non_send")]
    pub fn insert_non_send_resource<R: 'static>(&mut self, resource: R) -> &mut Self {
        self.world.insert_non_send_resource(resource);
        self
    }

    /// Initialize a [`Resource`] with standard starting values by adding it to the [`World`].
    ///
    /// If the [`Resource`] already exists, nothing happens.
    ///
    /// The [`Resource`] must implement the [`FromWorld`] trait.
    /// If the [`Default`] trait is implemented, the [`FromWorld`] trait will use
    /// the [`Default::default`] method to initialize the [`Resource`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use ens_app::prelude::*;
    /// # use ens::prelude::*;
    /// #
    /// #[derive(Resource)]
    /// struct MyCounter {
    ///     counter: usize,
    /// }
    ///
    /// impl Default for MyCounter {
    ///     fn default() -> MyCounter {
    ///         MyCounter {
    ///             counter: 100
    ///         }
    ///     }
    /// }
    ///
    /// App::new()
    ///     .init_resource::<MyCounter>();
    /// ```
    pub fn init_resource<R: Resource + FromWorld>(&mut self) -> &mut Self {
        self.world.init_resource::<R>();
        self
    }

    /// Initialize a non-send [`Resource`] with standard starting values by adding it to the [`World`].
    ///
    /// The [`Resource`] must implement the [`FromWorld`] trait.
    /// If the [`Default`] trait is implemented, the [`FromWorld`] trait will use
    /// the [`Default::default`] method to initialize the [`Resource`].
    #[cfg(feature = "non_send")]
    pub fn init_non_send_resource<R: 'static + FromWorld>(&mut self) -> &mut Self {
        self.world.init_non_send_resource::<R>();
        self
    }

    /// Sets the function that will be called when the app is run.
    ///
    /// The runner function `run_fn` is called only once by [`App::run`]. If the
    /// presence of a main loop in the app is desired, it is the responsibility of the runner
    /// function to provide it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ens_app::prelude::*;
    /// #
    /// fn my_runner(mut app: App) {
    ///     loop {
    ///         println!("In main loop");
    ///         app.update();
    ///     }
    /// }
    ///
    /// App::new()
    ///     .set_runner(my_runner);
    /// ```
    pub fn set_runner(&mut self, run_fn: impl FnOnce(App) + 'static + Send) -> &mut Self {
        self.runner = Box::new(run_fn);
        self
    }

    /// Boxed variant of [`add_plugins`](App::add_plugins) that can be used from a
    /// [`PluginGroup`](super::PluginGroup)
    pub(crate) fn add_boxed_plugin(
        &mut self,
        plugin: Box<dyn Plugin>,
    ) -> Result<&mut Self, AppError> {
        log::debug!("added plugin: {}", plugin.name());
        if plugin.is_unique() && !self.plugin_name_added.insert(plugin.name().into()) {
            Err(AppError::DuplicatePlugin {
                plugin_name: plugin.name().to_string(),
            })?;
        }

        // Reserve that position in the plugin registry. if a plugin adds plugins, they will be correctly ordered
        let plugin_position_in_registry = self.plugin_registry.len();
        self.plugin_registry.push(Box::new(PlaceholderPlugin));

        self.building_plugin_depth += 1;
        let result = catch_unwind(AssertUnwindSafe(|| plugin.build(self)));
        self.building_plugin_depth -= 1;
        if let Err(payload) = result {
            resume_unwind(payload);
        }
        self.plugin_registry[plugin_position_in_registry] = plugin;
        Ok(self)
    }

    /// Checks if a [`Plugin`] has already been added.
    ///
    /// This can be used by plugins to check if a plugin they depend upon has already been
    /// added.
    pub fn is_plugin_added<T>(&self) -> bool
    where
        T: Plugin,
    {
        self.plugin_registry.iter().any(|p| p.is::<T>())
    }

    /// Returns a vector of references to any plugins of type `T` that have been added.
    ///
    /// This can be used to read the settings of any already added plugins.
    /// This vector will be length zero if no plugins of that type have been added.
    /// If multiple copies of the same plugin are added to the [`App`], they will be listed in insertion order in this vector.
    ///
    /// ```
    /// # use ens_app::prelude::*;
    /// # #[derive(Default)]
    /// # struct ImagePlugin {
    /// #    default_sampler: bool,
    /// # }
    /// # impl Plugin for ImagePlugin {
    /// #    fn build(&self, app: &mut App) {}
    /// # }
    /// # let mut app = App::new();
    /// # app.add_plugins(ImagePlugin::default());
    /// let default_sampler = app.get_added_plugins::<ImagePlugin>()[0].default_sampler;
    /// ```
    pub fn get_added_plugins<T>(&self) -> Vec<&T>
    where
        T: Plugin,
    {
        self.plugin_registry
            .iter()
            .filter_map(|p| p.downcast_ref())
            .collect()
    }

    /// Adds one or more [`Plugin`]s.
    ///
    /// One of Ens's core principles is modularity. All Ens features are implemented
    /// as [`Plugin`]s.
    ///
    /// [`Plugin`]s can be grouped into a set by using a [`PluginGroup`].
    ///
    /// There are built-in [`PluginGroup`]s that provide core engine functionality.
    /// The [`PluginGroup`]s available by default are `DefaultPlugins` and `MinimalPlugins`.
    ///
    /// To customize the plugins in the group (reorder, disable a plugin, add a new plugin
    /// before / after another plugin), call [`build()`](super::PluginGroup::build) on the group,
    /// which will convert it to a [`PluginGroupBuilder`](crate::PluginGroupBuilder).
    ///
    /// You can also specify a group of [`Plugin`]s by using a tuple over [`Plugin`]s and
    /// [`PluginGroup`]s. See [`Plugins`] for more details.
    ///
    /// ## Examples
    /// ```
    /// # use ens_app::{prelude::*, PluginGroupBuilder, NoopPluginGroup as MinimalPlugins};
    /// #
    /// # pub struct LogPlugin;
    /// # impl Plugin for LogPlugin {
    /// #     fn build(&self, app: &mut App) {}
    /// # }
    /// App::new()
    ///     .add_plugins(MinimalPlugins);
    /// App::new()
    ///     .add_plugins((MinimalPlugins, LogPlugin));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if one of the plugins was already added to the application.
    ///
    /// [`PluginGroup`]:super::PluginGroup
    #[track_caller]
    pub fn add_plugins<M>(&mut self, plugins: impl Plugins<M>) -> &mut Self {
        if matches!(
            self.plugins_state(),
            PluginsState::Cleaned | PluginsState::Finished
        ) {
            panic!(
                "Plugins cannot be added after App::cleanup() or App::finish() has been called."
            );
        }
        plugins.add_to_app(self);
        self
    }

    /// Adds a new `schedule` to the [`App`].
    ///
    /// # Warning
    /// This method will overwrite any existing schedule with the same label.
    /// To avoid this behavior, use the `init_schedule` method instead.
    pub fn add_schedule(&mut self, schedule: Schedule) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();
        schedules.insert(schedule);

        self
    }

    /// Initializes a new empty `schedule` to the [`App`] under the provided `label` if it does not exists.
    ///
    /// See [`App::add_schedule`] to pass in a pre-constructed schedule.
    pub fn init_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self {
        let label = label.intern();
        let mut schedules = self.world.resource_mut::<Schedules>();
        if !schedules.contains(label) {
            schedules.insert(Schedule::new(label));
        }
        self
    }

    /// Gets read-only access to the [`Schedule`] with the provided `label` if it exists.
    pub fn get_schedule(&self, label: impl ScheduleLabel) -> Option<&Schedule> {
        let schedules = self.world.get_resource::<Schedules>()?;
        schedules.get(label)
    }

    /// Gets read-write access to a [`Schedule`] with the provided `label` if it exists.
    pub fn get_schedule_mut(&mut self, label: impl ScheduleLabel) -> Option<&mut Schedule> {
        let schedules = self.world.get_resource_mut::<Schedules>()?;
        // We need to call .into_inner here to satisfy the borrow checker:
        // it can reason about reborrows using ordinary references but not the `Mut` smart pointer.
        schedules.into_inner().get_mut(label)
    }

    /// Applies the function to the [`Schedule`] associated with `label`.
    ///
    /// **Note:** This will create the schedule if it does not already exist.
    pub fn edit_schedule(
        &mut self,
        label: impl ScheduleLabel,
        f: impl FnOnce(&mut Schedule),
    ) -> &mut Self {
        let label = label.intern();
        let mut schedules = self.world.resource_mut::<Schedules>();

        if schedules.get(label).is_none() {
            schedules.insert(Schedule::new(label));
        }

        let schedule = schedules.get_mut(label).unwrap();
        // Call the function f, passing in the schedule retrieved
        f(schedule);

        self
    }

    /// Applies the provided [`ScheduleBuildSettings`] to all schedules.
    pub fn configure_schedules(
        &mut self,
        schedule_build_settings: ScheduleBuildSettings,
    ) -> &mut Self {
        self.world
            .resource_mut::<Schedules>()
            .configure_schedules(schedule_build_settings);
        self
    }

    /// When doing [ambiguity checking](ScheduleBuildSettings) this
    /// ignores systems that are ambiguous on [`Component`] T.
    ///
    /// This settings only applies to the main world. To apply this to other worlds call the
    /// [corresponding method](World::allow_ambiguous_component) on World
    ///
    /// ## Example
    ///
    /// ```
    /// # use ens::prelude::*;
    /// # use ens::schedule::{LogLevel, ScheduleBuildSettings};
    /// # use ens_app::prelude::*;
    /// # use ens_utils::default;
    ///
    /// #[derive(Component)]
    /// struct A;
    ///
    /// // these systems are ambiguous on A
    /// fn system_1(_: Query<&mut A>) {}
    /// fn system_2(_: Query<&A>) {}
    ///
    /// let mut app = App::new();
    /// app.configure_schedules(ScheduleBuildSettings {
    ///   ambiguity_detection: LogLevel::Error,
    ///   ..default()
    /// });
    ///
    /// app.add_systems(Update, ( system_1, system_2 ));
    /// app.allow_ambiguous_component::<A>();
    ///
    /// // running the app does not error.
    /// app.update();
    /// ```
    pub fn allow_ambiguous_component<T: Component>(&mut self) -> &mut Self {
        self.world.allow_ambiguous_component::<T>();
        self
    }

    /// When doing [ambiguity checking](ScheduleBuildSettings) this
    /// ignores systems that are ambiguous on [`Resource`] T.
    ///
    /// This settings only applies to the main world. To apply this to other worlds call the
    /// [corresponding method](World::allow_ambiguous_resource) on World
    ///
    /// ## Example
    ///
    /// ```
    /// # use ens_app::prelude::*;
    /// # use ens::prelude::*;
    /// # use ens::schedule::{LogLevel, ScheduleBuildSettings};
    /// # use ens_utils::default;
    ///
    /// #[derive(Resource)]
    /// struct R;
    ///
    /// // these systems are ambiguous on R
    /// fn system_1(_: ResMut<R>) {}
    /// fn system_2(_: Res<R>) {}
    ///
    /// let mut app = App::new();
    /// app.configure_schedules(ScheduleBuildSettings {
    ///   ambiguity_detection: LogLevel::Error,
    ///   ..default()
    /// });
    /// app.insert_resource(R);
    ///
    /// app.add_systems(Update, ( system_1, system_2 ));
    /// app.allow_ambiguous_resource::<R>();
    ///
    /// // running the app does not error.
    /// app.update();
    /// ```
    pub fn allow_ambiguous_resource<T: Resource>(&mut self) -> &mut Self {
        self.world.allow_ambiguous_resource::<T>();
        self
    }

    /// Suppress warnings and errors that would result from systems in these sets having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    ///
    /// When possible, do this directly in the `.add_systems(Update, a.ambiguous_with(b))` call.
    /// However, sometimes two independent plugins `A` and `B` are reported as ambiguous, which you
    /// can only suppress as the consumer of both.
    #[track_caller]
    pub fn ignore_ambiguity<M1, M2, S1, S2>(
        &mut self,
        schedule: impl ScheduleLabel,
        a: S1,
        b: S2,
    ) -> &mut Self
    where
        S1: IntoSystemSet<M1>,
        S2: IntoSystemSet<M2>,
    {
        let schedule = schedule.intern();
        let mut schedules = self.world.resource_mut::<Schedules>();

        if let Some(schedule) = schedules.get_mut(schedule) {
            let schedule: &mut Schedule = schedule;
            schedule.ignore_ambiguity(a, b);
        } else {
            let mut new_schedule = Schedule::new(schedule);
            new_schedule.ignore_ambiguity(a, b);
            schedules.insert(new_schedule);
        }

        self
    }
}

fn run_once(mut app: App) {
    while app.plugins_state() == PluginsState::Adding {
        ens_tasks::tick_global_task_pools_on_main_thread();
    }
    app.finish();
    app.cleanup();

    app.update();
}

/// An event that indicates the [`App`] should exit. This will fully exit the app process at the
/// start of the next tick of the schedule.
///
/// You can also use this event to detect that an exit was requested. In order to receive it, systems
/// subscribing to this event should run after it was emitted and before the schedule of the same
/// frame is over. This is important since [`App::run()`] might never return.
///
/// If you don't require access to other components or resources, consider implementing the [`Drop`]
/// trait on components/resources for code that runs on exit. That saves you from worrying about
/// system schedule ordering, and is idiomatic Rust.
#[cfg(feature = "events")]
#[derive(Event, Debug, Clone, Default)]
pub struct AppExit;

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use ens::{
        schedule::{OnEnter, States},
        system::Commands,
    };

    use crate::{App, Plugin};

    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, _app: &mut App) {}
    }
    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, _app: &mut App) {}
    }
    struct PluginC<T>(T);
    impl<T: Send + Sync + 'static> Plugin for PluginC<T> {
        fn build(&self, _app: &mut App) {}
    }
    struct PluginD;
    impl Plugin for PluginD {
        fn build(&self, _app: &mut App) {}
        fn is_unique(&self) -> bool {
            false
        }
    }

    #[test]
    fn can_add_two_plugins() {
        App::new().add_plugins((PluginA, PluginB));
    }

    #[test]
    #[should_panic]
    fn cant_add_twice_the_same_plugin() {
        App::new().add_plugins((PluginA, PluginA));
    }

    #[test]
    fn can_add_twice_the_same_plugin_with_different_type_param() {
        App::new().add_plugins((PluginC(0), PluginC(true)));
    }

    #[test]
    fn can_add_twice_the_same_plugin_not_unique() {
        App::new().add_plugins((PluginD, PluginD));
    }

    #[test]
    #[should_panic]
    fn cant_call_app_run_from_plugin_build() {
        struct PluginRun;
        struct InnerPlugin;
        impl Plugin for InnerPlugin {
            fn build(&self, _: &mut App) {}
        }
        impl Plugin for PluginRun {
            fn build(&self, app: &mut App) {
                app.add_plugins(InnerPlugin).run();
            }
        }
        App::new().add_plugins(PluginRun);
    }

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum AppState {
        #[default]
        MainMenu,
    }
    fn bar(mut commands: Commands) {
        commands.spawn_empty();
    }

    fn foo(mut commands: Commands) {
        commands.spawn_empty();
    }

    #[test]
    fn add_systems_should_create_schedule_if_it_does_not_exist() {
        let mut app = App::new();
        app.init_state::<AppState>()
            .add_systems(OnEnter(AppState::MainMenu), (foo, bar));

        app.world.run_schedule(OnEnter(AppState::MainMenu));
        assert_eq!(app.world.entities().len(), 2);
    }

    #[test]
    fn add_systems_should_create_schedule_if_it_does_not_exist2() {
        let mut app = App::new();
        app.add_systems(OnEnter(AppState::MainMenu), (foo, bar))
            .init_state::<AppState>();

        app.world.run_schedule(OnEnter(AppState::MainMenu));
        assert_eq!(app.world.entities().len(), 2);
    }

    /// Custom runners should be in charge of when `app::update` gets called as they may need to
    /// coordinate some state.
    /// bug: <https://github.com/bevyengine/bevy/issues/10385>
    /// fix: <https://github.com/bevyengine/bevy/pull/10389>
    #[test]
    fn regression_test_10385() {
        use crate::PreUpdate;
        use ens::{access::Res, system::Resource};

        #[derive(Resource)]
        struct MyState {}

        fn my_runner(mut app: App) {
            let my_state = MyState {};
            app.world.insert_resource(my_state);

            for _ in 0..5 {
                app.update();
            }
        }

        fn my_system(_: Res<MyState>) {
            // access state during app update
        }

        // Should not panic due to missing resource
        App::new()
            .set_runner(my_runner)
            .add_systems(PreUpdate, my_system)
            .run();
    }
}
