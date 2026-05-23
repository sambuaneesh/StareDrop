use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use staredrop_camera::list_cameras;

use crate::app::LaunchMode;
use crate::receiver_page::ReceiverConfig;
use crate::sender_page::SenderConfig;
use crate::transfer::{SenderBuildOptions, build_file_sender_plan, build_text_sender_plan};

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
    #[command(about = "Open camera and scan QR frames")]
    Receiver(ReceiverArgs),
    #[command(about = "List available camera devices and exit")]
    ListCameras,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum InputFormat {
    Utf8,
    Base64,
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

    #[arg(long, default_value_t = 700, help = "Chunk size for --send-file mode")]
    pub chunk_size: usize,

    #[arg(long, default_value_t = 8.0, help = "Frame rate for sender animation")]
    pub fps: f32,
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
        })),
        Command::ListCameras => bail!("list-cameras does not launch the GUI"),
    }
}

fn resolve_sender(args: SenderArgs) -> Result<SenderConfig> {
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
        build_file_sender_plan(
            &path,
            SenderBuildOptions {
                chunk_size: args.chunk_size,
            },
        )?
    } else {
        unreachable!("validated selected_count");
    };

    Ok(SenderConfig {
        plan,
        fps: args.fps.max(0.5),
    })
}
