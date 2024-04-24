pub mod core;
pub mod snapshots;
pub mod prediction;
pub mod interpolation;
pub mod prelude {
    pub use crate::{
        RepliconSnapAppExt, RepliconSnapPlugin,
        core::*,
        prediction::*,
        interpolation::*,
        snapshots::{*, component_snapshots::*, event_snapshots::*}
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


#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SnapSet {
    ClientOnRecv,
    ClientOnUpdate,
    ServerOnRecv,
    ServerOnSend,
}

pub struct RepliconSnapPlugin {
    pub server_tick_rate: u16
}

impl Plugin for RepliconSnapPlugin {
    fn build(&self, app: &mut App) {
        app
        .configure_sets(
            PreUpdate, 
            SnapSet::ClientOnRecv.after(ClientSet::Receive)
        )
        .configure_sets(
            PostUpdate, 
            SnapSet::ClientOnUpdate
        )
        .configure_sets(
            PreUpdate, 
            SnapSet::ServerOnRecv.after(ServerSet::Receive)
        )
        .configure_sets(
            PostUpdate, 
            SnapSet::ServerOnSend.after(ServerSet::ReceivePackets)
        )
        .insert_resource(RepliconSnapConfig{
            server_tick_rate: self.server_tick_rate
        })
        .replicate::<NetworkOwner>();
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
    where E: IndexedEvent + Serialize + DeserializeOwned + Clone;

    fn use_component_snapshot<C>(
        &mut self
    ) -> &mut Self
    where C: Component + Serialize + DeserializeOwned + Clone; 
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
            .in_set(SnapSet::ClientOnRecv)
            .run_if(resource_exists::<RepliconClient>)
        );
        self.replicate::<C>()
    }
    
    fn use_client_event_snapshots<E>(
        &mut self, 
        channel: impl Into<RepliconChannel>,    
        max_buffer_size: usize
    ) -> &mut Self
    where E: IndexedEvent + Serialize + DeserializeOwned + Clone {
        if self.world.contains_resource::<RepliconServer>() {
            self
            .insert_resource(EventSnapshotClientMap::<E>::new(max_buffer_size))
            .add_systems(
                PreUpdate, 
                server_populate_client_event_buffer::<E>
                .in_set(SnapSet::ServerOnRecv)
            );
        }
        if self.world.contains_resource::<RepliconClient>() {
            self.add_systems(
                PostUpdate, 
                client_populate_client_event_buffer::<E>
                .in_set(SnapSet::ClientOnUpdate)
            );
        }
        self.add_client_event::<E>(channel)
    }

    fn use_component_snapshot<C>(
        &mut self
    ) -> &mut Self
    where C: Component + Serialize + DeserializeOwned + Clone {
        if self.world.contains_resource::<RepliconServer>() {
            self.add_systems(
                PostUpdate, 
                server_populate_component_buffer::<C>
                .in_set(SnapSet::ServerOnSend)
            );
        }
        if self.world.contains_resource::<RepliconClient>() {
            self.add_systems(
                PreUpdate,
                client_populate_component_buffer::<C>
                .in_set(SnapSet::ClientOnRecv)
            );
        }
        self
    }
}
