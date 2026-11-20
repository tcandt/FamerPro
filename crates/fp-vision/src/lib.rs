use anyhow::{bail, Context, Result};
use fp_domain::{Bounds, Frame, VisionTarget};
use image::{DynamicImage, GrayImage};

pub struct VisionEngine {
    pub min_confidence_default: f32,
}

impl Default for VisionEngine {
    fn default() -> Self {
        Self {
            min_confidence_default: 0.85,
        }
    }
}

impl VisionEngine {
    pub fn new() -> Self {
        Self::default()
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
        let min_conf = min_confidence.unwrap_or(self.min_confidence_default);

        let screen_img = self.load_image_from_bytes(&frame.bytes)?;
        let template_img = self.load_image_from_bytes(template_bytes)?;

        let screen_gray = screen_img.to_luma8();
        let template_gray = template_img.to_luma8();

        let (best_bounds, best_confidence) =
            self.match_template_ssd(&screen_gray, &template_gray)?;

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

    /// Fast Normalized Sum of Absolute Differences matching.
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

        let step = if sw > 1200 || sh > 1200 { 2 } else { 1 };
        let sample_step = 2;

        let mut max_confidence = -1.0f32;
        let mut best_x = 0;
        let mut best_y = 0;

        for y in (0..=(sh - th)).step_by(step) {
            for x in (0..=(sw - tw)).step_by(step) {
                let mut sum_diff = 0.0f32;
                let mut count = 0;

                for ty in (0..th).step_by(sample_step) {
                    for tx in (0..tw).step_by(sample_step) {
                        let sp = screen.get_pixel(x + tx, y + ty)[0] as f32;
                        let tp = template.get_pixel(tx, ty)[0] as f32;
                        sum_diff += (sp - tp).abs();
                        count += 1;
                    }
                }

                let avg_diff = sum_diff / (count as f32);
                let confidence = (1.0 - (avg_diff / 255.0)).clamp(0.0, 1.0);

                if confidence > max_confidence {
                    max_confidence = confidence;
                    best_x = x;
                    best_y = y;
                }
            }
        }

        let bounds = Bounds::new(best_x as f32, best_y as f32, tw as f32, th as f32);
        Ok((bounds, max_confidence))
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
}
