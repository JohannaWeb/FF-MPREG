use clap::Args;
use anyhow::Result;
use crate::codec::filter_pipeline::filter_video;
use crate::codec::options::{CodecChoice, EncodeOptions};
use crate::format::input::InputContext;

#[derive(Args)]
pub struct FilterArgs {
    /// Input file
    #[arg(short, long)]
    pub input: String,

    /// Output file
    #[arg(short, long)]
    pub output: String,

    /// FFmpeg filter chain (e.g. "yadif,scale=1280:720" or "eq=brightness=0.1")
    #[arg(short, long)]
    pub filters: String,

    /// Output codec: h264, h265, vp9
    #[arg(short, long, default_value = "h264")]
    pub codec: String,

    /// Target bitrate in bytes/sec (0 = codec default)
    #[arg(short, long, default_value = "0")]
    pub bitrate: usize,
}

pub fn run(args: FilterArgs) -> Result<()> {
    let codec: CodecChoice = args.codec.parse()?;
    let probe = InputContext::open(&args.input)?;

    let opts = EncodeOptions {
        codec,
        width: probe.width,
        height: probe.height,
        framerate: probe.avg_frame_rate,
        bitrate: args.bitrate,
        ..EncodeOptions::default()
    };

    let count = filter_video(&args.input, &args.output, &args.filters, &opts, |n| {
        print!("\r  filtering frame {n}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    })?;

    println!("\r✓ Filtered {count} frames → {} [{}]", args.output, args.filters);
    Ok(())
}
