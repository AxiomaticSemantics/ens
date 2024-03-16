//! Types that detect when their internal data mutate.

use crate::{ptr::PtrMut, system::Resource};

use ens_ptr::{Ptr, UnsafeCellDeref};

#[cfg(feature = "change_detection")]
use crate::{
    change_detection::{
        change_detection_impl, change_detection_mut_impl, DetectChanges, DetectChangesMut, Ticks,
        TicksMut,
    },
    component::{Tick, TickCells},
};

#[cfg(feature = "change_detection")]
use std::mem;

use std::ops::{Deref, DerefMut};

macro_rules! impl_methods {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> $name<$($generics),*> {
            /// Consume `self` and return a mutable reference to the
            /// contained value while marking `self` as "changed".
            #[inline]
            pub fn into_inner(mut self) -> &'w mut $target {
                #[cfg(feature = "change_detection")]
                self.set_changed();
                self.value
            }

            /// Returns a `Mut<>` with a smaller lifetime.
            /// This is useful if you have `&mut
            #[doc = stringify!($name)]
            /// <T>`, but you need a `Mut<T>`.
            pub fn reborrow(&mut self) -> Mut<'_, $target> {
                Mut {
                    value: self.value,
                    #[cfg(feature = "change_detection")]
                    ticks: TicksMut {
                        added: self.ticks.added,
                        changed: self.ticks.changed,
                        last_run: self.ticks.last_run,
                        this_run: self.ticks.this_run,
                    }
                }
            }

            /// Maps to an inner value by applying a function to the contained reference, without flagging a change.
            ///
            /// You should never modify the argument passed to the closure -- if you want to modify the data
            /// without flagging a change, consider using [`DetectChangesMut::bypass_change_detection`] to make your intent explicit.
            ///
            /// ```
            /// # use ens::prelude::*;
            /// # #[derive(PartialEq)] pub struct Vec2;
            /// # impl Vec2 { pub const ZERO: Self = Self; }
            /// # #[derive(Component)] pub struct Transform { translation: Vec2 }
            /// // When run, zeroes the translation of every entity.
            /// fn reset_positions(mut transforms: Query<&mut Transform>) {
            ///     for transform in &mut transforms {
            ///         // We pinky promise not to modify `t` within the closure.
            ///         // Breaking this promise will result in logic errors, but will never cause undefined behavior.
            ///         let mut translation = transform.map_unchanged(|t| &mut t.translation);
            ///         // Only reset the translation if it isn't already zero;
            ///         translation.set_if_neq(Vec2::ZERO);
            ///     }
            /// }
            /// # ens::system::assert_is_system(reset_positions);
            /// ```
            pub fn map_unchanged<U: ?Sized>(self, f: impl FnOnce(&mut $target) -> &mut U) -> Mut<'w, U> {
                Mut {
                    value: f(self.value),
                    #[cfg(feature = "change_detection")]
                    ticks: self.ticks,
                }
            }
            /// Allows you access to the dereferenced value of this pointer without immediately
            /// triggering change detection.
            pub fn as_deref_mut(&mut self) -> Mut<'_, <$target as Deref>::Target>
                where $target: DerefMut
            {
                self.reborrow().map_unchanged(|v| v.deref_mut())
            }

        }
    };
}

macro_rules! impl_deref {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),*: ?Sized $(+ $traits)?> Deref for $name<$($generics),*> {
            type Target = $target;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.value
            }
        }

        impl<$($generics),* $(: $traits)?> AsRef<$target> for $name<$($generics),*> {
            #[inline]
            fn as_ref(&self) -> &$target {
                self.deref()
            }
        }


    };
}

macro_rules! impl_deref_mut {
    ($name:ident < $( $generics:tt ),+ >, $target:ty, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> DerefMut for $name<$($generics),*> {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                #[cfg(featgure = "change_detection")]
                self.set_changed();
                self.value
            }
        }

        impl<$($generics),* $(: $traits)?> AsMut<$target> for $name<$($generics),*> {
            #[inline]
            fn as_mut(&mut self) -> &mut $target {
                self.deref_mut()
            }
        }

    };
}

