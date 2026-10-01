//! Cars: spawn, AI navigation along road grid, player driving.

use bevy::prelude::*;
use rand::Rng;

use crate::city::Building;
use crate::config::{DrivingConfig, GameConfig};
use crate::resources::{GameAssets, GameState, KeysPressed, CITY_HALF, GRID, STEP};

#[derive(Component)]
pub struct Car {
    pub axis: Axis,
    pub dir: f32,
    /// Cruising speed of the AI driver, m/s. Never touched by the player.
    pub speed: f32,
    /// Signed forward speed while a human drives the car, m/s
    /// (negative = reversing). Independent of the wheel visuals.
    pub velocity: f32,
    /// Set once a human has driven the car: from then on the AI never
    /// steers it again and an abandoned car coasts to a stop and stays put.
    pub parked: bool,
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
    /// Visual rolling angle of the wheels, radians (kept in `0..TAU`).
    /// Purely cosmetic — it must never feed back into the car's speed.
    pub angle: f32,
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
                    velocity: 0.0,
                    parked: false,
                    color_idx,
                },
                CarWheels {
                    fl: wheel_entities[0],
                    fr: wheel_entities[1],
                    rl: wheel_entities[2],
                    rr: wheel_entities[3],
                    angle: 0.0,
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
    config: Res<GameConfig>,
    keys: Res<KeysPressed>,
    mut game_state: ResMut<GameState>,
    mut cars: Query<(Entity, &mut Car, &mut Transform, &mut CarWheels)>,
    mut wheel_transforms: Query<&mut Transform, Without<Car>>,
    // Use GlobalTransform (not Transform) for the player so this read does not
    // conflict with `update_player`'s `&mut Transform` write on the player —
    // Bevy 0.15 would otherwise panic with B0001.
    player_q: Query<&GlobalTransform, With<crate::player::Player>>,
    buildings: Query<&Building>,
) {
    let mut rng = rand::thread_rng();
    let dt = time.delta_secs();
    // Paused (virtual clock frozen): the AI's random turns are per-frame, not
    // dt-scaled, so bail out to keep cars perfectly still in the pause menu.
    if dt == 0.0 {
        return;
    }
    let player_pos = player_q
        .get_single()
        .map(|gt| gt.translation())
        .unwrap_or(Vec3::ZERO);

    for (entity, mut car, mut transform, mut wheels) in cars.iter_mut() {
        let driven = game_state.in_vehicle == Some(entity);

        if driven {
            // ----- Player driving -----
            if !car.parked {
                // Carjacking: keep the speed the AI driver was cruising at
                // and take the car off the AI's hands for good.
                car.velocity = car.speed;
                car.parked = true;
            }
            let throttle = f32::from(keys.w) - f32::from(keys.s);
            car.velocity = step_drive_speed(car.velocity, throttle, dt, &config.driving);

            let mut steer = 0.0;
            if keys.a {
                steer -= 1.0;
            }
            if keys.d {
                steer += 1.0;
            }
            let speed_factor = (car.velocity.abs() / 6.0).min(1.0);
            let yaw_delta =
                steer * config.driving.steer_rate * dt * speed_factor * car.velocity.signum();
            let new_yaw = transform.rotation.to_euler(EulerRot::YXZ).0 + yaw_delta;
            transform.rotation = Quat::from_rotation_y(new_yaw);

            let fwd = transform.rotation * Vec3::new(0.0, 0.0, 1.0);
            let next = transform.translation + fwd * car.velocity * dt;

            if collides_buildings_at(next.x, next.z, 1.5, &buildings) {
                // Bounce off the wall; drag then brings the car to a stop.
                car.velocity *= -0.3;
            } else {
                transform.translation = next;
            }
            transform.translation.y = 0.0;

            let lim = CITY_HALF + 8.0;
            transform.translation.x = transform.translation.x.clamp(-lim, lim);
            transform.translation.z = transform.translation.z.clamp(-lim, lim);

            // Update HUD speedometer (m/s → km/h)
            game_state.last_speed_kmh = (car.velocity.abs() * 3.6).round();

            roll_wheels(&mut wheels, &mut wheel_transforms, car.velocity * dt);
            continue;
        }

        // ----- Abandoned car: coast to a stop and stay where it is -----
        if car.parked {
            car.velocity = step_drive_speed(car.velocity, 0.0, dt, &config.driving);
            if car.velocity != 0.0 {
                let fwd = transform.rotation * Vec3::Z;
                let next = transform.translation + fwd * car.velocity * dt;
                if collides_buildings_at(next.x, next.z, 1.5, &buildings) {
                    car.velocity = 0.0;
                } else {
                    transform.translation = next;
                    roll_wheels(&mut wheels, &mut wheel_transforms, car.velocity * dt);
                }
            }
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

        roll_wheels(
            &mut wheels,
            &mut wheel_transforms,
            car.speed * speed_scale * dt,
        );

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

/// Radius of the wheel mesh (see `Cylinder::new(0.35, ..)` in `resources.rs`).
const WHEEL_RADIUS: f32 = 0.35;

/// One integration step of a player-driven car's forward speed.
///
/// * `throttle`: `+1` gas, `-1` brake/reverse, `0` coast.
/// * Pushing the pedal against the direction of travel brakes harder than
///   accelerating (`brake` vs `accel`), and carries on into reverse.
/// * With no pedal the car loses speed to `drag` and finally stops.
pub fn step_drive_speed(speed: f32, throttle: f32, dt: f32, cfg: &DrivingConfig) -> f32 {
    let mut v = speed;
    if throttle != 0.0 {
        let braking = v * throttle < 0.0;
        let rate = if braking { cfg.brake } else { cfg.accel };
        v += throttle * rate * dt;
    } else {
        v *= (1.0 - cfg.drag * dt).max(0.0);
        if v.abs() < 0.3 {
            v = 0.0;
        }
    }
    v.clamp(-cfg.reverse_speed, cfg.max_speed)
}

/// Roll all four wheels by `distance` metres travelled along the car's
/// heading. Keeps `CarWheels::angle` separate from the car's speed.
fn roll_wheels(
    wheels: &mut CarWheels,
    transforms: &mut Query<&mut Transform, Without<Car>>,
    distance: f32,
) {
    // The wheel mesh is a cylinder rotated onto its side; its local Y axis
    // points along world -X, so rolling forward = decreasing angle.
    wheels.angle = (wheels.angle - distance / WHEEL_RADIUS).rem_euclid(std::f32::consts::TAU);
    let base = Quat::from_rotation_z(std::f32::consts::PI / 2.0);
    let final_rot = base * Quat::from_rotation_y(wheels.angle);
    for e in [wheels.fl, wheels.fr, wheels.rl, wheels.rr] {
        if let Ok(mut t) = transforms.get_mut(e) {
            t.rotation = final_rot;
        }
    }
}

pub fn collides_buildings_at(x: f32, z: f32, radius: f32, buildings: &Query<&Building>) -> bool {
    for b in buildings.iter() {
        let dx = (x - b.cx).abs();
        let dz = (z - b.cz).abs();
        if dx < b.w / 2.0 + radius && dz < b.d / 2.0 + radius {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn run(mut v: f32, throttle: f32, secs: f32) -> f32 {
        let cfg = DrivingConfig::default();
        for _ in 0..(secs / DT) as usize {
            v = step_drive_speed(v, throttle, DT, &cfg);
        }
        v
    }

    #[test]
    fn gas_reaches_but_never_exceeds_max_speed() {
        let cfg = DrivingConfig::default();
        assert_eq!(run(0.0, 1.0, 10.0), cfg.max_speed);
    }

    #[test]
    fn releasing_gas_slows_the_car_down_to_a_stop() {
        let cfg = DrivingConfig::default();
        let mut v = cfg.max_speed;
        let mut prev = v;
        for _ in 0..(15.0 / DT) as usize {
            v = step_drive_speed(v, 0.0, DT, &cfg);
            assert!(v <= prev, "speed must never grow while coasting");
            prev = v;
        }
        assert_eq!(v, 0.0);
    }

    #[test]
    fn braking_stops_faster_than_coasting() {
        let cfg = DrivingConfig::default();
        let braked = run(cfg.max_speed, -1.0, 0.5);
        let coasted = run(cfg.max_speed, 0.0, 0.5);
        assert!(braked < coasted, "{braked} !< {coasted}");
    }

    #[test]
    fn holding_brake_ends_in_reverse_capped_at_reverse_speed() {
        let cfg = DrivingConfig::default();
        assert_eq!(run(cfg.max_speed, -1.0, 10.0), -cfg.reverse_speed);
    }

    #[test]
    fn standing_car_stays_put() {
        assert_eq!(run(0.0, 0.0, 5.0), 0.0);
    }
}
