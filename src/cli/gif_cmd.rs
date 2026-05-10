use clap::Args;
use anyhow::Result;
use crate::codec::gif::{export_gif, GifOptions};

#[derive(Args)]
pub struct GifArgs {
    /// Input video file
    #[arg(short, long)]
    pub input: String,

    /// Output GIF path
    #[arg(short, long, default_value = "out.gif")]
    pub output: String,

    /// Output framerate (lower = smaller file)
    #[arg(short, long, default_value = "10")]
    pub fps: u32,

    /// Output width in pixels (height scales proportionally)
    #[arg(short, long, default_value = "480")]
    pub width: u32,

    /// Stop after this many GIF frames
    #[arg(short, long)]
    pub max: Option<u32>,
}

pub fn run(args: GifArgs) -> Result<()> {
    let opts = GifOptions {
        fps: args.fps,
        width: args.width,
        max_frames: args.max,
    };

    let count = export_gif(&args.input, &args.output, &opts, |n| {
        print!("\r  gif frame {n}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    })?;

    println!("\r✓ Exported {count} frames → {} [{}fps  {}px wide]",
        args.output, args.fps, args.width);
    Ok(())
}
