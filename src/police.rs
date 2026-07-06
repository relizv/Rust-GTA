//! Police: cop cars spawn while you're wanted, chase you, and flash their
//! red/blue light bars. Touch a cop for too long and you're busted — fine,
//! respawn, wanted reset.
//!
//! All tunables live in `config.rs` (`PoliceConfig`).

use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy::transform::components::GlobalTransform;
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::car::collides_buildings_at;
use crate::city::Building;
use crate::config::GameConfig;
use crate::player::Player;
use crate::resources::{GameAssets, GameState, CITY_HALF, GRID, ROAD_W, STEP};

#[derive(Component)]
pub struct PoliceCar;

/// Cop car hit points — shot down by the player's pistol (`weapons.rs`).
#[derive(Component)]
pub struct PoliceHealth {
    pub hp: f32,
}

/// Handles to the two roof lamps so `update_police` can swap their materials
/// to make them flash alternately.
#[derive(Component)]
pub struct PoliceLights {
    red: Entity,
    blue: Entity,
    timer: f32,
    red_phase: bool,
}

/// Keeps the number of live cop cars in sync with the wanted level.
pub fn manage_police(
    mut commands: Commands,
    assets: Res<GameAssets>,
    config: Res<GameConfig>,
    game_state: Res<GameState>,
    police: Query<Entity, With<PoliceCar>>,
    player_q: Query<&GlobalTransform, With<Player>>,
) {
    let target = if game_state.wanted == 0 {
        0
    } else {
        (game_state.wanted as usize * config.police.cars_per_star).min(config.police.max_cars)
    };
    let current = police.iter().count();

    if current > target {
        for e in police.iter().take(current - target) {
            commands.entity(e).despawn_recursive();
        }
        return;
    }

    if current < target {
        let player_pos = player_q
            .get_single()
            .map(|gt| gt.translation())
            .unwrap_or(Vec3::ZERO);
        let mut rng = rand::thread_rng();
        for _ in 0..(target - current) {
            spawn_police_car(&mut commands, &assets, &config, player_pos, &mut rng);
        }
    }
}

fn spawn_police_car<R: Rng>(
    commands: &mut Commands,
    assets: &GameAssets,
    config: &GameConfig,
    player_pos: Vec3,
    rng: &mut R,
) {
    // Drop the cop on a random road grid line, `spawn_distance` away from the
    // player (so they never pop into existence right in your face).
    let lane_idx = rng.gen_range(0..=GRID);
    let coord = -CITY_HALF + lane_idx as f32 * STEP;
    let lane_offset = (if rng.gen_bool(0.5) { -1.0 } else { 1.0 }) * 2.0;
    let d = config.police.spawn_distance * if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
    let pos = if rng.gen_bool(0.5) {
        Vec3::new(
            (player_pos.x + d).clamp(-CITY_HALF, CITY_HALF),
            0.0,
            coord + lane_offset,
        )
    } else {
        Vec3::new(
            coord + lane_offset,
            0.0,
            (player_pos.z + d).clamp(-CITY_HALF, CITY_HALF),
        )
    };

    // Face the player right away.
    let to = player_pos - pos;
    let yaw = to.x.atan2(to.z);

    // --- Wheels ---
    let wheel_pos = [
        (-0.95, 0.35, 1.3),
        (0.95, 0.35, 1.3),
        (-0.95, 0.35, -1.3),
        (0.95, 0.35, -1.3),
    ];
    let wheel_rot = Quat::from_rotation_z(FRAC_PI_2);
    let mut children: Vec<Entity> = wheel_pos
        .iter()
        .map(|(x, y, z)| {
            commands
                .spawn((
                    Mesh3d(assets.mesh_cylinder_wheel.clone()),
                    MeshMaterial3d(assets.mat_wheel.clone()),
                    Transform::from_xyz(*x, *y, *z).with_rotation(wheel_rot),
                ))
                .id()
        })
        .collect();

    // --- Body: white body, black cabin (classic cruiser look) ---
    children.push(
        commands
            .spawn((
                Mesh3d(assets.mesh_car_body.clone()),
                MeshMaterial3d(assets.mat_police_body.clone()),
                Transform::from_xyz(0.0, 0.7, 0.0),
            ))
            .id(),
    );
    children.push(
        commands
            .spawn((
                Mesh3d(assets.mesh_car_cabin.clone()),
                MeshMaterial3d(assets.mat_police_cabin.clone()),
                Transform::from_xyz(0.0, 1.3, -0.1),
            ))
            .id(),
    );
    children.push(
        commands
            .spawn((
                Mesh3d(assets.mesh_car_windshield.clone()),
                MeshMaterial3d(assets.mat_windshield.clone()),
                Transform::from_xyz(0.0, 1.3, 0.95)
                    .with_rotation(Quat::from_rotation_x(-FRAC_PI_2 + 0.5)),
            ))
            .id(),
    );
    for x in [-0.6_f32, 0.6] {
        children.push(
            commands
                .spawn((
                    Mesh3d(assets.mesh_car_headlight.clone()),
                    MeshMaterial3d(assets.mat_headlight.clone()),
                    Transform::from_xyz(x, 0.7, 2.1),
                ))
                .id(),
        );
    }

    // --- Light bar on the roof: dark base + red & blue lamps ---
    children.push(
        commands
            .spawn((
                Mesh3d(assets.mesh_unit_box.clone()),
                MeshMaterial3d(assets.mat_lightbar.clone()),
                Transform::from_xyz(0.0, 1.72, -0.1).with_scale(Vec3::new(1.1, 0.12, 0.42)),
            ))
            .id(),
    );
    let red = commands
        .spawn((
            Mesh3d(assets.mesh_unit_box.clone()),
            MeshMaterial3d(assets.mat_lamp_red_on.clone()),
            Transform::from_xyz(-0.28, 1.85, -0.1).with_scale(Vec3::new(0.42, 0.16, 0.34)),
            NotShadowCaster,
        ))
        .id();
    let blue = commands
        .spawn((
            Mesh3d(assets.mesh_unit_box.clone()),
            MeshMaterial3d(assets.mat_lamp_blue_off.clone()),
            Transform::from_xyz(0.28, 1.85, -0.1).with_scale(Vec3::new(0.42, 0.16, 0.34)),
            NotShadowCaster,
        ))
        .id();
    children.push(red);
    children.push(blue);

    let root = commands
        .spawn((
            Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::Visible,
            PoliceCar,
            PoliceHealth {
                hp: config.weapons.police_car_hp,
            },
            PoliceLights {
                red,
                blue,
                timer: 0.0,
                red_phase: true,
            },
        ))
        .id();
    commands.entity(root).add_children(&children);
}

