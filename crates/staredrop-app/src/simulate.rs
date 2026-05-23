use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use clap::{ArgAction, Args};
use staredrop_codec_qr::{decode_first_qr_text, encode_text_to_qr_luma};
use staredrop_crypto::hash::sha256_hex;
use tracing::info;

use crate::transfer::{
    OutputSpec, ReceiverSession, SenderBuildOptions, SenderPlan, build_file_sender_plan,
};

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
        help = "Corrupt every Nth DATA frame text before QR encode (0 disables)"
    )]
    pub corrupt_every: u32,
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
    protocol_overhead_ratio: f64,
}

pub fn run_simulation_suite(args: SimulateArgs) -> Result<()> {
    if args.chunk_size == 0 {
        bail!("--chunk-size must be > 0");
    }
    if args.loops == 0 {
        bail!("--loops must be > 0");
    }

    fs::create_dir_all(&args.output_dir)
        .with_context(|| format!("failed creating {}", args.output_dir.display()))?;

    let cases = resolve_cases(&args)?;
    if cases.is_empty() {
        bail!("no simulation cases found");
    }

    println!("StareDrop simulation");
    println!("  output-dir: {}", args.output_dir.display());
    println!("  chunk-size: {}", args.chunk_size);
    println!("  loops: {}", args.loops);
    println!("  fps(model): {:.2}", args.fps.max(0.5));
    println!("  reverse-data-order: {}", args.reverse_data_order);
    println!("  drop-every: {}", args.drop_every);
    println!("  corrupt-every: {}", args.corrupt_every);
    println!("  cases: {}", cases.len());
    println!();

    let mut results = Vec::with_capacity(cases.len());
    for case_path in cases {
        let metrics = run_case(&case_path, &args)?;
        print_case_summary(&metrics);
        results.push(metrics);
    }

    write_summary_files(&args.output_dir, &results)?;
    print_aggregate_summary(&results);
    Ok(())
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
            let qr = encode_text_to_qr_luma(&effective_text)
                .with_context(|| format!("failed encoding frame for case {}", case_name))?;
            encode_elapsed += enc_start.elapsed();
            frames_encoded += 1;

            let dec_start = Instant::now();
            let decoded = decode_first_qr_text(&qr)
                .with_context(|| format!("failed decoding frame for case {}", case_name))?;
            decode_elapsed += dec_start.elapsed();

            let Some(decoded_text) = decoded else {
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

    info!(
        case = %case_name,
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
        protocol_overhead_ratio,
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
        s.push_str("StareDrop Phase2 simulation line ");
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
    println!("  input: {} bytes ({})", m.bytes_in, m.input_path.display());
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
        "  rates: throughput={:.2} KiB/s, compression-ratio={:.4} (processed {} B), protocol-overhead={:.2}x",
        m.effective_kib_per_s, m.compression_ratio, m.processed_size, m.protocol_overhead_ratio
    );
    println!();
}

fn write_summary_files(output_dir: &Path, results: &[CaseMetrics]) -> Result<()> {
    let csv_path = output_dir.join("simulation-summary.csv");
    let mut csv = String::from(
        "case_name,bytes_in,bytes_out,processed_size,chunk_size,total_chunks,frames_plan,loops,frames_generated,frames_dropped,frames_encoded,frames_decoded,decode_failures,accepted_chunks,duplicate_chunks,invalid_chunks,completed,sha_match,byte_diff,total_ms,completion_ms,encode_ms,decode_ms,modeled_display_ms,throughput_kib_s,compression_ratio,protocol_overhead_ratio,input_sha256,output_sha256,input_path,output_path\n",
    );
    for m in results {
        let completion_ms = m
            .completion_elapsed
            .map(|d| format!("{:.3}", d.as_secs_f64() * 1000.0))
            .unwrap_or_default();
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.3},{},{:.3},{:.3},{:.3},{:.4},{:.6},{:.6},{},{},{},{}\n",
            escape_csv(&m.case_name),
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
            "  throughput_kib_s={:.4}, overhead={:.4}x\n\n",
            m.effective_kib_per_s, m.protocol_overhead_ratio
        ));
    }
    fs::write(&txt_path, text).with_context(|| format!("failed writing {}", txt_path.display()))?;

    println!("Wrote {}", csv_path.display());
    println!("Wrote {}", txt_path.display());
    println!();
    Ok(())
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
        build_transmission_order, byte_diff_count, corrupt_frame_text, generate_pattern_bytes,
    };

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
}
