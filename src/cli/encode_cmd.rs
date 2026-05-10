use clap::Args;
use anyhow::Result;
use crate::codec::encode::encode_video;
use crate::codec::options::{CodecChoice, EncodeOptions, Quality};

#[derive(Args)]
pub struct EncodeArgs {
    /// Output file path
    #[arg(short, long)]
    pub output: String,

    /// Video width in pixels
    #[arg(short, long, default_value = "640")]
    pub width: u32,

    /// Video height in pixels
    #[arg(long, default_value = "480")]
    pub height: u32,

    /// Number of frames to encode
    #[arg(short, long, default_value = "120")]
    pub frames: u32,

    /// Codec: h264, h265, vp9
    #[arg(short, long, default_value = "h264")]
    pub codec: String,

    /// Target bitrate in bytes/sec — overrides --quality when set
    #[arg(short, long, default_value = "0")]
    pub bitrate: usize,

    /// Quality preset: low, medium, high, lossless
    #[arg(short, long, default_value = "medium")]
    pub quality: String,
}

pub fn run(args: EncodeArgs) -> Result<()> {
    let codec: CodecChoice = args.codec.parse()?;
    let quality: Quality = args.quality.parse()?;
    let opts = EncodeOptions {
        codec,
        width: args.width,
        height: args.height,
        bitrate: args.bitrate,
        quality,
        ..EncodeOptions::default()
    };

    let total = args.frames;
    encode_video(&args.output, &opts, total, |n| {
        print!("\r  encoding frame {n}/{total}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    })?;

    println!("\r✓ Encoded {total} frames → {} [{codec:?}]", args.output);
    Ok(())
}
