# ffmpreg Implementation Plan

## Overview

`ffmpreg` is a Rust CLI tool for video encoding and decoding using FFmpeg bindings. It provides a minimal but complete vertical slice demonstrating both encode and decode workflows.

## Functional Decomposition

The system is broken into four functional layers, each with a clear responsibility boundary. The vertical slice currently lives flat in `src/`, but as the project grows each layer becomes its own module folder.

```
src/
├── cli/          # Argument parsing and command dispatch
├── codec/        # Encoder and decoder logic
├── frame/        # Frame types, generation, pixel format conversion
├── format/       # Container I/O (muxing/demuxing)
└── main.rs       # Wires everything together
```

### Layer 1 — `cli/`
Responsible for the user-facing interface only. No FFmpeg logic lives here.

- [x] `cli/mod.rs` — re-exports all commands
- [x] `cli/encode_cmd.rs` — `EncodeArgs` struct, calls into `codec::encode`
- [x] `cli/decode_cmd.rs` — `DecodeArgs` struct, calls into `codec::decode`

### Layer 2 — `codec/`
Core encode/decode logic. Owns the FFmpeg codec context lifecycle.

- [x] `codec/encode.rs` — H.264 encoding to MP4
- [x] `codec/decode.rs` — video decoding and frame counting
- [x] `codec/mod.rs` — module re-exports
- [x] `codec/options.rs` — `EncodeOptions` struct (codec, resolution, framerate, bitrate)

### Layer 3 — `frame/`
Raw frame handling, isolated from codec and format concerns.

- [x] `frame/mod.rs` — module re-exports
- [x] `frame/generate.rs` — grayscale ramp + SMPTE color bars generator
- [x] `frame/convert.rs` — RGB24 → YUV420P conversion (BT.601)

### Layer 4 — `format/`
Container I/O — opening files, finding streams, writing packets.

- [x] `format/mod.rs` — module re-exports
- [x] `format/input.rs` — `InputContext`: open file, find video stream, expose metadata
- [x] `format/output.rs` — `OutputContext`: open file, write packets, finalize

### Current State vs Target

| Module | Before | Now |
|---|---|---|
| CLI | flat in `main.rs` | `cli/encode_cmd.rs`, `cli/decode_cmd.rs` |
| Encode | `src/encode.rs` | `src/codec/encode.rs` |
| Decode | `src/decode.rs` | `src/codec/decode.rs` |
| Frame gen | inline in `encode.rs` | `src/frame/generate.rs` |
| Format I/O | inline in encode/decode | `src/format/input.rs`, `src/format/output.rs` |

---

## Architecture

### Core Components

```
ffmpreg/
├── src/
│   ├── main.rs      # CLI entry point, argument parsing (clap)
│   ├── encode.rs    # Video encoding to H.264/MP4
│   └── decode.rs    # Video decoding and frame extraction
├── Cargo.toml       # Dependencies and project metadata
└── docs/
    └── IMPLEMENTATION.md  (this file)
```

**Implementation Status:**
- [x] `src/main.rs` — CLI with `clap` derive, subcommand dispatch
- [x] `src/encode.rs` — H.264 encoding to MP4 with synthetic frames
- [x] `src/decode.rs` — Video decoding and metadata extraction
- [x] `Cargo.toml` — Dependencies configured

### Dependencies

- **ffmpeg-next = "6"** — Safe Rust bindings over FFmpeg 6.x C libraries
  - Wraps raw `ffmpeg-sys-next` C bindings with high-level APIs
  - Provides codec, format, frame, and packet abstractions
  
- **clap = "4"** (with `derive` feature) — CLI argument parsing
  - Declarative macro-style CLI definitions
  - Auto-generates help text and validation
  
- **anyhow = "1"** — Ergonomic error handling
  - `?` operator with context propagation
  - `Context` trait for adding messages to errors

### System Requirements

Install FFmpeg development headers before building:

```bash
sudo apt-get install -y libavcodec-dev libavformat-dev libavutil-dev \
                        libswscale-dev libswresample-dev
```

These provide the C libraries that `ffmpeg-next` binds to.

## Module Details

### `main.rs` — CLI Interface

**Responsibilities:**
- Parse command-line arguments using `clap` derive macros
- Dispatch to encode/decode subcommands
- Format and print results

**CLI Design:**
```
ffmpreg encode --output out.mp4 [--width 640] [--height 480] [--frames 120]
ffmpreg decode --input out.mp4
```

**Default values:**
- Width: 640 pixels
- Height: 480 pixels
- Frames: 120 frames
- Framerate: 30 FPS (hardcoded)

### `encode.rs` — Video Encoding

**Function:** `encode_video(output_path, width, height, num_frames) -> Result<()>`

**Encoding Pipeline:**

- [x] **Initialize FFmpeg**
  - `ffmpeg::init()` sets up global FFmpeg state

