use staredrop_codec_grid::{ColorGridConfig, ContrastPalette};

#[derive(Debug, Clone, Copy)]
pub struct ColorGridParams {
    pub grid_side: u16,
    pub cell_pixels: u16,
    pub quiet_zone_cells: u16,
}

impl Default for ColorGridParams {
    fn default() -> Self {
        Self {
            grid_side: 96,
            cell_pixels: 8,
            quiet_zone_cells: 2,
        }
    }
}

impl ColorGridParams {
    pub fn as_codec_config(self) -> ColorGridConfig {
        ColorGridConfig {
            grid_side: self.grid_side,
            cell_pixels: self.cell_pixels,
            quiet_zone_cells: self.quiet_zone_cells,
            palette: ContrastPalette::BwRg,
        }
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
