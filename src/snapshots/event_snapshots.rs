use std::collections::{VecDeque, vec_deque::Iter};
use bevy::{prelude::*, utils::HashMap};
use bevy_replicon::core::ClientId;

// Event with index. For use with unreliable transport.
pub trait IndexedEvent: Event {
    fn index(&self) -> usize;
}

pub struct EventSnapshot<E: IndexedEvent> {
    event: E,
    tick: u32
}

impl<E: IndexedEvent> EventSnapshot<E> {
    #[inline]
    pub fn new(event: E, tick: u32) -> Self {
        Self{
            event,
            tick
        }
    }

    #[inline]
    pub fn event(&self) -> &E {
        &self.event
    }

    #[inline]
    pub fn tick(&self) -> u32 {
        self.tick
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.event.index()
    } 
}

pub struct EventSnapshotBufferInner<E: IndexedEvent> {
    buffer: VecDeque<EventSnapshot<E>>,
    latest_snapshot_tick: u32,
    frontier_index: usize
}

impl<E: IndexedEvent> EventSnapshotBufferInner<E> {
    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        Self { 
            buffer: VecDeque::with_capacity(capacity), 
            latest_snapshot_tick: 0,
            frontier_index: 0 
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.buffer.len()
    }

    #[inline]
    fn latest_snapshot(&self) -> Option<&EventSnapshot<E>> {
        self.buffer.back()
    }

    #[inline]
    fn latest_snapshot_tick(&self) -> u32 {
        self.latest_snapshot_tick
    }

    #[inline]
    fn iter(&self) -> Iter<'_, EventSnapshot<E>> {
        self.buffer.iter()
    }

    #[inline]
    fn sort_with_index(&mut self) {
        self.buffer.make_contiguous().sort_by_key(|s| s.index());
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

        if snapshot.index() < self.frontier_index {
            warn!(
                "discarding a old event snapshot with index:{}, frontier:{}", 
                snapshot.index(), self.frontier_index
            );
            return;
        } 

        self.latest_snapshot_tick = snapshot.tick;
        self.buffer.push_back(snapshot);
    }

    #[inline]
    pub fn frontier(&mut self) -> Iter<'_, EventSnapshot<E>> {
        if let Some(begin) = self.buffer.iter()
        .position(|e| e.index() >= self.frontier_index) {
            // buffer is not empty here
            self.frontier_index = self.buffer.back().unwrap().index() + 1;
            self.buffer.range(begin..)
        } else {
            self.buffer.range(0..0)
        }
    }
}

#[derive(Component)]
pub struct EventSnapshotBuffer<E: IndexedEvent> {
    buffer: EventSnapshotBufferInner<E>,
    max_buffer_size: usize
}

impl<E: IndexedEvent> EventSnapshotBuffer<E> {
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
    pub fn latest_snapshot(&self) -> Option<&EventSnapshot<E>> {
        self.buffer.latest_snapshot()
    }

    #[inline]
    pub fn latest_snapshot_tick(&self) -> u32 {
        self.buffer.latest_snapshot_tick()
    }

    #[inline]
    pub fn insert(&mut self, event: E, tick: u32) {
        if self.max_buffer_size == 0 {
            return;
        }

        if self.buffer.len() >= self.max_buffer_size {
            self.buffer.pop_front();
        }

        self.buffer.insert(EventSnapshot::new(event, tick));
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, EventSnapshot<E>> {
        self.buffer.iter()
    }

    #[inline]
    pub fn sort_with_id(&mut self) {
        self.buffer.sort_with_index();
    }

    #[inline]
    pub fn frontier(&mut self) -> Iter<'_, EventSnapshot<E>> {
        self.buffer.frontier()
    }
}

#[derive(Resource)]
pub struct EventSnapshotClientMap<E: IndexedEvent> {
    buffer: HashMap<ClientId, EventSnapshotBufferInner<E>>,
    max_buffer_size: usize
}

impl<E: IndexedEvent> EventSnapshotClientMap<E> {
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
    pub fn len(&self, client_id: &ClientId) -> usize {
        match self.buffer.get(client_id) {
            Some(h) => h.len(),
            None => 0
        }
    }

    #[inline]
    pub fn latest_snapshot(&self, client_id: &ClientId) -> Option<&EventSnapshot<E>> {
        match self.buffer.get(client_id) {
            Some(b) => b.latest_snapshot(),
            None => None
        }
    }

    #[inline]
    pub fn latest_snapshot_tick(&self, client_id: &ClientId) -> u32 {
        match self.buffer.get(client_id) {
            Some(b) => b.latest_snapshot_tick(),
            None => 0
        }
    }

    #[inline]
    pub fn insert(&mut self, client_id: &ClientId, event: E, tick: u32) {
        let history = self.buffer
        .entry(*client_id)
        .or_insert(EventSnapshotBufferInner::with_capacity(self.max_buffer_size));

        if self.max_buffer_size == 0 {
            return;
        }

        if history.buffer.len() >= self.max_buffer_size {
            history.pop_front();
        }

        history.insert(EventSnapshot::new(event, tick));
    }

    #[inline]
    pub fn iter(&mut self, client_id: &ClientId) -> Iter<'_, EventSnapshot<E>> {
        self.buffer
        .entry(*client_id)
        .or_insert(EventSnapshotBufferInner::with_capacity(self.max_buffer_size))
        .iter()
    }

    #[inline]
    pub fn sort_with_id(&mut self, client_id: &ClientId) {
        self.buffer
        .entry(*client_id)
        .or_insert(EventSnapshotBufferInner::with_capacity(self.max_buffer_size))
        .sort_with_index();
    }

    #[inline]
    pub fn frontier(&mut self, client_id: &ClientId) 
    -> Iter<'_, EventSnapshot<E>> {
        let history = self.buffer
        .entry(*client_id)
        .or_insert(EventSnapshotBufferInner::with_capacity(self.max_buffer_size));

        history.frontier()
    }
}
