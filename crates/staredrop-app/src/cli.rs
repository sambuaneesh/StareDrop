use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use staredrop_camera::list_cameras;

use crate::app::LaunchMode;
use crate::receiver_page::ReceiverConfig;
use crate::sender_page::SenderConfig;

#[derive(Debug, Parser)]
#[command(
    name = "staredrop-app",
    about = "StareDrop CLI-first optical transfer app (Phase 1 static QR)"
)]
pub struct Cli {
    #[arg(long, default_value_t = true, help = "Launch fullscreen window")]
    pub fullscreen: bool,

    #[arg(long, default_value_t = true, help = "Show overlay status text on screen")]
    pub overlay: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Display a static QR from CLI payload input")]
    Sender(SenderArgs),
    #[command(about = "Open camera and scan QR in fullscreen")]
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
    #[arg(long, help = "Text payload to encode into QR")]
    pub text: Option<String>,

    #[arg(long, help = "Input file path for payload bytes")]
    pub input_file: Option<PathBuf>,

    #[arg(
        long,
        value_enum,
        default_value_t = InputFormat::Utf8,
        help = "How to map input-file bytes into QR text"
    )]
    pub input_format: InputFormat,
}

#[derive(Debug, clap::Args)]
pub struct ReceiverArgs {
    #[arg(long, default_value_t = 0, help = "Camera index from list-cameras output")]
    pub camera_index: usize,

    #[arg(long, default_value_t = false, help = "Start scanning immediately")]
    pub auto_start: bool,

    #[arg(long, default_value_t = true, help = "Print decoded text to terminal")]
    pub print_decoded: bool,
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
        })),
        Command::ListCameras => bail!("list-cameras does not launch the GUI"),
    }
}

fn resolve_sender(args: SenderArgs) -> Result<SenderConfig> {
    match (args.text, args.input_file) {
        (Some(text), None) => Ok(SenderConfig {
            source_label: "inline --text".to_string(),
            payload_text: text,
        }),
        (None, Some(path)) => {
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
            Ok(SenderConfig {
                source_label: path.display().to_string(),
                payload_text,
            })
        }
        (Some(_), Some(_)) => bail!("pass either --text or --input-file, not both"),
        (None, None) => bail!("sender requires one of --text or --input-file"),
    }
}
