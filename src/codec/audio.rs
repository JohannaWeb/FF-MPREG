use ffmpeg::{codec, encoder, frame, packet::Packet, software::resampling, ChannelLayout};
use anyhow::{Context as _, Result};

pub struct AudioOptions {
    pub codec_id: codec::Id,
    pub bitrate: usize,
}

impl Default for AudioOptions {
    fn default() -> Self {
        Self { codec_id: codec::Id::AAC, bitrate: 128_000 }
    }
}

pub struct AudioTranscoder {
    decoder: ffmpeg::decoder::Audio,
    encoder: ffmpeg::encoder::audio::Audio,
    resampler: resampling::Context,
    pub out_stream_index: usize,
}

impl AudioTranscoder {
    pub fn new(
        src_stream: &ffmpeg::format::stream::Stream,
        out_ctx: &mut ffmpeg::format::context::Output,
        opts: &AudioOptions,
    ) -> Result<Self> {
        let src_params = src_stream.codec();

        // Decoder
        let dec_codec = codec::decoder::find(src_params.id())
            .context("Cannot find audio decoder")?;
        let mut decoder = src_params.decoder().audio()?.open_as(dec_codec)?;

        // Encoder
        let enc_codec = encoder::find(opts.codec_id)
            .with_context(|| format!("Cannot find audio encoder {:?}", opts.codec_id))?;

        let mut enc_ctx = ffmpeg::codec::context::Context::new()
            .encoder()
            .audio()?;
        enc_ctx.set_rate(decoder.rate() as i32);
        enc_ctx.set_channel_layout(ChannelLayout::STEREO);
        enc_ctx.set_channels(2);
        enc_ctx.set_format(
            enc_codec
                .audio()
                .map(|a| a.formats().and_then(|mut f| f.next()).unwrap_or(ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed)))
                .unwrap_or(ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed)),
        );
        enc_ctx.set_time_base(ffmpeg::Rational(1, decoder.rate() as i32));
        if opts.bitrate > 0 {
            enc_ctx.set_bit_rate(opts.bitrate);
        }
        let encoder = enc_ctx.open_as(enc_codec)?;

        // Resampler: decoder output → encoder input format
        let resampler = resampling::Context::get(
            decoder.format(),
            decoder.channel_layout(),
            decoder.rate(),
            encoder.format(),
            ChannelLayout::STEREO,
            encoder.rate(),
        )?;

        // Output stream (copy codec params from encoder)
        let mut out_stream = out_ctx.add_stream(enc_codec)?;
        out_stream.set_time_base(ffmpeg::Rational(1, encoder.rate() as i32));
        let out_stream_index = out_stream.index();

        Ok(Self { decoder, encoder, resampler, out_stream_index })
    }

    /// Feed an encoded audio packet; returns re-encoded packets.
    pub fn transcode(&mut self, packet: &Packet) -> Result<Vec<Packet>> {
        self.decoder.send_packet(packet)?;
        let mut out_packets = Vec::new();
        let mut decoded = frame::Audio::empty();

        while self.decoder.receive_frame(&mut decoded).is_ok() {
            let mut resampled = frame::Audio::empty();
            self.resampler.run(&decoded, &mut resampled)?;

            self.encoder.send_frame(&resampled)?;
            let mut pkt = Packet::empty();
            while self.encoder.receive_packet(&mut pkt).is_ok() {
                pkt.set_stream(self.out_stream_index);
                out_packets.push(pkt.clone());
            }
        }

        Ok(out_packets)
    }

    pub fn flush(&mut self) -> Result<Vec<Packet>> {
        self.decoder.send_eof()?;
        let mut out_packets = Vec::new();
        let mut decoded = frame::Audio::empty();

        while self.decoder.receive_frame(&mut decoded).is_ok() {
            let mut resampled = frame::Audio::empty();
            self.resampler.run(&decoded, &mut resampled)?;
            self.encoder.send_frame(&resampled)?;
            let mut pkt = Packet::empty();
            while self.encoder.receive_packet(&mut pkt).is_ok() {
                pkt.set_stream(self.out_stream_index);
                out_packets.push(pkt.clone());
            }
        }

        self.encoder.send_eof()?;
        let mut pkt = Packet::empty();
        while self.encoder.receive_packet(&mut pkt).is_ok() {
            pkt.set_stream(self.out_stream_index);
            out_packets.push(pkt.clone());
        }

        Ok(out_packets)
    }
}
