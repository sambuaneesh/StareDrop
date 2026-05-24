use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::{ArgAction, Args, ValueEnum};
use staredrop_codec_grid::{
    ColorGridConfig, ContrastPalette, decode_color_grid_frame, encode_color_grid_frame,
};
use staredrop_codec_qr::{decode_first_qr_text, encode_text_to_qr_luma};
use staredrop_crypto::hash::sha256_hex;
use tracing::info;

use crate::transfer::{
    OutputSpec, ReceiverSession, SenderBuildOptions, SenderPlan, build_file_sender_plan,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum VisualCodecArg {
    Qr,
    ColorGrid,
}

impl VisualCodecArg {
    fn as_str(self) -> &'static str {
        match self {
            VisualCodecArg::Qr => "qr",
            VisualCodecArg::ColorGrid => "color-grid",
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct SimulateArgs {
    #[arg(
        long = "input-file",
        value_name = "PATH",
        action = ArgAction::Append,
        help = "Input file(s) to simulate. If omitted, default benchmark cases are generated."
    )]
    pub input_files: Vec<PathBuf>,

    #[arg(
        long,
        default_value = "manual-tests/sim-output",
        help = "Output directory for generated cases, reconstructed files, and benchmark reports"
    )]
    pub output_dir: PathBuf,

    #[arg(long, default_value_t = 700, help = "Chunk size")]
    pub chunk_size: usize,

    #[arg(
        long,
        default_value_t = 8.0,
        help = "Target sender FPS for modeled display time"
    )]
    pub fps: f32,

    #[arg(
        long,
        default_value_t = 1,
        help = "How many full sender frame loops to transmit"
    )]
    pub loops: u32,

    #[arg(
        long,
        default_value_t = false,
        action = ArgAction::Set,
        help = "Send DATA frames in reverse order each loop (manifest is still sent first)"
    )]
    pub reverse_data_order: bool,

    #[arg(
        long,
        default_value_t = 0,
        help = "Simulate frame loss by dropping every Nth DATA frame before decode (0 disables)"
    )]
    pub drop_every: u32,

    #[arg(
        long,
        default_value_t = 0,
        help = "Corrupt every Nth DATA frame text before visual encode (0 disables)"
    )]
    pub corrupt_every: u32,

    #[arg(
        long,
        value_enum,
        default_value_t = VisualCodecArg::Qr,
        help = "Visual codec used in simulation"
    )]
    pub visual_codec: VisualCodecArg,

    #[arg(
        long,
        default_value_t = 96,
        help = "Color-grid size (cells per side), only used with --visual-codec color-grid"
    )]
    pub grid_side: u16,

    #[arg(
        long,
        default_value_t = 8,
        help = "Color-grid cell pixel size, only used with --visual-codec color-grid"
    )]
    pub cell_pixels: u16,

    #[arg(
        long,
        default_value_t = 2,
        help = "Color-grid quiet-zone in cells, only used with --visual-codec color-grid"
    )]
    pub quiet_zone_cells: u16,

    #[arg(
        long,
        action = ArgAction::Set,
        default_value_t = true,
        help = "Append each simulation case to benchmark history CSV"
    )]
    pub record_history: bool,

    #[arg(
        long,
        default_value = "docs/research/benchmark-history.csv",
        help = "History CSV path appended when --record-history is true"
    )]
    pub history_file: PathBuf,

    #[arg(long, help = "Optional label stored in benchmark history rows")]
    pub run_label: Option<String>,
}

#[derive(Debug)]
struct CaseMetrics {
    case_name: String,
    input_path: PathBuf,
    output_path: Option<PathBuf>,
    bytes_in: usize,
    bytes_out: usize,
    input_sha256: String,
    output_sha256: Option<String>,
    sha_match: bool,
    byte_diff: Option<usize>,
    chunk_size: usize,
    total_chunks: u32,
    processed_size: u64,
    compression_ratio: f64,
    visual_codec: VisualCodecArg,
    frames_in_plan: usize,
    loops: u32,
    frames_generated: usize,
    frames_dropped: usize,
    frames_encoded: usize,
    frames_decoded: usize,
    decode_failures: usize,
    duplicate_chunks: u64,
    invalid_chunks: u64,
    accepted_chunks: u64,
    completed: bool,
    total_elapsed: Duration,
    encode_elapsed: Duration,
    decode_elapsed: Duration,
    completion_elapsed: Option<Duration>,
    modeled_display_elapsed: Duration,
    effective_kib_per_s: f64,
    modeled_link_kib_per_s: f64,
    protocol_overhead_ratio: f64,
}

