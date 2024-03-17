use ens::{
    component::Component,
    entity::{Entity, EntityMapper, MapEntities},
    world::{FromWorld, World},
};
use std::ops::Deref;

/// Holds a reference to the parent entity of this entity.
/// This component should only be present on entities that actually have a parent entity.
///
/// Parent entity must have this entity stored in its [`Children`] component.
/// It is hard to set up parent/child relationships manually,
/// consider using higher level utilities like [`BuildChildren::with_children`].
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: ens::system::Query
/// [`Children`]: super::children::Children
/// [`BuildChildren::with_children`]: crate::child_builder::BuildChildren::with_children
#[derive(Component, Debug, Eq, PartialEq)]
pub struct Parent(pub(crate) Entity);

impl Parent {
    /// Gets the [`Entity`] ID of the parent.
    #[inline(always)]
    pub fn get(&self) -> Entity {
        self.0
    }

    /// Gets the parent [`Entity`] as a slice of length 1.
    ///
    /// Useful for making APIs that require a type or homogeneous storage
    /// for both [`Children`] & [`Parent`] that is agnostic to edge direction.
    ///
    /// [`Children`]: super::children::Children
    #[inline(always)]
    pub fn as_slice(&self) -> &[Entity] {
        std::slice::from_ref(&self.0)
    }
}

impl FromWorld for Parent {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        Parent(Entity::PLACEHOLDER)
    }
}

impl MapEntities for Parent {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        self.0 = entity_mapper.map_entity(self.0);
    }
}

impl Deref for Parent {
    type Target = Entity;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
