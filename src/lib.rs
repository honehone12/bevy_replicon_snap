use std::collections::vec_deque::Iter;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::Cursor;

use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_replicon::bincode;
use bevy_replicon::bincode::deserialize_from;
use bevy_replicon::client::client_mapper::ServerEntityMap;
use bevy_replicon::core::replication_fns::{
    self, ComponentFns, DeserializeFn, RemoveFn, SerializeFn
};
use bevy_replicon::core::replicon_channels::RepliconChannel;
use bevy_replicon::core::replicon_tick::RepliconTick;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::renet::{transport::NetcodeClientTransport, RenetClient};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub use bevy_replicon_snap_macros;

pub struct RepliconSnapPlugin;

#[derive(Resource, Serialize, Deserialize, Debug)]
pub struct RepliconSnapConfig {
    /// Should reflect the server max tick rate
    pub max_tick_rate: u16,
    pub max_buffer_size: usize
}

/// Sets for interpolation systems.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum InterpolationSet {
    /// Systems that initializes buffers and flag components for replicated entities.
    ///
    /// Runs in `PreUpdate`.
    Init,
    /// Systems that calculating interpolation.
    ///
    /// Runs in `PreUpdate`.
    Interpolate,
}

impl Plugin for RepliconSnapPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Interpolated>()
            .register_type::<OwnerPredicted>()
            .register_type::<NetworkOwner>()
            .register_type::<Predicted>()
            .replicate::<NetworkOwner>()
            .replicate::<OwnerPredicted>()
            .configure_sets(PreUpdate, InterpolationSet::Init.after(ClientSet::Receive))
            .configure_sets(
                PreUpdate,
                InterpolationSet::Interpolate.after(InterpolationSet::Init),
            )
            .add_systems(
                Update,
                owner_prediction_init_system
                    .run_if(resource_exists::<NetcodeClientTransport>)
                    .in_set(InterpolationSet::Init),
            );
    }
}

#[derive(Component, Deserialize, Serialize, Reflect)]
pub struct Interpolated;

#[derive(Component, Deserialize, Serialize, Reflect)]
pub struct OwnerPredicted;

#[derive(Component, Deserialize, Serialize, Reflect)]
pub struct NetworkOwner(u64);

impl NetworkOwner {
    #[inline]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[inline]
    pub fn get(&self) -> u64 {
        self.0
    }
}

#[derive(Component, Reflect)]
pub struct Predicted;

pub trait Interpolate {
    fn interpolate(&self, other: Self, t: f32) -> Self;
}

#[derive(Deserialize, Serialize, Reflect)]
pub struct ComponentSnapshot<T: Component + Interpolate + Clone> {
    tick: u32,
    value: T,
}

impl<T: Component + Interpolate + Clone> ComponentSnapshot<T> {
    #[inline]
    pub fn new(value: T, tick: u32) -> Self {
        Self { 
            tick, 
            value 
        }
    }

    #[inline]
    pub fn tick(&self) -> u32 {
        self.tick
    }

    #[inline]
    pub fn value(&self) -> &T {
        &self.value
    }
}

#[derive(Component, Deserialize, Serialize, Reflect)]
pub struct ComponentSnapshotBuffer<T: Component + Interpolate + Clone> {
    buffer: VecDeque<ComponentSnapshot<T>>,
    time_since_last_snapshot: f32,
    latest_snapshot_tick: u32,
    max_buffer_size: usize
}

impl<T: Component + Interpolate + Clone> ComponentSnapshotBuffer<T> {
    #[inline]
    pub fn new(max_buffer_size: usize) -> Self {
        Self {
            buffer: VecDeque::new(),
            time_since_last_snapshot: 0.0,
            latest_snapshot_tick: 0,
            max_buffer_size
        }
    }

    #[inline]
    pub fn insert(&mut self, element: T, tick: u32) {
        if self.buffer.len() >= self.max_buffer_size {
            self.buffer.pop_front();
        }

        // !!
        // check tick
        // because transport might be unreliable

        self.buffer.push_back(ComponentSnapshot::new(element, tick));
        self.time_since_last_snapshot = 0.0;
        self.latest_snapshot_tick = tick;
    }

    #[inline]
    pub fn latest_snapshot(&self) -> Option<&ComponentSnapshot<T>> {
        self.buffer.back()
    }

    #[inline]
    pub fn latest_snapshot_tick(&self) -> u32 {
        self.latest_snapshot_tick
    }