macro_rules! impl_debug {
    ($name:ident < $( $generics:tt ),+ >, $($traits:ident)?) => {
        impl<$($generics),* : ?Sized $(+ $traits)?> std::fmt::Debug for $name<$($generics),*>
            where T: std::fmt::Debug
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(&self.value)
                    .finish()
            }
        }

    };
}

/// Shared borrow of a [`Resource`].
///
/// See the [`Resource`] documentation for usage.
///
/// If you need a unique mutable borrow, use [`ResMut`] instead.
///
/// # Panics
///
/// Panics when used as a [`SystemParameter`](crate::system::SystemParam) if the resource does not exist.
///
/// Use `Option<Res<T>>` instead if the resource might not always exist.
pub struct Res<'w, T: ?Sized + Resource> {
    pub(crate) value: &'w T,
    #[cfg(feature = "change_detection")]
    pub(crate) ticks: Ticks<'w>,
}

impl<'w, T: Resource> Res<'w, T> {
    /// Copies a reference to a resource.
    ///
    /// Note that unless you actually need an instance of `Res<T>`, you should
    /// prefer to just convert it to `&T` which can be freely copied.
    #[allow(clippy::should_implement_trait)]
    pub fn clone(this: &Self) -> Self {
        Self {
            value: this.value,
            #[cfg(feature = "change_detection")]
            ticks: this.ticks.clone(),
        }
    }

    /// Due to lifetime limitations of the `Deref` trait, this method can be used to obtain a
    /// reference of the [`Resource`] with a lifetime bound to `'w` instead of the lifetime of the
    /// struct itself.
    pub fn into_inner(self) -> &'w T {
        self.value
    }
}

impl<'w, T: Resource> From<ResMut<'w, T>> for Res<'w, T> {
    fn from(res: ResMut<'w, T>) -> Self {
        Self {
            value: res.value,
            #[cfg(feature = "change_detection")]
            ticks: res.ticks.into(),
        }
    }
}

impl<'w, 'a, T: Resource> IntoIterator for &'a Res<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}

