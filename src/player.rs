//! Player: spawn, movement, limb animation, enter/exit vehicles, punch.

use bevy::input::mouse::MouseButton;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::transform::components::GlobalTransform;
use std::f32::consts::PI;

use crate::car::Car;
use crate::pedestrian::Pedestrian;
use crate::resources::{GameAssets, GameState, InputState, KeysPressed, CITY_HALF, ROAD_W};

#[derive(Component)]
pub struct Player;

/// Marker component attached to each player limb entity so that
/// `update_player`'s `&mut Transform` access on limbs is disjoint (at the
/// archetype level) from `update_peds`'s `&mut Transform` access on ped limbs.
/// Without this, Bevy 0.15 panics with B0001 because both queries are filtered
/// only by `Without<Player>` / `Without<Pedestrian>` and would overlap on any
/// Transform entity that has neither marker.
#[derive(Component)]
pub struct PlayerLimb;

#[derive(Component)]
pub struct PlayerState {
    pub vel: Vec3,
    pub yaw: f32,
    pub on_ground: bool,
}

#[derive(Component)]
pub struct PlayerLimbs {
    pub arm_l: Entity,
    pub arm_r: Entity,
    pub leg_l: Entity,
    pub leg_r: Entity,
}

pub fn spawn_player(mut commands: Commands, assets: Res<GameAssets>) {
    let arm_l = commands
        .spawn((
            Mesh3d(assets.mesh_player_arm.clone()),
            MeshMaterial3d(assets.mat_player_shirt.clone()),
            Transform::from_xyz(-0.36, 1.05, 0.0),
            PlayerLimb,
        ))
        .id();
    let arm_r = commands
        .spawn((
            Mesh3d(assets.mesh_player_arm.clone()),
            MeshMaterial3d(assets.mat_player_shirt.clone()),
            Transform::from_xyz(0.36, 1.05, 0.0),
            PlayerLimb,
        ))
        .id();
    let leg_l = commands
        .spawn((
            Mesh3d(assets.mesh_player_leg.clone()),
            MeshMaterial3d(assets.mat_player_pants.clone()),
            Transform::from_xyz(-0.14, 0.35, 0.0),
            PlayerLimb,
        ))
        .id();
    let leg_r = commands
        .spawn((
            Mesh3d(assets.mesh_player_leg.clone()),
            MeshMaterial3d(assets.mat_player_pants.clone()),
            Transform::from_xyz(0.14, 0.35, 0.0),
            PlayerLimb,
        ))
        .id();
    let torso = commands
        .spawn((
            Mesh3d(assets.mesh_player_torso.clone()),
            MeshMaterial3d(assets.mat_player_shirt.clone()),
            Transform::from_xyz(0.0, 1.05, 0.0),
        ))
        .id();
    let head = commands
        .spawn((
            Mesh3d(assets.mesh_player_head.clone()),
            MeshMaterial3d(assets.mat_player_skin.clone()),
            Transform::from_xyz(0.0, 1.6, 0.0),
        ))
        .id();
    let hair = commands
        .spawn((
            Mesh3d(assets.mesh_unit_box.clone()),
            MeshMaterial3d(assets.mat_player_hair.clone()),
            Transform::from_xyz(0.0, 1.78, 0.0).with_scale(Vec3::new(0.34, 0.1, 0.34)),
        ))
        .id();

    let player_root = commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, ROAD_W + 2.0),
            Visibility::Visible,
            Player,
            PlayerState {
                vel: Vec3::ZERO,
                yaw: 0.0,
                on_ground: true,
            },
            PlayerLimbs {
                arm_l,
                arm_r,
                leg_l,
                leg_r,
            },
        ))
        .id();

    commands
        .entity(player_root)
        .add_children(&[torso, head, hair, arm_l, arm_r, leg_l, leg_r]);
}

