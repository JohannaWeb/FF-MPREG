use std::fs::File;
use ffmpeg::{codec, format::Pixel, frame, software::scaling};
use image::{codecs::gif::{GifEncoder, Repeat}, Delay, Frame, RgbaImage};
use anyhow::{Context as _, Result};
use crate::format::input::InputContext;
use crate::frame::convert::yuv420p_to_rgb;

pub struct GifOptions {
    pub fps: u32,
    pub width: u32,
    pub max_frames: Option<u32>,
}

impl Default for GifOptions {
    fn default() -> Self {
        Self { fps: 10, width: 480, max_frames: None }
    }
}

pub fn export_gif(
    input_path: &str,
    output_path: &str,
    opts: &GifOptions,
    on_progress: impl Fn(u32),
) -> Result<u32> {
    ffmpeg::init()?;

    let mut input = InputContext::open(input_path)?;

    let src_fps = input.fps();
    // How many input frames to skip per output frame
    let step = (src_fps / opts.fps as f64).round().max(1.0) as u32;

    let video_stream = input.inner.stream(input.stream_index).unwrap();
    let codec = codec::decoder::find(input.codec_id)
        .context("Cannot find video decoder")?;
    let mut decoder = video_stream.codec().decoder().video()?.open_as(codec)?;

    // Scale keeping aspect ratio
    let src_w = decoder.width();
    let src_h = decoder.height();
    let out_w = opts.width;
    let out_h = (src_h as f64 * out_w as f64 / src_w as f64).round() as u32;
    // GIF dimensions must be even
    let out_w = out_w & !1;
    let out_h = out_h & !1;

    let mut scaler = scaling::Context::get(
        decoder.format(), src_w, src_h,
        Pixel::YUV420P, out_w, out_h,
        scaling::Flags::BILINEAR,
    )?;

    let file = File::create(output_path)
        .with_context(|| format!("Cannot create GIF: {output_path}"))?;
    let mut encoder = GifEncoder::new(file);
    encoder.set_repeat(Repeat::Infinite)?;

    // centiseconds per frame (GIF delay unit)
    let delay_cs = (100.0 / opts.fps as f64).round() as u16;

    let mut decoded = frame::Video::empty();
    let mut scaled = frame::Video::new(Pixel::YUV420P, out_w, out_h);
    let mut input_frame_idx = 0u32;
    let mut gif_frame_count = 0u32;
    let stream_index = input.stream_index;

    for (stream, packet) in input.inner.packets() {
        if stream.index() != stream_index {
            continue;
        }
        decoder.send_packet(&packet)?;
        while decoder.receive_frame(&mut decoded).is_ok() {
            if input_frame_idx % step == 0 {
                scaler.run(&decoded, &mut scaled)?;
                let rgb = yuv420p_to_rgb(&scaled);
                let rgba = rgb_to_rgba(rgb, out_w, out_h);
                let frame = Frame::from_parts(rgba, 0, 0, Delay::from_numer_denom_ms(delay_cs as u32 * 10, 1));
                encoder.encode_frame(frame)?;
                gif_frame_count += 1;
                on_progress(gif_frame_count);

                if opts.max_frames.map_or(false, |m| gif_frame_count >= m) {
                    return Ok(gif_frame_count);
                }
            }
            input_frame_idx += 1;
        }
    }

    decoder.send_eof()?;
    while decoder.receive_frame(&mut decoded).is_ok() {
        if input_frame_idx % step == 0 {
            scaler.run(&decoded, &mut scaled)?;
            let rgb = yuv420p_to_rgb(&scaled);
            let rgba = rgb_to_rgba(rgb, out_w, out_h);
            let frame = Frame::from_parts(rgba, 0, 0, Delay::from_numer_denom_ms(delay_cs as u32 * 10, 1));
            encoder.encode_frame(frame)?;
            gif_frame_count += 1;
            on_progress(gif_frame_count);
        }
        input_frame_idx += 1;
    }

    Ok(gif_frame_count)
}

fn rgb_to_rgba(rgb: image::RgbImage, w: u32, h: u32) -> RgbaImage {
    let mut rgba = RgbaImage::new(w, h);
    for (x, y, px) in rgb.enumerate_pixels() {
        rgba.put_pixel(x, y, image::Rgba([px[0], px[1], px[2], 255]));
    }
    rgba
}
