use clap::Args;
use anyhow::Result;
use crate::codec::decode::decode_video;

#[derive(Args)]
pub struct DecodeArgs {
    /// Input file path
    #[arg(short, long)]
    pub input: String,
}

pub fn run(args: DecodeArgs) -> Result<()> {
    let info = decode_video(&args.input)?;
    println!("✓ Decoded video info:");
    println!("  Codec: {}", info.codec_name);
    println!("  Resolution: {}x{}", info.width, info.height);
    println!("  FPS: {:.2}", info.avg_frame_rate);
    println!("  Frames: {}", info.frame_count);
    println!("  Duration: {:.2}s", info.duration);
    Ok(())
}