#[allow(clippy::too_many_arguments)]
pub fn update_player(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<KeysPressed>,
    input_state: Res<InputState>,
    mut game_state: ResMut<GameState>,
    mut player_q: Query<
        (
            &mut PlayerState,
            &mut Transform,
            &PlayerLimbs,
            &mut Visibility,
        ),
        (With<Player>, Without<PlayerLimb>),
    >,
    mut limb_q: Query<&mut Transform, With<PlayerLimb>>,
    // Read car positions via `GlobalTransform` (not `Transform`) so that this
    // system's read does not conflict with `update_ai_cars`'s `&mut Transform`
    // write on the same components — Bevy 0.15 panics with B0001 otherwise.
    // `GlobalTransform` is a separate component, so the scheduler is happy.
    // GlobalTransform is propagated at end of frame, so we see last frame's
    // car pose — fine for player-vs-car proximity checks at 60 FPS.
    cars: Query<(Entity, &Car, &GlobalTransform), Without<Player>>,
    buildings: Query<&crate::city::Building>,
) {
    let Ok((mut state, mut transform, limbs, mut vis)) = player_q.get_single_mut() else {
        return;
    };

    // --- Edge-triggered actions ---
    if keys.f_pressed {
        if game_state.in_vehicle.is_some() {
            if let Some(car_entity) = game_state.in_vehicle.take() {
                if let Ok((_, _, car_gt)) = cars.get(car_entity) {
                    // Use the GlobalTransform's rotation/translation.
                    let car_pos = car_gt.translation();
                    let car_rot = car_gt.rotation();
                    // Yaw angle (Euler Y) from the world-space rotation.
                    let (yaw, _, _) = car_rot.to_euler(EulerRot::YXZ);
                    let right = Quat::from_rotation_y(yaw) * Vec3::new(1.0, 0.0, 0.0);
                    let exit = car_pos + right * 2.0;
                    state.vel = Vec3::ZERO;
                    transform.translation = Vec3::new(exit.x, 0.0, exit.z);
                }
                *vis = Visibility::Visible;
                game_state.show_toast("Вы вышли из машины");
            }
        } else {
            let player_pos = transform.translation;
            let mut nearest: Option<(Entity, f32)> = None;
            for (e, _, gt) in cars.iter() {
                let d = gt.translation().distance(player_pos);
                if d < 4.0 && (nearest.is_none() || d < nearest.unwrap().1) {
                    nearest = Some((e, d));
                }
            }
            if let Some((e, _)) = nearest {
                game_state.in_vehicle = Some(e);
                *vis = Visibility::Hidden;
                game_state.add_wanted(1);
                game_state.show_toast("Машина угнана!");
            } else {
                game_state.show_toast("Рядом нет машин");
            }
        }
    }

    if keys.r_pressed {
        if let Some(car_entity) = game_state.in_vehicle.take() {
            commands.entity(car_entity).despawn_recursive();
        }
        transform.translation = Vec3::new(0.0, 0.0, ROAD_W + 2.0);
        state.vel = Vec3::ZERO;
        *vis = Visibility::Visible;
        game_state.show_toast("Позиция сброшена");
    }

    // NOTE: punch handling was moved into the separate `player_punch` system
    // to avoid the B0001 conflict with `update_peds` (both wrote `&mut
    // Transform` on ped entities).

    // --- Movement ---
    if let Some(car_entity) = game_state.in_vehicle {
        if let Ok((_, _, car_gt)) = cars.get(car_entity) {
            transform.translation = car_gt.translation();
            let (yaw, _, _) = car_gt.rotation().to_euler(EulerRot::YXZ);
            state.yaw = yaw;
        }
        return;
    }

    let yaw = input_state.yaw;
    let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());

    let mut move_vec = Vec3::ZERO;
    if keys.w {
        move_vec += forward;
    }
    if keys.s {
        move_vec -= forward;
    }
    if keys.d {
        move_vec += right;
    }
    if keys.a {
        move_vec -= right;
    }

    let speed = if keys.shift { 9.0 } else { 4.5 };

    if move_vec.length_squared() > 0.0 {
        move_vec = move_vec.normalize() * speed;
        state.vel.x = lerp(state.vel.x, move_vec.x, 0.2);
        state.vel.z = lerp(state.vel.z, move_vec.z, 0.2);
        let target_yaw = move_vec.x.atan2(move_vec.z);
        state.yaw = lerp_angle(state.yaw, target_yaw, 0.18);
    } else {
        state.vel.x *= 0.8;
        state.vel.z *= 0.8;
    }

    if keys.space && state.on_ground {
        state.vel.y = 7.5;
        state.on_ground = false;
    }
    state.vel.y -= 22.0 * time.delta_secs();

    transform.translation += state.vel * time.delta_secs();
    if transform.translation.y <= 0.0 {
        transform.translation.y = 0.0;
        state.vel.y = 0.0;
        state.on_ground = true;
    }

    collide_buildings(&mut transform.translation, 0.6, &buildings);

    let lim = CITY_HALF + 10.0;
    transform.translation.x = transform.translation.x.clamp(-lim, lim);
    transform.translation.z = transform.translation.z.clamp(-lim, lim);

    transform.rotation = Quat::from_rotation_y(state.yaw);

    // --- Animate limbs ---
    let speed2 = state.vel.x.hypot(state.vel.z);
    let t = time.elapsed_secs() * if keys.shift { 1.6 } else { 1.0 };
    animate_limb(&mut limb_q, limbs.arm_l, speed2, t, 0.5, true);
    animate_limb(&mut limb_q, limbs.arm_r, speed2, t, 0.5, false);
    animate_limb(&mut limb_q, limbs.leg_l, speed2, t, 0.7, true);
    animate_limb(&mut limb_q, limbs.leg_r, speed2, t, 0.7, false);
}

