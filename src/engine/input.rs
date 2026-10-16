use std::collections::HashSet;
use winit::keyboard::KeyCode;

/// Keys held this frame. Orbit dragging is tracked on the galaxy app.
pub struct InputState {
    pub keys_held: HashSet<KeyCode>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_held: HashSet::new(),
        }
    }

    pub fn is_held(&self, key: KeyCode) -> bool {
        self.keys_held.contains(&key)
    }
}