- [x] **Create Output Context**
  - Auto-detects muxer from file extension (.mp4 → mov/mp4 muxer)
  - Sets global header flag for proper MP4 structure

- [x] **Add Video Stream**
  - Create video stream with H.264 codec (libx264)
  - Configure: resolution, pixel format (YUV420P), framerate (30 FPS)
  - YUV420P is the standard compressed format (2:1 horizontal, 2:1 vertical chroma subsampling)

- [x] **Open Encoder**
  - Instantiate codec context and open encoder
  - Write MP4 header

- [x] **Generate and Encode Frames**
  - Loop for `num_frames` iterations:
    - Call `fill_frame()` to generate synthetic YUV420P data
    - Set frame PTS (presentation timestamp) to frame index
    - Send frame to encoder
    - Receive encoded packets and write to output file
  
- [x] **Flush and Finalize**
  - Send EOF to encoder to flush remaining packets
  - Write MP4 trailer (fixes up atom sizes and offsets)

**Frame Generation (`fill_frame`):**

Creates a synthetic YUV420P frame with color cycling:
- **Y plane** (luma): Grayscale value cycling 0–255 based on frame index
  - Full resolution (width × height)
- **U & V planes** (chroma): Constant neutral gray (128, 128)
  - Half resolution each (width/2 × height/2)

This produces a grayscale video that gradually brightens then resets.

### `decode.rs` — Video Decoding

**Function:** `decode_video(input_path) -> Result<VideoInfo>`

**Returns:** `VideoInfo` struct containing:
- `codec_name` — Name of video codec (e.g., "h264")
- `width`, `height` — Resolution in pixels
- `avg_frame_rate` — Frames per second
- `frame_count` — Total frames decoded
- `duration` — Duration in seconds

**Decoding Pipeline:**

- [x] **Initialize FFmpeg**
  - `ffmpeg::init()` sets up global state

- [x] **Open Input File**
  - Auto-detects container format and demuxes
  - Finds best video stream (returns error if none found)

- [x] **Get Codec Parameters**
  - Extract codec ID, resolution, timebase, framerate from stream

- [x] **Create and Open Decoder**
  - Find codec by ID
  - Create decoder context and open it

- [x] **Read Packets and Decode**
  - Iterate over all packets in input file
  - Filter to video stream only (by stream index)
  - Send packets to decoder
  - Receive decoded frames and count them
   
- [x] **Flush Decoder**
  - Send EOF marker to drain any remaining frames in buffer

**Note:** Frames are counted but not inspected or stored; the decode is stream-based with frame-at-a-time processing to minimize memory usage.

## Encoding/Decoding Details

### H.264 Codec

- **Codec ID:** `ffmpeg::codec::Id::H264`
- **Implementation:** libx264 (default H.264 encoder)
- **Pixel Format:** YUV420P (standard compressed)
- **Profile:** Baseline (default, for compatibility)
- **Framerate:** 30 FPS
- **Container:** MP4 (mov/mp4 muxer)

### YUV420P Format

The Y'UV420 color space separates luma (brightness) from chroma (color):
- **Y plane:** Full resolution, grayscale intensity
- **U plane:** Chroma blue, 1/4 the pixels (2:1 horizontal and vertical downsampling)
- **V plane:** Chroma red, 1/4 the pixels

Memory layout in `ffmpeg-next`:
```
frame.data(0)  → Y plane (line_size = width)
frame.data(1)  → U plane (line_size = width/2)
frame.data(2)  → V plane (line_size = width/2)
```

### Packet & Frame Flow

**Encoding:**
```
Frame (raw YUV420P) 
  → encoder.send_frame() 
  → encoder.receive_packet() 
  → octx.write_interleaved() 
  → MP4 file
```

**Decoding:**
```
MP4 file 
  → packet 
  → decoder.send_packet() 
  → decoder.receive_frame() 
  → Frame (raw YUV420P)
```

Both are asynchronous: sending doesn't immediately produce output (buffering), so `receive_*` may need to be called in a loop.

## Build & Test Checklist

### Prerequisites
- [x] Cargo project initialized
- [x] Dependencies added to Cargo.toml
- [x] FFmpeg modules created (encode, decode, main)
- [ ] Install system FFmpeg headers:
  ```bash
  sudo apt-get install -y libavcodec-dev libavformat-dev libavutil-dev \
                          libswscale-dev libswresample-dev
  ```

### Build
- [ ] Run `cargo build --release`
- [ ] Resolve any compilation errors

### Roundtrip Test
- [ ] Encode test video:
  ```bash
  ./target/release/ffmpreg encode --output /tmp/test.mp4 --frames 60
  ```
- [ ] Verify output file exists and is non-empty:
  ```bash
  ls -lh /tmp/test.mp4
  ```
