use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use staredrop_camera::list_cameras;
use staredrop_codec_grid::ColorGridConfig;
use staredrop_protocol::{
    frame_json::{DataFrameV1, serialize_data_frame, serialize_manifest_frame},
    manifest::ManifestFrameV1,
};

use crate::app::LaunchMode;
use crate::receiver_page::ReceiverConfig;
use crate::sender_page::SenderConfig;
use crate::simulate::{SimulateArgs, run_simulation_suite};
use crate::transfer::{SenderBuildOptions, build_file_sender_plan, build_text_sender_plan};
use crate::visual_codec::{ColorGridParams, VisualCodecConfig};

#[derive(Debug, Parser)]
#[command(
    name = "staredrop-app",
    about = "StareDrop CLI-first optical transfer app (Phase 2 animated QR file transfer)"
)]
pub struct Cli {
    #[arg(
        long,
        action = ArgAction::Set,
        default_value_t = true,
        help = "Launch fullscreen window"
    )]
    pub fullscreen: bool,

    #[arg(
        long,
        action = ArgAction::Set,
        default_value_t = true,
        help = "Show overlay status text on screen"
    )]
    pub overlay: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Display sender frames (text or file transfer)")]
    Sender(SenderArgs),
    #[command(about = "Open camera and scan visual frames")]
    Receiver(ReceiverArgs),
    #[command(about = "List available camera devices and exit")]
    ListCameras,
    #[command(
        about = "Run camera-free end-to-end sender->visual->decoder->receiver simulation and benchmark"
    )]
    Simulate(SimulateArgs),
}

#[derive(Debug, Clone, ValueEnum)]
pub enum InputFormat {
    Utf8,
    Base64,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum VisualCodecKind {
    Qr,
    ColorGrid,
}

#[derive(Debug, Clone, Args)]
pub struct ColorGridCliArgs {
    #[arg(
        long,
        default_value_t = 96,
        help = "Color-grid side length in cells (for --visual-codec color-grid)"
    )]
    pub grid_side: u16,

    #[arg(
        long,
        default_value_t = 8,
        help = "Color-grid cell size in pixels (for --visual-codec color-grid)"
    )]
    pub cell_pixels: u16,

    #[arg(
        long,
        default_value_t = 2,
        help = "Color-grid quiet-zone in cells (for --visual-codec color-grid)"
    )]
    pub quiet_zone_cells: u16,
}

#[derive(Debug, clap::Args)]
pub struct SenderArgs {
    #[arg(long, help = "Text payload to encode into one static QR")]
    pub text: Option<String>,

    #[arg(long, help = "Input text file for static QR mode")]
    pub input_file: Option<PathBuf>,

    #[arg(long, help = "File path for Phase 2 animated file transfer mode")]
    pub send_file: Option<PathBuf>,

    #[arg(
        long,
        value_enum,
        default_value_t = InputFormat::Utf8,
        help = "How to map --input-file bytes into QR text"
    )]
    pub input_format: InputFormat,

    #[arg(
        long,
        help = "Chunk size for --send-file mode (auto-calculated for color-grid when omitted)"
    )]
    pub chunk_size: Option<usize>,

    #[arg(long, default_value_t = 8.0, help = "Frame rate for sender animation")]
    pub fps: f32,

    #[arg(
        long,
        value_enum,
        default_value_t = VisualCodecKind::Qr,
        help = "Visual codec used to render sender frames"
    )]
    pub visual_codec: VisualCodecKind,

    #[command(flatten)]
    pub color_grid: ColorGridCliArgs,
}

