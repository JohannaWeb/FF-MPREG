use clap::Args;
use anyhow::Result;
use crate::codec::probe::probe;

#[derive(Args)]
pub struct ProbeArgs {
    /// Input file to inspect
    #[arg(short, long)]
    pub input: String,
}

pub fn run(args: ProbeArgs) -> Result<()> {
    let info = probe(&args.input)?;

    println!("Format:   {}", info.format);
    println!("Duration: {:.2}s", info.duration_secs);
    println!("Bitrate:  {} kb/s", info.bitrate / 1000);
    println!("Streams:");
    for s in &info.streams {
        println!("  [{}] {} — {}  {}", s.index, s.kind, s.codec, s.details);
    }
    if !info.tags.is_empty() {
        println!("Tags:");
        let mut sorted: Vec<_> = info.tags.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());
        for (k, v) in sorted {
            println!("  {k}: {v}");
        }
    }
    Ok(())
}
