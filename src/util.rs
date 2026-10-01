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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angle_diff_takes_shortest_arc() {
        assert!((angle_diff(0.1, -0.1) + 0.2).abs() < 1e-5);
        // 170° -> -170° is a 20° step across the seam, not -340°.
        let d = angle_diff(170f32.to_radians(), (-170f32).to_radians());
        assert!((d - 20f32.to_radians()).abs() < 1e-5, "{d}");
    }
}
