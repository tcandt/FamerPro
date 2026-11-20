use anyhow::{bail, Context, Result};
use fp_domain::{Bounds, Frame, VisionTarget};
use image::{imageops::FilterType, DynamicImage, GrayImage};

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
}
