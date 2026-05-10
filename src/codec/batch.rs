use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use rayon::prelude::*;
use anyhow::Result;
use crate::codec::options::EncodeOptions;
use crate::codec::transcode::transcode_video;

pub struct BatchResult {
    pub succeeded: Vec<PathBuf>,
    pub failed: Vec<(PathBuf, String)>,
    pub total_frames: u32,
}

pub fn batch_transcode(
    inputs: Vec<PathBuf>,
    output_dir: &str,
    opts: &EncodeOptions,
    jobs: usize,
    on_file: impl Fn(&Path, usize, usize) + Sync,
) -> BatchResult {
    std::fs::create_dir_all(output_dir).ok();

    rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .expect("thread pool")
        .install(|| {
            let total_files = inputs.len();
            let total_frames = Arc::new(AtomicU32::new(0));
            let succeeded = std::sync::Mutex::new(Vec::new());
            let failed = std::sync::Mutex::new(Vec::new());

            inputs.par_iter().enumerate().for_each(|(i, input)| {
                on_file(input, i + 1, total_files);

                let stem = input.file_stem().unwrap_or_default().to_string_lossy();
                let ext = match opts.codec {
                    crate::codec::options::CodecChoice::Vp9 => "webm",
                    _ => "mp4",
                };
                let output = PathBuf::from(output_dir).join(format!("{stem}.{ext}"));

                let frames_ref = Arc::clone(&total_frames);
                let result = transcode_video(
                    input.to_str().unwrap_or(""),
                    output.to_str().unwrap_or(""),
                    opts,
                    None, // audio: stream copy
                    |_| { frames_ref.fetch_add(1, Ordering::Relaxed); },
                );

                match result {
                    Ok(_) => succeeded.lock().unwrap().push(output),
                    Err(e) => failed.lock().unwrap().push((input.clone(), e.to_string())),
                }
            });

            BatchResult {
                succeeded: succeeded.into_inner().unwrap(),
                failed: failed.into_inner().unwrap(),
                total_frames: total_frames.load(Ordering::Relaxed),
            }
        })
}
