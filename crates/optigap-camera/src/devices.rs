use anyhow::Result;
use glob::glob;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDeviceInfo {
    pub index: usize,
    pub human_name: String,
}

pub fn list_cameras() -> Result<Vec<CameraDeviceInfo>> {
    let mut devices = Vec::new();
    for (index, entry) in glob("/dev/video*")?.flatten().enumerate() {
        devices.push(CameraDeviceInfo {
            index,
            human_name: entry.display().to_string(),
        });
    }
    Ok(devices)
}
