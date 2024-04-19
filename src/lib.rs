pub mod core;
pub mod snapshots;
pub mod prediction;
pub mod interpolation;
pub mod prelude {
    pub use crate::{
        RepliconSnapAppExt, RepliconSnapPlugin, RepliconSnapSet,
        core::*,
        prediction::*,
        interpolation::*,
        snapshots::{*, components::*, events::*}
    };
}

use std::fmt::Debug;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
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
        );
    }
}

pub trait RepliconSnapAppExt {
    fn interpolate_replication<C>(&mut self) -> &mut Self
    where C: Component + Interpolate + Serialize + DeserializeOwned;

    fn use_client_event_snapshots<E>(
        &mut self, 
        channel: impl Into<RepliconChannel>,
        max_buffer_size: usize
    ) -> &mut Self
    where E: Event + Serialize + DeserializeOwned;
}

impl RepliconSnapAppExt for App {
    fn interpolate_replication<C>(&mut self) -> &mut Self
    where C: Component + Interpolate + Serialize + DeserializeOwned {
        self.add_systems(
            PreUpdate, (
                add_snapshots_age_system::<C>,
                interpolate_replication_system::<C>,
            )
            .chain()
            .in_set(RepliconSnapSet::Update)
            .run_if(resource_exists::<RepliconClient>)
        );
        self.replicate::<C>()
    }
    
    fn use_client_event_snapshots<E>(
        &mut self, 
        channel: impl Into<RepliconChannel>,    
        max_buffer_size: usize
    ) -> &mut Self 
    where E: Event + Serialize + DeserializeOwned{
        if self.world.contains_resource::<RepliconClient>() {
            self.insert_resource(EventSnapshotBuffer::<E>::new(max_buffer_size));
        } else if self.world.contains_resource::<RepliconServer>() {
            self.insert_resource(EventSnapshotClientMap::<E>::new(max_buffer_size));
        } else {
            panic!("please build replicon server or client before the call")
        }
        self.add_client_event::<E>(channel)
    }

    
}

pub trait ClientEventSnapshotsEventWriterExt {

}

pub trait ClientEventSnapshotsEventReaderExt {

}