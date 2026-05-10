use ffmpeg::{codec, format::Pixel, Rational};
use std::str::FromStr;
use anyhow::anyhow;

#[derive(Clone, Copy, Debug, Default)]
pub enum CodecChoice {
    #[default]
    H264,
    H265,
    Vp9,
}

impl CodecChoice {
    pub fn codec_id(self) -> codec::Id {
        match self {
            Self::H264 => codec::Id::H264,
            Self::H265 => codec::Id::HEVC,
            Self::Vp9 => codec::Id::VP9,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::H264 => "h264",
            Self::H265 => "h265",
            Self::Vp9 => "vp9",
        }
    }
}

impl FromStr for CodecChoice {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "h264" | "avc" => Ok(Self::H264),
            "h265" | "hevc" => Ok(Self::H265),
            "vp9" => Ok(Self::Vp9),
            _ => Err(anyhow!("Unknown codec '{s}'. Use: h264, h265, vp9")),
        }
    }
}

/// Friendly quality tier — maps to a target bitrate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Quality {
    Low,
    #[default]
    Medium,
    High,
    Lossless,
}

impl Quality {
    /// Target bitrate in bytes/sec. Returns 0 for Lossless (codec decides).
    pub fn bitrate_bps(self) -> usize {
        match self {
            Self::Low => 500_000,
            Self::Medium => 2_000_000,
            Self::High => 8_000_000,
            Self::Lossless => 0,
        }
    }
}

impl FromStr for Quality {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" | "med" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "lossless" => Ok(Self::Lossless),
            _ => Err(anyhow!("Unknown quality '{s}'. Use: low, medium, high, lossless")),
        }
    }
}

pub struct EncodeOptions {
    pub codec: CodecChoice,
    pub width: u32,
    pub height: u32,
    pub framerate: Rational,
    pub pixel_format: Pixel,
    /// Explicit bitrate — takes precedence over `quality` when non-zero.
    pub bitrate: usize,
    pub quality: Quality,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            codec: CodecChoice::H264,
            width: 640,
            height: 480,
            framerate: Rational(30, 1),
            pixel_format: Pixel::YUV420P,
            bitrate: 0,
            quality: Quality::default(),
        }
    }
}

impl EncodeOptions {
    pub fn codec_id(&self) -> codec::Id {
        self.codec.codec_id()
    }

    pub fn time_base(&self) -> Rational {
        Rational(self.framerate.1, self.framerate.0)
    }

    /// Resolved bitrate: explicit `bitrate` wins; falls back to quality tier.
    pub fn effective_bitrate(&self) -> usize {
        if self.bitrate > 0 {
            self.bitrate
        } else {
            self.quality.bitrate_bps()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_choice_roundtrip() {
        assert!(matches!("h264".parse::<CodecChoice>().unwrap(), CodecChoice::H264));
        assert!(matches!("avc".parse::<CodecChoice>().unwrap(), CodecChoice::H264));
        assert!(matches!("h265".parse::<CodecChoice>().unwrap(), CodecChoice::H265));
        assert!(matches!("hevc".parse::<CodecChoice>().unwrap(), CodecChoice::H265));
        assert!(matches!("vp9".parse::<CodecChoice>().unwrap(), CodecChoice::Vp9));
        assert!("bogus".parse::<CodecChoice>().is_err());
    }

    #[test]
    fn quality_bitrates_are_ordered() {
        assert!(Quality::Low.bitrate_bps() < Quality::Medium.bitrate_bps());
        assert!(Quality::Medium.bitrate_bps() < Quality::High.bitrate_bps());
        assert_eq!(Quality::Lossless.bitrate_bps(), 0);
    }

    #[test]
    fn effective_bitrate_prefers_explicit() {
        let opts = EncodeOptions { bitrate: 1_000_000, quality: Quality::Low, ..Default::default() };
        assert_eq!(opts.effective_bitrate(), 1_000_000);
    }

    #[test]
    fn effective_bitrate_falls_back_to_quality() {
        let opts = EncodeOptions { bitrate: 0, quality: Quality::High, ..Default::default() };
        assert_eq!(opts.effective_bitrate(), Quality::High.bitrate_bps());
    }
}