pub fn run_simulation_suite(args: SimulateArgs) -> Result<()> {
    validate_args(&args)?;

    fs::create_dir_all(&args.output_dir)
        .with_context(|| format!("failed creating {}", args.output_dir.display()))?;
    let cases = resolve_cases(&args)?;
    if cases.is_empty() {
        bail!("no simulation cases found");
    }

    print_run_header(&args, cases.len());

    let mut results = Vec::with_capacity(cases.len());
    for case_path in cases {
        let metrics = run_case(&case_path, &args)?;
        print_case_summary(&metrics);
        results.push(metrics);
    }

    write_summary_files(&args.output_dir, &results)?;
    if args.record_history {
        append_history_csv(&args, &results)?;
    }
    print_aggregate_summary(&results);
    Ok(())
}

fn validate_args(args: &SimulateArgs) -> Result<()> {
    if args.chunk_size == 0 {
        bail!("--chunk-size must be > 0");
    }
    if args.loops == 0 {
        bail!("--loops must be > 0");
    }
    if args.fps <= 0.0 {
        bail!("--fps must be > 0");
    }
    if matches!(args.visual_codec, VisualCodecArg::ColorGrid) {
        if args.grid_side < 16 {
            bail!("--grid-side must be >= 16 for color-grid");
        }
        if args.cell_pixels == 0 {
            bail!("--cell-pixels must be > 0");
        }
    }
    Ok(())
}

fn print_run_header(args: &SimulateArgs, case_count: usize) {
    println!("StareDrop simulation");
    println!("  output-dir: {}", args.output_dir.display());
    println!("  visual-codec: {}", args.visual_codec.as_str());
    println!("  chunk-size: {}", args.chunk_size);
    println!("  loops: {}", args.loops);
    println!("  fps(model): {:.2}", args.fps.max(0.5));
    println!("  reverse-data-order: {}", args.reverse_data_order);
    println!("  drop-every: {}", args.drop_every);
    println!("  corrupt-every: {}", args.corrupt_every);
    if matches!(args.visual_codec, VisualCodecArg::ColorGrid) {
        println!(
            "  color-grid: side={}, cell-px={}, quiet-zone={}",
            args.grid_side, args.cell_pixels, args.quiet_zone_cells
        );
    }
    println!("  cases: {}", case_count);
    if args.record_history {
        println!("  history-file: {}", args.history_file.display());
    }
    println!();
}

