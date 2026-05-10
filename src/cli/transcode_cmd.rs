use clap::Args;
use anyhow::Result;
use ffmpeg::codec;
use crate::codec::audio::AudioOptions;
use crate::codec::transcode::transcode_video;
use crate::codec::options::{CodecChoice, EncodeOptions, Quality};
use crate::format::input::InputContext;

#[derive(Args)]
pub struct TranscodeArgs {
    /// Input file path
    #[arg(short, long)]
    pub input: String,

    /// Output file path
    #[arg(short, long)]
    pub output: String,

    /// Video codec: h264, h265, vp9
    #[arg(short, long, default_value = "h264")]
    pub codec: String,

    /// Quality preset: low, medium, high, lossless
    #[arg(short, long, default_value = "medium")]
    pub quality: String,

    /// Target bitrate in bytes/sec (overrides --quality)
    #[arg(short, long, default_value = "0")]
    pub bitrate: usize,

    /// Output width in pixels (default: same as input)
    #[arg(long)]
    pub width: Option<u32>,

    /// Output height in pixels (default: same as input)
    #[arg(long)]
    pub height: Option<u32>,

    /// Audio handling: copy (stream copy) or aac / mp3 (transcode)
    #[arg(long, default_value = "copy")]
    pub audio_codec: String,

    /// Audio bitrate in bytes/sec when transcoding audio
    #[arg(long, default_value = "128000")]
    pub audio_bitrate: usize,
}

pub fn run(args: TranscodeArgs) -> Result<()> {
    let codec: CodecChoice = args.codec.parse()?;
    let quality: Quality = args.quality.parse()?;
    let probe = InputContext::open(&args.input)?;

    let opts = EncodeOptions {
        codec,
        quality,
        bitrate: args.bitrate,
        width: args.width.unwrap_or(probe.width),
        height: args.height.unwrap_or(probe.height),
        framerate: probe.avg_frame_rate,
        ..EncodeOptions::default()
    };

    let audio_opts = if args.audio_codec == "copy" {
        None
    } else {
        let audio_codec_id = match args.audio_codec.to_lowercase().as_str() {
            "aac" => codec::Id::AAC,
            "mp3" => codec::Id::MP3,
            "opus" => codec::Id::OPUS,
            other => anyhow::bail!("Unknown audio codec '{other}'. Use: copy, aac, mp3, opus"),
        };
        Some(AudioOptions { codec_id: audio_codec_id, bitrate: args.audio_bitrate })
    };

    let count = transcode_video(&args.input, &args.output, &opts, audio_opts.as_ref(), |n| {
        print!("\r  transcoding frame {n}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    })?;

    let audio_label = &args.audio_codec;
    println!("\r✓ Transcoded {count} frames → {} [{codec:?} / audio:{audio_label}  {}x{}]",
        args.output, opts.width, opts.height);
    Ok(())
}
