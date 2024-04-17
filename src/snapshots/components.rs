use std::collections::{VecDeque, vec_deque::Iter};
use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Deserialize, Serialize)]
pub struct ComponentSnapshot<C: Component> {
    tick: u32,
    value: C,
}

impl<C: Component> ComponentSnapshot<C> {
    #[inline]
    pub fn new(value: C, tick: u32) -> Self {
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
    pub fn value(&self) -> &C {
        &self.value
    }
}

#[derive(Component, Deserialize, Serialize)]
pub struct ComponentSnapshotBuffer<C: Component> {
    buffer: VecDeque<ComponentSnapshot<C>>,
    time_since_last_snapshot: f32,
    latest_snapshot_tick: u32,
    max_buffer_size: usize
}

impl<C: Component> ComponentSnapshotBuffer<C> {
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
    pub fn insert(&mut self, element: C, tick: u32) {
        if tick <= self.latest_snapshot_tick {
            warn!(
                "discarding a old component snapshot with tick:{}, latest:{}", 
                tick, self.latest_snapshot_tick
            );
            return;
        }

        if self.buffer.len() >= self.max_buffer_size {
            self.buffer.pop_front();
        }

        self.buffer.push_back(ComponentSnapshot::new(element, tick));
        self.time_since_last_snapshot = 0.0;
        self.latest_snapshot_tick = tick;
    }

    #[inline]
    pub fn latest_snapshot(&self) -> Option<&ComponentSnapshot<C>> {
        self.buffer.back()
    }

    #[inline]
    pub fn latest_snapshot_tick(&self) -> u32 {
        self.latest_snapshot_tick
    }

    #[inline]
    pub fn at(&self, at: usize) -> Option<&ComponentSnapshot<C>> {
        self.buffer.get(at)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, ComponentSnapshot<C>> {
        self.buffer.iter()
    }

    #[inline]
    pub fn age(&self) -> f32 {
        self.time_since_last_snapshot
    }

    #[inline]
    pub(crate) fn add_age(&mut self, add: f32) {
        self.time_since_last_snapshot += add;
    }
}