fn run_case(case_path: &Path, args: &SimulateArgs) -> Result<CaseMetrics> {
    let input_bytes = fs::read(case_path)
        .with_context(|| format!("failed to read case {}", case_path.display()))?;
    let input_sha = sha256_hex(&input_bytes);
    let case_name = case_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("case")
        .to_string();

    let plan = build_file_sender_plan(
        case_path,
        SenderBuildOptions {
            chunk_size: args.chunk_size,
        },
    )?;
    let SenderPlan::File { manifest, frames } = plan else {
        bail!("expected file sender plan");
    };

    let case_out_dir = args.output_dir.join("received").join(&case_name);
    fs::create_dir_all(&case_out_dir)
        .with_context(|| format!("failed creating {}", case_out_dir.display()))?;
    let output_spec = OutputSpec::Directory(case_out_dir);
    let mut rx = ReceiverSession::new();

    let transmission = build_transmission_order(&frames, args.reverse_data_order);
    let color_cfg = color_grid_cfg(args);

    let start = Instant::now();
    let mut encode_elapsed = Duration::ZERO;
    let mut decode_elapsed = Duration::ZERO;
    let mut completion_elapsed = None;
    let mut frames_generated = 0usize;
    let mut frames_dropped = 0usize;
    let mut frames_encoded = 0usize;
    let mut frames_decoded = 0usize;
    let mut decode_failures = 0usize;
    let mut total_frame_text_bytes = 0usize;
    let mut data_frame_counter = 0u32;
    let mut saved_path = None;

    'tx: for _ in 0..args.loops {
        for frame_text in &transmission {
            frames_generated += 1;
            let is_data_frame = frame_text.contains("\"frame_type\":\"DATA\"");
            if is_data_frame {
                data_frame_counter = data_frame_counter.saturating_add(1);
                if args.drop_every > 0 && data_frame_counter % args.drop_every == 0 {
                    frames_dropped += 1;
                    continue;
                }
            }

            let effective_text = if is_data_frame
                && args.corrupt_every > 0
                && data_frame_counter % args.corrupt_every == 0
            {
                corrupt_frame_text(frame_text)
            } else {
                frame_text.clone()
            };
            total_frame_text_bytes += effective_text.len();

            let enc_start = Instant::now();
            let decoded_text = match args.visual_codec {
                VisualCodecArg::Qr => {
                    let qr = encode_text_to_qr_luma(&effective_text).with_context(|| {
                        format!("failed encoding QR frame for case {}", case_name)
                    })?;
                    encode_elapsed += enc_start.elapsed();
                    frames_encoded += 1;

                    let dec_start = Instant::now();
                    let decoded = decode_first_qr_text(&qr).with_context(|| {
                        format!("failed decoding QR frame for case {}", case_name)
                    })?;
                    decode_elapsed += dec_start.elapsed();
                    decoded
                }
                VisualCodecArg::ColorGrid => {
                    let encoded = encode_color_grid_frame(
                        effective_text.as_bytes(),
                        color_cfg.expect("color config set"),
                    )
                    .with_context(|| {
                        format!("failed encoding color-grid frame for case {}", case_name)
                    })?;
                    encode_elapsed += enc_start.elapsed();
                    frames_encoded += 1;

                    let dec_start = Instant::now();
                    let bytes = decode_color_grid_frame(
                        &encoded.image,
                        color_cfg.expect("color config set"),
                    )
                    .with_context(|| {
                        format!("failed decoding color-grid frame for case {}", case_name)
                    })?;
                    decode_elapsed += dec_start.elapsed();
                    Some(
                        String::from_utf8(bytes)
                            .context("decoded color-grid frame is not utf-8 text")?,
                    )
                }
            };

            let Some(decoded_text) = decoded_text else {
                decode_failures += 1;
                continue;
            };
            frames_decoded += 1;

            match rx.on_decoded_text(&decoded_text, &output_spec, true) {
                Ok(Some(path)) => {
                    if completion_elapsed.is_none() {
                        completion_elapsed = Some(start.elapsed());
                    }
                    saved_path = Some(path);
                    if args.loops == 1 && args.drop_every == 0 && args.corrupt_every == 0 {
                        break 'tx;
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    decode_failures += 1;
                }
            }
        }
    }

    if saved_path.is_none() && rx.progress().received_chunks == manifest.total_chunks as usize {
        saved_path = Some(rx.try_finalize(&output_spec)?);
        if completion_elapsed.is_none() {
            completion_elapsed = Some(start.elapsed());
        }
    }

    let total_elapsed = start.elapsed();
    let modeled_display_elapsed =
        Duration::from_secs_f64(frames_generated as f64 / f64::from(args.fps.max(0.5)));

    let (bytes_out, output_sha, sha_match, diff_count) = if let Some(path) = &saved_path {
        let output_bytes = fs::read(path)
            .with_context(|| format!("failed reading reconstructed output {}", path.display()))?;
        let output_sha = sha256_hex(&output_bytes);
        let diff_count = byte_diff_count(&input_bytes, &output_bytes);
        (
            output_bytes.len(),
            Some(output_sha.clone()),
            output_sha == input_sha,
            Some(diff_count),
        )
    } else {
        (0, None, false, None)
    };

    let completed = saved_path.is_some();
    let throughput_base = completion_elapsed.unwrap_or(total_elapsed);
    let effective_kib_per_s = if throughput_base.is_zero() {
        0.0
    } else {
        (bytes_out as f64 / 1024.0) / throughput_base.as_secs_f64()
    };
    let protocol_overhead_ratio = if input_bytes.is_empty() {
        0.0
    } else {
        total_frame_text_bytes as f64 / input_bytes.len() as f64
    };
    let modeled_link_kib_per_s = if modeled_display_elapsed.is_zero() {
        0.0
    } else {
        (bytes_out as f64 / 1024.0) / modeled_display_elapsed.as_secs_f64()
    };

    info!(
        case = %case_name,
        codec = %args.visual_codec.as_str(),
        completed,
        frames_generated,
        frames_decoded,
        "simulation case complete"
    );

    Ok(CaseMetrics {
        case_name,
        input_path: case_path.to_path_buf(),
        output_path: saved_path,
        bytes_in: input_bytes.len(),
        bytes_out,
        input_sha256: input_sha,
        output_sha256: output_sha,
        sha_match,
        byte_diff: diff_count,
        chunk_size: args.chunk_size,
        total_chunks: manifest.total_chunks,
        processed_size: manifest.processed_file_size,
        compression_ratio: if manifest.original_file_size == 0 {
            1.0
        } else {
            manifest.processed_file_size as f64 / manifest.original_file_size as f64
        },
        visual_codec: args.visual_codec,
        frames_in_plan: frames.len(),
        loops: args.loops,
        frames_generated,
        frames_dropped,
        frames_encoded,
        frames_decoded,
        decode_failures,
        duplicate_chunks: rx.duplicate_chunks,
        invalid_chunks: rx.invalid_chunks,
        accepted_chunks: rx.accepted_chunks,
        completed,
        total_elapsed,
        encode_elapsed,
        decode_elapsed,
        completion_elapsed,
        modeled_display_elapsed,
        effective_kib_per_s,
        modeled_link_kib_per_s,
        protocol_overhead_ratio,
    })
}

