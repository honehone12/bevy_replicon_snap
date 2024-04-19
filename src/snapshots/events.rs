use std::collections::{VecDeque, vec_deque::Iter};
use bevy::{prelude::*, utils::HashMap};
use bevy_replicon::core::ClientId;

pub struct EventSnapshot<E: Event> {
    event: E,
    id: usize,
    tick: u32
}

impl<E: Event> EventSnapshot<E> {
    #[inline]
    pub fn new(event: E, id: usize, tick: u32) -> Self {
        Self{
            event,
            id,
            tick
        }
    }

    #[inline]
    pub fn value(&self) -> &E {
        &self.event
    }

    #[inline]
    pub fn tick(&self) -> u32 {
        self.tick
    }

    #[inline]
    pub fn id(&self) -> usize {
        self.id
    } 
}

pub struct EventSnapshotBufferInner<E: Event> {
    buffer: VecDeque<EventSnapshot<E>>,
    latest_snapshot_tick: u32
}

impl<E: Event> EventSnapshotBufferInner<E> {
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self { 
            buffer: VecDeque::with_capacity(capacity), 
            latest_snapshot_tick: 0 
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[inline]
    pub fn latest_snapshot(&self) -> Option<&EventSnapshot<E>> {
        self.buffer.back()
    }

    #[inline]
    pub fn latest_snapshot_tick(&self) -> u32 {
        self.latest_snapshot_tick
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, EventSnapshot<E>> {
        self.buffer.iter()
    }

    #[inline]
    pub fn sort_with_id(&mut self) {
        self.buffer.make_contiguous().sort_by_key(|s| s.id);
    }

    #[inline]
    fn pop_front(&mut self) {
        self.buffer.pop_front();
    }

    #[inline]
    fn insert(&mut self, snapshot: EventSnapshot<E>) {
        if snapshot.tick < self.latest_snapshot_tick {
            warn!(
                "discarding a old event snapshot with tick:{}, latest:{}", 
                snapshot.tick, self.latest_snapshot_tick
            );
            return;
        }

        self.latest_snapshot_tick = snapshot.tick;
        self.buffer.push_back(snapshot);
    }
}

#[derive(Resource)]
pub struct EventSnapshotBuffer<E: Event> {
    buffer: EventSnapshotBufferInner<E>,
    max_buffer_size: usize
}

impl<E: Event> EventSnapshotBuffer<E> {
    #[inline]
    pub fn new(max_buffer_size: usize) -> Self {
        Self{
            buffer: EventSnapshotBufferInner::with_capacity(max_buffer_size),
            max_buffer_size
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[inline]
    pub fn insert(&mut self, event: E, event_id: usize, tick: u32) {
        if self.max_buffer_size == 0 {
            return;
        }

        if self.buffer.len() >= self.max_buffer_size {
            self.buffer.pop_front();
        }

        self.buffer.insert(EventSnapshot::new(event, event_id, tick));
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, EventSnapshot<E>> {
        self.buffer.iter()
    }

    #[inline]
    pub fn sort_with_id(&mut self) {
        self.buffer.sort_with_id();
    }
}

#[derive(Resource)]
pub struct EventSnapshotClientMap<E: Event> {
    buffer: HashMap<ClientId, EventSnapshotBufferInner<E>>,
    max_buffer_size: usize
}

impl<E: Event> EventSnapshotClientMap<E> {
    #[inline]
    pub fn new(max_buffer_size: usize) -> Self {
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
    pub fn len(&self, client_id: ClientId) -> usize {
        match self.buffer.get(&client_id) {
            Some(h) => h.len(),
            None => 0
        }
    }

    #[inline]
    pub fn insert(&mut self, client_id: ClientId, event: E, event_id: usize, tick: u32) {
        let history = self.buffer
        .entry(client_id)
        .or_insert(EventSnapshotBufferInner::with_capacity(self.max_buffer_size));

        if self.max_buffer_size == 0 {
            return;
        }

        if history.buffer.len() >= self.max_buffer_size {
            history.pop_front();
        }

        history.insert(EventSnapshot::new(event, event_id, tick));
    }

    #[inline]
    pub fn iter(&mut self, client_id: ClientId) -> Iter<'_, EventSnapshot<E>> {
        self.buffer
        .entry(client_id)
        .or_insert(EventSnapshotBufferInner::with_capacity(self.max_buffer_size))
        .iter()
    }

    #[inline]
    pub fn sort_with_id(&mut self, client_id: ClientId) {
        self.buffer
        .entry(client_id)
        .or_insert(EventSnapshotBufferInner::with_capacity(self.max_buffer_size))
        .sort_with_id();
    }

    #[inline]
    pub fn frontier(&mut self, client_id: ClientId, frontier_tick: u32) 
    -> Iter<'_, EventSnapshot<E>> {
        let history = self.buffer
        .entry(client_id)
        .or_insert(EventSnapshotBufferInner::with_capacity(self.max_buffer_size));

        if let Some(last_index) = history.buffer
        .iter()
        .position(|v| v.tick >= frontier_tick) {
            history.buffer.range(last_index..)
        } else {
            history.buffer.range(0..0)
        }
    }
}
