use ffmpeg::frame;
use image::RgbImage;

/// Convert a YUV420P `ffmpeg` frame into an `RgbImage` (BT.601 full-swing).
pub fn yuv420p_to_rgb(frame: &frame::Video) -> RgbImage {
    let w = frame.width() as usize;
    let h = frame.height() as usize;
    let y_stride = frame.line_size(0);
    let u_stride = frame.line_size(1);
    let v_stride = frame.line_size(2);
    let y_data = frame.data(0);
    let u_data = frame.data(1);
    let v_data = frame.data(2);

    let mut rgb = vec![0u8; w * h * 3];
    for row in 0..h {
        for col in 0..w {
            let y = y_data[row * y_stride + col] as f32;
            let u = u_data[(row / 2) * u_stride + col / 2] as f32;
            let v = v_data[(row / 2) * v_stride + col / 2] as f32;
            let yy = 1.164 * (y - 16.0);
            let r = (yy + 1.596 * (v - 128.0)).clamp(0.0, 255.0) as u8;
            let g = (yy - 0.392 * (u - 128.0) - 0.813 * (v - 128.0)).clamp(0.0, 255.0) as u8;
            let b = (yy + 2.017 * (u - 128.0)).clamp(0.0, 255.0) as u8;
            rgb[(row * w + col) * 3] = r;
            rgb[(row * w + col) * 3 + 1] = g;
            rgb[(row * w + col) * 3 + 2] = b;
        }
    }
    RgbImage::from_raw(w as u32, h as u32, rgb).expect("buffer size correct by construction")
}

/// Convert a packed RGB24 buffer into a YUV420P `ffmpeg` frame.
///
/// `rgb` must be exactly `width * height * 3` bytes in row-major order.
///
/// `rgb` must be exactly `width * height * 3` bytes in row-major order.
pub fn rgb24_to_yuv420p(frame: &mut frame::Video, rgb: &[u8], width: u32, height: u32) {
    let w = width as usize;
    let h = height as usize;

    let y_stride = frame.line_size(0);
    let u_stride = frame.line_size(1);
    let v_stride = frame.line_size(2);

    let y_plane = frame.data_mut(0);
    for row in 0..h {
        for col in 0..w {
            let r = rgb[(row * w + col) * 3] as f32;
            let g = rgb[(row * w + col) * 3 + 1] as f32;
            let b = rgb[(row * w + col) * 3 + 2] as f32;
            y_plane[row * y_stride + col] =
                (0.257 * r + 0.504 * g + 0.098 * b + 16.0).clamp(16.0, 235.0) as u8;
        }
    }

    let u_plane = frame.data_mut(1);
    for row in 0..h / 2 {
        for col in 0..w / 2 {
            let r = rgb[(row * 2 * w + col * 2) * 3] as f32;
            let g = rgb[(row * 2 * w + col * 2) * 3 + 1] as f32;
            let b = rgb[(row * 2 * w + col * 2) * 3 + 2] as f32;
            u_plane[row * u_stride + col] =
                (-0.148 * r - 0.291 * g + 0.439 * b + 128.0).clamp(16.0, 240.0) as u8;
        }
    }

    let v_plane = frame.data_mut(2);
    for row in 0..h / 2 {
        for col in 0..w / 2 {
            let r = rgb[(row * 2 * w + col * 2) * 3] as f32;
            let g = rgb[(row * 2 * w + col * 2) * 3 + 1] as f32;
            let b = rgb[(row * 2 * w + col * 2) * 3 + 2] as f32;
            v_plane[row * v_stride + col] =
                (0.439 * r - 0.368 * g - 0.071 * b + 128.0).clamp(16.0, 240.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// White (255,255,255) in BT.601 should give Y≈235, U≈128, V≈128.
    #[test]
    fn white_maps_to_near_max_luma() {
        let rgb = vec![255u8; 4 * 4 * 3];
        let result = convert_pixel(255.0, 255.0, 255.0);
        assert!(result.0 >= 210, "Y for white should be near 235, got {}", result.0);
        assert!((result.1 as i32 - 128).abs() < 10, "U for white should be ~128");
        assert!((result.2 as i32 - 128).abs() < 10, "V for white should be ~128");
    }

    #[test]
    fn black_maps_to_min_luma() {
        let result = convert_pixel(0.0, 0.0, 0.0);
        assert!(result.0 <= 20, "Y for black should be near 16, got {}", result.0);
    }

    #[test]
    fn red_has_high_v_component() {
        let result = convert_pixel(255.0, 0.0, 0.0);
        assert!(result.2 > 200, "V (red chroma) should be high for pure red, got {}", result.2);
    }

    #[test]
    fn blue_has_high_u_component() {
        let result = convert_pixel(0.0, 0.0, 255.0);
        assert!(result.1 > 200, "U (blue chroma) should be high for pure blue, got {}", result.1);
    }

    fn convert_pixel(r: f32, g: f32, b: f32) -> (u8, u8, u8) {
        let y = (0.257 * r + 0.504 * g + 0.098 * b + 16.0).clamp(16.0, 235.0) as u8;
        let u = (-0.148 * r - 0.291 * g + 0.439 * b + 128.0).clamp(16.0, 240.0) as u8;
        let v = (0.439 * r - 0.368 * g - 0.071 * b + 128.0).clamp(16.0, 240.0) as u8;
        (y, u, v)
    }
}