fn color_grid_cfg(args: &SimulateArgs) -> Option<ColorGridConfig> {
    if !matches!(args.visual_codec, VisualCodecArg::ColorGrid) {
        return None;
    }
    Some(ColorGridConfig {
        grid_side: args.grid_side,
        cell_pixels: args.cell_pixels,
        quiet_zone_cells: args.quiet_zone_cells,
        palette: ContrastPalette::BwRg,
    })
}

fn build_transmission_order(frames: &[String], reverse_data_order: bool) -> Vec<String> {
    if !reverse_data_order || frames.len() <= 2 {
        return frames.to_vec();
    }
    let mut out = Vec::with_capacity(frames.len());
    out.push(frames[0].clone());
    let mut data = frames[1..].to_vec();
    data.reverse();
    out.extend(data);
    out
}

fn resolve_cases(args: &SimulateArgs) -> Result<Vec<PathBuf>> {
    if !args.input_files.is_empty() {
        let mut out = Vec::with_capacity(args.input_files.len());
        for path in &args.input_files {
            if !path.is_file() {
                bail!(
                    "input file does not exist or is not a file: {}",
                    path.display()
                );
            }
            out.push(path.clone());
        }
        return Ok(out);
    }

    let generated_dir = args.output_dir.join("generated-inputs");
    fs::create_dir_all(&generated_dir)
        .with_context(|| format!("failed creating {}", generated_dir.display()))?;

    let defaults = vec![
        ("case-1kb.bin", generate_pattern_bytes(1024)),
        ("case-10kb.bin", generate_pattern_bytes(10 * 1024)),
        ("case-100kb.bin", generate_pattern_bytes(100 * 1024)),
        ("case-1mb.bin", generate_pattern_bytes(1024 * 1024)),
        ("case-text.txt", generate_text_case()),
    ];

    let mut out = Vec::with_capacity(defaults.len());
    for (name, bytes) in defaults {
        let path = generated_dir.join(name);
        fs::write(&path, bytes)
            .with_context(|| format!("failed writing generated case {}", path.display()))?;
        out.push(path);
    }
    Ok(out)
}

fn generate_pattern_bytes(size: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(size);
    for i in 0..size {
        out.push(((i * 37 + 17) % 251) as u8);
    }
    out
}

fn generate_text_case() -> Vec<u8> {
    let mut s = String::new();
    for idx in 0..600 {
        s.push_str("StareDrop simulation line ");
        s.push_str(&idx.to_string());
        s.push('\n');
    }
    s.into_bytes()
}

