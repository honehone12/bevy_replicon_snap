use bevy::prelude::*;
use crate::snapshots::component_snapshots::ComponentSnapshotBuffer;

pub trait Interpolate {
    fn interpolate(&self, other: &Self, t: f32) -> Self;
}

#[derive(Component, Default)]
pub struct InterpolatedReplication;

/// Interpolate between snapshots.
pub fn interpolate<C: Component + Interpolate>(
    component: &mut C,
    snapshot_buffer: &ComponentSnapshotBuffer<C>,
    delta_time: f32,
    network_tick_delta: f32
) {
    let buff_len =  snapshot_buffer.len();
    if buff_len < 2 {
        return;
    }

    // network_tick_delta = 100%
    // calc elapsed = ?%
    // into 0.0 ~ 1.0

    let elapsed = snapshot_buffer.age();
    if elapsed > network_tick_delta + delta_time {
        debug!("discarding interpolation for old snapshot... elapsed: {elapsed}");
        return;
    }
    let t = (elapsed / network_tick_delta).clamp(0.0, 1.0);

    let mut iter = snapshot_buffer.iter().rev();
    let latest = iter.next().unwrap(); //buffer is longer than 2
    let second = iter.next().unwrap();

    info!("performing interpolare at t: {t}");
    *component = second.component().interpolate(latest.component(), t);
}

/// Advances the snapshot buffer time for entities.
pub(crate) fn add_snapshots_age_system<C: Component>(
    mut q: Query<&mut ComponentSnapshotBuffer<C>>,
    time: Res<Time>,
) {
    for mut snapshot_buffer in q.iter_mut() {
        snapshot_buffer.add_age(time.delta_seconds());
    }
}
