use ffmpeg::{codec, format, media::Type, Rational};
use anyhow::{Context as _, Result};

pub struct InputContext {
    pub inner: format::context::Input,
    pub stream_index: usize,
    pub codec_id: codec::Id,
    pub codec_name: String,
    pub width: u32,
    pub height: u32,
    pub time_base: Rational,
    pub avg_frame_rate: Rational,
    pub duration: i64,
}

impl InputContext {
    pub fn open(path: &str) -> Result<Self> {
        let ctx = format::input(path)
            .with_context(|| format!("Failed to open input: {path}"))?;

        let stream = ctx
            .streams()
            .best(Type::Video)
            .context("No video stream found")?;

        let stream_index = stream.index();
        let codec_params = stream.codec();
        let codec_id = codec_params.id();

        let codec_name = codec::decoder::find(codec_id)
            .map(|c| c.name().to_string())
            .unwrap_or_else(|| "unknown".into());

        let time_base = stream.time_base();
        let avg_frame_rate = stream.avg_frame_rate();
        let duration = stream.duration();

        let video_ctx = codec_params.decoder().video()?;

        Ok(Self {
            inner: ctx,
            stream_index,
            codec_id,
            codec_name,
            width: video_ctx.width(),
            height: video_ctx.height(),
            time_base,
            avg_frame_rate,
            duration,
        })
    }

    pub fn duration_secs(&self) -> f64 {
        self.duration as f64 * self.time_base.numerator() as f64
            / self.time_base.denominator() as f64
    }

    pub fn fps(&self) -> f64 {
        self.avg_frame_rate.numerator() as f64
            / self.avg_frame_rate.denominator().max(1) as f64
    }
}
