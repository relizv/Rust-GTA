//! Third-person follow camera. Lags smoothly toward the desired position.
//!
//! Two GTA-style behaviours live here:
//! - **In-vehicle free look**: the mouse orbits the camera around the car, and
//!   after `car_recenter_delay` seconds of no mouse input it drifts back
//!   behind the car by itself.
//! - **Over-the-shoulder aiming**: holding RMB slides the camera to the right
//!   so the character stops covering whatever you're shooting at.

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::transform::components::GlobalTransform;
use std::f32::consts::{PI, TAU};

use crate::car::Car;
use crate::config::GameConfig;
use crate::player::Player;
use crate::resources::{GameState, InputState};
use crate::weapons::WeaponState;

/// Shortest signed angle from `from` to `to`, in (-PI, PI]. Keeps the in-car
/// recenter from taking the long way around when yaw wraps past ±π.
fn angle_diff(from: f32, to: f32) -> f32 {
    (to - from + PI).rem_euclid(TAU) - PI
}

#[allow(clippy::too_many_arguments)]
pub fn update_camera(
    time: Res<Time>,
    config: Res<GameConfig>,
    // Own EventReader cursor, independent of the one in `input.rs`: we only
    // need to know WHETHER the mouse moved, `input.rs` still owns the yaw and
    // pitch integration.
    mut mouse_motion: EventReader<MouseMotion>,
    // Mutable so the in-car camera can ease `yaw`/`pitch` back behind the car.
    // Safe: `update_camera` is `.chain()`ed after every system that reads them.
    mut input_state: ResMut<InputState>,
    game_state: Res<GameState>,
    weapon: Res<WeaponState>,
    // Use GlobalTransform (not Transform) so this read does not conflict with
    // `update_player`'s `&mut Transform` write on the player — Bevy 0.15 would
    // otherwise panic with B0001 even though both systems are in `.chain()`.
    player_q: Query<&GlobalTransform, With<Player>>,
    cars: Query<&GlobalTransform, With<Car>>,
    mut camera_q: Query<&mut Transform, With<Camera>>,
    /// Seconds since the player last moved the mouse.
    mut look_idle: Local<f32>,
    /// Smoothed 0..1 blend of the over-the-shoulder aim offset.
    mut shoulder_blend: Local<f32>,
) {
    let dt = time.delta_secs();

    // --- Track mouse idle time (drives the in-car auto-recenter) ---
    let mut mouse_moved = false;
    for ev in mouse_motion.read() {
        if ev.delta.length_squared() > 0.0 {
            mouse_moved = true;
        }
    }
    if mouse_moved && input_state.cursor_locked {
        *look_idle = 0.0;
    } else {
        *look_idle += dt;
    }

    let Ok(player_t) = player_q.get_single() else {
        return;
    };
    let Ok(mut camera_t) = camera_q.get_single_mut() else {
        return;
    };

    let player_pos = player_t.translation();
    let (target, yaw, pitch, dist, height) = if let Some(car_entity) = game_state.in_vehicle {
        let car_gt = cars.get(car_entity).ok();
        let car_pos = car_gt.map(|gt| gt.translation()).unwrap_or(player_pos);
        let car_yaw = car_gt
            .map(|gt| gt.rotation().to_euler(EulerRot::YXZ).0)
            .unwrap_or(0.0);

        // Free look: the mouse orbits the camera around the car exactly like
        // on foot. Once the mouse has been still for a moment, ease back to
        // the default chase pose behind the car.
        if *look_idle >= config.camera.car_recenter_delay {
            let t = 1.0 - (-config.camera.car_recenter_rate * dt).exp();
            let rear_yaw = car_yaw + PI;
            let yaw_now = input_state.yaw;
            input_state.yaw = yaw_now + angle_diff(yaw_now, rear_yaw) * t;
            let pitch_now = input_state.pitch;
            input_state.pitch = pitch_now + (config.camera.car_pitch - pitch_now) * t;
        }

        (
            car_pos,
            input_state.yaw,
            input_state.pitch,
            config.camera.car_distance,
            config.camera.car_height,
        )
    } else {
        // Aiming down sights pulls the camera in for an over-the-shoulder view.
        let dist = if weapon.aiming {
            config.weapons.aim_zoom_dist
        } else {
            config.camera.distance
        };
        (player_pos, input_state.yaw, input_state.pitch, dist, 1.8)
    };

    let offset = Vec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    ) * dist;

    // --- Over-the-shoulder offset (GTA V style) ---
    // Both the camera AND its look-at point shift by the same vector, so the
    // view direction stays parallel to the aim direction — the crosshair keeps
    // pointing where the shot goes, the character just moves to the left of
    // the screen. `player_shoot` starts its ray at the player's depth, so the
    // offset never causes the character to eat their own bullets.
    let want = if weapon.aiming { 1.0 } else { 0.0 };
    let blend = 1.0 - (-config.camera.aim_shoulder_lerp * dt).exp();
    *shoulder_blend += (want - *shoulder_blend) * blend;
    // Camera-right on the horizontal plane, derived from yaw.
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    let shoulder = right * (config.camera.aim_shoulder_offset * *shoulder_blend);

    let mut desired = target + Vec3::new(0.0, height, 0.0) + offset + shoulder;

    // A negative pitch (looking up) swings the camera downward — clamp it so
    // it never sinks through the pavement behind the player.
    let floor = target.y + config.camera.min_height;
    if desired.y < floor {
        desired.y = floor;
    }

    let t = 1.0 - (-12.0 * dt).exp();
    camera_t.translation = camera_t.translation.lerp(desired, t);

    let look_height = if game_state.in_vehicle.is_some() {
        1.4
    } else {
        1.3 + config.camera.aim_look_height * *shoulder_blend
    };
    let look_at = target + Vec3::new(0.0, look_height, 0.0) + shoulder;
    camera_t.look_at(look_at, Vec3::Y);
}