fn corrupt_frame_text(frame_text: &str) -> String {
    let mut bytes = frame_text.as_bytes().to_vec();
    let mut i = bytes.len().saturating_sub(1);
    while i > 0 {
        if bytes[i].is_ascii_digit() {
            bytes[i] = if bytes[i] == b'9' { b'0' } else { bytes[i] + 1 };
            return String::from_utf8(bytes).unwrap_or_else(|_| frame_text.to_string());
        }
        i -= 1;
    }
    format!("{frame_text}x")
}

fn byte_diff_count(a: &[u8], b: &[u8]) -> usize {
    let common = a.len().min(b.len());
    let mut diff = 0usize;
    for idx in 0..common {
        if a[idx] != b[idx] {
            diff += 1;
        }
    }
    diff + a.len().abs_diff(b.len())
}

fn print_case_summary(m: &CaseMetrics) {
    let completion_ms = m
        .completion_elapsed
        .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
        .unwrap_or_else(|| "n/a".to_string());
    println!("Case: {}", m.case_name);
    println!(
        "  codec: {}, input: {} bytes ({})",
        m.visual_codec.as_str(),
        m.bytes_in,
        m.input_path.display()
    );
    println!(
        "  output: {}",
        m.output_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "not reconstructed".to_string())
    );
    println!(
        "  chunks: {} total, chunk-size {}",
        m.total_chunks, m.chunk_size
    );
    println!(
        "  frames: plan={}, generated={}, dropped={}, encoded={}, decoded={}, decode-fail={}",
        m.frames_in_plan,
        m.frames_generated,
        m.frames_dropped,
        m.frames_encoded,
        m.frames_decoded,
        m.decode_failures
    );
    println!(
        "  receiver: accepted={}, duplicate={}, invalid={}",
        m.accepted_chunks, m.duplicate_chunks, m.invalid_chunks
    );
    println!(
        "  integrity: complete={}, sha-match={}, byte-diff={}",
        m.completed,
        m.sha_match,
        m.byte_diff
            .map(|v| v.to_string())
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!(
        "  timing-ms: total={:.2}, completion={}, encode={:.2}, decode={:.2}, modeled-display={:.2}",
        m.total_elapsed.as_secs_f64() * 1000.0,
        completion_ms,
        m.encode_elapsed.as_secs_f64() * 1000.0,
        m.decode_elapsed.as_secs_f64() * 1000.0,
        m.modeled_display_elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "  rates: host-throughput={:.2} KiB/s, modeled-link={:.2} KiB/s, compression-ratio={:.4} (processed {} B), protocol-overhead={:.2}x",
        m.effective_kib_per_s,
        m.modeled_link_kib_per_s,
        m.compression_ratio,
        m.processed_size,
        m.protocol_overhead_ratio
    );
    println!();
}

