use anyhow::{Result, bail};
use image::{ImageBuffer, Rgb, RgbImage};

const FRAME_MAGIC: [u8; 4] = *b"SDC1";
const HEADER_LEN: usize = 12;

#[derive(Debug, Clone, Copy)]
pub enum ContrastPalette {
    BwRg,
}

impl ContrastPalette {
    pub fn bits_per_cell(self) -> usize {
        2
    }

    pub fn color(self, symbol: u8) -> [u8; 3] {
        match (self, symbol & 0b11) {
            (ContrastPalette::BwRg, 0) => [0, 0, 0],
            (ContrastPalette::BwRg, 1) => [255, 255, 255],
            (ContrastPalette::BwRg, 2) => [255, 32, 32],
            (ContrastPalette::BwRg, 3) => [32, 224, 32],
            _ => [0, 0, 0],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ColorGridConfig {
    pub grid_side: u16,
    pub cell_pixels: u16,
    pub quiet_zone_cells: u16,
    pub palette: ContrastPalette,
}

impl Default for ColorGridConfig {
    fn default() -> Self {
        Self {
            grid_side: 96,
            cell_pixels: 8,
            quiet_zone_cells: 2,
            palette: ContrastPalette::BwRg,
        }
    }
}

impl ColorGridConfig {
    pub fn max_payload_bytes(&self) -> usize {
        let cell_count = self.grid_side as usize * self.grid_side as usize;
        let bit_capacity = cell_count * self.palette.bits_per_cell();
        let byte_capacity = bit_capacity / 8;
        byte_capacity.saturating_sub(HEADER_LEN)
    }
}

#[derive(Debug, Clone)]
pub struct EncodedColorGridFrame {
    pub image: RgbImage,
    pub payload_bytes: usize,
    pub encoded_bytes: usize,
}

pub fn encode_color_grid_frame(
    payload: &[u8],
    cfg: ColorGridConfig,
) -> Result<EncodedColorGridFrame> {
    let max_payload = cfg.max_payload_bytes();
    if payload.len() > max_payload {
        bail!(
            "payload too large for color grid frame: {} > {} (grid_side={}, cell_pixels={})",
            payload.len(),
            max_payload,
            cfg.grid_side,
            cfg.cell_pixels
        );
    }

    let mut framed = Vec::with_capacity(HEADER_LEN + payload.len());
    framed.extend_from_slice(&FRAME_MAGIC);
    framed.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    framed.extend_from_slice(&crc32fast::hash(payload).to_le_bytes());
    framed.extend_from_slice(payload);

    let cell_count = cfg.grid_side as usize * cfg.grid_side as usize;
    let symbols = bytes_to_symbols_2bit(&framed);
    if symbols.len() > cell_count {
        bail!("frame overflow after symbol packing");
    }

    let size_cells = cfg.grid_side + cfg.quiet_zone_cells * 2;
    let pixel_side = size_cells as u32 * cfg.cell_pixels as u32;
    let mut img: RgbImage = ImageBuffer::from_pixel(pixel_side, pixel_side, Rgb([255, 255, 255]));

    let quiet = cfg.quiet_zone_cells as u32;
    for idx in 0..cell_count {
        let symbol = symbols.get(idx).copied().unwrap_or(0);
        let color = cfg.palette.color(symbol);
        let x = idx % cfg.grid_side as usize;
        let y = idx / cfg.grid_side as usize;
        draw_cell(
            &mut img,
            quiet + x as u32,
            quiet + y as u32,
            cfg.cell_pixels as u32,
            color,
        );
    }

    Ok(EncodedColorGridFrame {
        image: img,
        payload_bytes: payload.len(),
        encoded_bytes: framed.len(),
    })
}

fn draw_cell(image: &mut RgbImage, cell_x: u32, cell_y: u32, cell_pixels: u32, color: [u8; 3]) {
    let x0 = cell_x * cell_pixels;
    let y0 = cell_y * cell_pixels;
    let px = Rgb(color);
    for y in y0..(y0 + cell_pixels) {
        for x in x0..(x0 + cell_pixels) {
            image.put_pixel(x, y, px);
        }
    }
}

fn bytes_to_symbols_2bit(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() * 4);
    for b in bytes {
        out.push((b >> 6) & 0b11);
        out.push((b >> 4) & 0b11);
        out.push((b >> 2) & 0b11);
        out.push(b & 0b11);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{ColorGridConfig, encode_color_grid_frame};

    #[test]
    fn encode_produces_square_image() {
        let encoded =
            encode_color_grid_frame(b"hello", ColorGridConfig::default()).expect("encode");
        assert_eq!(encoded.image.width(), encoded.image.height());
        assert!(encoded.payload_bytes >= 5);
    }
}
