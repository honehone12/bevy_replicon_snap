use std::collections::{VecDeque, vec_deque::Iter};
use bevy::{prelude::*, utils::HashMap};

pub struct EventSnapshot<E: Event> {
    value: E,
    tick: u32,
    delta_time: f32,
}

impl<E: Event> EventSnapshot<E> {
    #[inline]
    pub fn new(value: E, tick: u32, delta_time: f32) -> Self {
        Self{
            value,
            tick,
            delta_time
        }
    }

    #[inline]
    pub fn value(&self) -> &E {
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

pub struct EventSnapshotBuffer<E: Event> {
    buffer: VecDeque<EventSnapshot<E>>,
    latest_snapshot_tick: u32
}

impl<E: Event> EventSnapshotBuffer<E> {
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
}

#[derive(Resource)]
pub struct EventSnapshotHistory<E: Event> {
    buffer: HashMap<u64, EventSnapshotBuffer<E>>,
    max_buffer_size: usize
}

impl<E: Event> EventSnapshotHistory<E> {
    #[inline]
    pub fn new(max_buffer_size: usize) -> EventSnapshotHistory<E> {
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
    pub fn history(&self, client_id: u64) -> Option<&EventSnapshotBuffer<E>> {
        self.buffer.get(&client_id)
    }

    pub fn insert(&mut self, client_id: u64, value: E, tick: u32, delta_time: f32) {
        let history = self.buffer
        .entry(client_id)
        .or_insert(EventSnapshotBuffer::new());

        if tick < history.latest_snapshot_tick {
            warn!(
                "discarding a old event snapshot with tick:{}, latest:{}", 
                tick, history.latest_snapshot_tick
            );
            return;
        }

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
    -> Iter<'_, EventSnapshot<E>> {
        let history = self.buffer
        .entry(client_id)
        .or_insert(EventSnapshotBuffer::new());

        if let Some(last_index) = history.buffer
        .iter()
        .position(|v| v.tick >= frontier_tick) {
            history.buffer.range(last_index..)
        } else {
            history.buffer.range(0..0)
        }
    }
}
