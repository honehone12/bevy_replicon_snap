use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Component, Deserialize, Serialize)]
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
