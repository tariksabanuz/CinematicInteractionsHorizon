// The one winit type the helpers name (the anvil's rename key).
pub mod keyboard {
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub enum KeyCode { Enter, NumpadEnter, Escape, Backspace }
}
