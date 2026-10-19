use std::collections::HashSet;
use winit::keyboard::KeyCode;

/// Keys held this frame. Orbit dragging is tracked on the galaxy app.
pub struct InputState {
    keys_held: HashSet<KeyCode>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_held: HashSet::new(),
        }
    }

    pub fn press(&mut self, key: KeyCode) -> bool {
        self.keys_held.insert(key)
    }

    pub fn release(&mut self, key: KeyCode) {
        self.keys_held.remove(&key);
    }

    pub fn clear(&mut self) {
        self.keys_held.clear();
    }

    pub fn is_held(&self, key: KeyCode) -> bool {
        self.keys_held.contains(&key)
    }

    fn axis(&self, positive: KeyCode, negative: KeyCode) -> f32 {
        u8::from(self.is_held(positive)) as f32 - u8::from(self.is_held(negative)) as f32
    }

    pub fn movement(&self, flying: bool) -> (f32, f32, f32) {
        let up = if flying {
            u8::from(self.is_held(KeyCode::Space)) as f32
                - u8::from(self.is_held(KeyCode::ShiftLeft) || self.is_held(KeyCode::ShiftRight))
                    as f32
        } else {
            self.axis(KeyCode::KeyE, KeyCode::KeyQ)
        };
        (
            self.axis(KeyCode::KeyD, KeyCode::KeyA),
            up,
            self.axis(KeyCode::KeyW, KeyCode::KeyS),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_presses_only_trigger_once() {
        let mut input = InputState::new();
        assert!(input.press(KeyCode::KeyF));
        assert!(!input.press(KeyCode::KeyF));
        input.release(KeyCode::KeyF);
        assert!(input.press(KeyCode::KeyF));
    }

    #[test]
    fn clearing_input_stops_movement() {
        let mut input = InputState::new();
        for key in [KeyCode::KeyW, KeyCode::KeyA, KeyCode::ShiftLeft] {
            input.press(key);
        }
        assert_eq!(input.movement(true), (-1.0, -1.0, 1.0));
        input.clear();
        assert_eq!(input.movement(true), (0.0, 0.0, 0.0));
        assert_eq!(input.movement(false), (0.0, 0.0, 0.0));
    }

    #[test]
    fn opposing_keys_cancel() {
        let mut input = InputState::new();
        for key in [KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD] {
            input.press(key);
        }
        assert_eq!(input.movement(true), (0.0, 0.0, 0.0));
        input.release(KeyCode::KeyS);
        input.release(KeyCode::KeyA);
        assert_eq!(input.movement(true), (1.0, 0.0, 1.0));
    }

    #[test]
    fn vertical_controls_depend_on_camera_mode() {
        let mut input = InputState::new();
        input.press(KeyCode::KeyE);
        assert_eq!(input.movement(false).1, 1.0);
        assert_eq!(input.movement(true).1, 0.0);
        input.press(KeyCode::Space);
        assert_eq!(input.movement(true).1, 1.0);
        for key in [KeyCode::ShiftLeft, KeyCode::ShiftRight] {
            input.press(key);
            assert_eq!(input.movement(true).1, 0.0);
            input.release(KeyCode::Space);
            assert_eq!(input.movement(true).1, -1.0);
            input.release(key);
            input.press(KeyCode::Space);
        }
    }
}
