use ffmpeg::{encoder, frame, media::Type, packet::Packet};
use anyhow::{Context as _, Result};
use crate::codec::options::EncodeOptions;
use crate::filter::graph::VideoFilterGraph;
use crate::format::{input::InputContext, output::OutputContext};

pub fn filter_video(
    input_path: &str,
    output_path: &str,
    filter_spec: &str,
    opts: &EncodeOptions,
    on_progress: impl Fn(u32),
) -> Result<u32> {
    ffmpeg::init()?;

    let input = InputContext::open(input_path)?;

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
    let codec = ffmpeg::codec::decoder::find(input.codec_id)
        .context("Cannot find video decoder")?;
    let mut decoder = video_stream.codec().decoder().video()?.open_as(codec)?;

    let mut filter_graph = VideoFilterGraph::new(
        filter_spec,
        decoder.width(),
        decoder.height(),
        decoder.format(),
        video_stream.time_base(),
        input.avg_frame_rate,
    )?;

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

    let mut decoded = frame::Video::empty();
    let mut filtered = frame::Video::empty();
    let mut frame_count = 0u32;
    let video_idx = input.stream_index;

    for (stream, mut packet) in input.inner.packets() {
        let idx = stream.index();

        if idx == video_idx {
            decoder.send_packet(&packet)?;
            while decoder.receive_frame(&mut decoded).is_ok() {
                filter_graph.push(&decoded)?;
                while filter_graph.pull(&mut filtered) {
                    encoder.send_frame(&filtered)?;
                    drain_encoder(&mut encoder, &mut out)?;
                    frame_count += 1;
                    on_progress(frame_count);
                }
            }
        } else if let Some(&(_, out_idx)) = copy_map.iter().find(|&&(i, _)| i == idx) {
            packet.rescale_ts(stream.time_base(), out.inner.stream(out_idx).unwrap().time_base());
            packet.set_stream(out_idx);
            out.write_interleaved(&mut packet)?;
        }
    }

    // Flush decoder → filter → encoder
    decoder.send_eof()?;
    while decoder.receive_frame(&mut decoded).is_ok() {
        filter_graph.push(&decoded)?;
        while filter_graph.pull(&mut filtered) {
            encoder.send_frame(&filtered)?;
            drain_encoder(&mut encoder, &mut out)?;
            frame_count += 1;
            on_progress(frame_count);
        }
    }

    filter_graph.flush()?;
    while filter_graph.pull(&mut filtered) {
        encoder.send_frame(&filtered)?;
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
