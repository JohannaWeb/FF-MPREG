use ffmpeg::{encoder, frame, media::Type, packet::Packet, software::scaling};
use anyhow::{Context as _, Result};
use crate::codec::options::EncodeOptions;
use crate::format::{input::InputContext, output::OutputContext};

const AV_TIME_BASE: i64 = 1_000_000;

pub fn clip_video(
    input_path: &str,
    output_path: &str,
    start_secs: f64,
    end_secs: Option<f64>,
    opts: &EncodeOptions,
    on_progress: impl Fn(u32),
) -> Result<u32> {
    anyhow::ensure!(
        end_secs.map_or(true, |e| e > start_secs),
        "end time must be greater than start time"
    );

    ffmpeg::init()?;

    let mut input = InputContext::open(input_path)?;

    // Seek to start
    let start_us = (start_secs * AV_TIME_BASE as f64) as i64;
    input.inner.seek(start_us, ..start_us)
        .with_context(|| format!("Cannot seek to {start_secs:.2}s"))?;

    let mut out = OutputContext::new(output_path)?;
    out.add_video_stream(opts)?;
    let copy_map: Vec<(usize, usize)> = input
        .inner
        .streams()
        .filter(|s| matches!(s.codec().medium(), Type::Audio | Type::Subtitle))
        .map(|s| {
            let idx = out.add_copy_stream(&s).expect("add copy stream");
            (s.index(), idx)
        })
        .collect();
    out.write_header()?;

    let video_stream = input.inner.stream(input.stream_index).unwrap();
    let src_tb = video_stream.time_base();
    let codec = ffmpeg::codec::decoder::find(input.codec_id)
        .context("Cannot find video decoder")?;
    let mut decoder = video_stream.codec().decoder().video()?.open_as(codec)?;

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
    // PTS of the first encoded frame — used to re-zero the output timeline
    let mut pts_origin: Option<i64> = None;
    let video_idx = input.stream_index;

    'outer: for (stream, mut packet) in input.inner.packets() {
        let idx = stream.index();

        if idx == video_idx {
            // Check packet PTS against end boundary
            if let (Some(pts), Some(end)) = (packet.pts(), end_secs) {
                let pts_secs = pts as f64 * src_tb.numerator() as f64
                    / src_tb.denominator() as f64;
                if pts_secs > end {
                    break 'outer;
                }
            }

            decoder.send_packet(&packet)?;
            while decoder.receive_frame(&mut decoded).is_ok() {
                scaler.run(&decoded, &mut scaled)?;

                let raw_pts = decoded.pts().unwrap_or(0);
                let origin = pts_origin.get_or_insert(raw_pts);
                scaled.set_pts(Some(raw_pts - *origin));

                encoder.send_frame(&scaled)?;
                drain_encoder(&mut encoder, &mut out)?;
                frame_count += 1;
                on_progress(frame_count);
            }
        } else if let Some(&(_, out_idx)) = copy_map.iter().find(|&&(i, _)| i == idx) {
            packet.rescale_ts(stream.time_base(), out.inner.stream(out_idx).unwrap().time_base());
            packet.set_stream(out_idx);
            out.write_interleaved(&mut packet)?;
        }
    }

    decoder.send_eof()?;
    while decoder.receive_frame(&mut decoded).is_ok() {
        scaler.run(&decoded, &mut scaled)?;
        let raw_pts = decoded.pts().unwrap_or(0);
        let origin = pts_origin.get_or_insert(raw_pts);
        scaled.set_pts(Some(raw_pts - *origin));
        encoder.send_frame(&scaled)?;
        drain_encoder(&mut encoder, &mut out)?;
        frame_count += 1;
        on_progress(frame_count);
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
