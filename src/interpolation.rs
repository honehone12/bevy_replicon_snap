use bevy::prelude::*;
use crate::{
    RepliconSnapConfig,
    prediction::OwnerControlling, 
    snapshots::components::ComponentSnapshotBuffer
};

pub trait Interpolate {
    fn interpolate(&self, other: &Self, t: f32) -> Self;
}

#[derive(Component, Default)]
pub struct InterpolatedReplication;

/// Interpolate between snapshots.
pub(crate) fn interpolate_replication_system<C: Component + Interpolate>(
    mut query: Query<
        (&mut C, &ComponentSnapshotBuffer<C>), 
        (With<InterpolatedReplication>, Without<OwnerControlling>)
    >,
    time: Res<Time>,
    snap_config: Res<RepliconSnapConfig>,
) {
    for (mut component, snapshot_buffer) in query.iter_mut() {
        let buff_len =  snapshot_buffer.len();
        if buff_len < 2 {
            continue;
        }

        let delta_time = time.delta_seconds();
        let elapsed = snapshot_buffer.age();
        let tick_duration = 1.0 / (snap_config.server_tick_rate as f32);
        if elapsed > tick_duration + delta_time {
            continue;
        }

        let t = (elapsed / tick_duration).clamp(0.0, 1.0);
        let mut iter = snapshot_buffer.iter().rev();
        let latest = iter.next().unwrap(); //buffer is longer than 2
        let second = iter.next().unwrap();

        *component = second.value().interpolate(latest.value(), t);
    }
}

/// Advances the snapshot buffer time for entities.
pub(crate) fn add_snapshots_age_system<C: Component + Interpolate>(
    mut q: Query<&mut ComponentSnapshotBuffer<C>>,
    time: Res<Time>,
) {
    for mut snapshot_buffer in q.iter_mut() {
        snapshot_buffer.add_age(time.delta_seconds());
    }
}
