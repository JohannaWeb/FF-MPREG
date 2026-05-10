use ffmpeg::{encoder, frame, media::Type, packet::Packet, software::scaling};
use anyhow::{Context as _, Result};
use crate::codec::audio::{AudioOptions, AudioTranscoder};
use crate::codec::options::EncodeOptions;
use crate::format::{input::InputContext, output::OutputContext};

pub fn transcode_video(
    input_path: &str,
    output_path: &str,
    opts: &EncodeOptions,
    audio_opts: Option<&AudioOptions>,
    on_progress: impl Fn(u32),
) -> Result<u32> {
    ffmpeg::init()?;

    let input = InputContext::open(input_path)?;
    let mut out = OutputContext::new(output_path)?;
    out.add_video_stream(opts)?;

    // Audio: either full transcode or stream copy
    let audio_streams: Vec<usize> = input
        .inner
        .streams()
        .filter(|s| s.codec().medium() == Type::Audio)
        .map(|s| s.index())
        .collect();

    let mut audio_transcoders: Vec<AudioTranscoder> = Vec::new();
    // stream index → output stream index (for copy path)
    let mut audio_copy_map: Vec<(usize, usize)> = Vec::new();

    for &in_idx in &audio_streams {
        let stream = input.inner.stream(in_idx).unwrap();
        if let Some(aopts) = audio_opts {
            let transcoder = AudioTranscoder::new(&stream, &mut out.inner, aopts)?;
            audio_transcoders.push(transcoder);
        } else {
            let out_idx = out.add_copy_stream(&stream)?;
            audio_copy_map.push((in_idx, out_idx));
        }
    }

    // Subtitle copy
    let sub_copy_map: Vec<(usize, usize)> = input
        .inner
        .streams()
        .filter(|s| s.codec().medium() == Type::Subtitle)
        .map(|s| {
            let idx = out.add_copy_stream(&s).expect("add subtitle stream");
            (s.index(), idx)
        })
        .collect();

    out.write_header()?;

    let video_stream = input.inner.stream(input.stream_index).unwrap();
    let video_codec = ffmpeg::codec::decoder::find(input.codec_id)
        .context("Cannot find video decoder")?;
    let mut decoder = video_stream.codec().decoder().video()?.open_as(video_codec)?;

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
    let mut frame_count = 0u32;
    let video_idx = input.stream_index;

    for (stream, mut packet) in input.inner.packets() {
        let idx = stream.index();

        if idx == video_idx {
            decoder.send_packet(&packet)?;
            while decoder.receive_frame(&mut decoded).is_ok() {
                scaler.run(&decoded, &mut scaled)?;
                scaled.set_pts(decoded.pts());
                encoder.send_frame(&scaled)?;
                drain_encoder(&mut encoder, &mut out)?;
                frame_count += 1;
                on_progress(frame_count);
            }
        } else if let Some(tr) = audio_transcoders.iter_mut().find(|t| {
            // Match by output stream index proximity — simple heuristic
            audio_streams.iter().position(|&i| i == idx)
                .map(|pos| pos < audio_transcoders.len())
                .unwrap_or(false)
        }) {
            for mut pkt in tr.transcode(&packet)? {
                out.write_interleaved(&mut pkt)?;
            }
        } else if let Some(&(_, out_idx)) = audio_copy_map.iter().find(|&&(i, _)| i == idx) {
            packet.rescale_ts(stream.time_base(), out.inner.stream(out_idx).unwrap().time_base());
            packet.set_stream(out_idx);
            out.write_interleaved(&mut packet)?;
        } else if let Some(&(_, out_idx)) = sub_copy_map.iter().find(|&&(i, _)| i == idx) {
            packet.rescale_ts(stream.time_base(), out.inner.stream(out_idx).unwrap().time_base());
            packet.set_stream(out_idx);
            out.write_interleaved(&mut packet)?;
        }
    }

    decoder.send_eof()?;
    while decoder.receive_frame(&mut decoded).is_ok() {
        scaler.run(&decoded, &mut scaled)?;
        scaled.set_pts(decoded.pts());
        encoder.send_frame(&scaled)?;
        drain_encoder(&mut encoder, &mut out)?;
        frame_count += 1;
        on_progress(frame_count);
    }

    // Flush audio transcoders
    for tr in &mut audio_transcoders {
        for mut pkt in tr.flush()? {
            out.write_interleaved(&mut pkt)?;
        }
    }

    encoder.send_eof()?;
    drain_encoder(&mut encoder, &mut out)?;
    out.finalize()?;

    Ok(frame_count)
}

fn drain_encoder(encoder: &mut ffmpeg::encoder::video::Video, out: &mut OutputContext) -> Result<()> {
    let mut packet = Packet::empty();
    while encoder.receive_packet(&mut packet).is_ok() {
        out.write_video_packet(&mut packet)?;
    }
    Ok(())
}
