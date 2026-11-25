use anyhow::{bail, Context, Result};
use fp_domain::{Bounds, Frame, Point, ScreenGeometry, VisionTarget};
use image::{imageops::FilterType, DynamicImage, GrayImage, ImageFormat, Rgb};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoiRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Crops an exact `(x, y, width, height)` region from encoded PNG/JPEG frame bytes
/// and re-encodes as lossless PNG with exact dimensions.
pub fn crop_roi_png(frame_bytes: &[u8], roi: RoiRect) -> Result<(Vec<u8>, u32, u32)> {
    if roi.width == 0 || roi.height == 0 {
        bail!("ROI_INVALID: ROI width and height must be > 0");
    }
    let img = image::load_from_memory(frame_bytes)
        .context("ROI_DECODE_FAILED: Failed decoding source capture frame")?;
    let (sw, sh) = (img.width(), img.height());
    if roi.x.saturating_add(roi.width) > sw || roi.y.saturating_add(roi.height) > sh {
        bail!(
            "ROI_OUT_OF_BOUNDS: ROI ({},{},{}x{}) exceeds captured frame {}x{}",
            roi.x,
            roi.y,
            roi.width,
            roi.height,
            sw,
            sh
        );
    }

    let cropped = img.crop_imm(roi.x, roi.y, roi.width, roi.height);
    let mut out = Vec::new();
    cropped
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .context("ROI_ENCODE_FAILED: Failed encoding cropped ROI as PNG")?;
    Ok((out, cropped.width(), cropped.height()))
}

/// Converts coordinates / bounds between two device resolutions (e.g. reference 1920x1080 -> 1280x720).
pub fn scale_bounds_for_geometry(
    bounds: Bounds,
    from: &ScreenGeometry,
    to: &ScreenGeometry,
) -> Result<Bounds> {
    if from.width_px == 0 || from.height_px == 0 || to.width_px == 0 || to.height_px == 0 {
        bail!("GEOMETRY_INVALID: Screen geometry dimensions must be non-zero");
    }
    let sx = to.width_px as f32 / from.width_px as f32;
    let sy = to.height_px as f32 / from.height_px as f32;
    Ok(Bounds::new(
        bounds.x * sx,
        bounds.y * sy,
        bounds.width * sx,
        bounds.height * sy,
    ))
}

/// Annotates a captured frame with the detected bounding box and optional safe tap crosshair
/// for visual audit/evidence output.
pub fn annotate_match_and_tap_png(
    frame_bytes: &[u8],
    bounds: Bounds,
    tap_point: Option<Point>,
) -> Result<Vec<u8>> {
    let img = image::load_from_memory(frame_bytes)
        .context("ANNOTATE_DECODE_FAILED: Failed decoding frame bytes")?;
    let mut rgb = img.to_rgb8();
    let (w, h) = (rgb.width(), rgb.height());

    let x0 = (bounds.x.max(0.0) as u32).min(w.saturating_sub(1));
    let y0 = (bounds.y.max(0.0) as u32).min(h.saturating_sub(1));
    let x1 = ((bounds.x + bounds.width).max(0.0) as u32).min(w.saturating_sub(1));
    let y1 = ((bounds.y + bounds.height).max(0.0) as u32).min(h.saturating_sub(1));

    let box_col = Rgb([16, 224, 130]); // emerald green
    for thick in 0..2u32 {
        let bx0 = x0.saturating_add(thick).min(w - 1);
        let bx1 = x1.saturating_sub(thick);
        let by0 = y0.saturating_add(thick).min(h - 1);
        let by1 = y1.saturating_sub(thick);
        for x in x0..=x1 {
            rgb.put_pixel(x, by0, box_col);
            rgb.put_pixel(x, by1, box_col);
        }
        for y in y0..=y1 {
            rgb.put_pixel(bx0, y, box_col);
            rgb.put_pixel(bx1, y, box_col);
        }
    }

    if let Some(pt) = tap_point {
        let cx = (pt.x.max(0.0) as i32).clamp(0, w as i32 - 1);
        let cy = (pt.y.max(0.0) as i32).clamp(0, h as i32 - 1);
        let cross_col = Rgb([244, 63, 94]); // rose red
        for d in -8i32..=8 {
            if (cx + d) >= 0 && (cx + d) < w as i32 {
                rgb.put_pixel((cx + d) as u32, cy as u32, cross_col);
            }
            if (cy + d) >= 0 && (cy + d) < h as i32 {
                rgb.put_pixel(cx as u32, (cy + d) as u32, cross_col);
            }
        }
    }

    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(rgb)
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .context("ANNOTATE_ENCODE_FAILED: Failed writing annotated PNG")?;
    Ok(out)
}

