pub mod components;
pub mod events;

use bevy::prelude::*;
use bevy_replicon::{client::ServerEntityTicks, core::replicon_tick::RepliconTick};
use components::ComponentSnapshotBuffer;

pub fn server_populate_buffer<C: Component + Clone>(
    mut query: Query<(&C, &mut ComponentSnapshotBuffer<C>), Changed<C>>,
    replicon_tick: Res<RepliconTick>
) {
    for (c, mut buff) in query.iter_mut() {
        buff.insert(c.clone(), replicon_tick.get());
    }
}

pub fn client_populate_buffer<C: Component + Clone>(
    mut query: Query<(Entity , &C, &mut ComponentSnapshotBuffer<C>), Changed<C>>,
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
                    warn!("server tick is not mapped for this entity: {e:?}");
                }
            }
        }
    }
}
