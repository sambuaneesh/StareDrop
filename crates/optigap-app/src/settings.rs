#[derive(Debug, Clone)]
pub struct RuntimeSettings {
    pub fps: f32,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self { fps: 10.0 }
    }
}