pub struct VisionEngine {
    pub min_confidence_default: f32,
    pub default_scales: Vec<f32>,
}

impl Default for VisionEngine {
    fn default() -> Self {
        Self {
            min_confidence_default: 0.85,
            default_scales: vec![1.0],
        }
    }
}

impl VisionEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_scales(scales: Vec<f32>) -> Self {
        Self {
            min_confidence_default: 0.85,
            default_scales: scales,
        }
    }

    pub fn load_image_from_bytes(&self, bytes: &[u8]) -> Result<DynamicImage> {
        image::load_from_memory(bytes).context("Failed to decode image from memory bytes")
    }

    pub fn find_template(
        &self,
        frame: &Frame,
        template_bytes: &[u8],
        min_confidence: Option<f32>,
    ) -> Result<VisionTarget> {
        self.find_template_multiscale(frame, template_bytes, min_confidence, &self.default_scales)
    }

    pub fn find_template_multiscale(
        &self,
        frame: &Frame,
        template_bytes: &[u8],
        min_confidence: Option<f32>,
        scales: &[f32],
    ) -> Result<VisionTarget> {
        let min_conf = min_confidence.unwrap_or(self.min_confidence_default);

        let screen_img = self.load_image_from_bytes(&frame.bytes)?;
        let template_img = self.load_image_from_bytes(template_bytes)?;

        let screen_gray = screen_img.to_luma8();
        let template_gray = template_img.to_luma8();

        let mut best_target: Option<(Bounds, f32)> = None;

        for &scale in scales {
            let scaled_template = if (scale - 1.0).abs() < 0.01 {
                template_gray.clone()
            } else {
                let n_w = ((template_gray.width() as f32) * scale).round() as u32;
                let n_h = ((template_gray.height() as f32) * scale).round() as u32;
                if n_w == 0 || n_h == 0 {
                    continue;
                }
                image::imageops::resize(&template_gray, n_w, n_h, FilterType::Triangle)
            };

            if let Ok((bounds, conf)) = self.match_template_ssd(&screen_gray, &scaled_template) {
                if let Some((_, best_conf)) = &best_target {
                    if conf > *best_conf {
                        best_target = Some((bounds, conf));
                    }
                } else {
                    best_target = Some((bounds, conf));
                }
            }
        }

        let (best_bounds, best_confidence) =
            best_target.context("Failed to perform template matching across requested scales")?;

        if best_confidence < min_conf {
            bail!(
                "Target not found with sufficient confidence: got {:.3}, required >= {:.3}",
                best_confidence,
                min_conf
            );
        }

        Ok(VisionTarget {
            bounds: best_bounds,
            confidence: best_confidence,
            geometry: frame.geometry.clone(),
        })
    }

    /// Fast coarse-to-fine Normalized Sum of Absolute Differences matching.
    /// Confidence is in [0.0, 1.0], where 1.0 is an exact pixel match.
    pub fn match_template_ssd(
        &self,
        screen: &GrayImage,
        template: &GrayImage,
    ) -> Result<(Bounds, f32)> {
        let (sw, sh) = screen.dimensions();
        let (tw, th) = template.dimensions();

        if tw > sw || th > sh {
            bail!(
                "Template dimensions ({}x{}) exceed screen dimensions ({}x{})",
                tw,
                th,
                sw,
                sh
            );
        }

        if tw == 0 || th == 0 {
            bail!("Template has zero dimensions");
        }

        let screen_raw = screen.as_raw();
        let template_raw = template.as_raw();
        let sw_usize = sw as usize;
        let tw_usize = tw as usize;

        // Determine coarse stepping based on screen size
        let is_large = sw > 400 || sh > 400;
        let coarse_step: u32 = if is_large { 6 } else { 1 };
        let coarse_sample_step: u32 = if is_large {
            (tw.min(th) / 8).clamp(3, 8)
        } else {
            1
        };

        let mut max_coarse_conf = -1.0f32;
        let mut best_cx: u32 = 0;
        let mut best_cy: u32 = 0;

        // Phase 1: Coarse search
        for y in (0..=(sh - th)).step_by(coarse_step as usize) {
            for x in (0..=(sw - tw)).step_by(coarse_step as usize) {
                let mut sum_diff = 0.0f32;
                let mut count = 0;

                for ty in (0..th).step_by(coarse_sample_step as usize) {
                    let s_row = (y + ty) as usize * sw_usize;
                    let t_row = ty as usize * tw_usize;

                    for tx in (0..tw).step_by(coarse_sample_step as usize) {
                        let sp = screen_raw[s_row + (x + tx) as usize] as f32;
                        let tp = template_raw[t_row + tx as usize] as f32;
                        sum_diff += (sp - tp).abs();
                        count += 1;
                    }
                }

                let avg_diff = sum_diff / (count as f32);
                let confidence = (1.0 - (avg_diff / 255.0)).clamp(0.0, 1.0);

                if confidence > max_coarse_conf {
                    max_coarse_conf = confidence;
                    best_cx = x;
                    best_cy = y;
                }
            }
        }

        // Phase 2: Fine search around best coarse point if coarse_step > 1
        let mut final_best_x = best_cx;
        let mut final_best_y = best_cy;
        let mut final_max_conf = max_coarse_conf;

        if coarse_step > 1 {
            let fine_sample_step = if tw > 50 || th > 50 { 2 } else { 1 };
            let x_min = best_cx.saturating_sub(coarse_step);
            let x_max = (best_cx + coarse_step).min(sw - tw);
            let y_min = best_cy.saturating_sub(coarse_step);
            let y_max = (best_cy + coarse_step).min(sh - th);

            for y in y_min..=y_max {
                for x in x_min..=x_max {
                    let mut sum_diff = 0.0f32;
                    let mut count = 0;

                    for ty in (0..th).step_by(fine_sample_step) {
                        let s_row = (y + ty) as usize * sw_usize;
                        let t_row = ty as usize * tw_usize;

                        for tx in (0..tw).step_by(fine_sample_step) {
                            let sp = screen_raw[s_row + (x + tx) as usize] as f32;
                            let tp = template_raw[t_row + tx as usize] as f32;
                            sum_diff += (sp - tp).abs();
                            count += 1;
                        }
                    }

                    let avg_diff = sum_diff / (count as f32);
                    let confidence = (1.0 - (avg_diff / 255.0)).clamp(0.0, 1.0);

                    if confidence > final_max_conf {
                        final_max_conf = confidence;
                        final_best_x = x;
                        final_best_y = y;
                    }
                }
            }
        }

        let bounds = Bounds::new(
            final_best_x as f32,
            final_best_y as f32,
            tw as f32,
            th as f32,
        );
        Ok((bounds, final_max_conf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    #[test]
    fn test_perfect_template_match() {
        let mut screen = GrayImage::new(100, 100);
        let mut template = GrayImage::new(20, 20);

        for pixel in screen.pixels_mut() {
            *pixel = Luma([40]);
        }
        for pixel in template.pixels_mut() {
            *pixel = Luma([200]);
        }

        for ty in 0..20 {
            for tx in 0..20 {
                screen.put_pixel(30 + tx, 40 + ty, Luma([200]));
            }
        }

        let engine = VisionEngine::new();
        let (bounds, confidence) = engine.match_template_ssd(&screen, &template).unwrap();

        assert_eq!(bounds.x as u32, 30);
        assert_eq!(bounds.y as u32, 40);
        assert!(confidence > 0.98, "Confidence was {}", confidence);
    }

    #[test]
    fn test_large_screen_template_match_speed_and_accuracy() {
        // Test high-resolution 1080x1920 screen with a 120x60 button
        let mut screen = GrayImage::new(1080, 1920);
        let mut template = GrayImage::new(120, 60);

        for pixel in screen.pixels_mut() {
            *pixel = Luma([16]);
        }
        for pixel in template.pixels_mut() {
            *pixel = Luma([220]);
        }

        for ty in 0..60 {
            for tx in 0..120 {
                screen.put_pixel(240 + tx, 480 + ty, Luma([220]));
            }
        }

        let start = std::time::Instant::now();
        let engine = VisionEngine::new();
        let (bounds, confidence) = engine.match_template_ssd(&screen, &template).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(bounds.x as u32, 240);
        assert_eq!(bounds.y as u32, 480);
        assert!(confidence > 0.98);
        assert!(
            elapsed.as_millis() < 500,
            "Matching 1080x1920 took too long: {} ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_multiscale_template_match() {
        use fp_domain::ScreenGeometry;
        use image::RgbImage;
        use std::io::Cursor;

        // Screen has a 120x60 button, template is 100x50 (0.833x scale)
        let mut screen = RgbImage::new(400, 400);
        for pixel in screen.pixels_mut() {
            *pixel = image::Rgb([16, 23, 34]);
        }
        for y in 100..160 {
            for x in 150..270 {
                screen.put_pixel(x, y, image::Rgb([24, 120, 245]));
            }
        }

        let mut screen_bytes = Vec::new();
        screen
            .write_to(&mut Cursor::new(&mut screen_bytes), image::ImageFormat::Png)
            .unwrap();

        let mut template = RgbImage::new(100, 50);
        for pixel in template.pixels_mut() {
            *pixel = image::Rgb([24, 120, 245]);
        }
        let mut template_bytes = Vec::new();
        template
            .write_to(
                &mut Cursor::new(&mut template_bytes),
                image::ImageFormat::Png,
            )
            .unwrap();

        let frame = Frame {
            bytes: screen_bytes,
            geometry: ScreenGeometry::new(400, 400, 0),
            timestamp_ms: 0,
        };

        let engine = VisionEngine::new();
        let target = engine
            .find_template_multiscale(&frame, &template_bytes, Some(0.85), &[0.8, 1.0, 1.2])
            .unwrap();

        assert!(target.confidence >= 0.85);
        assert!((target.bounds.x - 150.0).abs() <= 10.0);
        assert!((target.bounds.y - 100.0).abs() <= 10.0);
    }

    #[test]
    fn test_roi_crop_resolution_scaling_and_annotation() {
        let mut screen = image::RgbImage::new(400, 300);
        for p in screen.pixels_mut() {
            *p = Rgb([10, 20, 30]);
        }
        for y in 80..120 {
            for x in 100..180 {
                screen.put_pixel(x, y, Rgb([200, 120, 40]));
            }
        }
        let mut src_png = Vec::new();
        screen
            .write_to(&mut Cursor::new(&mut src_png), ImageFormat::Png)
            .unwrap();

        let (cropped_png, w, h) = crop_roi_png(
            &src_png,
            RoiRect {
                x: 100,
                y: 80,
                width: 80,
                height: 40,
            },
        )
        .unwrap();
        assert_eq!((w, h), (80, 40));
        let cropped_img = image::load_from_memory(&cropped_png).unwrap().to_rgb8();
        assert_eq!(*cropped_img.get_pixel(5, 5), Rgb([200, 120, 40]));

        // Out of bounds rejected
        assert!(crop_roi_png(
            &src_png,
            RoiRect {
                x: 350,
                y: 80,
                width: 80,
                height: 40
            }
        )
        .is_err());

        // Geometry scaling 1920x1080 -> 1280x720
        let from = ScreenGeometry::new(1920, 1080, 0);
        let to = ScreenGeometry::new(1280, 720, 0);
        let scaled =
            scale_bounds_for_geometry(Bounds::new(300.0, 150.0, 150.0, 90.0), &from, &to).unwrap();
        assert!((scaled.x - 200.0).abs() < 0.01);
        assert!((scaled.y - 100.0).abs() < 0.01);
        assert!((scaled.width - 100.0).abs() < 0.01);
        assert!((scaled.height - 60.0).abs() < 0.01);

        // Annotation produces valid larger/different PNG with emerald bbox + tap marker
        let ann = annotate_match_and_tap_png(
            &src_png,
            Bounds::new(100.0, 80.0, 80.0, 40.0),
            Some(Point { x: 140.0, y: 100.0 }),
        )
        .unwrap();
        let ann_img = image::load_from_memory(&ann).unwrap().to_rgb8();
        assert_eq!(*ann_img.get_pixel(100, 80), Rgb([16, 224, 130]));
        assert_eq!(*ann_img.get_pixel(140, 100), Rgb([244, 63, 94]));
    }
}
