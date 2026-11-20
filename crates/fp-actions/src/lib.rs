use anyhow::{bail, Result};
use fp_domain::{Bounds, Point, ScreenGeometry};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeTapPolicy {
    pub margin_ratio: f32,          // e.g. 0.18
    pub min_area_px: f32,           // e.g. 64.0
    pub avoid_recent_points: usize, // e.g. 5
    pub min_distance_px: f32,       // e.g. 8.0
    pub max_tries: usize,           // e.g. 50
}

impl Default for SafeTapPolicy {
    fn default() -> Self {
        Self {
            margin_ratio: 0.18,
            min_area_px: 64.0,
            avoid_recent_points: 5,
            min_distance_px: 8.0,
            max_tries: 50,
        }
    }
}

pub struct SafeRegionSampler {
    pub policy: SafeTapPolicy,
    recent_points: Vec<Point>,
}

impl SafeRegionSampler {
    pub fn new(policy: SafeTapPolicy) -> Self {
        Self {
            policy,
            recent_points: Vec::new(),
        }
    }

    /// Shrinks the bounding box inward by margin_ratio on all sides
    pub fn apply_margin(bounds: &Bounds, margin_ratio: f32) -> Option<Bounds> {
        let margin_x = bounds.width * margin_ratio;
        let margin_y = bounds.height * margin_ratio;

        let new_w = bounds.width - (2.0 * margin_x);
        let new_h = bounds.height - (2.0 * margin_y);

        if new_w > 0.0 && new_h > 0.0 {
            Some(Bounds::new(
                bounds.x + margin_x,
                bounds.y + margin_y,
                new_w,
                new_h,
            ))
        } else {
            None
        }
    }

    /// Rotates a point between standard 0, 90, 180, 270 degree orientations
    pub fn rotate_point(
        point: Point,
        from_deg: u16,
        to_deg: u16,
        width: u32,
        height: u32,
    ) -> Point {
        let diff = ((to_deg as i32 - from_deg as i32) % 360 + 360) % 360;
        let w = width as f32;
        let h = height as f32;

        match diff {
            90 => Point {
                x: h - 1.0 - point.y,
                y: point.x,
            },
            180 => Point {
                x: w - 1.0 - point.x,
                y: h - 1.0 - point.y,
            },
            270 => Point {
                x: point.y,
                y: w - 1.0 - point.x,
            },
            _ => point,
        }
    }

    /// Transforms a point from capture coordinates to device display coordinates
    pub fn transform_to_display(
        point: Point,
        capture_geom: &ScreenGeometry,
        display_geom: &ScreenGeometry,
    ) -> Result<Point> {
        let pt = if capture_geom.rotation_deg != display_geom.rotation_deg {
            Self::rotate_point(
                point,
                capture_geom.rotation_deg,
                display_geom.rotation_deg,
                capture_geom.width_px,
                capture_geom.height_px,
            )
        } else {
            point
        };

        let scale_x = display_geom.width_px as f32 / capture_geom.width_px as f32;
        let scale_y = display_geom.height_px as f32 / capture_geom.height_px as f32;

        Ok(Point {
            x: (pt.x * scale_x).clamp(0.0, display_geom.width_px as f32),
            y: (pt.y * scale_y).clamp(0.0, display_geom.height_px as f32),
        })
    }