#[derive(Debug, clap::Args)]
pub struct ReceiverArgs {
    #[arg(
        long,
        default_value_t = 0,
        help = "Camera index from list-cameras output"
    )]
    pub camera_index: usize,

    #[arg(long, default_value_t = false, help = "Start scanning immediately")]
    pub auto_start: bool,

    #[arg(
        long,
        action = ArgAction::Set,
        default_value_t = false,
        help = "Print decoded frame text to terminal"
    )]
    pub print_decoded: bool,

    #[arg(long, help = "Exact output file path. Fails if path already exists")]
    pub output_file: Option<PathBuf>,

    #[arg(
        long,
        default_value = ".",
        help = "Output directory when --output-file is not provided"
    )]
    pub output_dir: PathBuf,

    #[arg(
        long,
        action = ArgAction::Set,
        default_value_t = true,
        help = "Automatically save once all chunks are received and verified"
    )]
    pub auto_save: bool,

    #[arg(
        long,
        value_enum,
        default_value_t = VisualCodecKind::Qr,
        help = "Visual codec decoder used by receiver"
    )]
    pub visual_codec: VisualCodecKind,

    #[command(flatten)]
    pub color_grid: ColorGridCliArgs,
}

pub fn list_cameras_cli() -> Result<()> {
    let devices = list_cameras().context("camera enumeration failed")?;
    if devices.is_empty() {
        println!("No camera devices found.");
        return Ok(());
    }

    for device in devices {
        println!("{}: {}", device.index, device.human_name);
    }
    Ok(())
}

pub fn resolve_launch_mode(command: Command) -> Result<LaunchMode> {
    match command {
        Command::Sender(args) => resolve_sender(args).map(LaunchMode::Sender),
        Command::Receiver(args) => Ok(LaunchMode::Receiver(ReceiverConfig {
            camera_index: args.camera_index,
            auto_start: args.auto_start,
            print_decoded: args.print_decoded,
            output_file: args.output_file,
            output_dir: args.output_dir,
            auto_save: args.auto_save,
            visual_codec: resolve_visual_codec(args.visual_codec, args.color_grid.clone())?,
        })),
        Command::ListCameras | Command::Simulate(_) => {
            bail!("list-cameras/simulate do not launch the GUI")
        }
    }
}

pub fn run_simulate_cli(args: SimulateArgs) -> Result<()> {
    run_simulation_suite(args)
}

fn resolve_sender(args: SenderArgs) -> Result<SenderConfig> {
    let visual_codec = resolve_visual_codec(args.visual_codec, args.color_grid.clone())?;

    let selected_count = usize::from(args.text.is_some())
        + usize::from(args.input_file.is_some())
        + usize::from(args.send_file.is_some());
    if selected_count != 1 {
        bail!("sender requires exactly one of --text, --input-file, or --send-file");
    }

    let plan = if let Some(text) = args.text {
        build_text_sender_plan(&text)?
    } else if let Some(path) = args.input_file {
        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read input file {}", path.display()))?;
        let payload_text = match args.input_format {
            InputFormat::Utf8 => String::from_utf8(bytes).context(
                "input file is not valid UTF-8; use --input-format base64 for raw bytes",
            )?,
            InputFormat::Base64 => {
                use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
                BASE64.encode(bytes)
            }
        };
        build_text_sender_plan(&payload_text)?
    } else if let Some(path) = args.send_file {
        let chunk_size = resolve_effective_chunk_size(&path, args.chunk_size, visual_codec)?;
        build_file_sender_plan(&path, SenderBuildOptions { chunk_size })?
    } else {
        unreachable!("validated selected_count");
    };

    Ok(SenderConfig {
        plan,
        fps: args.fps.max(0.5),
        visual_codec,
    })
}

fn resolve_visual_codec(
    kind: VisualCodecKind,
    grid: ColorGridCliArgs,
) -> Result<VisualCodecConfig> {
    match kind {
        VisualCodecKind::Qr => Ok(VisualCodecConfig::Qr),
        VisualCodecKind::ColorGrid => {
            if grid.grid_side < 16 {
                bail!("--grid-side must be >= 16 for color-grid");
            }
            if grid.cell_pixels == 0 {
                bail!("--cell-pixels must be > 0 for color-grid");
            }
            Ok(VisualCodecConfig::ColorGrid(ColorGridParams {
                grid_side: grid.grid_side,
                cell_pixels: grid.cell_pixels,
                quiet_zone_cells: grid.quiet_zone_cells,
            }))
        }
    }
}

