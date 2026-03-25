use std::collections::HashSet;
use winit::keyboard::KeyCode;

/// Tracks keyboard and mouse input state each frame.
pub struct InputState {
    pub keys_held: HashSet<KeyCode>,
    pub mouse_delta: (f32, f32),
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_held: HashSet::new(),
            mouse_delta: (0.0, 0.0),
        }
    }

    pub fn is_held(&self, key: KeyCode) -> bool {
        self.keys_held.contains(&key)
    }

    pub fn reset_frame(&mut self) {
        self.mouse_delta = (0.0, 0.0);
    }
}
