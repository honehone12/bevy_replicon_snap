pub mod component_snapshots;
pub mod event_snapshots;

use bevy::prelude::*;
use bevy_replicon::{client::ServerEntityTicks, core::replicon_tick::RepliconTick, network_event::client_event::FromClient};
use component_snapshots::ComponentSnapshotBuffer;
use serde::{Serialize, de::DeserializeOwned};
use crate::{EventSnapshotBuffer, EventSnapshotClientMap, IndexedEvent};

pub(crate) fn server_populate_component_buffer<C: Component + Clone>(
    mut query: Query<
        (&C, &mut ComponentSnapshotBuffer<C>), 
        Or<(Added<C>, Changed<C>)>
    >,
    replicon_tick: Res<RepliconTick>
) {
    for (c, mut buff) in query.iter_mut() {
        buff.insert(c.clone(), replicon_tick.get());
    }
}

pub(crate) fn client_populate_component_buffer<C: Component + Clone>(
    mut query: Query<
        (Entity , &C, &mut ComponentSnapshotBuffer<C>), 
        Or<(Added<C>, Changed<C>)>
    >,
    server_tick: Res<ServerEntityTicks>,
) {
    for (e, c, mut buff) in query.iter_mut() {
        match server_tick.get(&e) {
            Some(tick) => {
                buff.insert(c.clone(), tick.get());
            }
            None => {
                if cfg!(debug_assertions) {
                    panic!("server tick is not mapped for this entity: {e:?}");
                } else {
                    warn!("server tick is not mapped for this entity: {e:?}, discarding...");
                }
            }
        }
    }
}

pub(crate) fn server_populate_client_event_buffer<E>(
    mut events: EventReader<FromClient<E>>,
    mut buffer: ResMut<EventSnapshotClientMap<E>>,
    replicon_tick: Res<RepliconTick>
) 
where E: IndexedEvent + Serialize + DeserializeOwned + Clone {
    for FromClient { client_id, event } in events.read() {
        buffer.insert(client_id, event.clone(), replicon_tick.get())
    }
}

pub(crate) fn client_populate_client_event_buffer<E>(
    mut query: Query<(Entity, &mut EventSnapshotBuffer<E>)>,
    mut events: EventReader<E>,
    server_ticks: Res<ServerEntityTicks>
)
where E: IndexedEvent + Serialize + DeserializeOwned + Clone {
    for event in events.read() {
        for (e, mut buff) in query.iter_mut() {
            match server_ticks.get(&e) {
                Some(tick) => {
                    buff.insert(event.clone(), tick.get());
                }
                None => {
                    if cfg!(debug_assertions) {
                        panic!("server tick is not mapped for this entity: {e:?}");
                    } else {
                        warn!("server tick is not mapped for this entity: {e:?}, discarding...");
                    }   
                }
            }
        }
    }
}
