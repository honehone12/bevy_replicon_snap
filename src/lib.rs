pub mod core;
pub mod snapshots;
pub mod prediction;
pub mod interpolation;
pub mod prelude {
    pub use crate::{
        RepliconSnapExt, RepliconSnapPlugin, RepliconSnapSet,
        core::*,
        prediction::*,
        interpolation::*,
        snapshots::{*, components::*, events::*}
    };
}

use std::{fmt::Debug, io::Cursor};
use bevy::prelude::*;
use bevy_replicon::{
    bincode::{self, Options}, 
    client::client_mapper::ServerEntityMap, 
    core::{
        replication_fns::{
            self, ComponentFns, DeserializeFn, RemoveFn, SerializeFn
        }, 
        replicon_tick::RepliconTick
    }, 
    prelude::*
};
use bevy_replicon_renet::renet::{transport::NetcodeClientTransport, RenetClient};
use serde::{Serialize, de::DeserializeOwned};
use prelude::*;

#[derive(Resource, Debug)]
pub(crate) struct RepliconSnapConfig {
    server_tick_rate: u16
}

/// Sets for interpolation systems.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum RepliconSnapSet {
    /// Systems that initializes buffers and flag components for replicated entities.
    /// Runs in `PreUpdate`.
    Init,
    /// Systems for actual calculation.
    /// Runs in `PreUpdate`.
    Update,
}
pub struct RepliconSnapPlugin {
    pub server_tick_rate: u16
}

impl Plugin for RepliconSnapPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(RepliconSnapConfig{
            server_tick_rate: self.server_tick_rate
        })
        .replicate::<NetworkOwner>()
        .configure_sets(
            PreUpdate, 
            RepliconSnapSet::Init.after(ClientSet::Receive)
        )
        .configure_sets(
            PreUpdate, 
            RepliconSnapSet::Update.after(RepliconSnapSet::Init),
        )
        .add_systems(
            Update,
            init_prediction
            .in_set(RepliconSnapSet::Init)
            .run_if(resource_exists::<NetcodeClientTransport>)
        );
    }
}

pub fn deserialize_snap_component<C: Clone + Interpolate + Component + DeserializeOwned>(
    entity: &mut EntityWorldMut,
    _entity_map: &mut ServerEntityMap,
    cursor: &mut Cursor<&[u8]>,
    tick: RepliconTick,
) -> bincode::Result<()> {
    
    
    // !!
    // need also for server
    // but serialize fn does not have access for the entity

    let component: C = bincode::DefaultOptions::new().deserialize_from(cursor)?;
    if let Some(mut buffer) = entity.get_mut::<ComponentSnapshotBuffer<C>>() {
        buffer.insert(component, tick.get());
    } else {
        entity.insert(component);
    }

    Ok(())
}

pub trait RepliconSnapExt {
    fn use_event_snapshots<E>(&mut self, max_buffer_size: usize) -> &mut Self
    where E: Event;

    fn interpolate_replication<C>(&mut self) -> &mut Self
    where C: Component + Interpolate + Clone + Serialize + DeserializeOwned;

    fn interpolate_replication_with<C>(
        &mut self,
        serialize: SerializeFn,
        deserialize: DeserializeFn,
        remove: RemoveFn,
    ) -> &mut Self
    where C: Component + Interpolate + Clone;
}

impl RepliconSnapExt for App {
    fn use_event_snapshots<C>(&mut self, max_buffer_size: usize) -> &mut Self
    where C: Event {
        let history = EventSnapshotHistory::<C>::new(max_buffer_size);
        self.insert_resource(history)
    }

    fn interpolate_replication<C>(&mut self) -> &mut Self
    where
        C: Component + Interpolate + Clone + Serialize + DeserializeOwned,
    {
        self.interpolate_replication_with::<C>(
            replication_fns::serialize::<C>,
            deserialize_snap_component::<C>,
            replication_fns::remove::<C>,
        )
    }

    fn interpolate_replication_with<C>(
        &mut self,
        serialize: SerializeFn,
        deserialize: DeserializeFn,
        remove: RemoveFn,
    ) -> &mut Self
    where
        C: Component + Interpolate + Clone,
    {
        self.add_systems(
            PreUpdate, (
                add_snapshots_age_system::<C>,
                interpolate_replication_system::<C>,
            )
            .chain()
            .in_set(RepliconSnapSet::Update)
            .run_if(resource_exists::<RenetClient>)
        );
        unsafe {
            self.replicate_with::<C>(ComponentFns{
                serialize, 
                deserialize, 
                remove
            })
        }
    }
}

