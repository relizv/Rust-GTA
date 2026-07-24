//! Third-person follow camera. Lags smoothly toward the desired position.

use bevy::prelude::*;
use bevy::transform::components::GlobalTransform;

use crate::car::Car;
use crate::config::GameConfig;
use crate::player::Player;
use crate::resources::{GameState, InputState};
use crate::weapons::WeaponState;

pub fn update_camera(
    time: Res<Time>,
    config: Res<GameConfig>,
    input_state: Res<InputState>,
    game_state: Res<GameState>,
    weapon: Res<WeaponState>,
    // Use GlobalTransform (not Transform) so this read does not conflict with
    // `update_player`'s `&mut Transform` write on the player — Bevy 0.15 would
    // otherwise panic with B0001 even though both systems are in `.chain()`.
    player_q: Query<&GlobalTransform, With<Player>>,
    cars: Query<&GlobalTransform, With<Car>>,
    mut camera_q: Query<&mut Transform, With<Camera>>,
) {
    let Ok(player_t) = player_q.get_single() else {
        return;
    };
    let Ok(mut camera_t) = camera_q.get_single_mut() else {
        return;
    };

    let player_pos = player_t.translation();
    let (target, yaw, pitch, dist) = if let Some(car_entity) = game_state.in_vehicle {
        let car_pos = cars
            .get(car_entity)
            .map(|gt| gt.translation())
            .unwrap_or(player_pos);
        let car_yaw = cars
            .get(car_entity)
            .map(|gt| gt.rotation().to_euler(EulerRot::YXZ).0)
            .unwrap_or(0.0)
            + std::f32::consts::PI;
        (car_pos, car_yaw, 0.35, 9.0)
    } else {
        // Aiming down sights pulls the camera in for an over-the-shoulder view.
        let dist = if weapon.aiming {
            config.weapons.aim_zoom_dist
        } else {
            config.camera.distance
        };
        (player_pos, input_state.yaw, input_state.pitch, dist)
    };

    let offset = Vec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    ) * dist;

    let height = if game_state.in_vehicle.is_some() {
        3.0
    } else {
        1.8
    };
    let mut desired = target + Vec3::new(0.0, height, 0.0) + offset;

    // A negative pitch (looking up) swings the camera downward — clamp it so
    // it never sinks through the pavement behind the player.
    let floor = target.y + config.camera.min_height;
    if desired.y < floor {
        desired.y = floor;
    }

    let t = 1.0 - (-12.0 * time.delta_secs()).exp();
    camera_t.translation = camera_t.translation.lerp(desired, t);

    let look_height = if game_state.in_vehicle.is_some() {
        1.4
    } else {
        1.3
    };
    let look_at = target + Vec3::new(0.0, look_height, 0.0);
    camera_t.look_at(look_at, Vec3::Y);
}
