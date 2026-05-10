use clap::Args;
use anyhow::Result;
use crate::codec::concat::concat_videos;
use crate::codec::options::{CodecChoice, EncodeOptions};
use crate::format::input::InputContext;

#[derive(Args)]
pub struct ConcatArgs {
    /// Input files to concatenate, in order
    #[arg(short, long, num_args = 2..)]
    pub inputs: Vec<String>,

    /// Output file
    #[arg(short, long)]
    pub output: String,

    /// Output codec: h264, h265, vp9
    #[arg(short, long, default_value = "h264")]
    pub codec: String,

    /// Target bitrate in bytes/sec (0 = codec default)
    #[arg(short, long, default_value = "0")]
    pub bitrate: usize,
}

pub fn run(args: ConcatArgs) -> Result<()> {
    let codec: CodecChoice = args.codec.parse()?;
    // Use the first input's resolution/framerate as the output spec
    let probe = InputContext::open(&args.inputs[0])?;

    let opts = EncodeOptions {
        codec,
        width: probe.width,
        height: probe.height,
        framerate: probe.avg_frame_rate,
        bitrate: args.bitrate,
        ..EncodeOptions::default()
    };

    let refs: Vec<&str> = args.inputs.iter().map(String::as_str).collect();
    let count = concat_videos(&refs, &args.output, &opts, |n| {
        print!("\r  concat frame {n}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    })?;

    println!("\r✓ Concatenated {} files → {} ({count} frames total)", refs.len(), args.output);
    Ok(())
}
