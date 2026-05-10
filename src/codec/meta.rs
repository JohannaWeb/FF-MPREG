use ffmpeg::{format, media::Type};
use anyhow::{Context as _, Result};
use std::collections::HashMap;

/// Read all metadata tags from a file.
pub fn read_tags(input_path: &str) -> Result<HashMap<String, String>> {
    ffmpeg::init()?;
    let ctx = format::input(input_path)
        .with_context(|| format!("Cannot open: {input_path}"))?;
    let tags = ctx
        .metadata()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    Ok(tags)
}

/// Remux `input_path` to `output_path`, stream-copying everything, and
/// applying the provided `tags` on top of any existing metadata.
pub fn write_tags(
    input_path: &str,
    output_path: &str,
    tags: &HashMap<String, String>,
) -> Result<()> {
    ffmpeg::init()?;

    let mut ictx = format::input(input_path)
        .with_context(|| format!("Cannot open: {input_path}"))?;
    let mut octx = format::output(output_path)
        .with_context(|| format!("Cannot create: {output_path}"))?;

    // Copy each stream
    for stream in ictx.streams() {
        let mut out_stream = octx.add_stream(ffmpeg::codec::Id::None)?;
        out_stream.set_parameters(stream.codec());
        unsafe {
            (*out_stream.parameters().as_mut_ptr()).codec_tag = 0;
        }
    }

    // Copy existing metadata then overlay new tags
    let existing: Vec<(String, String)> = ictx
        .metadata()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    for (k, v) in &existing {
        if !tags.contains_key(k.as_str()) {
            octx.metadata_mut().set(k, v);
        }
    }
    for (k, v) in tags {
        octx.metadata_mut().set(k, v);
    }

    octx.write_header()?;

    // Copy all packets
    for (in_stream, mut packet) in ictx.packets() {
        let out_stream = octx.stream(in_stream.index()).unwrap();
        packet.rescale_ts(in_stream.time_base(), out_stream.time_base());
        packet.set_stream(in_stream.index());
        packet.write_interleaved(&mut octx)?;
    }

    octx.write_trailer()?;
    Ok(())
}
