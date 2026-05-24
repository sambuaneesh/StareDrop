use staredrop_codec_grid::{ColorGridConfig, ContrastPalette};

#[derive(Debug, Clone, Copy)]
pub struct ColorGridParams {
    pub pixel_size: u16,
}

impl Default for ColorGridParams {
    fn default() -> Self {
        Self { pixel_size: 8 }
    }
}

impl ColorGridParams {
    pub const QUIET_ZONE_CELLS: u16 = 2;
    pub const MIN_GRID_SIDE: u16 = 16;
    pub const MAX_GRID_SIDE: u16 = 512;

    pub fn config_for_grid_side(self, grid_side: u16) -> ColorGridConfig {
        ColorGridConfig {
            grid_side,
            cell_pixels: self.pixel_size,
            quiet_zone_cells: Self::QUIET_ZONE_CELLS,
            palette: ContrastPalette::BwRg,
        }
    }

    pub fn grid_side_for_square_points(self, square_points: f32) -> u16 {
        let px = self.pixel_size.max(1) as f32;
        let raw = (square_points / px).floor() as i32 - (Self::QUIET_ZONE_CELLS as i32 * 2);
        raw.max(Self::MIN_GRID_SIDE as i32)
            .min(Self::MAX_GRID_SIDE as i32) as u16
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VisualCodecConfig {
    Qr,
    ColorGrid(ColorGridParams),
}

impl VisualCodecConfig {
    pub fn as_str(self) -> &'static str {
        match self {
            VisualCodecConfig::Qr => "qr",
            VisualCodecConfig::ColorGrid(_) => "color-grid",
        }
    }
}
