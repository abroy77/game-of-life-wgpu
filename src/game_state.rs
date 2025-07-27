use crate::config::CONFIG;
use rand::{Rng, rng, rngs::ThreadRng};

pub struct GameState {
    rng: ThreadRng,
    state: Vec<u32>,
}

impl Default for GameState {
    fn default() -> Self {
        let mut rng = rng();
        let state = (0..CONFIG.num_elements)
            .map(|_| rng.random_bool(0.7) as u32)
            .collect();
        Self { rng, state }
    }
}

impl GameState {
    pub fn update(&mut self) {
        self.state = (0..CONFIG.num_elements)
            .map(|_| self.rng.random_bool(0.7) as u32)
            .collect();
    }

    pub fn get_state(&self) -> &Vec<u32> {
        &self.state
    }
}
