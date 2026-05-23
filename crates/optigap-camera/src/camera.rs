use anyhow::{Context, Result, bail};
use glob::glob;
use image::RgbImage;
use rscam::{Camera, Config};

use crate::frame::CapturedFrame;

pub struct CameraCapture {
    camera: Camera,
    stream_format: StreamFormat,
}

#[derive(Debug, Clone, Copy)]
enum StreamFormat {
    Rgb3 { width: u32, height: u32 },
    Mjpg,
}

impl CameraCapture {
    pub fn open(index: usize) -> Result<Self> {
        let mut entries = Vec::new();
        for path in glob("/dev/video*")?.flatten() {
            entries.push(path);
        }
        let device = entries
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("camera index {} not found", index))?;
        let mut camera =
            Camera::new(device.to_string_lossy().as_ref()).context("failed to create camera")?;

        let stream_format = if camera
            .start(&Config {
                interval: (1, 30),
                resolution: (640, 480),
                format: b"RGB3",
                ..Default::default()
            })
            .is_ok()
        {
            StreamFormat::Rgb3 {
                width: 640,
                height: 480,
            }
        } else {
            camera
                .start(&Config {
                    interval: (1, 30),
                    resolution: (640, 480),
                    format: b"MJPG",
                    ..Default::default()
                })
                .context("failed to start camera stream in RGB3 or MJPG format")?;
            StreamFormat::Mjpg
        };

        Ok(Self {
            camera,
            stream_format,
        })
    }

    pub fn frame(&mut self) -> Result<CapturedFrame> {
        let frame = self
            .camera
            .capture()
            .context("camera frame capture failed")?;
        let rgb = match self.stream_format {
            StreamFormat::Rgb3 { width, height } => {
                let bytes = frame.to_vec();
                RgbImage::from_raw(width, height, bytes).ok_or_else(|| {
                    anyhow::anyhow!("failed to build RGB frame buffer from camera bytes")
                })?
            }
            StreamFormat::Mjpg => image::load_from_memory(&frame)
                .context("failed to decode MJPG frame")?
                .to_rgb8(),
        };
        if rgb.is_empty() {
            bail!("camera returned empty frame");
        }
        Ok(CapturedFrame { rgb })
    }
}