    #[inline]
    pub fn at(&self, at: usize) -> Option<&ComponentSnapshot<T>> {
        self.buffer.get(at)
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, ComponentSnapshot<T>> {
        self.buffer.iter()
    }

    #[inline]
    pub fn age(&self) -> f32 {
        self.time_since_last_snapshot
    }
}

pub struct EventSnapshot<T: Event> {
    value: T,
    tick: u32,
    delta_time: f32,
}

impl<T: Event> EventSnapshot<T> {
    #[inline]
    pub fn new(value: T, tick: u32, delta_time: f32) -> Self {
        Self{
            value,
            tick,
            delta_time
        }
    }

    #[inline]
    pub fn value(&self) -> &T {
        &self.value
    }

    #[inline]
    pub fn tick(&self) -> u32 {
        self.tick
    }

    #[inline]
    pub fn delta_time(&self) -> f32 {
        self.delta_time
    } 
}

pub struct EventSnapshotHistory<T: Event> {
    buffer: VecDeque<EventSnapshot<T>>,
    latest_snapshot_tick: u32
}

impl<T: Event> EventSnapshotHistory<T> {
    #[inline]
    pub fn new() -> Self {
        Self { 
            buffer: VecDeque::new(), 
            latest_snapshot_tick: 0 
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[inline]
    pub fn latest_snapshot_tick(&self) -> u32 {
        self.latest_snapshot_tick
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, EventSnapshot<T>> {
        self.buffer.iter()
    }
} 

#[derive(Resource)]
pub struct PredictedEventHistory<T: Event> {
    buffer: HashMap<u64, EventSnapshotHistory<T>>,
    max_buffer_size: usize
}

impl<T: Event> PredictedEventHistory<T> {
    #[inline]
    pub fn new(max_buffer_size: usize) -> PredictedEventHistory<T> {
        Self{
            buffer: default(),
            max_buffer_size
        }
    }

    #[inline]
    pub fn clients_count(&self) -> usize {
        self.buffer.len()
    }

    #[inline]
    pub fn len(&self, client_id: u64) -> usize {
        match self.buffer.get(&client_id) {
            Some(h) => h.len(),
            None => 0
        }
    }

    #[inline] 
    pub fn history(&self, client_id: u64) -> Option<&EventSnapshotHistory<T>> {
        self.buffer.get(&client_id)
    }

    pub fn insert(&mut self, client_id: u64, value: T, tick: u32, delta_time: f32) {
        let history = self.buffer
        .entry(client_id)
        .or_insert(EventSnapshotHistory::new());

        // !!
        // check tick
        // because transport might be unreliable

        if history.buffer.len() >= self.max_buffer_size {
            history.buffer.pop_front();
        }

        history.buffer.push_back(EventSnapshot {
            value,
            tick,
            delta_time,
        });
        history.latest_snapshot_tick = tick;
    }

    pub fn frontier(&mut self, client_id: u64, frontier_tick: u32) 
    -> Iter<'_, EventSnapshot<T>> {
        let history = self.buffer
        .entry(client_id)
        .or_insert(EventSnapshotHistory::new());

        if let Some(last_index) = history.buffer
        .iter()
        .position(|v| v.tick >= frontier_tick) {
            history.buffer.range(last_index..)
        } else {
            history.buffer.range(0..0)
        }
    }
}

fn owner_prediction_init_system(
    q_owners: Query<(Entity, &NetworkOwner), Added<OwnerPredicted>>,
    client: Res<NetcodeClientTransport>,
    mut commands: Commands,
) {
    let client_id = client.client_id();
    for (e, id) in q_owners.iter() {
        if id.0 == client_id.raw() {
            commands.entity(e).insert(Predicted);
        } else {
            commands.entity(e).insert(Interpolated);
        }
    }    
}

/// Initialize snapshot buffers for new entities.
fn component_snapshot_buffer_init_system<T: Component + Interpolate + Clone>(
    mut commands: Commands,
    q_new: Query<Entity, (With<T>, Added<OwnerPredicted>)>,
    snap_config: Res<RepliconSnapConfig>
) {
    // !!
    // no need to cache all components
    // want config for each

    for e in q_new.iter() {
        let buffer = ComponentSnapshotBuffer::<T>::new(snap_config.max_buffer_size);
        commands.entity(e).insert(buffer).log_components();
    }
}

/// Interpolate between snapshots.
fn snapshot_interpolation_system<T: Component + Interpolate + Clone>(
    mut q: Query<(&mut T, &mut ComponentSnapshotBuffer<T>), (With<Interpolated>, Without<Predicted>)>,
    time: Res<Time>,
    snap_config: Res<RepliconSnapConfig>,
) {
    for (mut component, mut snapshot_buffer) in q.iter_mut() {
        let buff_len =  snapshot_buffer.buffer.len();
        if buff_len < 2 {
            continue;
        }

        let elapsed = snapshot_buffer.time_since_last_snapshot;
        let tick_duration = 1.0 / (snap_config.max_tick_rate as f32);
        if elapsed > tick_duration + time.delta_seconds() {
            continue;
        }

        let t = (elapsed / tick_duration).clamp(0., 1.);
        let mut iter = snapshot_buffer.iter().rev();
        let latest = iter.next().unwrap(); //buffer is longer than 2
        let second = iter.next().unwrap();

        *component = second.value.interpolate(latest.value.clone(), t);
        snapshot_buffer.time_since_last_snapshot += time.delta_seconds();
    }
}

/// Advances the snapshot buffer time for predicted entities.
fn predicted_snapshot_system<T: Component + Interpolate + Clone>(
    mut q: Query<&mut ComponentSnapshotBuffer<T>, (Without<Interpolated>, With<Predicted>)>,
    time: Res<Time>,
) {
    for mut snapshot_buffer in q.iter_mut() {
        snapshot_buffer.time_since_last_snapshot += time.delta_seconds();
    }
}

pub fn deserialize_snap_component<C: Clone + Interpolate + Component + DeserializeOwned>(
    entity: &mut EntityWorldMut,
    _entity_map: &mut ServerEntityMap,
    cursor: &mut Cursor<&[u8]>,
    tick: RepliconTick,
) -> bincode::Result<()> {
    let component: C = deserialize_from(cursor)?;
    if let Some(mut buffer) = entity.get_mut::<ComponentSnapshotBuffer<C>>() {
        buffer.insert(component, tick.get());
    } else {
        entity.insert(component);
    }

    Ok(())
}

pub trait RepliconSnapExt {
    /// TODO: Add docs
    fn replicate_interpolated<C>(&mut self) -> &mut Self
    where
        C: Component + Interpolate + Clone + Serialize + DeserializeOwned;

    /// TODO: Add docs
    fn replicate_interpolated_with<C>(
        &mut self,
        serialize: SerializeFn,
        deserialize: DeserializeFn,
        remove: RemoveFn,
    ) -> &mut Self
    where
        C: Component + Interpolate + Clone;

    fn add_client_predicted_event<C>(&mut self, channel: impl Into<RepliconChannel>) -> &mut Self
    where
        C: Event + Serialize + DeserializeOwned + Debug + Clone;
}

impl RepliconSnapExt for App {
    fn replicate_interpolated<C>(&mut self) -> &mut Self
    where
        C: Component + Interpolate + Clone + Serialize + DeserializeOwned,
    {
        self.replicate_interpolated_with::<C>(
            replication_fns::serialize::<C>,
            deserialize_snap_component::<C>,
            replication_fns::remove::<C>,
        )
    }

    fn replicate_interpolated_with<T>(
        &mut self,
        serialize: SerializeFn,
        deserialize: DeserializeFn,
        remove: RemoveFn,
    ) -> &mut Self
    where
        T: Component + Interpolate + Clone,
    {
        self.add_systems(
            PreUpdate,
            (component_snapshot_buffer_init_system::<T>.after(owner_prediction_init_system))
                .in_set(InterpolationSet::Init)
        )
        .add_systems(
            PreUpdate,
            (
                snapshot_interpolation_system::<T>,
                predicted_snapshot_system::<T>,
            )
                .chain()
                .in_set(InterpolationSet::Interpolate)
                .run_if(resource_exists::<RenetClient>),
        );
        unsafe {
            self.replicate_with::<T>(ComponentFns{
                serialize, 
                deserialize, 
                remove
            })
        }
    }

    fn add_client_predicted_event<C>(&mut self, channel: impl Into<RepliconChannel>) -> &mut Self
    where
        C: Event + Serialize + DeserializeOwned + Debug + Clone,
    {
        // !!
        // no need to cache all events
        // want config for each

        let config = self.world.resource::<RepliconSnapConfig>();
        let history = PredictedEventHistory::<C>::new(config.max_buffer_size);
        self.insert_resource(history);
        self.add_client_event::<C>(channel)
    }
}
