use std::collections::HashMap;
use clap::Args;
use anyhow::Result;
use crate::codec::meta::write_tags;

#[derive(Args)]
pub struct MetaArgs {
    /// Input file
    #[arg(short, long)]
    pub input: String,

    /// Output file (remuxed copy with new tags)
    #[arg(short, long)]
    pub output: String,

    /// Tags to set in key=value format (repeatable)
    /// e.g. --tag title="My Video" --tag artist="Me"
    #[arg(short, long = "tag", value_name = "KEY=VALUE", num_args = 1)]
    pub tags: Vec<String>,
}

pub fn run(args: MetaArgs) -> Result<()> {
    let mut tags = HashMap::new();
    for tag in &args.tags {
        let (k, v) = tag.split_once('=')
            .ok_or_else(|| anyhow::anyhow!("Invalid tag format '{tag}'. Use KEY=VALUE"))?;
        tags.insert(k.to_string(), v.to_string());
    }

    write_tags(&args.input, &args.output, &tags)?;

    println!("✓ Remuxed with {} tag(s) → {}", tags.len(), args.output);
    for (k, v) in &tags {
        println!("  {k} = {v}");
    }
    Ok(())
}