    /// Samples a safe point within target bounds while subtracting exclusion zones
    pub fn sample_safe_point(
        &mut self,
        target_bounds: &Bounds,
        screen_bounds: &Bounds,
        exclusion_masks: &[Bounds],
    ) -> Result<Point> {
        // 1. Shrunk bounds
        let shrunk = match Self::apply_margin(target_bounds, self.policy.margin_ratio) {
            Some(b) => b,
            None => bail!("Target area too small to apply safe margin"),
        };

        // 2. Intersect with screen bounds
        let safe_box = match shrunk.intersect(screen_bounds) {
            Some(b) => b,
            None => bail!("Target lies completely outside of screen bounds"),
        };

        // 3. Check area
        if safe_box.area() < self.policy.min_area_px {
            bail!(
                "Safe region area ({:.1} px) is below minimum required ({:.1} px)",
                safe_box.area(),
                self.policy.min_area_px
            );
        }

        // 4. RNG Sampling avoiding exclusion masks and recent points
        let mut rng = rand::thread_rng();
        let mut candidate_fallback = None;

        for _ in 0..self.policy.max_tries {
            let rx = rng.gen_range(safe_box.x..=safe_box.right());
            let ry = rng.gen_range(safe_box.y..=safe_box.bottom());
            let pt = Point { x: rx, y: ry };

            // Check if point falls inside any exclusion mask
            let in_exclusion = exclusion_masks
                .iter()
                .any(|mask| mask.contains_point(pt.x, pt.y));
            if in_exclusion {
                continue;
            }

            candidate_fallback = Some(pt);

            // Check distance from recent points
            let too_close = self.recent_points.iter().any(|recent| {
                let dx = pt.x - recent.x;
                let dy = pt.y - recent.y;
                (dx * dx + dy * dy).sqrt() < self.policy.min_distance_px
            });

            if !too_close {
                self.record_point(pt);
                return Ok(pt);
            }
        }

        // Fallback if safe area is too tight to avoid recent points
        if let Some(fallback) = candidate_fallback {
            self.record_point(fallback);
            Ok(fallback)
        } else {
            bail!("Failed to sample a point free from exclusion zones within retry limit");
        }
    }

    fn record_point(&mut self, pt: Point) {
        self.recent_points.push(pt);
        if self.recent_points.len() > self.policy.avoid_recent_points {
            self.recent_points.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_margin_shrink() {
        let b = Bounds::new(100.0, 100.0, 100.0, 100.0);
        let shrunk = SafeRegionSampler::apply_margin(&b, 0.20).unwrap();
        assert_eq!(shrunk.x, 120.0);
        assert_eq!(shrunk.y, 120.0);
        assert_eq!(shrunk.width, 60.0);
        assert_eq!(shrunk.height, 60.0);
    }

    #[test]
    fn test_sample_safe_point_inside_bounds() {
        let mut sampler = SafeRegionSampler::new(SafeTapPolicy::default());
        let target = Bounds::new(200.0, 400.0, 200.0, 80.0);
        let screen = Bounds::new(0.0, 0.0, 1080.0, 1920.0);
        let exclusions = vec![Bounds::new(200.0, 400.0, 20.0, 20.0)];

        for _ in 0..50 {
            let pt = sampler
                .sample_safe_point(&target, &screen, &exclusions)
                .unwrap();
            assert!(target.contains_point(pt.x, pt.y));
            assert!(!exclusions[0].contains_point(pt.x, pt.y));
        }
    }

    #[test]
    fn test_coordinate_transformation() {
        let pt = Point { x: 540.0, y: 960.0 };
        let capture_geom = ScreenGeometry::new(1080, 1920, 0);
        let display_geom = ScreenGeometry::new(2160, 3840, 0);

        let transformed =
            SafeRegionSampler::transform_to_display(pt, &capture_geom, &display_geom).unwrap();
        assert_eq!(transformed.x, 1080.0);
        assert_eq!(transformed.y, 1920.0);
    }

    #[test]
    fn test_point_rotation_and_landscape_transform() {
        let pt = Point { x: 100.0, y: 200.0 };
        // 90 deg rotation on 1080x1920
        let rotated_90 = SafeRegionSampler::rotate_point(pt, 0, 90, 1080, 1920);
        assert_eq!(rotated_90.x, 1920.0 - 1.0 - 200.0);
        assert_eq!(rotated_90.y, 100.0);

        // 180 deg
        let rotated_180 = SafeRegionSampler::rotate_point(pt, 0, 180, 1080, 1920);
        assert_eq!(rotated_180.x, 1080.0 - 1.0 - 100.0);
        assert_eq!(rotated_180.y, 1920.0 - 1.0 - 200.0);
    }
}
