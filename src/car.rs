//! Cars: spawn, AI navigation along road grid, player driving.

use bevy::prelude::*;
use rand::Rng;

use crate::city::Building;
use crate::resources::{GameAssets, GameState, KeysPressed, CITY_HALF, GRID, STEP};

#[derive(Component)]
pub struct Car {
    pub axis: Axis,
    pub dir: f32,
    pub speed: f32,
    pub color_idx: usize,
}

#[derive(Component, Clone, Copy, PartialEq)]
pub enum Axis {
    X,
    Z,
}

#[derive(Component)]
pub struct CarWheels {
    pub fl: Entity,
    pub fr: Entity,
    pub rl: Entity,
    pub rr: Entity,
    pub spin: f32,
}

pub fn spawn_cars(mut commands: Commands, assets: Res<GameAssets>) {
    let mut rng = rand::thread_rng();
    let count = 14;
    for _ in 0..count {
        let color_idx = rng.gen_range(0..assets.mat_car_colors.len());
        let axis = if rng.gen_bool(0.5) { Axis::X } else { Axis::Z };
        let lane = rng.gen_range(0..=GRID);
        let coord = -CITY_HALF + lane as f32 * STEP;
        let along = -CITY_HALF + rng.gen::<f32>() * (CITY_HALF * 2.0);
        let lane_offset = (if rng.gen_bool(0.5) { -1.0 } else { 1.0 }) * 2.0;
        let dir = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };

        let (pos, rot_y) = match axis {
            Axis::X => (
                Vec3::new(along, 0.0, coord + lane_offset),
                std::f32::consts::PI / 2.0,
            ),
            Axis::Z => (Vec3::new(coord + lane_offset, 0.0, along), 0.0),
        };

        // Wheels
        let wheel_pos = [
            (-0.95, 0.35, 1.3),
            (0.95, 0.35, 1.3),
            (-0.95, 0.35, -1.3),
            (0.95, 0.35, -1.3),
        ];
        let wheel_rot = Quat::from_rotation_z(std::f32::consts::PI / 2.0);
        let wheel_entities: Vec<Entity> = wheel_pos
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

        let body = commands
            .spawn((
                Mesh3d(assets.mesh_car_body.clone()),
                MeshMaterial3d(assets.mat_car_colors[color_idx].clone()),
                Transform::from_xyz(0.0, 0.7, 0.0),
            ))
            .id();
        let cabin = commands
            .spawn((
                Mesh3d(assets.mesh_car_cabin.clone()),
                MeshMaterial3d(assets.mat_car_colors[color_idx].clone()),
                Transform::from_xyz(0.0, 1.3, -0.1),
            ))
            .id();
        let windshield = commands
            .spawn((
                Mesh3d(assets.mesh_car_windshield.clone()),
                MeshMaterial3d(assets.mat_windshield.clone()),
                Transform::from_xyz(0.0, 1.3, 0.95)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::PI / 2.0 + 0.5)),
            ))
            .id();
        let headlights: Vec<Entity> = [-0.6_f32, 0.6]
            .iter()
            .map(|x| {
                commands
                    .spawn((
                        Mesh3d(assets.mesh_car_headlight.clone()),
                        MeshMaterial3d(assets.mat_headlight.clone()),
                        Transform::from_xyz(*x, 0.7, 2.1),
                    ))
                    .id()
            })
            .collect();
        let taillights: Vec<Entity> = [-0.6_f32, 0.6]
            .iter()
            .map(|x| {
                commands
                    .spawn((
                        Mesh3d(assets.mesh_car_headlight.clone()),
                        MeshMaterial3d(assets.mat_taillight.clone()),
                        Transform::from_xyz(*x, 0.7, -2.1),
                    ))
                    .id()
            })
            .collect();

        let car_entity = commands
            .spawn((
                Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(rot_y)),
                Visibility::Visible,
                Car {
                    axis,
                    dir,
                    speed: 6.0 + rng.gen::<f32>() * 6.0,
                    color_idx,
                },
                CarWheels {
                    fl: wheel_entities[0],
                    fr: wheel_entities[1],
                    rl: wheel_entities[2],
                    rr: wheel_entities[3],
                    spin: 0.0,
                },
            ))
            .id();

        let mut all_children = wheel_entities.clone();
        all_children.push(body);
        all_children.push(cabin);
        all_children.push(windshield);
        all_children.extend(headlights);
        all_children.extend(taillights);
        commands.entity(car_entity).add_children(&all_children);
    }
}

