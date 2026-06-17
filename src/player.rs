//! Player: spawn, movement, limb animation, enter/exit vehicles, punch.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::input::mouse::MouseButton;
use std::f32::consts::PI;

use crate::resources::{GameAssets, GameState, InputState, KeysPressed, CITY_HALF, ROAD_W};
use crate::car::Car;
use crate::pedestrian::Pedestrian;

#[derive(Component)]
pub struct Player;

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
        .spawn(PbrBundle {
            mesh: assets.mesh_player_arm.clone(),
            material: assets.mat_player_shirt.clone(),
            transform: Transform::from_xyz(-0.36, 1.05, 0.0),
            ..default()
        })
        .id();
    let arm_r = commands
        .spawn(PbrBundle {
            mesh: assets.mesh_player_arm.clone(),
            material: assets.mat_player_shirt.clone(),
            transform: Transform::from_xyz(0.36, 1.05, 0.0),
            ..default()
        })
        .id();
    let leg_l = commands
        .spawn(PbrBundle {
            mesh: assets.mesh_player_leg.clone(),
            material: assets.mat_player_pants.clone(),
            transform: Transform::from_xyz(-0.14, 0.35, 0.0),
            ..default()
        })
        .id();
    let leg_r = commands
        .spawn(PbrBundle {
            mesh: assets.mesh_player_leg.clone(),
            material: assets.mat_player_pants.clone(),
            transform: Transform::from_xyz(0.14, 0.35, 0.0),
            ..default()
        })
        .id();
    let torso = commands
        .spawn(PbrBundle {
            mesh: assets.mesh_player_torso.clone(),
            material: assets.mat_player_shirt.clone(),
            transform: Transform::from_xyz(0.0, 1.05, 0.0),
            ..default()
        })
        .id();
    let head = commands
        .spawn(PbrBundle {
            mesh: assets.mesh_player_head.clone(),
            material: assets.mat_player_skin.clone(),
            transform: Transform::from_xyz(0.0, 1.6, 0.0),
            ..default()
        })
        .id();
    let hair = commands
        .spawn(PbrBundle {
            mesh: assets.mesh_unit_box.clone(),
            material: assets.mat_player_hair.clone(),
            transform: Transform::from_xyz(0.0, 1.78, 0.0).with_scale(Vec3::new(0.34, 0.1, 0.34)),
            ..default()
        })
        .id();

    let player_root = commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_xyz(0.0, 0.0, ROAD_W + 2.0),
                visibility: Visibility::Visible,
                ..default()
            },
            Player,
            PlayerState {
                vel: Vec3::ZERO,
                yaw: 0.0,
                on_ground: true,
            },
            PlayerLimbs { arm_l, arm_r, leg_l, leg_r },
        ))
        .id();

    commands
        .entity(player_root)
        .push_children(&[torso, head, hair, arm_l, arm_r, leg_l, leg_r]);
}

#[allow(clippy::too_many_arguments)]
pub fn update_player(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<KeysPressed>,
    input_state: Res<InputState>,
    mut game_state: ResMut<GameState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut player_q: Query<
        (&mut PlayerState, &mut Transform, &PlayerLimbs, &mut Visibility),
        With<Player>,
    >,
    mut limb_q: Query<&mut Transform, Without<Player>>,
    cars: Query<(Entity, &Car, &Transform), Without<Player>>,
    mut peds: Query<(Entity, &mut Transform, &mut Pedestrian), Without<Player>>,
    buildings: Query<&crate::city::Building>,
) {
    let Ok((mut state, mut transform, limbs, mut vis)) = player_q.get_single_mut() else {
        return;
    };

    // --- Edge-triggered actions ---
    if keys.f_pressed {
        if game_state.in_vehicle.is_some() {
            if let Some(car_entity) = game_state.in_vehicle.take() {
                if let Ok((_, _, car_transform)) = cars.get(car_entity) {
                    let right =
                        Quat::from_rotation_y(car_transform.rotation.y) * Vec3::new(1.0, 0.0, 0.0);
                    let exit = car_transform.translation + right * 2.0;
                    state.vel = Vec3::ZERO;
                    transform.translation = Vec3::new(exit.x, 0.0, exit.z);
                }
                *vis = Visibility::Visible;
                game_state.show_toast("Вы вышли из машины");
            }
        } else {
            let player_pos = transform.translation;
            let mut nearest: Option<(Entity, f32)> = None;
            for (e, _, t) in cars.iter() {
                let d = t.translation.distance(player_pos);
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

    // LMB = punch (only when on foot)
    if mouse_buttons.just_pressed(MouseButton::Left)
        && game_state.in_vehicle.is_none()
    {
        do_punch(&transform, &state, &mut peds, &mut game_state);
    }

    // --- Movement ---
    if let Some(car_entity) = game_state.in_vehicle {
        if let Ok((_, _, car_transform)) = cars.get(car_entity) {
            transform.translation = car_transform.translation;
            state.yaw = car_transform.rotation.y;
        }
        return;
    }

    let yaw = input_state.yaw;
    let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());

    let mut move_vec = Vec3::ZERO;
    if keys.w { move_vec += forward; }
    if keys.s { move_vec -= forward; }
    if keys.d { move_vec += right; }
    if keys.a { move_vec -= right; }

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
    state.vel.y -= 22.0 * time.delta_seconds();

    transform.translation += state.vel * time.delta_seconds();
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
    let t = time.elapsed_seconds() * if keys.shift { 1.6 } else { 1.0 };
    animate_limb(&mut limb_q, limbs.arm_l, speed2, t, 0.5, true);
    animate_limb(&mut limb_q, limbs.arm_r, speed2, t, 0.5, false);
    animate_limb(&mut limb_q, limbs.leg_l, speed2, t, 0.7, true);
    animate_limb(&mut limb_q, limbs.leg_r, speed2, t, 0.7, false);
}

fn animate_limb(
    q: &mut Query<&mut Transform, Without<Player>>,
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

fn do_punch(
    transform: &Transform,
    state: &PlayerState,
    peds: &mut Query<(Entity, &mut Transform, &mut Pedestrian), Without<Player>>,
    game_state: &mut GameState,
) {
    let forward = Vec3::new(state.yaw.sin(), 0.0, state.yaw.cos());
    let hit_pos = transform.translation + forward * 1.2;
    let mut hit_count = 0;
    for (_, mut ped_t, _) in peds.iter_mut() {
        if ped_t.translation.distance(hit_pos) < 1.4 {
            let mut knock = (ped_t.translation - hit_pos).normalize_or_zero() * 2.5;
            knock.y = 0.0;
            ped_t.translation.x += knock.x;
            ped_t.translation.z += knock.z;
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
        *t -= time.delta_seconds();
        if *t <= 0.0 {
            game_state.toast = None;
        }
    }
    if game_state.wanted > 0 {
        game_state.wanted_decay_timer += time.delta_seconds();
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

fn collide_buildings(
    pos: &mut Vec3,
    radius: f32,
    buildings: &Query<&crate::city::Building>,
) {
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
