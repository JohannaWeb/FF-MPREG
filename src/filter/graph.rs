use ffmpeg::{filter, format::Pixel, frame, Rational};
use anyhow::{Context as _, Result};

/// Wraps an ffmpeg filter graph for video with named `in` / `out` pads.
pub struct VideoFilterGraph {
    graph: filter::Graph,
}

impl VideoFilterGraph {
    /// Build a graph that applies `spec` (e.g. `"yadif,scale=1280:720"`) between
    /// a buffer source and a buffersink.
    pub fn new(
        spec: &str,
        width: u32,
        height: u32,
        pix_fmt: Pixel,
        time_base: Rational,
        frame_rate: Rational,
    ) -> Result<Self> {
        let mut graph = filter::Graph::new();

        let src_args = format!(
            "video_size={}x{}:pix_fmt={}:time_base={}/{}:pixel_aspect=1/1:frame_rate={}/{}",
            width,
            height,
            pix_fmt as i32,
            time_base.0,
            time_base.1,
            frame_rate.0,
            frame_rate.1,
        );

        graph.add(
            &filter::find("buffer").context("buffer filter not found")?,
            "in",
            &src_args,
        )?;
        graph.add(
            &filter::find("buffersink").context("buffersink filter not found")?,
            "out",
            "",
        )?;

        // Connect: in → <spec> → out
        graph
            .output("in", 0)?
            .input("out", 0)?
            .parse(spec)?;

        graph.validate()?;

        Ok(Self { graph })
    }

    /// Push a decoded frame into the filter graph.
    pub fn push(&mut self, frame: &frame::Video) -> Result<()> {
        self.graph
            .get("in")
            .context("filter source 'in' not found")?
            .source()
            .add(frame)?;
        Ok(())
    }

    /// Pull the next filtered frame. Returns `false` when no more are available.
    pub fn pull(&mut self, frame: &mut frame::Video) -> bool {
        self.graph
            .get("out")
            .map(|mut ctx| ctx.sink().frame(frame).is_ok())
            .unwrap_or(false)
    }

    /// Signal end-of-stream to the graph.
    pub fn flush(&mut self) -> Result<()> {
        self.graph
            .get("in")
            .context("filter source 'in' not found")?
            .source()
            .flush()?;
        Ok(())
    }
}
