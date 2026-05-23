use image::{GrayImage, RgbImage};

#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub rgb: RgbImage,
}

impl CapturedFrame {
    pub fn width(&self) -> u32 {
        self.rgb.width()
    }

    pub fn height(&self) -> u32 {
        self.rgb.height()
    }

    pub fn to_gray(&self) -> GrayImage {
        image::DynamicImage::ImageRgb8(self.rgb.clone()).to_luma8()
    }

    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        image::DynamicImage::ImageRgb8(self.rgb.clone())
            .to_rgba8()
            .into_raw()
    }
}
