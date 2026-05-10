use clap::Args;
use anyhow::Result;
use crate::codec::frames::{extract_frames, FrameExtractOptions};

#[derive(Args)]
pub struct FramesArgs {
    /// Input file path
    #[arg(short, long)]
    pub input: String,

    /// Directory to write PNG frames into
    #[arg(short, long, default_value = "./frames")]
    pub output_dir: String,

    /// Save every Nth frame (1 = every frame)
    #[arg(short, long, default_value = "1")]
    pub every: u32,

    /// Stop after saving this many frames
    #[arg(short, long)]
    pub max: Option<u32>,
}

pub fn run(args: FramesArgs) -> Result<()> {
    let opts = FrameExtractOptions {
        every_n: args.every,
        max_frames: args.max,
    };

    let saved = extract_frames(&args.input, &args.output_dir, opts, |n| {
        print!("\r  saved frame {n}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    })?;

    println!("\r✓ Saved {saved} frames → {}", args.output_dir);
    Ok(())
}
