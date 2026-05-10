use std::collections::HashMap;
use ffmpeg::{codec, format, media::Type};
use anyhow::{Context as _, Result};

pub struct StreamInfo {
    pub index: usize,
    pub kind: String,
    pub codec: String,
    pub details: String,
}

pub struct ProbeResult {
    pub format: String,
    pub duration_secs: f64,
    pub bitrate: usize,
    pub streams: Vec<StreamInfo>,
    pub tags: HashMap<String, String>,
}

pub fn probe(input_path: &str) -> Result<ProbeResult> {
    ffmpeg::init()?;

    let ctx = format::input(input_path)
        .with_context(|| format!("Cannot open: {input_path}"))?;

    let fmt = ctx.format();
    let duration = ctx.duration() as f64 / 1_000_000.0;
    let bitrate = ctx.bit_rate() as usize;

    let streams = ctx
        .streams()
        .map(|s| {
            let params = s.codec();
            let kind = match params.medium() {
                Type::Video => "video",
                Type::Audio => "audio",
                Type::Subtitle => "subtitle",
                Type::Data => "data",
                _ => "unknown",
            };

            let codec_name = codec::decoder::find(params.id())
                .map(|c| c.name().to_string())
                .unwrap_or_else(|| format!("{:?}", params.id()));

            let details = match params.medium() {
                Type::Video => {
                    if let Ok(v) = params.decoder().video() {
                        let tb = s.avg_frame_rate();
                        let fps = tb.numerator() as f64 / tb.denominator().max(1) as f64;
                        format!("{}x{}  {fps:.2} fps", v.width(), v.height())
                    } else {
                        String::new()
                    }
                }
                Type::Audio => {
                    if let Ok(a) = params.decoder().audio() {
                        format!(
                            "{} Hz  {} ch  {}",
                            a.rate(),
                            a.channels(),
                            format!("{:?}", a.format()).to_lowercase()
                        )
                    } else {
                        String::new()
                    }
                }
                _ => String::new(),
            };

            StreamInfo {
                index: s.index(),
                kind: kind.to_string(),
                codec: codec_name,
                details,
            }
        })
        .collect();

    let tags = ctx
        .metadata()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    Ok(ProbeResult {
        format: fmt.name().to_string(),
        duration_secs: duration,
        bitrate,
        streams,
        tags,
    })
}
