mod cli;
mod codec;
mod filter;
mod format;
mod frame;

use clap::{Parser, Subcommand};
use anyhow::Result;
use cli::{
    batch_cmd::BatchArgs,
    clip_cmd::ClipArgs,
    concat_cmd::ConcatArgs,
    decode_cmd::DecodeArgs,
    encode_cmd::EncodeArgs,
    filter_cmd::FilterArgs,
    frames_cmd::FramesArgs,
    gif_cmd::GifArgs,
    meta_cmd::MetaArgs,
    probe_cmd::ProbeArgs,
    thumbnail_cmd::ThumbnailArgs,
    transcode_cmd::TranscodeArgs,
};

#[derive(Parser)]
#[command(name = "ffmpreg")]
#[command(about = "Video encode / decode / transcode / probe utility", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode a synthetic color-bars test pattern to video
    Encode(EncodeArgs),
    /// Decode a video file and print stream info
    Decode(DecodeArgs),
    /// Transcode to a different codec or resolution (copies audio + subtitles)
    Transcode(TranscodeArgs),
    /// Apply an FFmpeg filter chain and re-encode
    Filter(FilterArgs),
    /// Extract a time range from a video file
    Clip(ClipArgs),
    /// Concatenate multiple video files
    Concat(ConcatArgs),
    /// Batch transcode files matching a glob pattern (parallel)
    Batch(BatchArgs),
    /// Extract video frames as PNG images
    Frames(FramesArgs),
    /// Export a video segment as an animated GIF
    Gif(GifArgs),
    /// Extract a single thumbnail at a given timestamp
    Thumbnail(ThumbnailArgs),
    /// Inspect a media file — streams, bitrate, metadata tags
    Probe(ProbeArgs),
    /// Remux a file with updated metadata tags
    Meta(MetaArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Encode(args)    => cli::encode_cmd::run(args),
        Commands::Decode(args)    => cli::decode_cmd::run(args),
        Commands::Transcode(args) => cli::transcode_cmd::run(args),
        Commands::Filter(args)    => cli::filter_cmd::run(args),
        Commands::Clip(args)      => cli::clip_cmd::run(args),
        Commands::Concat(args)    => cli::concat_cmd::run(args),
        Commands::Batch(args)     => cli::batch_cmd::run(args),
        Commands::Frames(args)    => cli::frames_cmd::run(args),
        Commands::Gif(args)       => cli::gif_cmd::run(args),
        Commands::Thumbnail(args) => cli::thumbnail_cmd::run(args),
        Commands::Probe(args)     => cli::probe_cmd::run(args),
        Commands::Meta(args)      => cli::meta_cmd::run(args),
    }
}
