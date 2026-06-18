//! Pedestrians: spawn, AI walking along sidewalks, limb animation.

use bevy::prelude::*;
use bevy::transform::components::GlobalTransform;
use rand::Rng;
use std::f32::consts::PI;

use crate::player::Player;
use crate::resources::{GameAssets, GameState, CITY_HALF, GRID, ROAD_W, STEP};

#[derive(Component)]
pub struct Pedestrian {
    pub speed: f32,
    pub phase: f32,
    pub change_in: f32,
    /// Last-frame world position, synced at the end of `update_peds`. Used by
    /// `player_punch` (which cannot read `Transform` due to B0001 conflict)
    /// to test punch hit distance.
    pub pos: Vec3,
    /// Accumulated knockback impulse to apply on the next `update_peds` tick.
    /// Written by `player_punch`, consumed and zeroed by `update_peds`.
    pub knockback: Vec3,
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
            .spawn((
                Mesh3d(assets.mesh_player_arm.clone()),
                MeshMaterial3d(assets.mat_player_shirt.clone()),
                Transform::from_xyz(-0.36, 1.05, 0.0),
            ))
            .id();
        let arm_r = commands
            .spawn((
                Mesh3d(assets.mesh_player_arm.clone()),
                MeshMaterial3d(assets.mat_player_shirt.clone()),
                Transform::from_xyz(0.36, 1.05, 0.0),
            ))
            .id();
        let leg_l = commands
            .spawn((
                Mesh3d(assets.mesh_player_leg.clone()),
                MeshMaterial3d(assets.mat_player_pants.clone()),
                Transform::from_xyz(-0.14, 0.35, 0.0),
            ))
            .id();
        let leg_r = commands
            .spawn((
                Mesh3d(assets.mesh_player_leg.clone()),
                MeshMaterial3d(assets.mat_player_pants.clone()),
                Transform::from_xyz(0.14, 0.35, 0.0),
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
                if rng.gen_bool(0.5) {
                    PI / 2.0
                } else {
                    -PI / 2.0
                },
            )
        };

        let ped_root = commands
            .spawn((
                Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(rot_y)),
                Visibility::Visible,
                Pedestrian {
                    speed: 1.0 + rng.gen::<f32>() * 0.7,
                    phase: rng.gen::<f32>() * 2.0 * PI,
                    change_in: 2.0 + rng.gen::<f32>() * 5.0,
                    pos,
                    knockback: Vec3::ZERO,
                },
                PedLimbs {
                    arm_l,
                    arm_r,
                    leg_l,
                    leg_r,
                },
            ))
            .id();

        commands
            .entity(ped_root)
            .add_children(&[torso, head, arm_l, arm_r, leg_l, leg_r]);
    }
}

pub fn update_peds(
    time: Res<Time>,
    game_state: Res<GameState>,
    mut peds: Query<(&mut Pedestrian, &mut Transform, &PedLimbs), Without<Player>>,
    mut limb_q: Query<&mut Transform, (Without<Player>, Without<Pedestrian>)>,
    // Read player position via GlobalTransform to avoid B0001 conflict with
    // `update_player`'s `&mut Transform` write on the player.
    player_q: Query<&GlobalTransform, With<Player>>,
) {
    let dt = time.delta_secs();
    let player_pos = player_q
        .get_single()
        .map(|gt| gt.translation())
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
        // Apply punch knockback (written by `player_punch` system).
        transform.translation += ped.knockback;
        ped.knockback = Vec3::ZERO;
        transform.translation.y = 0.3;

        pull_to_sidewalk(&mut transform.translation);

        let lim = CITY_HALF + 4.0;
        transform.translation.x = transform.translation.x.clamp(-lim, lim);
        transform.translation.z = transform.translation.z.clamp(-lim, lim);

        // Avoid player if too close
        if game_state.in_vehicle.is_none() && player_pos.distance(transform.translation) < 1.2 {
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

        // Sync `ped.pos` so `player_punch` (which cannot read Transform) can
        // test punch distance using this cached value.
        ped.pos = transform.translation;
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
