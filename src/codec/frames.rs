use std::path::Path;
use ffmpeg::{codec, format::Pixel, frame, software::scaling};
use anyhow::{Context as _, Result};
use crate::format::input::InputContext;
use crate::frame::convert::yuv420p_to_rgb;

pub struct FrameExtractOptions {
    pub every_n: u32,
    pub max_frames: Option<u32>,
}

impl Default for FrameExtractOptions {
    fn default() -> Self {
        Self { every_n: 1, max_frames: None }
    }
}

pub fn extract_frames(
    input_path: &str,
    output_dir: &str,
    opts: FrameExtractOptions,
    on_progress: impl Fn(u32),
) -> Result<u32> {
    ffmpeg::init()?;

    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Cannot create output dir: {output_dir}"))?;

    let mut input = InputContext::open(input_path)?;
    let stream = input.inner.stream(input.stream_index).unwrap();
    let src_format = stream.codec().decoder().video()?.format();
    let src_width = stream.codec().decoder().video()?.width();
    let src_height = stream.codec().decoder().video()?.height();

    let codec = codec::decoder::find(input.codec_id)
        .context("Cannot find decoder")?;
    let mut decoder = stream.codec().decoder().video()?.open_as(codec)?;

    let mut scaler = scaling::Context::get(
        src_format,
        src_width,
        src_height,
        Pixel::YUV420P,
        src_width,
        src_height,
        scaling::Flags::BILINEAR,
    )?;

    let mut decoded = frame::Video::empty();
    let mut normalized = frame::Video::new(Pixel::YUV420P, src_width, src_height);
    let mut frame_count = 0u32;
    let mut saved = 0u32;
    let stream_index = input.stream_index;

    for (stream, packet) in input.inner.packets() {
        if stream.index() != stream_index {
            continue;
        }
        decoder.send_packet(&packet)?;
        while decoder.receive_frame(&mut decoded).is_ok() {
            frame_count += 1;

            if frame_count % opts.every_n != 0 {
                continue;
            }

            scaler.run(&decoded, &mut normalized)?;
            let img = yuv420p_to_rgb(&normalized);
            let path = Path::new(output_dir).join(format!("frame_{frame_count:06}.png"));
            img.save(&path)
                .with_context(|| format!("Failed to save {}", path.display()))?;
            saved += 1;
            on_progress(saved);

            if opts.max_frames.map_or(false, |max| saved >= max) {
                return Ok(saved);
            }
        }
    }

    decoder.send_eof()?;
    while decoder.receive_frame(&mut decoded).is_ok() {
        frame_count += 1;
        if frame_count % opts.every_n == 0 {
            scaler.run(&decoded, &mut normalized)?;
            let img = yuv420p_to_rgb(&normalized);
            let path = Path::new(output_dir).join(format!("frame_{frame_count:06}.png"));
            img.save(&path)?;
            saved += 1;
            on_progress(saved);
        }
    }

    Ok(saved)
}