fn write_summary_files(output_dir: &Path, results: &[CaseMetrics]) -> Result<()> {
    let csv_path = output_dir.join("simulation-summary.csv");
    let mut csv = String::from(
        "case_name,visual_codec,bytes_in,bytes_out,processed_size,chunk_size,total_chunks,frames_plan,loops,frames_generated,frames_dropped,frames_encoded,frames_decoded,decode_failures,accepted_chunks,duplicate_chunks,invalid_chunks,completed,sha_match,byte_diff,total_ms,completion_ms,encode_ms,decode_ms,modeled_display_ms,throughput_kib_s,modeled_link_kib_s,compression_ratio,protocol_overhead_ratio,input_sha256,output_sha256,input_path,output_path\n",
    );
    for m in results {
        let completion_ms = m
            .completion_elapsed
            .map(|d| format!("{:.3}", d.as_secs_f64() * 1000.0))
            .unwrap_or_default();
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.3},{},{:.3},{:.3},{:.3},{:.4},{:.4},{:.6},{:.6},{},{},{},{}\n",
            escape_csv(&m.case_name),
            m.visual_codec.as_str(),
            m.bytes_in,
            m.bytes_out,
            m.processed_size,
            m.chunk_size,
            m.total_chunks,
            m.frames_in_plan,
            m.loops,
            m.frames_generated,
            m.frames_dropped,
            m.frames_encoded,
            m.frames_decoded,
            m.decode_failures,
            m.accepted_chunks,
            m.duplicate_chunks,
            m.invalid_chunks,
            m.completed,
            m.sha_match,
            m.byte_diff.map(|v| v.to_string()).unwrap_or_default(),
            m.total_elapsed.as_secs_f64() * 1000.0,
            completion_ms,
            m.encode_elapsed.as_secs_f64() * 1000.0,
            m.decode_elapsed.as_secs_f64() * 1000.0,
            m.modeled_display_elapsed.as_secs_f64() * 1000.0,
            m.effective_kib_per_s,
            m.modeled_link_kib_per_s,
            m.compression_ratio,
            m.protocol_overhead_ratio,
            escape_csv(&m.input_sha256),
            escape_csv(m.output_sha256.as_deref().unwrap_or("")),
            escape_csv(&m.input_path.display().to_string()),
            escape_csv(
                &m.output_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            )
        ));
    }
    fs::write(&csv_path, csv).with_context(|| format!("failed writing {}", csv_path.display()))?;

    let txt_path = output_dir.join("simulation-summary.txt");
    let mut text = String::new();
    for m in results {
        text.push_str(&format!("Case: {}\n", m.case_name));
        text.push_str(&format!("  codec: {}\n", m.visual_codec.as_str()));
        text.push_str(&format!("  input: {} bytes\n", m.bytes_in));
        text.push_str(&format!("  output: {} bytes\n", m.bytes_out));
        text.push_str(&format!(
            "  complete={}, sha_match={}, byte_diff={}\n",
            m.completed,
            m.sha_match,
            m.byte_diff
                .map(|v| v.to_string())
                .unwrap_or_else(|| "n/a".to_string())
        ));
        text.push_str(&format!(
            "  host_throughput_kib_s={:.4}, modeled_link_kib_s={:.4}, overhead={:.4}x\n\n",
            m.effective_kib_per_s, m.modeled_link_kib_per_s, m.protocol_overhead_ratio
        ));
    }
    fs::write(&txt_path, text).with_context(|| format!("failed writing {}", txt_path.display()))?;

    println!("Wrote {}", csv_path.display());
    println!("Wrote {}", txt_path.display());
    println!();
    Ok(())
}

fn append_history_csv(args: &SimulateArgs, results: &[CaseMetrics]) -> Result<()> {
    if let Some(parent) = args.history_file.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating {}", parent.display()))?;
    }

    let header = "timestamp_utc,run_label,git_commit,visual_codec,input_case,chunk_size,fps,loops,drop_every,corrupt_every,reverse_data_order,grid_side,cell_pixels,quiet_zone_cells,bytes_in,bytes_out,total_chunks,frames_plan,frames_generated,frames_dropped,frames_decoded,decode_failures,accepted_chunks,duplicate_chunks,invalid_chunks,completed,sha_match,byte_diff,completion_ms,total_ms,throughput_kib_s,modeled_link_kib_s,protocol_overhead_ratio,input_path,output_path,run_output_dir\n";

    let exists = args.history_file.exists();
    let mut history = if exists {
        fs::read_to_string(&args.history_file)
            .with_context(|| format!("failed reading {}", args.history_file.display()))?
    } else {
        String::new()
    };

    if exists {
        let first = history.lines().next().unwrap_or_default();
        if !first.contains("modeled_link_kib_s") {
            let legacy = args.history_file.with_extension("legacy.csv");
            fs::rename(&args.history_file, &legacy).with_context(|| {
                format!(
                    "failed migrating old history {} -> {}",
                    args.history_file.display(),
                    legacy.display()
                )
            })?;
            println!(
                "History schema upgraded; previous file moved to {}",
                legacy.display()
            );
            history.clear();
        }
    }

    if history.is_empty() {
        history.push_str(header);
    }

    let ts = Utc::now().to_rfc3339();
    let label = args.run_label.as_deref().unwrap_or("");
    let commit = git_commit_short().unwrap_or_else(|| "unknown".to_string());

    for m in results {
        let completion_ms = m
            .completion_elapsed
            .map(|d| format!("{:.3}", d.as_secs_f64() * 1000.0))
            .unwrap_or_default();
        let row = vec![
            escape_csv(&ts),
            escape_csv(label),
            escape_csv(&commit),
            m.visual_codec.as_str().to_string(),
            escape_csv(&m.case_name),
            m.chunk_size.to_string(),
            format!("{:.2}", args.fps),
            args.loops.to_string(),
            args.drop_every.to_string(),
            args.corrupt_every.to_string(),
            args.reverse_data_order.to_string(),
            args.grid_side.to_string(),
            args.cell_pixels.to_string(),
            args.quiet_zone_cells.to_string(),
            m.bytes_in.to_string(),
            m.bytes_out.to_string(),
            m.total_chunks.to_string(),
            m.frames_in_plan.to_string(),
            m.frames_generated.to_string(),
            m.frames_dropped.to_string(),
            m.frames_decoded.to_string(),
            m.decode_failures.to_string(),
            m.accepted_chunks.to_string(),
            m.duplicate_chunks.to_string(),
            m.invalid_chunks.to_string(),
            m.completed.to_string(),
            m.sha_match.to_string(),
            m.byte_diff.map(|v| v.to_string()).unwrap_or_default(),
            completion_ms,
            format!("{:.3}", m.total_elapsed.as_secs_f64() * 1000.0),
            format!("{:.4}", m.effective_kib_per_s),
            format!("{:.4}", m.modeled_link_kib_per_s),
            format!("{:.6}", m.protocol_overhead_ratio),
            escape_csv(&m.input_path.display().to_string()),
            escape_csv(
                &m.output_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            ),
            escape_csv(&args.output_dir.display().to_string()),
        ];
        history.push_str(&row.join(","));
        history.push('\n');
    }

    fs::write(&args.history_file, history)
        .with_context(|| format!("failed writing {}", args.history_file.display()))?;
    println!(
        "Appended benchmark history to {}",
        args.history_file.display()
    );
    println!();
    Ok(())
}

