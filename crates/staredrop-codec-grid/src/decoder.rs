use anyhow::{Result, bail};
use image::{Rgb, RgbImage};

use crate::encoder::{ColorGridConfig, ContrastPalette};

const FRAME_MAGIC: [u8; 4] = *b"SDC1";
const HEADER_LEN: usize = 12;

pub fn decode_color_grid_frame(image: &RgbImage, cfg: ColorGridConfig) -> Result<Vec<u8>> {
    let size_cells = cfg.grid_side + cfg.quiet_zone_cells * 2;
    let expected_px = size_cells as u32 * cfg.cell_pixels as u32;
    if image.width() != expected_px || image.height() != expected_px {
        bail!(
            "unexpected image size: got {}x{}, expected {}x{}",
            image.width(),
            image.height(),
            expected_px,
            expected_px
        );
    }

    let quiet = cfg.quiet_zone_cells as u32;
    let mut symbols = Vec::with_capacity(cfg.grid_side as usize * cfg.grid_side as usize);
    for y in 0..cfg.grid_side as usize {
        for x in 0..cfg.grid_side as usize {
            let sx = (quiet + x as u32) * cfg.cell_pixels as u32 + cfg.cell_pixels as u32 / 2;
            let sy = (quiet + y as u32) * cfg.cell_pixels as u32 + cfg.cell_pixels as u32 / 2;
            let px = image.get_pixel(sx, sy);
            let sym = nearest_symbol(*px, cfg.palette);
            symbols.push(sym);
        }
    }

    let bytes = symbols_2bit_to_bytes(&symbols);
    if bytes.len() < HEADER_LEN {
        bail!("decoded frame too short");
    }
    if bytes[0..4] != FRAME_MAGIC {
        bail!("invalid frame magic");
    }

    let len = u32::from_le_bytes(bytes[4..8].try_into().expect("slice size")) as usize;
    let expected_crc = u32::from_le_bytes(bytes[8..12].try_into().expect("slice size"));
    let end = HEADER_LEN + len;
    if bytes.len() < end {
        bail!(
            "decoded frame truncated: expected {} payload bytes, only {} available",
            len,
            bytes.len().saturating_sub(HEADER_LEN)
        );
    }
    let payload = &bytes[HEADER_LEN..end];
    let actual_crc = crc32fast::hash(payload);
    if actual_crc != expected_crc {
        bail!(
            "payload crc mismatch: expected {}, got {}",
            expected_crc,
            actual_crc
        );
    }
    Ok(payload.to_vec())
}

pub fn decode_color_grid_frame_resampled(
    image: &RgbImage,
    cfg: ColorGridConfig,
) -> Result<Vec<u8>> {
    let size_cells = cfg.grid_side + cfg.quiet_zone_cells * 2;
    let expected_px = size_cells as u32 * cfg.cell_pixels as u32;
    if expected_px == 0 {
        bail!("invalid expected pixel size");
    }

    let min_side = image.width().min(image.height());
    if min_side < 16 {
        bail!(
            "camera frame too small for color-grid decode: {}x{}",
            image.width(),
            image.height()
        );
    }
    let x0 = (image.width() - min_side) / 2;
    let y0 = (image.height() - min_side) / 2;
    let square = image::imageops::crop_imm(image, x0, y0, min_side, min_side).to_image();
    let normalized = if min_side == expected_px {
        square
    } else {
        image::imageops::resize(
            &square,
            expected_px,
            expected_px,
            image::imageops::FilterType::Nearest,
        )
    };
    decode_color_grid_frame(&normalized, cfg)
}

fn nearest_symbol(px: Rgb<u8>, palette: ContrastPalette) -> u8 {
    let mut best_symbol = 0u8;
    let mut best_dist = u32::MAX;
    for symbol in 0..4u8 {
        let [r, g, b] = palette.color(symbol);
        let dr = px[0].abs_diff(r) as u32;
        let dg = px[1].abs_diff(g) as u32;
        let db = px[2].abs_diff(b) as u32;
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best_dist = dist;
            best_symbol = symbol;
        }
    }
    best_symbol
}

fn symbols_2bit_to_bytes(symbols: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(symbols.len() / 4 + 1);
    let mut idx = 0;
    while idx + 3 < symbols.len() {
        let b = ((symbols[idx] & 0b11) << 6)
            | ((symbols[idx + 1] & 0b11) << 4)
            | ((symbols[idx + 2] & 0b11) << 2)
            | (symbols[idx + 3] & 0b11);
        out.push(b);
        idx += 4;
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::{
        decoder::{decode_color_grid_frame, decode_color_grid_frame_resampled},
        encoder::{ColorGridConfig, encode_color_grid_frame},
    };

    #[test]
    fn round_trip() {
        let cfg = ColorGridConfig::default();
        let encoded = encode_color_grid_frame(b"staredrop-color-grid", cfg).expect("encode");
        let decoded = decode_color_grid_frame(&encoded.image, cfg).expect("decode");
        assert_eq!(decoded, b"staredrop-color-grid");
    }

    #[test]
    fn round_trip_resampled() {
        let cfg = ColorGridConfig::default();
        let encoded = encode_color_grid_frame(b"staredrop-color-grid", cfg).expect("encode");
        let larger = image::imageops::resize(
            &encoded.image,
            encoded.image.width() * 2,
            encoded.image.height() * 2,
            image::imageops::FilterType::Nearest,
        );
        let decoded = decode_color_grid_frame_resampled(&larger, cfg).expect("decode");
        assert_eq!(decoded, b"staredrop-color-grid");
    }
}
