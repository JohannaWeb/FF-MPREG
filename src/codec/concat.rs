use ffmpeg::{encoder, frame, packet::Packet, software::scaling, Rational};
use anyhow::{Context as _, Result};
use crate::codec::options::EncodeOptions;
use crate::format::{input::InputContext, output::OutputContext};

/// Concatenate multiple video files into one, re-encoding to `opts`.
/// Audio is dropped; use transcode on the result to mux audio back in.
pub fn concat_videos(
    inputs: &[&str],
    output_path: &str,
    opts: &EncodeOptions,
    on_progress: impl Fn(u32),
) -> Result<u32> {
    anyhow::ensure!(!inputs.is_empty(), "No input files provided");

    ffmpeg::init()?;

    let mut out = OutputContext::open(output_path, opts)?;

    let mut encoder_ctx = ffmpeg::codec::context::Context::new()
        .encoder()
        .video()?;
    encoder_ctx.set_width(opts.width);
    encoder_ctx.set_height(opts.height);
    encoder_ctx.set_format(opts.pixel_format);
    encoder_ctx.set_frame_rate(Some(opts.framerate));
    encoder_ctx.set_time_base(opts.time_base());
    encoder_ctx.set_bit_rate(opts.effective_bitrate());
    let mut encoder = encoder_ctx.open_as(encoder::find(opts.codec_id()))?;

    let mut total_frames = 0u32;
    // pts accumulates across files so the output timeline is continuous
    let mut pts_offset: i64 = 0;

    for input_path in inputs {
        let input = InputContext::open(input_path)?;
        let video_stream = input.inner.stream(input.stream_index).unwrap();
        let src_tb = video_stream.time_base();

        let codec = ffmpeg::codec::decoder::find(input.codec_id)
            .with_context(|| format!("Cannot find decoder for {input_path}"))?;
        let mut decoder = video_stream.codec().decoder().video()?.open_as(codec)?;

        let mut scaler = scaling::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            opts.pixel_format,
            opts.width,
            opts.height,
            scaling::Flags::BILINEAR,
        )?;

        let mut decoded = frame::Video::empty();
        let mut scaled = frame::Video::new(opts.pixel_format, opts.width, opts.height);
        let mut file_frames: i64 = 0;

        for (stream, packet) in input.inner.packets() {
            if stream.index() != input.stream_index {
                continue;
            }
            decoder.send_packet(&packet)?;
            while decoder.receive_frame(&mut decoded).is_ok() {
                scaler.run(&decoded, &mut scaled)?;

                // Rescale PTS from input timebase to encoder timebase, then add offset
                let enc_tb = opts.time_base();
                let raw_pts = decoded.pts().unwrap_or(file_frames);
                let rescaled = ffmpeg::rescale::CHANNEL_MASK.rescale(raw_pts, src_tb, enc_tb);
                scaled.set_pts(Some(rescaled + pts_offset));

                encoder.send_frame(&scaled)?;
                drain_encoder(&mut encoder, &mut out)?;
                total_frames += 1;
                file_frames += 1;
                on_progress(total_frames);
            }
        }

        decoder.send_eof()?;
        while decoder.receive_frame(&mut decoded).is_ok() {
            scaler.run(&decoded, &mut scaled)?;
            let enc_tb = opts.time_base();
            let raw_pts = decoded.pts().unwrap_or(file_frames);
            let rescaled = ffmpeg::rescale::CHANNEL_MASK.rescale(raw_pts, src_tb, enc_tb);
            scaled.set_pts(Some(rescaled + pts_offset));
            encoder.send_frame(&scaled)?;
            drain_encoder(&mut encoder, &mut out)?;
            total_frames += 1;
            file_frames += 1;
            on_progress(total_frames);
        }

        // Advance offset by this file's frame count in encoder timebase
        let fps_num = opts.framerate.0 as i64;
        let fps_den = opts.framerate.1 as i64;
        pts_offset += file_frames * fps_den / fps_num;
    }

    encoder.send_eof()?;
    drain_encoder(&mut encoder, &mut out)?;
    out.finalize()?;

    Ok(total_frames)
}

fn drain_encoder(encoder: &mut ffmpeg::encoder::video::Video, out: &mut OutputContext) -> Result<()> {
    let mut packet = Packet::empty();
    while encoder.receive_packet(&mut packet).is_ok() {
        out.write_video_packet(&mut packet)?;
    }
    Ok(())
}
