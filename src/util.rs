//! Small math helpers shared by several systems.

use std::f32::consts::PI;

/// Plain linear interpolation.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Interpolate between two angles (radians) along the shortest arc.
pub fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    a + angle_diff(a, b) * t
}

/// Signed shortest difference `b - a`, wrapped into `-PI..=PI`.
pub fn angle_diff(a: f32, b: f32) -> f32 {
    (b - a + PI).rem_euclid(2.0 * PI) - PI
}

/// Frame-rate independent smoothing factor for `lerp`.
///
/// `lerp(a, b, damp(rate, dt))` closes a fraction `1 - e^(-rate * dt)` of the
/// gap each frame, so the motion looks the same at 30, 60 or 240 FPS (unlike
/// a hard-coded `lerp(a, b, 0.2)` per frame). Higher `rate` = snappier.
pub fn damp(rate: f32, dt: f32) -> f32 {
    1.0 - (-rate * dt).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damp_is_framerate_independent() {
        let rate = 10.0;
        // One 1/30 s step vs. two 1/60 s steps must land on the same value.
        let one = lerp(0.0, 1.0, damp(rate, 1.0 / 30.0));
        let mut two = 0.0;
        for _ in 0..2 {
            two = lerp(two, 1.0, damp(rate, 1.0 / 60.0));
        }
        assert!((one - two).abs() < 1e-5, "{one} vs {two}");
    }

    #[test]
    fn damp_zero_dt_is_noop() {
        assert_eq!(damp(10.0, 0.0), 0.0);
    }

    #[test]
    fn angle_diff_takes_shortest_arc() {
        assert!((angle_diff(0.1, -0.1) + 0.2).abs() < 1e-5);
        // 170° -> -170° is a 20° step across the seam, not -340°.
        let d = angle_diff(170f32.to_radians(), (-170f32).to_radians());
        assert!((d - 20f32.to_radians()).abs() < 1e-5, "{d}");
    }
}