fn animate_limb(
    q: &mut Query<&mut Transform, With<PlayerLimb>>,
    entity: Entity,
    speed: f32,
    t: f32,
    amp: f32,
    invert: bool,
) {
    if let Ok(mut tr) = q.get_mut(entity) {
        if speed > 0.5 {
            let s = if invert { 1.0 } else { -1.0 };
            tr.rotation = Quat::from_rotation_x((t * 12.0).sin() * amp * s);
        } else {
            tr.rotation = Quat::slerp(tr.rotation, Quat::IDENTITY, 0.2);
        }
    }
}

/// Handle the LMB punch action as a separate system so that its `&mut Transform`
/// access on ped entities does not conflict with `update_peds` (Bevy 0.15 B0001).
/// Must run AFTER `update_peds` so that the punch knockback is applied on top of
/// the ped's sidewalk movement this frame.
pub fn player_punch(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    // Use GlobalTransform for the player position to avoid B0001 with
    // `update_player`'s `&mut Transform` write on the player.
    player_q: Query<(&GlobalTransform, &PlayerState), With<Player>>,
    // We only write to `Pedestrian.knockback` (a Vec3 field), not to Transform,
    // so there is no B0001 with `update_peds`'s `&mut Transform` access.
    mut peds: Query<&mut Pedestrian, Without<Player>>,
    // Single mutable access to GameState: it is also readable through this
    // borrow, so we don't need a separate `Res<GameState>` (that caused B0002).
    mut game_state: ResMut<GameState>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) || game_state.in_vehicle.is_some() {
        return;
    }
    let Ok((player_gt, state)) = player_q.get_single() else {
        return;
    };
    let player_pos = player_gt.translation();
    let forward = Vec3::new(state.yaw.sin(), 0.0, state.yaw.cos());
    let hit_pos = player_pos + forward * 1.2;
    let mut hit_count = 0;
    for mut ped in peds.iter_mut() {
        // Use ped.knockback as a temp position proxy: we need to know the ped's
        // current position to test distance, but we cannot read Transform here.
        // Workaround: store the last known position in Pedestrian each frame
        // (see `update_peds` where we sync `ped.pos = transform.translation`).
        if ped.pos.distance(hit_pos) < 1.4 {
            let mut knock = (ped.pos - hit_pos).normalize_or_zero() * 2.5;
            knock.y = 0.0;
            ped.knockback += knock;
            hit_count += 1;
        }
    }
    if hit_count > 0 {
        game_state.cash += 5 * hit_count;
        game_state.show_toast(format!("+${}", 5 * hit_count));
    }
}

pub fn update_wanted_decay(time: Res<Time>, mut game_state: ResMut<GameState>) {
    if let Some((_, t)) = &mut game_state.toast {
        *t -= time.delta_secs();
        if *t <= 0.0 {
            game_state.toast = None;
        }
    }
    if game_state.wanted > 0 {
        game_state.wanted_decay_timer += time.delta_secs();
        if game_state.wanted_decay_timer > 18.0 {
            game_state.wanted -= 1;
            game_state.wanted_decay_timer = 0.0;
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut diff = b - a;
    while diff > PI {
        diff -= 2.0 * PI;
    }
    while diff < -PI {
        diff += 2.0 * PI;
    }
    a + diff * t
}

fn collide_buildings(pos: &mut Vec3, radius: f32, buildings: &Query<&crate::city::Building>) {
    for b in buildings.iter() {
        let dx = (pos.x - b.cx).abs();
        let dz = (pos.z - b.cz).abs();
        let half_w = b.w / 2.0 + radius;
        let half_d = b.d / 2.0 + radius;
        if dx < half_w && dz < half_d {
            let pen_x = half_w - dx;
            let pen_z = half_d - dz;
            if pen_x < pen_z {
                pos.x = b.cx + (pos.x - b.cx).signum() * half_w;
            } else {
                pos.z = b.cz + (pos.z - b.cz).signum() * half_d;
            }
        }
    }
}