pub fn update_ai_cars(
    time: Res<Time>,
    keys: Res<KeysPressed>,
    mut game_state: ResMut<GameState>,
    mut cars: Query<(Entity, &mut Car, &mut Transform, &mut CarWheels)>,
    mut wheel_transforms: Query<&mut Transform, Without<Car>>,
    player_q: Query<&Transform, With<crate::player::Player>>,
    buildings: Query<&Building>,
) {
    let mut rng = rand::thread_rng();
    let dt = time.delta_secs();
    let player_pos = player_q
        .get_single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    for (entity, mut car, mut transform, mut wheels) in cars.iter_mut() {
        let driven = game_state.in_vehicle == Some(entity);

        if driven {
            // ----- Player driving -----
            let max_speed = 28.0_f32;
            if keys.w {
                wheels.spin += 18.0 * dt;
            } else if keys.s {
                wheels.spin -= 18.0 * dt;
            } else {
                wheels.spin *= (1.0 - 1.4 * dt).max(0.0);
                if wheels.spin.abs() < 0.05 {
                    wheels.spin = 0.0;
                }
            }
            wheels.spin = wheels.spin.clamp(-10.0, max_speed);

            let mut steer = 0.0;
            if keys.a {
                steer -= 1.0;
            }
            if keys.d {
                steer += 1.0;
            }
            let speed_factor = (wheels.spin.abs() / 6.0).min(1.0);
            let yaw_delta = steer * 1.6 * dt * speed_factor * wheels.spin.signum();
            let new_yaw = transform.rotation.to_euler(EulerRot::YXZ).0 + yaw_delta;
            transform.rotation = Quat::from_rotation_y(new_yaw);

            let fwd = transform.rotation * Vec3::new(0.0, 0.0, 1.0);
            let next = transform.translation + fwd * wheels.spin * dt;

            if collides_buildings_at(next.x, next.z, 1.5, &buildings) {
                wheels.spin *= -0.3;
            } else {
                transform.translation = next;
            }
            transform.translation.y = 0.0;

            let lim = CITY_HALF + 8.0;
            transform.translation.x = transform.translation.x.clamp(-lim, lim);
            transform.translation.z = transform.translation.z.clamp(-lim, lim);

            // Update HUD speedometer (m/s → km/h)
            game_state.last_speed_kmh = (wheels.spin.abs() * 3.6).round();

            // Apply wheel spin visually.
            // Read `wheels.spin * dt` before the mutable borrow of `wheels`.
            let spin_delta = wheels.spin * dt;
            apply_wheel_spin(&mut wheels, &mut wheel_transforms, spin_delta);

            car.speed = wheels.spin;
            continue;
        }

        // ----- AI car -----
        let mut speed_scale = 1.0;
        if game_state.in_vehicle.is_none() {
            let fwd = transform.rotation * Vec3::new(0.0, 0.0, 1.0);
            let ahead = transform.translation + fwd * 3.0;
            if player_pos.distance(ahead) < 1.5 {
                speed_scale = 0.2;
            }
        }

        let delta = car.dir * car.speed * speed_scale * dt;
        match car.axis {
            Axis::X => {
                transform.translation.x += delta;
                transform.rotation = Quat::from_rotation_y(if car.dir > 0.0 {
                    std::f32::consts::PI / 2.0
                } else {
                    -std::f32::consts::PI / 2.0
                });
                if transform.translation.x > CITY_HALF + 5.0 {
                    transform.translation.x = -CITY_HALF - 5.0;
                }
                if transform.translation.x < -CITY_HALF - 5.0 {
                    transform.translation.x = CITY_HALF + 5.0;
                }
            }
            Axis::Z => {
                transform.translation.z += delta;
                transform.rotation = Quat::from_rotation_y(if car.dir > 0.0 {
                    0.0
                } else {
                    std::f32::consts::PI
                });
                if transform.translation.z > CITY_HALF + 5.0 {
                    transform.translation.z = -CITY_HALF - 5.0;
                }
                if transform.translation.z < -CITY_HALF - 5.0 {
                    transform.translation.z = CITY_HALF + 5.0;
                }
            }
        }

        apply_wheel_spin(&mut wheels, &mut wheel_transforms, delta);

        // Random turn at intersection
        if rng.gen_bool(0.006) {
            for i in 0..=GRID {
                let c1 = -CITY_HALF + i as f32 * STEP;
                match car.axis {
                    Axis::X => {
                        if (transform.translation.x - c1).abs() < 1.5 && rng.gen_bool(0.5) {
                            car.axis = Axis::Z;
                            transform.translation.x = c1;
                            car.dir = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
                            break;
                        }
                    }
                    Axis::Z => {
                        if (transform.translation.z - c1).abs() < 1.5 && rng.gen_bool(0.5) {
                            car.axis = Axis::X;
                            transform.translation.z = c1;
                            car.dir = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// Update each wheel's local rotation = base_z_rotation * spin_y_rotation.
/// Note: we copy Entity IDs out first to avoid the simultaneous
/// `&mut wheels` (for `wheels.spin`) and `&wheels.fl/fr/...` borrow.
fn apply_wheel_spin(
    wheels: &mut CarWheels,
    transforms: &mut Query<&mut Transform, Without<Car>>,
    delta: f32,
) {
    wheels.spin += delta * 2.0;
    let spin_value = wheels.spin;
    let ents = [wheels.fl, wheels.fr, wheels.rl, wheels.rr];

    let base = Quat::from_rotation_z(std::f32::consts::PI / 2.0);
    let spin = Quat::from_rotation_y(spin_value);
    let final_rot = base * spin;
    for e in ents {
        if let Ok(mut t) = transforms.get_mut(e) {
            t.rotation = final_rot;
        }
    }
}

fn collides_buildings_at(x: f32, z: f32, radius: f32, buildings: &Query<&Building>) -> bool {
    for b in buildings.iter() {
        let dx = (x - b.cx).abs();
        let dz = (z - b.cz).abs();
        if dx < b.w / 2.0 + radius && dz < b.d / 2.0 + radius {
            return true;
        }
    }
    false
}