/// Chase AI + contact damage + busted logic + flashing lights.
#[allow(clippy::too_many_arguments)]
pub fn update_police(
    time: Res<Time>,
    config: Res<GameConfig>,
    assets: Res<GameAssets>,
    mut game_state: ResMut<GameState>,
    // `Without<Player>` makes this provably disjoint from `player_q` below
    // (Bevy 0.15 B0001).
    mut police: Query<(&mut Transform, &mut PoliceLights), (With<PoliceCar>, Without<Player>)>,
    mut lamp_mats: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut player_q: Query<(&mut Transform, &mut Visibility), With<Player>>,
    buildings: Query<&Building>,
) {
    let dt = time.delta_secs();
    let Ok((mut player_tf, mut player_vis)) = player_q.get_single_mut() else {
        return;
    };
    let player_pos = player_tf.translation;

    let mut busted = false;

    for (mut tf, mut lights) in police.iter_mut() {
        // --- Chase steering: turn toward the player, drive forward ---
        let mut to = player_pos - tf.translation;
        to.y = 0.0;
        let dist = to.length();
        let target_yaw = to.x.atan2(to.z);
        let current_yaw = tf.rotation.to_euler(EulerRot::YXZ).0;
        let yaw = lerp_angle(
            current_yaw,
            target_yaw,
            (config.police.steer_rate * dt).min(1.0),
        );
        tf.rotation = Quat::from_rotation_y(yaw);

        // Slow down when close so they don't orbit the player at full speed.
        let speed = if dist > 5.0 {
            config.police.chase_speed
        } else {
            config.police.chase_speed * 0.4
        };
        let fwd = tf.rotation * Vec3::new(0.0, 0.0, 1.0);
        let next = tf.translation + fwd * speed * dt;

        if !collides_buildings_at(next.x, next.z, 1.4, &buildings) {
            tf.translation = next;
        } else {
            // Blocked by a building — slide along whichever axis is free.
            if !collides_buildings_at(next.x, tf.translation.z, 1.4, &buildings) {
                tf.translation.x = next.x;
            } else if !collides_buildings_at(tf.translation.x, next.z, 1.4, &buildings) {
                tf.translation.z = next.z;
            }
        }
        tf.translation.y = 0.0;
        let lim = CITY_HALF + 8.0;
        tf.translation.x = tf.translation.x.clamp(-lim, lim);
        tf.translation.z = tf.translation.z.clamp(-lim, lim);

        // --- Contact: cop grabs you, HP drains ---
        if dist < config.police.contact_radius {
            game_state.hp -= config.police.contact_damage_per_sec * dt;
            if game_state.hp <= 0.0 {
                busted = true;
            }
        }

        // --- Flashing light bar: swap red/blue lamp materials ---
        lights.timer += dt;
        if lights.timer >= 1.0 / config.police.flash_hz {
            lights.timer = 0.0;
            lights.red_phase = !lights.red_phase;
            let (red_mat, blue_mat) = if lights.red_phase {
                (
                    assets.mat_lamp_red_on.clone(),
                    assets.mat_lamp_blue_off.clone(),
                )
            } else {
                (
                    assets.mat_lamp_red_off.clone(),
                    assets.mat_lamp_blue_on.clone(),
                )
            };
            if let Ok(mut m) = lamp_mats.get_mut(lights.red) {
                m.0 = red_mat;
            }
            if let Ok(mut m) = lamp_mats.get_mut(lights.blue) {
                m.0 = blue_mat;
            }
        }
    }

    if busted {
        let fine = (game_state.cash as f32 * config.police.busted_fine_frac) as i32;
        game_state.cash -= fine;
        game_state.hp = 100.0;
        game_state.wanted = 0;
        game_state.wanted_decay_timer = 0.0;
        game_state.in_vehicle = None;
        *player_vis = Visibility::Visible;
        player_tf.translation = Vec3::new(0.0, 0.0, ROAD_W + 2.0);
        game_state.show_toast(format!("🚔 ЗАДЕРЖАН! Штраф ${}", fine));
    }
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
