use anyhow::Result;
use image::{ImageBuffer, Luma};
use qrcode::QrCode;

pub fn encode_text_to_qr_luma(text: &str) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>> {
    let code = QrCode::new(text.as_bytes())?;
    let image = code
        .render::<Luma<u8>>()
        .min_dimensions(384, 384)
        .quiet_zone(true)
        .build();
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::encode_text_to_qr_luma;

    #[test]
    fn encode_qr() {
        let img = encode_text_to_qr_luma("hello").expect("encode");
        assert!(img.width() >= 100);
        assert_eq!(img.width(), img.height());
    }
}
