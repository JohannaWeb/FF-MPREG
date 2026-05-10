use std::path::PathBuf;
use clap::Args;
use anyhow::Result;
use glob::glob;
use crate::codec::batch::batch_transcode;
use crate::codec::options::{CodecChoice, EncodeOptions};

#[derive(Args)]
pub struct BatchArgs {
    /// Glob pattern for input files (e.g. "videos/*.mp4")
    #[arg(short, long)]
    pub input: String,

    /// Output directory
    #[arg(short, long, default_value = "./out")]
    pub output_dir: String,

    /// Output codec: h264, h265, vp9
    #[arg(short, long, default_value = "h264")]
    pub codec: String,

    /// Target bitrate in bytes/sec (0 = codec default)
    #[arg(short, long, default_value = "0")]
    pub bitrate: usize,

    /// Number of parallel jobs
    #[arg(short, long, default_value = "2")]
    pub jobs: usize,
}

pub fn run(args: BatchArgs) -> Result<()> {
    let codec: CodecChoice = args.codec.parse()?;
    let opts = EncodeOptions {
        codec,
        bitrate: args.bitrate,
        ..EncodeOptions::default()
    };

    let inputs: Vec<PathBuf> = glob(&args.input)?
        .filter_map(|e| e.ok())
        .collect();

    anyhow::ensure!(!inputs.is_empty(), "No files matched: {}", args.input);

    println!("Batch: {} files  {} jobs  [{codec:?}]", inputs.len(), args.jobs);

    let result = batch_transcode(inputs, &args.output_dir, &opts, args.jobs, |path, i, total| {
        println!("  [{i}/{total}] {}", path.display());
    });

    println!("\n✓ Succeeded: {}  Failed: {}  Total frames: {}",
        result.succeeded.len(), result.failed.len(), result.total_frames);

    for (path, err) in &result.failed {
        eprintln!("  ✗ {}: {err}", path.display());
    }

    if !result.failed.is_empty() {
        anyhow::bail!("{} file(s) failed", result.failed.len());
    }

    Ok(())
}