- [ ] Decode the video:
  ```bash
  ./target/release/ffmpreg decode --input /tmp/test.mp4
  ```
- [ ] Verify expected output:
  ```
  ✓ Decoded video info:
    Codec: h264
    Resolution: 640x480
    FPS: 30.00
    Frames: 60
    Duration: 2.00s
  ```

### Verification Tests
- [ ] Encode with custom resolution:
  ```bash
  ./target/release/ffmpreg encode --output /tmp/custom.mp4 \
    --width 1280 --height 720 --frames 30
  ```
- [ ] Decode custom file and verify resolution
- [ ] Encode with different frame count and verify frame count in decode
- [ ] Test error handling (decode non-existent file)

## Vertical Slice Completion

Core features delivered:

- [x] Rust project with FFmpeg bindings (`ffmpeg-next`)
- [x] CLI interface with `clap` — encode, decode, transcode, frames subcommands
- [x] **Encode module**: color-bars test pattern → H.264/H.265/VP9 MP4
- [x] **Decode module**: video metadata extraction and frame counting
- [x] **Transcode module**: decode input → software scale → re-encode to new codec/resolution
- [x] **Frames module**: extract decoded frames as PNG (every-N, max-frames options)
- [x] **Codec selection**: `CodecChoice` enum (H.264, H.265, VP9) parsed from CLI
- [x] **Progress reporting**: live frame counter in encode, transcode, frame extraction
- [x] **Thumbnail command**: seek to timestamp, decode one frame, save as PNG
- [x] **Probe command**: inspect all streams (codec, resolution, fps, audio channels/rate)
- [x] **Audio stream copy**: transcode preserves audio tracks without re-encoding
- [x] **Resize in transcode**: `--width`/`--height` override (defaults to input dimensions)
- [x] Proper error handling with `anyhow`

## Future Extensions

Possible enhancements beyond this vertical slice:

- [x] **Read input video files** for re-encoding (`transcode` subcommand)
- [x] **Multiple codecs** — H.264, H.265/HEVC, VP9 via `--codec` flag + `CodecChoice` enum
- [x] **Pixel format conversion** — RGB24 ↔ YUV420P (BT.601), software scaler in transcode
- [x] **Rate control** — `--bitrate` flag passed through `EncodeOptions`
- [x] **Frame-level access** — `frames` subcommand extracts PNGs (`--every N`, `--max`)
- [x] **Progress reporting** — live `\r` counter in encode, transcode, and frame extraction
- [x] **Thumbnail generation** — `thumbnail` subcommand: seek to timestamp, save PNG
- [x] **Audio track support** — transcode copies audio streams without re-encoding
- [x] **Resize** — `transcode --width`/`--height` rescales via swscale
- [x] **Filtering** — `filter` subcommand with arbitrary FFmpeg filter chain (`"yadif,scale=1280:720"`, `"eq=brightness=0.1"`, etc.)
- [x] **Async/parallel encoding** — `batch` subcommand with glob input + rayon thread pool, configurable `--jobs`
- [x] **Video concatenation** — `concat` subcommand, continuous PTS across files, re-encodes to uniform resolution
- [x] **Subtitle handling** — transcode and filter commands copy subtitle streams alongside audio
- [x] **Quality presets** — `low / medium / high / lossless` on all encode commands; `effective_bitrate()` resolves preset vs explicit
- [x] **Clip / trim** — `clip` subcommand with `--start` / `--end` timestamps, seeks then re-zeros PTS
- [x] **Audio transcoding** — `AudioTranscoder` in `codec/audio.rs` (decode → resample → AAC encode)
- [x] **Unit tests** — codec option parsing, quality bitrate ordering, hue-to-RGB boundary checks, BT.601 YUV conversion invariants
- [x] **Audio transcoding wired** — `transcode --audio-codec aac/mp3/opus` triggers full decode→resample→encode; default is still stream copy
- [x] **GIF export** — `gif` subcommand via `image` crate GifEncoder; auto-scales to `--width`, samples at `--fps`
- [x] **Metadata read/write** — `probe` shows tags; `meta` subcommand remuxes with updated `--tag KEY=VALUE` pairs

## Error Handling

All public functions return `anyhow::Result<T>` for ergonomic error propagation:
- `?` operator unwraps success or returns error with context
- `with_context()` and `context()` add diagnostic messages
- FFmpeg errors are captured and bubbled up as `anyhow` errors

## Performance Notes

- **Encoding:** Synthetic frame generation is CPU-bound (not real-time)
- **Decoding:** Stream-based (no frame buffering), minimal memory overhead
- **Optimization opportunities:**
  - Use faster (lower-quality) H.264 presets for real-time
  - Parallel encode for batch jobs (requires thread-safe FFmpeg init)
  - Hardware acceleration (NVENC, Quick Sync, VideoToolbox)
