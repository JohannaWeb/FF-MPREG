use ffmpeg::{encoder, frame, packet::Packet};
use anyhow::Result;
use crate::codec::options::EncodeOptions;
use crate::format::output::OutputContext;
use crate::frame::generate::color_bars;

pub fn encode_video(
    output_path: &str,
    opts: &EncodeOptions,
    num_frames: u32,
    on_progress: impl Fn(u32),
) -> Result<()> {
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
    let mut raw_frame = frame::Video::new(opts.pixel_format, opts.width, opts.height);

    for i in 0..num_frames {
        color_bars(&mut raw_frame, i, opts.width, opts.height);
        raw_frame.set_pts(Some(i as i64));
        encoder.send_frame(&raw_frame)?;
        drain_encoder(&mut encoder, &mut out)?;
        on_progress(i + 1);
    }

    encoder.send_eof()?;
    drain_encoder(&mut encoder, &mut out)?;

    out.finalize()?;
    Ok(())
}

fn drain_encoder(
    encoder: &mut ffmpeg::encoder::video::Video,
    out: &mut OutputContext,
) -> Result<()> {
    let mut packet = Packet::empty();
    while encoder.receive_packet(&mut packet).is_ok() {
        out.write_video_packet(&mut packet)?;
    }
    Ok(())
}