fn git_commit_short() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn print_aggregate_summary(results: &[CaseMetrics]) {
    let total = results.len();
    let completed = results.iter().filter(|m| m.completed).count();
    let hash_ok = results.iter().filter(|m| m.sha_match).count();
    let avg_throughput = if results.is_empty() {
        0.0
    } else {
        results.iter().map(|m| m.effective_kib_per_s).sum::<f64>() / results.len() as f64
    };
    println!("Aggregate:");
    println!("  completed: {}/{}", completed, total);
    println!("  sha256 match: {}/{}", hash_ok, total);
    println!("  avg throughput: {:.2} KiB/s", avg_throughput);
}

fn escape_csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        VisualCodecArg, build_transmission_order, byte_diff_count, color_grid_cfg,
        corrupt_frame_text, generate_pattern_bytes,
    };
    use crate::simulate::SimulateArgs;

    #[test]
    fn pattern_size_matches() {
        let bytes = generate_pattern_bytes(4097);
        assert_eq!(bytes.len(), 4097);
        assert_ne!(bytes[0], bytes[1]);
    }

    #[test]
    fn order_reverses_data_only() {
        let frames = vec![
            "manifest".to_string(),
            "data-1".to_string(),
            "data-2".to_string(),
        ];
        let out = build_transmission_order(&frames, true);
        assert_eq!(out, vec!["manifest", "data-2", "data-1"]);
    }

    #[test]
    fn corrupt_changes_payload() {
        let input = "{\"chunk_index\":12,\"crc32\":12345}";
        let out = corrupt_frame_text(input);
        assert_ne!(input, out);
    }

    #[test]
    fn byte_diff_counts_mismatch_and_len() {
        assert_eq!(byte_diff_count(b"abc", b"abc"), 0);
        assert_eq!(byte_diff_count(b"abc", b"axc"), 1);
        assert_eq!(byte_diff_count(b"abc", b"axcd"), 2);
    }

    #[test]
    fn color_cfg_only_for_color_codec() {
        let base = SimulateArgs {
            input_files: Vec::new(),
            output_dir: "x".into(),
            chunk_size: 700,
            fps: 8.0,
            loops: 1,
            reverse_data_order: false,
            drop_every: 0,
            corrupt_every: 0,
            visual_codec: VisualCodecArg::Qr,
            grid_side: 96,
            cell_pixels: 8,
            quiet_zone_cells: 2,
            record_history: true,
            history_file: "h.csv".into(),
            run_label: None,
        };
        assert!(color_grid_cfg(&base).is_none());

        let mut color = base;
        color.visual_codec = VisualCodecArg::ColorGrid;
        assert!(color_grid_cfg(&color).is_some());
    }
}