fn resolve_effective_chunk_size(
    path: &PathBuf,
    chunk_size_arg: Option<usize>,
    visual_codec: VisualCodecConfig,
) -> Result<usize> {
    match visual_codec {
        VisualCodecConfig::Qr => Ok(chunk_size_arg.unwrap_or(700).max(1)),
        VisualCodecConfig::ColorGrid(grid) => {
            let cfg = grid.as_codec_config();
            let max_fit = max_color_grid_chunk_size(path, cfg)?;
            let selected = chunk_size_arg.unwrap_or(max_fit);
            if selected == 0 {
                bail!("--chunk-size must be > 0");
            }
            if selected > max_fit {
                eprintln!(
                    "StareDrop: requested chunk-size {} exceeds color-grid/frame capacity; using {}",
                    selected, max_fit
                );
                return Ok(max_fit);
            }
            Ok(selected)
        }
    }
}

fn max_color_grid_chunk_size(path: &PathBuf, cfg: ColorGridConfig) -> Result<usize> {
    let file_size = fs::metadata(path)
        .with_context(|| format!("failed to stat input file {}", path.display()))?
        .len() as usize;
    if file_size == 0 {
        return Ok(1);
    }

    let frame_text_cap = cfg.max_payload_bytes();
    if frame_text_cap <= 32 {
        bail!(
            "color-grid capacity too small for protocol overhead (grid_side={}, cell_pixels={})",
            cfg.grid_side,
            cfg.cell_pixels
        );
    }

    let file_name = sanitize_file_name(
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("payload.bin"),
    );

    let mut lo = 1usize;
    let mut hi = file_size.max(1);
    let mut best = 1usize;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        if color_grid_chunk_fits(file_size, mid, &file_name, frame_text_cap)? {
            best = mid;
            lo = mid.saturating_add(1);
        } else {
            hi = mid.saturating_sub(1);
        }
    }

    Ok(best.max(1))
}

fn color_grid_chunk_fits(
    file_size: usize,
    chunk_size: usize,
    file_name: &str,
    frame_text_cap: usize,
) -> Result<bool> {
    if chunk_size == 0 {
        return Ok(false);
    }
    let total_chunks = file_size.div_ceil(chunk_size).max(1) as u32;
    let payload_len = chunk_size.min(file_size).max(1);
    let payload = vec![0u8; payload_len];

    // Check worst-case DATA frame length for this chunk size.
    let data = DataFrameV1 {
        magic: "STAREDROP".to_string(),
        version: 1,
        frame_type: "DATA".to_string(),
        session_id: "00000000-0000-0000-0000-000000000000".to_string(),
        file_id: "11111111-1111-1111-1111-111111111111".to_string(),
        file_name: file_name.to_string(),
        file_size: file_size as u64,
        chunk_index: total_chunks.saturating_sub(1),
        total_chunks,
        payload_base64: BASE64.encode(payload),
        crc32: u32::MAX,
    };
    let data_text = serialize_data_frame(&data)?;
    if data_text.len() > frame_text_cap {
        return Ok(false);
    }

    // Check MANIFEST frame too.
    let manifest = ManifestFrameV1 {
        magic: "STAREDROP".to_string(),
        version: 1,
        frame_type: "MANIFEST".to_string(),
        session_id: "00000000-0000-0000-0000-000000000000".to_string(),
        file_id: "11111111-1111-1111-1111-111111111111".to_string(),
        file_name: file_name.to_string(),
        mime_type: None,
        original_file_size: file_size as u64,
        processed_file_size: file_size as u64,
        chunk_size: chunk_size as u32,
        total_chunks,
        compression: "none".to_string(),
        encryption: "none".to_string(),
        original_sha256: "0".repeat(64),
        processed_sha256: "0".repeat(64),
    };
    let manifest_text = serialize_manifest_frame(&manifest)?;
    Ok(manifest_text.len() <= frame_text_cap)
}

fn sanitize_file_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "output.bin".to_string()
    } else {
        out
    }
}
