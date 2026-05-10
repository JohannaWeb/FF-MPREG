use std::path::Path;
use ffmpeg::{codec, format::Pixel, frame, software::scaling};
use anyhow::{Context as _, Result};
use crate::format::input::InputContext;
use crate::frame::convert::yuv420p_to_rgb;

pub fn extract_thumbnail(input_path: &str, output_path: &str, time_secs: f64) -> Result<()> {
    ffmpeg::init()?;

    let mut input = InputContext::open(input_path)?;

    // Seek to requested timestamp (AV_TIME_BASE = 1_000_000 µs)
    let ts_us = (time_secs * 1_000_000.0) as i64;
    input.inner.seek(ts_us, ..ts_us)
        .with_context(|| format!("Failed to seek to {time_secs:.2}s"))?;

    let stream = input.inner.stream(input.stream_index).unwrap();
    let codec = codec::decoder::find(input.codec_id)
        .context("Cannot find video decoder")?;
    let mut decoder = stream.codec().decoder().video()?.open_as(codec)?;

    let mut scaler = scaling::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        Pixel::YUV420P,
        decoder.width(),
        decoder.height(),
        scaling::Flags::BILINEAR,
    )?;

    let mut decoded = frame::Video::empty();
    let mut normalized = frame::Video::new(Pixel::YUV420P, decoder.width(), decoder.height());
    let stream_index = input.stream_index;

    for (stream, packet) in input.inner.packets() {
        if stream.index() != stream_index {
            continue;
        }
        decoder.send_packet(&packet)?;
        if decoder.receive_frame(&mut decoded).is_ok() {
            scaler.run(&decoded, &mut normalized)?;
            let img = yuv420p_to_rgb(&normalized);
            img.save(Path::new(output_path))
                .with_context(|| format!("Failed to save thumbnail to {output_path}"))?;
            return Ok(());
        }
    }

    // Flush — in case frame is buffered
    decoder.send_eof()?;
    if decoder.receive_frame(&mut decoded).is_ok() {
        scaler.run(&decoded, &mut normalized)?;
        let img = yuv420p_to_rgb(&normalized);
        img.save(Path::new(output_path))?;
        return Ok(());
    }

    anyhow::bail!("No frame found at {time_secs:.2}s in {input_path}")
}
