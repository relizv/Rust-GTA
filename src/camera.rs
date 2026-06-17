//! Third-person follow camera. Lags smoothly toward the desired position,
//! which is computed from the player's transform and `InputState.yaw/pitch`
//! (or the vehicle's yaw when driving).

use bevy::prelude::*;

use crate::player::Player;
use crate::resources::{GameState, InputState};
use crate::car::Car;

pub fn update_camera(
    time: Res<Time>,
    input_state: Res<InputState>,
    game_state: Res<GameState>,
    player_q: Query<&Transform, With<Player>>,
    cars: Query<&Transform, With<Car>>,
    mut camera_q: Query<&mut Transform, With<Camera>>,
) {
    let Ok(player_t) = player_q.get_single() else { return; };
    let Ok(mut camera_t) = camera_q.get_single_mut() else { return; };

    // Determine target + yaw/pitch/dist
    let (target, yaw, pitch, dist) = if let Some(car_entity) = game_state.in_vehicle {
        let car_t = cars.get(car_entity).copied().unwrap_or(*player_t);
        let car_yaw = car_t.rotation.to_euler(EulerRot::YXZ).0 + std::f32::consts::PI;
        (car_t.translation, car_yaw, 0.35, 9.0)
    } else {
        (player_t.translation, input_state.yaw, input_state.pitch, 7.0)
    };

    let offset = Vec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    ) * dist;

    let desired = target + Vec3::new(0.0, if game_state.in_vehicle.is_some() { 3.0 } else { 1.8 }, 0.0) + offset;

    // Smooth follow
    let t = 1.0 - (-12.0 * time.delta_seconds()).exp();
    camera_t.translation = camera_t.translation.lerp(desired, t);

    let look_at = target + Vec3::new(0.0, if game_state.in_vehicle.is_some() { 1.4 } else { 1.3 }, 0.0);
    camera_t.look_at(look_at, Vec3::Y);
}
