use clap::Args;
use anyhow::Result;
use crate::codec::clip::clip_video;
use crate::codec::options::{CodecChoice, EncodeOptions, Quality};
use crate::format::input::InputContext;

#[derive(Args)]
pub struct ClipArgs {
    /// Input file
    #[arg(short, long)]
    pub input: String,

    /// Output file
    #[arg(short, long)]
    pub output: String,

    /// Start time in seconds
    #[arg(short, long, default_value = "0.0")]
    pub start: f64,

    /// End time in seconds (omit to clip to end of file)
    #[arg(short, long)]
    pub end: Option<f64>,

    /// Output codec: h264, h265, vp9
    #[arg(short, long, default_value = "h264")]
    pub codec: String,

    /// Quality preset: low, medium, high, lossless
    #[arg(short, long, default_value = "medium")]
    pub quality: String,

    /// Target bitrate in bytes/sec (overrides --quality)
    #[arg(short, long, default_value = "0")]
    pub bitrate: usize,
}

pub fn run(args: ClipArgs) -> Result<()> {
    let codec: CodecChoice = args.codec.parse()?;
    let quality: Quality = args.quality.parse()?;
    let probe = InputContext::open(&args.input)?;

    let opts = EncodeOptions {
        codec,
        quality,
        bitrate: args.bitrate,
        width: probe.width,
        height: probe.height,
        framerate: probe.avg_frame_rate,
        ..EncodeOptions::default()
    };

    let end_label = args.end
        .map(|e| format!("{e:.2}s"))
        .unwrap_or_else(|| "end".to_string());

    let count = clip_video(&args.input, &args.output, args.start, args.end, &opts, |n| {
        print!("\r  clipping frame {n}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    })?;

    println!(
        "\r✓ Clipped {count} frames ({:.2}s → {end_label}) → {}",
        args.start, args.output
    );
    Ok(())
}
