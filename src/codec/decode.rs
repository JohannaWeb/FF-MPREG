use ffmpeg::{codec, frame};
use anyhow::{Context as _, Result};
use crate::format::input::InputContext;

pub struct VideoInfo {
    pub codec_name: String,
    pub width: u32,
    pub height: u32,
    pub avg_frame_rate: f64,
    pub frame_count: u32,
    pub duration: f64,
}

pub fn decode_video(input_path: &str) -> Result<VideoInfo> {
    ffmpeg::init()?;

    let mut input = InputContext::open(input_path)?;

    let codec = codec::decoder::find(input.codec_id)
        .context("Unable to find decoder for codec")?;

    let codec_ctx = input.inner.stream(input.stream_index).unwrap().codec();
    let mut decoder = codec_ctx.decoder().video()?.open_as(codec)?;

    let mut frame_count = 0u32;
    let mut decoded = frame::Video::empty();
    let stream_index = input.stream_index;

    for (stream, packet) in input.inner.packets() {
        if stream.index() != stream_index {
            continue;
        }
        decoder.send_packet(&packet)?;
        while decoder.receive_frame(&mut decoded).is_ok() {
            frame_count += 1;
        }
    }

    decoder.send_eof()?;
    while decoder.receive_frame(&mut decoded).is_ok() {
        frame_count += 1;
    }

    Ok(VideoInfo {
        codec_name: input.codec_name,
        width: input.width,
        height: input.height,
        avg_frame_rate: input.fps(),
        frame_count,
        duration: input.duration_secs(),
    })
}
