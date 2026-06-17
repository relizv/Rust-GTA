//! Pedestrians: spawn, AI walking along sidewalks, limb animation.

use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

use crate::resources::{GameAssets, GameState, CITY_HALF, GRID, ROAD_W, STEP};
use crate::player::Player;

#[derive(Component)]
pub struct Pedestrian {
    pub speed: f32,
    pub phase: f32,
    pub change_in: f32,
}

#[derive(Component)]
pub struct PedLimbs {
    pub arm_l: Entity,
    pub arm_r: Entity,
    pub leg_l: Entity,
    pub leg_r: Entity,
}

pub fn spawn_peds(mut commands: Commands, assets: Res<GameAssets>) {
    let mut rng = rand::thread_rng();

    for _ in 0..22 {
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

        // Place on a sidewalk
        let lane = rng.gen_range(0..=GRID);
        let coord = -CITY_HALF + lane as f32 * STEP;
        let along = -CITY_HALF + rng.gen::<f32>() * (CITY_HALF * 2.0);
        let side_offset = (if rng.gen_bool(0.5) { -1.0 } else { 1.0 }) * (ROAD_W / 2.0 + 1.0);
        let (pos, rot_y) = if rng.gen_bool(0.5) {
            (
                Vec3::new(along, 0.3, coord + side_offset),
                if rng.gen_bool(0.5) { 0.0 } else { PI },
            )
        } else {
            (
                Vec3::new(coord + side_offset, 0.3, along),
                if rng.gen_bool(0.5) { PI / 2.0 } else { -PI / 2.0 },
            )
        };

        let ped_root = commands
            .spawn((
                SpatialBundle {
                    transform: Transform::from_translation(pos)
                        .with_rotation(Quat::from_rotation_y(rot_y)),
                    visibility: Visibility::Visible,
                    ..default()
                },
                Pedestrian {
                    speed: 1.0 + rng.gen::<f32>() * 0.7,
                    phase: rng.gen::<f32>() * 2.0 * PI,
                    change_in: 2.0 + rng.gen::<f32>() * 5.0,
                },
                PedLimbs { arm_l, arm_r, leg_l, leg_r },
            ))
            .id();

        commands
            .entity(ped_root)
            .push_children(&[torso, head, arm_l, arm_r, leg_l, leg_r]);
    }
}

pub fn update_peds(
    time: Res<Time>,
    game_state: Res<GameState>,
    mut peds: Query<(&mut Pedestrian, &mut Transform, &PedLimbs), Without<Player>>,
    mut limb_q: Query<&mut Transform, (Without<Player>, Without<Pedestrian>)>,
    player_q: Query<&Transform, With<Player>>,
) {
    let dt = time.delta_seconds();
    let player_pos = player_q
        .get_single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    for (mut ped, mut transform, limbs) in peds.iter_mut() {
        ped.change_in -= dt;
        if ped.change_in <= 0.0 {
            let yaw = transform.rotation.to_euler(EulerRot::YXZ).0;
            let new_yaw = yaw + (rand::random::<f32>() - 0.5) * PI;
            transform.rotation = Quat::from_rotation_y(new_yaw);
            ped.change_in = 2.0 + rand::random::<f32>() * 5.0;
        }

        let fwd = transform.rotation * Vec3::new(0.0, 0.0, 1.0);
        transform.translation += fwd * ped.speed * dt;
        transform.translation.y = 0.3;

        pull_to_sidewalk(&mut transform.translation);

        let lim = CITY_HALF + 4.0;
        transform.translation.x = transform.translation.x.clamp(-lim, lim);
        transform.translation.z = transform.translation.z.clamp(-lim, lim);

        // Avoid player if too close
        if game_state.in_vehicle.is_none()
            && player_pos.distance(transform.translation) < 1.2
        {
            let mut away = transform.translation - player_pos;
            away.y = 0.0;
            away = away.normalize_or_zero();
            transform.translation += away * 0.1;
        }

        // Animate
        ped.phase += dt * 6.0;
        let swing_l = (ped.phase).sin() * 0.5;
        let swing_r = -(ped.phase).sin() * 0.5;
        let arm_l_swing = -(ped.phase).sin() * 0.4;
        let arm_r_swing = (ped.phase).sin() * 0.4;
        if let Ok(mut t) = limb_q.get_mut(limbs.leg_l) {
            t.rotation = Quat::from_rotation_x(swing_l);
        }
        if let Ok(mut t) = limb_q.get_mut(limbs.leg_r) {
            t.rotation = Quat::from_rotation_x(swing_r);
        }
        if let Ok(mut t) = limb_q.get_mut(limbs.arm_l) {
            t.rotation = Quat::from_rotation_x(arm_l_swing);
        }
        if let Ok(mut t) = limb_q.get_mut(limbs.arm_r) {
            t.rotation = Quat::from_rotation_x(arm_r_swing);
        }
    }
}

fn pull_to_sidewalk(pos: &mut Vec3) {
    let mut best_axis: Option<&str> = None;
    let mut best_coord = 0.0;
    let mut best_dist = f32::MAX;
    for i in 0..=GRID {
        let c = -CITY_HALF + i as f32 * STEP;
        let dx = (pos.x - c).abs();
        if dx < best_dist {
            best_dist = dx;
            best_axis = Some("x");
            best_coord = c;
        }
        let dz = (pos.z - c).abs();
        if dz < best_dist {
            best_dist = dz;
            best_axis = Some("z");
            best_coord = c;
        }
    }
    let offset = ROAD_W / 2.0 + 1.2;
    if best_axis == Some("x") {
        let side = if pos.x > best_coord { 1.0 } else { -1.0 };
        pos.x = lerp(pos.x, best_coord + side * offset, 0.05);
    } else if best_axis == Some("z") {
        let side = if pos.z > best_coord { 1.0 } else { -1.0 };
        pos.z = lerp(pos.z, best_coord + side * offset, 0.05);
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