#[cfg(feature = "change_detection")]
change_detection_impl!(Res<'w, T>, T, Resource);
impl_deref!(Res<'w, T>, T, Resource);
impl_debug!(Res<'w, T>, Resource);

/// Unique mutable borrow of a [`Resource`].
///
/// See the [`Resource`] documentation for usage.
///
/// If you need a shared borrow, use [`Res`] instead.
///
/// # Panics
///
/// Panics when used as a [`SystemParam`](crate::system::SystemParam) if the resource does not exist.
///
/// Use `Option<ResMut<T>>` instead if the resource might not always exist.
pub struct ResMut<'w, T: ?Sized + Resource> {
    pub(crate) value: &'w mut T,
    #[cfg(feature = "change_detection")]
    pub(crate) ticks: TicksMut<'w>,
}

impl<'w, 'a, T: Resource> IntoIterator for &'a ResMut<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}

impl<'w, 'a, T: Resource> IntoIterator for &'a mut ResMut<'w, T>
where
    &'a mut T: IntoIterator,
{
    type Item = <&'a mut T as IntoIterator>::Item;
    type IntoIter = <&'a mut T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        #[cfg(feature = "change_detection")]
        self.set_changed();
        self.value.into_iter()
    }
}

#[cfg(feature = "change_detection")]
change_detection_impl!(ResMut<'w, T>, T, Resource);
#[cfg(feature = "change_detection")]
change_detection_mut_impl!(ResMut<'w, T>, T, Resource);
impl_methods!(ResMut<'w, T>, T, Resource);
impl_deref_mut!(ResMut<'w, T>, T, Resource);
impl_deref!(ResMut<'w, T>, T, Resource);
impl_debug!(ResMut<'w, T>, Resource);

impl<'w, T: Resource> From<ResMut<'w, T>> for Mut<'w, T> {
    /// Convert this `ResMut` into a `Mut`. This allows keeping the change-detection feature of `Mut`
    /// while losing the specificity of `ResMut` for resources.
    fn from(other: ResMut<'w, T>) -> Mut<'w, T> {
        Mut {
            value: other.value,
            #[cfg(feature = "change_detection")]
            ticks: other.ticks,
        }
    }
}

/// Unique borrow of a non-[`Send`] resource.
///
/// Only [`Send`] resources may be accessed with the [`ResMut`] [`SystemParam`](crate::system::SystemParam). In case that the
/// resource does not implement `Send`, this `SystemParam` wrapper can be used. This will instruct
/// the scheduler to instead run the system on the main thread so that it doesn't send the resource
/// over to another thread.
///
/// # Panics
///
/// Panics when used as a `SystemParameter` if the resource does not exist.
///
/// Use `Option<NonSendMut<T>>` instead if the resource might not always exist.
pub struct NonSendMut<'w, T: ?Sized + 'static> {
    pub(crate) value: &'w mut T,
    #[cfg(feature = "change_detection")]
    pub(crate) ticks: TicksMut<'w>,
}

#[cfg(feature = "change_detection")]
change_detection_impl!(NonSendMut<'w, T>, T,);
#[cfg(feature = "change_detection")]
change_detection_mut_impl!(NonSendMut<'w, T>, T,);
impl_methods!(NonSendMut<'w, T>, T,);
impl_deref!(NonSendMut<'w, T>, T,);
impl_deref_mut!(NonSendMut<'w, T>, T,);
impl_debug!(NonSendMut<'w, T>,);

impl<'w, T: 'static> From<NonSendMut<'w, T>> for Mut<'w, T> {
    /// Convert this `NonSendMut` into a `Mut`. This allows keeping the change-detection feature of `Mut`
    /// while losing the specificity of `NonSendMut`.
    fn from(other: NonSendMut<'w, T>) -> Mut<'w, T> {
        Mut {
            value: other.value,
            #[cfg(feature = "change_detection")]
            ticks: other.ticks,
        }
    }
}

/// Shared borrow of an entity's component with access to change detection.
/// Similar to [`Mut`] but is immutable and so doesn't require unique access.
///
/// # Examples
///
/// These two systems produce the same output.
///
/// ```
/// # use ens::change_detection::DetectChanges;
/// # use ens::query::{Changed, With};
/// # use ens::system::Query;
/// # use ens::world::Ref;
/// # use ens_macros::Component;
/// # #[derive(Component)]
/// # struct MyComponent;
///
/// fn how_many_changed_1(query: Query<(), Changed<MyComponent>>) {
///     println!("{} changed", query.iter().count());
/// }
///
/// fn how_many_changed_2(query: Query<Ref<MyComponent>>) {
///     println!("{} changed", query.iter().filter(|c| c.is_changed()).count());
/// }
/// ```
pub struct Ref<'w, T: ?Sized> {
    pub(crate) value: &'w T,
    #[cfg(feature = "change_detection")]
    pub(crate) ticks: Ticks<'w>,
}

impl<'w, T: ?Sized> Ref<'w, T> {
    /// Returns the reference wrapped by this type. The reference is allowed to outlive `self`, which makes this method more flexible than simply borrowing `self`.
    pub fn into_inner(self) -> &'w T {
        self.value
    }

    /// Map `Ref` to a different type using `f`.
    ///
    /// This doesn't do anything else than call `f` on the wrapped value.
    /// This is equivalent to [`Mut::map_unchanged`].
    pub fn map<U: ?Sized>(self, f: impl FnOnce(&T) -> &U) -> Ref<'w, U> {
        Ref {
            value: f(self.value),
            #[cfg(feature = "change_detection")]
            ticks: self.ticks,
        }
    }

    /// Create a new `Ref` using provided values.
    ///
    /// This is an advanced feature, `Ref`s are designed to be _created_ by
    /// engine-internal code and _consumed_ by end-user code.
    ///
    /// - `value` - The value wrapped by `Ref`.
    /// - `added` - A [`Tick`] that stores the tick when the wrapped value was created.
    /// - `changed` - A [`Tick`] that stores the last time the wrapped value was changed.
    /// - `last_run` - A [`Tick`], occurring before `this_run`, which is used
    ///    as a reference to determine whether the wrapped value is newly added or changed.
    /// - `this_run` - A [`Tick`] corresponding to the current point in time -- "now".
    pub fn new(
        value: &'w T,
        #[cfg(feature = "change_detection")] added: &'w Tick,
        #[cfg(feature = "change_detection")] changed: &'w Tick,
        #[cfg(feature = "change_detection")] last_run: Tick,
        #[cfg(feature = "change_detection")] this_run: Tick,
    ) -> Ref<'w, T> {
        Ref {
            value,
            #[cfg(feature = "change_detection")]
            ticks: Ticks {
                added,
                changed,
                last_run,
                this_run,
            },
        }
    }
}

impl<'w, 'a, T> IntoIterator for &'a Ref<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}

#[cfg(feature = "change_detection")]
change_detection_impl!(Ref<'w, T>, T,);
impl_debug!(Ref<'w, T>,);

/// Unique mutable borrow of an entity's component or of a resource.
pub struct Mut<'w, T: ?Sized> {
    pub(crate) value: &'w mut T,
    #[cfg(feature = "change_detection")]
    pub(crate) ticks: TicksMut<'w>,
}

impl<'w, T: ?Sized> Mut<'w, T> {
    /// Creates a new change-detection enabled smart pointer.
    /// In almost all cases you do not need to call this method manually,
    /// as instances of `Mut` will be created by engine-internal code.
    ///
    /// Many use-cases of this method would be better served by [`Mut::map_unchanged`]
    /// or [`Mut::reborrow`].
    ///
    /// - `value` - The value wrapped by this smart pointer.
    /// - `added` - A [`Tick`] that stores the tick when the wrapped value was created.
    /// - `last_changed` - A [`Tick`] that stores the last time the wrapped value was changed.
    ///   This will be updated to the value of `change_tick` if the returned smart pointer
    ///   is modified.
    /// - `last_run` - A [`Tick`], occurring before `this_run`, which is used
    ///   as a reference to determine whether the wrapped value is newly added or changed.
    /// - `this_run` - A [`Tick`] corresponding to the current point in time -- "now".
    pub fn new(
        value: &'w mut T,
        #[cfg(feature = "change_detection")] added: &'w mut Tick,
        #[cfg(feature = "change_detection")] last_changed: &'w mut Tick,
        #[cfg(feature = "change_detection")] last_run: Tick,
        #[cfg(feature = "change_detection")] this_run: Tick,
    ) -> Self {
        Self {
            value,
            #[cfg(feature = "change_detection")]
            ticks: TicksMut {
                added,
                changed: last_changed,
                last_run,
                this_run,
            },
        }
    }
}

impl<'w, T: ?Sized> From<Mut<'w, T>> for Ref<'w, T> {
    fn from(mut_ref: Mut<'w, T>) -> Self {
        Self {
            value: mut_ref.value,
            #[cfg(feature = "change_detection")]
            ticks: mut_ref.ticks.into(),
        }
    }
}

impl<'w, 'a, T> IntoIterator for &'a Mut<'w, T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.value.into_iter()
    }
}

impl<'w, 'a, T> IntoIterator for &'a mut Mut<'w, T>
where
    &'a mut T: IntoIterator,
{
    type Item = <&'a mut T as IntoIterator>::Item;
    type IntoIter = <&'a mut T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        #[cfg(feature = "change_detection")]
        {
            self.set_changed();
        }
        self.value.into_iter()
    }
}

#[cfg(feature = "change_detection")]
change_detection_impl!(Mut<'w, T>, T,);
#[cfg(feature = "change_detection")]
change_detection_mut_impl!(Mut<'w, T>, T,);
impl_methods!(Mut<'w, T>, T,);
impl_deref!(Mut<'w, T>, T,);
impl_deref_mut!(Mut<'w, T>, T,);
impl_debug!(Mut<'w, T>,);

/// Unique mutable borrow of resources or an entity's component.
///
/// Similar to [`Mut`], but not generic over the component type, instead
/// exposing the raw pointer as a `*mut ()`.
///
/// Usually you don't need to use this and can instead use the APIs returning a
/// [`Mut`], but in situations where the types are not known at compile time
/// or are defined outside of rust this can be used.
pub struct MutUntyped<'w> {
    pub(crate) value: PtrMut<'w>,
    #[cfg(feature = "change_detection")]
    pub(crate) ticks: TicksMut<'w>,
}

impl<'w> MutUntyped<'w> {
    /// Returns the pointer to the value, marking it as changed.
    ///
    /// In order to avoid marking the value as changed, you need to call [`bypass_change_detection`](DetectChangesMut::bypass_change_detection).
    #[cfg(feature = "change_detection")]
    #[inline]
    pub fn into_inner(mut self) -> PtrMut<'w> {
        self.set_changed();
        self.value
    }

    #[cfg(not(feature = "change_detection"))]
    #[inline]
    pub fn into_inner(mut self) -> PtrMut<'w> {
        self.value
    }

    /// Returns a [`MutUntyped`] with a smaller lifetime.
    /// This is useful if you have `&mut MutUntyped`, but you need a `MutUntyped`.
    #[inline]
    pub fn reborrow(&mut self) -> MutUntyped {
        MutUntyped {
            value: self.value.reborrow(),
            #[cfg(feature = "change_detection")]
            ticks: TicksMut {
                added: self.ticks.added,
                changed: self.ticks.changed,
                last_run: self.ticks.last_run,
                this_run: self.ticks.this_run,
            },
        }
    }

    /// Returns a pointer to the value without taking ownership of this smart pointer, marking it as changed.
    ///
    /// In order to avoid marking the value as changed, you need to call [`bypass_change_detection`](DetectChangesMut::bypass_change_detection).
    #[inline(always)]
    #[cfg(feature = "change_detection")]
    pub fn as_mut(&mut self) -> PtrMut<'_> {
        self.set_changed();
        self.value.reborrow()
    }

    #[inline(always)]
    #[cfg(not(feature = "change_detection"))]
    pub fn as_mut(&mut self) -> PtrMut<'_> {
        self.value.reborrow()
    }

    /// Returns an immutable pointer to the value without taking ownership.
    #[inline(always)]
    pub fn as_ref(&self) -> Ptr<'_> {
        self.value.as_ref()
    }

    /// Turn this [`MutUntyped`] into a [`Mut`] by mapping the inner [`PtrMut`] to another value,
    /// without flagging a change.
    /// This function is the untyped equivalent of [`Mut::map_unchanged`].
    ///
    /// You should never modify the argument passed to the closure – if you want to modify the data without flagging a change, consider using [`bypass_change_detection`](DetectChangesMut::bypass_change_detection) to make your intent explicit.
    ///
    /// If you know the type of the value you can do
    /// ```no_run
    /// # use ens::access::{Mut, MutUntyped};
    /// # let mut_untyped: MutUntyped = unimplemented!();
    /// // SAFETY: ptr is of type `u8`
    /// mut_untyped.map_unchanged(|ptr| unsafe { ptr.deref_mut::<u8>() });
    /// ```
    pub fn map_unchanged<T: ?Sized>(self, f: impl FnOnce(PtrMut<'w>) -> &'w mut T) -> Mut<'w, T> {
        Mut {
            value: f(self.value),
            #[cfg(feature = "change_detection")]
            ticks: self.ticks,
        }
    }

    /// Transforms this [`MutUntyped`] into a [`Mut<T>`] with the same lifetime.
    ///
    /// # Safety
    /// - `T` must be the erased pointee type for this [`MutUntyped`].
    pub unsafe fn with_type<T>(self) -> Mut<'w, T> {
        Mut {
            // SAFETY: `value` is `Aligned` and caller ensures the pointee type is `T`.
            value: unsafe { self.value.deref_mut() },
            #[cfg(feature = "change_detection")]
            ticks: self.ticks,
        }
    }
}

impl std::fmt::Debug for MutUntyped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("MutUntyped")
            .field(&self.value.as_ptr())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use ens_macros::Resource;
    use ens_ptr::PtrMut;
    use std::ops::{Deref, DerefMut};

    use crate::{
        self as ens,
        access::{Mut, MutUntyped, NonSendMut, Ref, ResMut},
        change_detection::{TicksMut, CHECK_TICK_THRESHOLD, MAX_CHANGE_AGE},
        component::{Component, ComponentTicks, Tick},
        system::{IntoSystem, Query, System},
        world::World,
    };

    use super::{DetectChanges, DetectChangesMut};

    #[derive(Component, PartialEq)]
    struct C;

    #[derive(Resource)]
    struct R;

    #[derive(Resource, PartialEq)]
    struct R2(u8);

    impl Deref for R2 {
        type Target = u8;
        fn deref(&self) -> &u8 {
            &self.0
        }
    }

    impl DerefMut for R2 {
        fn deref_mut(&mut self) -> &mut u8 {
            &mut self.0
        }
    }

    #[test]
    fn mut_from_res_mut() {
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let ticks = TicksMut {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            last_run: Tick::new(3),
            this_run: Tick::new(4),
        };
        let mut res = R {};
        let res_mut = ResMut {
            value: &mut res,
            ticks,
        };

        let into_mut: Mut<R> = res_mut.into();
        assert_eq!(1, into_mut.ticks.added.get());
        assert_eq!(2, into_mut.ticks.changed.get());
        assert_eq!(3, into_mut.ticks.last_run.get());
        assert_eq!(4, into_mut.ticks.this_run.get());
    }

    #[test]
    fn mut_new() {
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(3),
        };
        let mut res = R {};

        let val = Mut::new(
            &mut res,
            &mut component_ticks.added,
            &mut component_ticks.changed,
            Tick::new(2), // last_run
            Tick::new(4), // this_run
        );

        assert!(!val.is_added());
        assert!(val.is_changed());
    }

    #[test]
    fn mut_from_non_send_mut() {
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let ticks = TicksMut {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            last_run: Tick::new(3),
            this_run: Tick::new(4),
        };
        let mut res = R {};
        let non_send_mut = NonSendMut {
            value: &mut res,
            ticks,
        };

        let into_mut: Mut<R> = non_send_mut.into();
        assert_eq!(1, into_mut.ticks.added.get());
        assert_eq!(2, into_mut.ticks.changed.get());
        assert_eq!(3, into_mut.ticks.last_run.get());
        assert_eq!(4, into_mut.ticks.this_run.get());
    }

    #[test]
    fn map_mut() {
        use super::*;
        struct Outer(i64);

        let last_run = Tick::new(2);
        let this_run = Tick::new(3);
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let ticks = TicksMut {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            last_run,
            this_run,
        };

        let mut outer = Outer(0);
        let ptr = Mut {
            value: &mut outer,
            ticks,
        };
        assert!(!ptr.is_changed());

        // Perform a mapping operation.
        let mut inner = ptr.map_unchanged(|x| &mut x.0);
        assert!(!inner.is_changed());

        // Mutate the inner value.
        *inner = 64;
        assert!(inner.is_changed());
        // Modifying one field of a component should flag a change for the entire component.
        assert!(component_ticks.is_changed(last_run, this_run));
    }

    #[test]
    fn set_if_neq() {
        let mut world = World::new();

        world.insert_resource(R2(0));
        // Resources are Changed when first added
        world.increment_change_tick();
        // This is required to update world::last_change_tick
        world.clear_trackers();

        let mut r = world.resource_mut::<R2>();
        assert!(!r.is_changed(), "Resource must begin unchanged.");

        r.set_if_neq(R2(0));
        assert!(
            !r.is_changed(),
            "Resource must not be changed after setting to the same value."
        );

        r.set_if_neq(R2(3));
        assert!(
            r.is_changed(),
            "Resource must be changed after setting to a different value."
        );
    }

    #[test]
    fn as_deref_mut() {
        let mut world = World::new();

        world.insert_resource(R2(0));
        // Resources are Changed when first added
        world.increment_change_tick();
        // This is required to update world::last_change_tick
        world.clear_trackers();

        let mut r = world.resource_mut::<R2>();
        assert!(!r.is_changed(), "Resource must begin unchanged.");

        let mut r = r.as_deref_mut();
        assert!(
            !r.is_changed(),
            "Dereferencing should not mark the item as changed yet"
        );

        r.set_if_neq(3);
        assert!(
            r.is_changed(),
            "Resource must be changed after setting to a different value."
        );
    }
}
