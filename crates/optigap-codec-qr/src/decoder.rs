use anyhow::Result;
use image::GrayImage;

pub fn decode_first_qr_text(image: &GrayImage) -> Result<Option<String>> {
    let mut prepared = rqrr::PreparedImage::prepare(image.clone());
    let grids = prepared.detect_grids();
    for grid in grids {
        if let Ok((_, text)) = grid.decode() {
            return Ok(Some(text));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::decode_first_qr_text;
    use crate::encoder::encode_text_to_qr_luma;

    #[test]
    fn decode_round_trip() {
        let input = "optigap-phase1";
        let encoded = encode_text_to_qr_luma(input).expect("encode");
        let decoded = decode_first_qr_text(&encoded).expect("decode");
        assert_eq!(decoded.as_deref(), Some(input));
    }
}
