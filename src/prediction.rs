use bevy::prelude::*;
use bevy_replicon_renet::renet::transport::NetcodeClientTransport;
use serde::{Serialize, Deserialize};
use crate::NetworkOwner;

#[derive(Component, Serialize, Deserialize)]
pub struct Predicted;

#[derive(Component)]
pub struct OwnerControlling;

pub(crate) fn init_prediction(
    q: Query<(Entity, &NetworkOwner), Added<Predicted>>,
    transport: Res<NetcodeClientTransport>,
    mut commands: Commands,
) {
    for (e, o) in q.iter() {
        if o.get() == transport.client_id().raw() {
            commands.entity(e).insert(OwnerControlling);
        }
    }    
}
