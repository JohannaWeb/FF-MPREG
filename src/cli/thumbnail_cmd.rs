use clap::Args;
use anyhow::Result;
use crate::codec::thumbnail::extract_thumbnail;

#[derive(Args)]
pub struct ThumbnailArgs {
    /// Input video file
    #[arg(short, long)]
    pub input: String,

    /// Output PNG path
    #[arg(short, long, default_value = "thumbnail.png")]
    pub output: String,

    /// Timestamp in seconds to capture
    #[arg(short, long, default_value = "0.0")]
    pub time: f64,
}

pub fn run(args: ThumbnailArgs) -> Result<()> {
    extract_thumbnail(&args.input, &args.output, args.time)?;
    println!("✓ Thumbnail saved → {} (at {:.2}s)", args.output, args.time);
    Ok(())
}
