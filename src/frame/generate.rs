use ffmpeg::frame;
use crate::frame::convert::rgb24_to_yuv420p;

/// Grayscale ramp that cycles brightness by frame index.
pub fn synthetic_yuv420p(frame: &mut frame::Video, frame_idx: u32, width: u32, height: u32) {
    let y_val = ((frame_idx * 2) % 256) as u8;

    let linesize = frame.line_size(0);
    let data = frame.data_mut(0);
    for row in 0..height as usize {
        for col in 0..width as usize {
            data[row * linesize + col] = y_val;
        }
    }

    let uv_linesize = frame.line_size(1);
    let uv_height = height as usize / 2;
    let uv_width = width as usize / 2;

    let u_data = frame.data_mut(1);
    for row in 0..uv_height {
        for col in 0..uv_width {
            u_data[row * uv_linesize + col] = 128;
        }
    }

    let v_data = frame.data_mut(2);
    for row in 0..uv_height {
        for col in 0..uv_width {
            v_data[row * uv_linesize + col] = 128;
        }
    }
}

/// SMPTE-style color bars: 8 vertical stripes cycling hue per frame.
pub fn color_bars(frame: &mut frame::Video, frame_idx: u32, width: u32, height: u32) {
    let w = width as usize;
    let h = height as usize;
    let hue_shift = (frame_idx * 3) % 360;

    let mut rgb = vec![0u8; w * h * 3];
    for row in 0..h {
        for col in 0..w {
            let bar = (col * 8) / w;
            let hue = (bar as u32 * 45 + hue_shift) % 360;
            let (r, g, b) = hue_to_rgb(hue);
            rgb[(row * w + col) * 3] = r;
            rgb[(row * w + col) * 3 + 1] = g;
            rgb[(row * w + col) * 3 + 2] = b;
        }
    }

    rgb24_to_yuv420p(frame, &rgb, width, height);
}

pub(crate) fn hue_to_rgb(hue: u32) -> (u8, u8, u8) {
    let h = hue as f32 / 60.0;
    let x = (1.0 - (h % 2.0 - 1.0).abs()) * 255.0;
    let v = 255.0f32;
    let (r, g, b) = match hue / 60 {
        0 => (v, x, 0.0),
        1 => (x, v, 0.0),
        2 => (0.0, v, x),
        3 => (0.0, x, v),
        4 => (x, 0.0, v),
        _ => (v, 0.0, x),
    };
    (r as u8, g as u8, b as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hue_red_is_max_r() {
        let (r, g, b) = hue_to_rgb(0);
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn hue_green_is_max_g() {
        let (r, g, b) = hue_to_rgb(120);
        assert_eq!(g, 255);
        assert_eq!(r, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn hue_blue_is_max_b() {
        let (r, g, b) = hue_to_rgb(240);
        assert_eq!(b, 255);
        assert_eq!(r, 0);
        assert_eq!(g, 0);
    }

    #[test]
    fn hue_shift_stays_in_bounds() {
        for hue in (0..360).step_by(15) {
            let (r, g, b) = hue_to_rgb(hue);
            assert!(r <= 255 && g <= 255 && b <= 255);
        }
    }
}
