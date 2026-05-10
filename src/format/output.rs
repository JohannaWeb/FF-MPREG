use ffmpeg::{encoder, format, media::Type, packet::Packet, Rational};
use anyhow::Result;
use crate::codec::options::EncodeOptions;

pub struct OutputContext {
    pub inner: format::context::Output,
    pub video_stream_index: usize,
}

impl OutputContext {
    /// Create a bare output context with no streams yet. Call `add_*` then `write_header`.
    pub fn new(path: &str) -> Result<Self> {
        let mut ctx = format::output(path)?;
        if ctx.format().flags().global_header() {
            ctx.set_flag(format::flag::Flags::GLOBAL_HEADER);
        }
        Ok(Self { inner: ctx, video_stream_index: 0 })
    }

    /// Add the video encoding stream. Must be called before `write_header`.
    pub fn add_video_stream(&mut self, opts: &EncodeOptions) -> Result<usize> {
        let mut stream = self.inner.add_stream(encoder::find(opts.codec_id()))?;
        stream.set_time_base(opts.time_base());
        self.video_stream_index = stream.index();
        Ok(stream.index())
    }

    /// Copy a stream from the input (e.g. audio) without re-encoding.
    pub fn add_copy_stream(&mut self, src: &format::stream::Stream) -> Result<usize> {
        let mut stream = self.inner.add_stream(ffmpeg::codec::Id::None)?;
        stream.set_parameters(src.codec());
        // Codec tag must be 0 for stream copy
        unsafe {
            (*stream.parameters().as_mut_ptr()).codec_tag = 0;
        }
        Ok(stream.index())
    }

    pub fn write_header(&mut self) -> Result<()> {
        self.inner.write_header()?;
        Ok(())
    }

    /// Write a video encoder packet (sets stream index automatically).
    pub fn write_video_packet(&mut self, packet: &mut Packet) -> Result<()> {
        packet.set_stream(self.video_stream_index);
        packet.write_interleaved(&mut self.inner)?;
        Ok(())
    }

    /// Write a packet that already has its stream index set (e.g. copied audio).
    pub fn write_interleaved(&mut self, packet: &mut Packet) -> Result<()> {
        packet.write_interleaved(&mut self.inner)?;
        Ok(())
    }

    pub fn finalize(&mut self) -> Result<()> {
        self.inner.write_trailer()?;
        Ok(())
    }

    /// Convenience: create, add one video stream, write header.
    pub fn open(path: &str, opts: &EncodeOptions) -> Result<Self> {
        let mut ctx = Self::new(path)?;
        ctx.add_video_stream(opts)?;
        ctx.write_header()?;
        Ok(ctx)
    }
}
