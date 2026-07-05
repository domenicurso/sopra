use keel_core::SurfaceFrame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayCapture {
    pub frames: Vec<SurfaceFrame>,
}

impl ReplayCapture {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.frames)
    }
}
