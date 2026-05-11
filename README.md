# ffmpreg
<img width="1128" height="1600" alt="image" src="https://github.com/user-attachments/assets/94464371-b50e-4395-a14f-2700095cd09a" />

A video encoding, decoding, and processing CLI written in Rust, built on top of [FFmpeg](https://ffmpeg.org/) via the [`ffmpeg-next`](https://crates.io/crates/ffmpeg-next) bindings.

## Features

| Subcommand | Description |
|---|---|
| `encode` | Encode a synthetic color-bars test pattern to H.264/H.265/VP9 |
| `decode` | Decode a video and print stream info |
| `transcode` | Re-encode to a different codec, resolution, or audio codec |
| `filter` | Apply an FFmpeg filter chain (e.g. `yadif,scale=1280:720`) |
| `clip` | Extract a time range from a video |
| `concat` | Concatenate multiple video files |
| `batch` | Transcode a glob of files in parallel |
| `frames` | Extract decoded frames as PNG images |
| `gif` | Export a video segment as an animated GIF |
| `thumbnail` | Grab a single frame at a given timestamp |
| `probe` | Inspect all streams, bitrate, and metadata tags |
| `meta` | Remux a file with updated metadata tags |

## Requirements

FFmpeg development headers (Ubuntu/Debian):

```bash
sudo apt-get install -y \
  libavcodec-dev libavformat-dev libavutil-dev \
  libswscale-dev libswresample-dev
```

Rust 1.70+ (uses the 2021 edition).

## Build

```bash
cargo build --release
```

The binary is at `target/release/ffmpreg`.

## Usage

```bash
# Encode a 5-second color-bars test pattern
ffmpreg encode --output test.mp4 --frames 150 --codec h264 --quality medium

# Probe a file
ffmpreg probe --input test.mp4

# Transcode to H.265, half resolution, transcode audio to AAC
ffmpreg transcode --input test.mp4 --output out.h265.mp4 \
  --codec h265 --width 320 --height 240 --audio-codec aac

# Apply a deinterlace + scale filter
ffmpreg filter --input input.mp4 --output filtered.mp4 \
  --filters "yadif,scale=1280:720"

# Clip from 10s to 30s
ffmpreg clip --input input.mp4 --output clip.mp4 --start 10 --end 30

# Concatenate files
ffmpreg concat --inputs a.mp4 b.mp4 c.mp4 --output joined.mp4

# Batch transcode all MP4s in a folder using 4 parallel jobs
ffmpreg batch --input "raw/*.mp4" --output-dir ./out --codec h265 --jobs 4

# Extract every 5th frame as PNG
ffmpreg frames --input input.mp4 --output-dir ./frames --every 5

# Export a GIF at 10fps, 480px wide
ffmpreg gif --input input.mp4 --output out.gif --fps 10 --width 480

# Grab a thumbnail at 5.5 seconds
ffmpreg thumbnail --input input.mp4 --output thumb.png --time 5.5

# Set metadata tags
ffmpreg meta --input input.mp4 --output tagged.mp4 \
  --tag title="My Video" --tag artist="JohannaWeb"
```

## Architecture

```
src/
├── cli/       # Argument parsing and command dispatch (one file per subcommand)
├── codec/     # Encode, decode, transcode, filter, clip, concat, batch, gif, probe, meta
├── filter/    # FFmpeg filter graph wrapper (VideoFilterGraph)
├── format/    # Container I/O (InputContext, OutputContext)
└── frame/     # Frame generation, RGB↔YUV conversion
```

See [`docs/IMPLEMENTATION.md`](docs/IMPLEMENTATION.md) for a full implementation plan with status checkboxes.

## License

Mozilla Public License 2.0 — see [LICENSE](LICENSE).
