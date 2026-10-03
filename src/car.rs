//! Cars: spawn, AI navigation along road grid, player driving.

use bevy::prelude::*;
use rand::Rng;

use crate::city::Building;
use crate::config::{DrivingConfig, GameConfig};
use crate::resources::{GameAssets, GameState, KeysPressed, CITY_HALF, GRID, STEP};
use crate::util::{damp, lerp, lerp_angle};

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
    /// Seconds left before the AI may turn again. Stops a car from making a
    /// second turn inside the same intersection.
    pub turn_cooldown: f32,
    pub color_idx: usize,
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub enum Axis {
    X,
    Z,
}

/// Distance of a lane's centre line from the road's centre line, m.
const LANE_OFFSET: f32 = 2.0;
/// Chance that an AI car turns at an intersection it drives through.
const TURN_CHANCE: f64 = 0.35;
/// After a turn the AI won't turn again for this long, s (≈ one intersection).
const TURN_COOLDOWN_SECS: f32 = 1.0;
/// How fast a car settles into its lane after a turn, 1/s.
const LANE_SETTLE_RATE: f32 = 8.0;
/// How fast a car swings its nose to the new heading after a turn, 1/s.
const HEADING_RATE: f32 = 10.0;

/// Lateral offset (m) from the road centre of the lane used when driving in
/// direction `dir` (±1) along `axis`. Right-hand traffic: opposing cars get
/// opposite offsets, so they pass each other instead of overlapping.
pub fn lane_offset(axis: Axis, dir: f32) -> f32 {
    match axis {
        Axis::X => dir * LANE_OFFSET,
        Axis::Z => -dir * LANE_OFFSET,
    }
}

/// Yaw that makes a car face direction `dir` (±1) along `axis`.
pub fn heading_yaw(axis: Axis, dir: f32) -> f32 {
    match axis {
        Axis::X => dir * std::f32::consts::FRAC_PI_2,
        Axis::Z => {
            if dir > 0.0 {
                0.0
            } else {
                std::f32::consts::PI
            }
        }
    }
}

/// Coordinate of the road centre line closest to `v`.
fn nearest_road_center(v: f32) -> f32 {
    let i = ((v + CITY_HALF) / STEP).round().clamp(0.0, GRID as f32);
    -CITY_HALF + i * STEP
}

/// Did moving from `old` to `new` along a road pass over a crossing road's
/// centre line (i.e. through an intersection)?
fn crossed_road_center(old: f32, new: f32) -> bool {
    old != new
        && (0..=GRID).any(|i| {
            let c = -CITY_HALF + i as f32 * STEP;
            (old - c) * (new - c) <= 0.0
        })
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
        let road = rng.gen_range(0..=GRID);
        let coord = -CITY_HALF + road as f32 * STEP;
        let along = -CITY_HALF + rng.gen::<f32>() * (CITY_HALF * 2.0);
        let dir = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
        let lane = coord + lane_offset(axis, dir);

        let pos = match axis {
            Axis::X => Vec3::new(along, 0.0, lane),
            Axis::Z => Vec3::new(lane, 0.0, along),
        };
        let rot_y = heading_yaw(axis, dir);

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
                    turn_cooldown: 0.0,
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

        car.turn_cooldown = (car.turn_cooldown - dt).max(0.0);

        // Drive along the road.
        let step = car.dir * car.speed * speed_scale * dt;
        let (old_along, new_along) = match car.axis {
            Axis::X => {
                let old = transform.translation.x;
                transform.translation.x += step;
                (old, transform.translation.x)
            }
            Axis::Z => {
                let old = transform.translation.z;
                transform.translation.z += step;
                (old, transform.translation.z)
            }
        };

        // Maybe turn: decided once per intersection driven through (not once
        // per frame), so the behaviour doesn't depend on the frame rate.
        if car.turn_cooldown == 0.0
            && crossed_road_center(old_along, new_along)
            && rng.gen_bool(TURN_CHANCE)
        {
            car.axis = match car.axis {
                Axis::X => Axis::Z,
                Axis::Z => Axis::X,
            };
            car.dir = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
            car.turn_cooldown = TURN_COOLDOWN_SECS;
        }

        // Keep to the right-hand lane of the current road: after a turn this
        // eases the car sideways into its new lane instead of snapping it to
        // the road centre (where it would overlap oncoming traffic).
        let cross = match car.axis {
            Axis::X => &mut transform.translation.z,
            Axis::Z => &mut transform.translation.x,
        };
        let lane_target = nearest_road_center(*cross) + lane_offset(car.axis, car.dir);
        *cross = lerp(*cross, lane_target, damp(LANE_SETTLE_RATE, dt));

        // Swing the nose to the heading of the current road.
        let yaw_now = transform.rotation.to_euler(EulerRot::YXZ).0;
        let yaw = lerp_angle(
            yaw_now,
            heading_yaw(car.axis, car.dir),
            damp(HEADING_RATE, dt),
        );
        transform.rotation = Quat::from_rotation_y(yaw);

        // Leaving the map on one side re-enters on the opposite one.
        let wrap = CITY_HALF + 5.0;
        let pos = &mut transform.translation;
        for v in [&mut pos.x, &mut pos.z] {
            if *v > wrap {
                *v = -wrap;
            } else if *v < -wrap {
                *v = wrap;
            }
        }

        roll_wheels(
            &mut wheels,
            &mut wheel_transforms,
            car.speed * speed_scale * dt,
        );
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
    fn opposing_traffic_uses_different_lanes() {
        for axis in [Axis::X, Axis::Z] {
            assert_eq!(lane_offset(axis, 1.0), -lane_offset(axis, -1.0));
            assert_ne!(lane_offset(axis, 1.0), 0.0);
        }
    }

    #[test]
    fn lanes_keep_right() {
        // Right of a car heading `fwd` (Y up, right-handed) is `fwd × Y`.
        for (axis, dir, fwd) in [
            (Axis::X, 1.0, Vec3::X),
            (Axis::X, -1.0, -Vec3::X),
            (Axis::Z, 1.0, Vec3::Z),
            (Axis::Z, -1.0, -Vec3::Z),
        ] {
            let right = fwd.cross(Vec3::Y);
            let off = match axis {
                Axis::X => Vec3::Z,
                Axis::Z => Vec3::X,
            } * lane_offset(axis, dir);
            assert!(off.dot(right) > 0.0, "{axis:?} {dir}");
        }
    }

    #[test]
    fn heading_yaw_points_along_travel_direction() {
        for (axis, dir, want) in [
            (Axis::X, 1.0, Vec3::X),
            (Axis::X, -1.0, -Vec3::X),
            (Axis::Z, 1.0, Vec3::Z),
            (Axis::Z, -1.0, -Vec3::Z),
        ] {
            let fwd = Quat::from_rotation_y(heading_yaw(axis, dir)) * Vec3::Z;
            assert!((fwd - want).length() < 1e-5, "{axis:?} {dir}: {fwd}");
        }
    }

    #[test]
    fn intersection_crossing_is_detected_once() {
        let c = -CITY_HALF + STEP; // a road centre line
        assert!(crossed_road_center(c - 0.1, c + 0.1));
        assert!(!crossed_road_center(c + 0.1, c + 0.3));
        assert!(!crossed_road_center(c, c));
    }

    #[test]
    fn nearest_road_center_ignores_lane_offset() {
        let c = -CITY_HALF + 2.0 * STEP;
        assert_eq!(nearest_road_center(c + LANE_OFFSET), c);
        assert_eq!(nearest_road_center(c - LANE_OFFSET), c);
    }

    /// Runs the real `update_ai_cars` system headlessly for a minute of game
    /// time (which also proves its queries don't conflict at runtime).
    #[test]
    fn ai_traffic_keeps_to_its_lane_and_actually_turns() {
        use bevy::time::TimeUpdateStrategy;
        use std::time::Duration;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                DT,
            )))
            .insert_resource(GameConfig::default())
            .insert_resource(GameState::default())
            .insert_resource(KeysPressed::default())
            .add_systems(Update, update_ai_cars);

        let mut cars = Vec::new();
        for k in 0..14usize {
            let axis = if k % 2 == 0 { Axis::X } else { Axis::Z };
            let dir = if k % 4 < 2 { 1.0 } else { -1.0 };
            let road = -CITY_HALF + (k % (GRID + 1)) as f32 * STEP;
            let lane = road + lane_offset(axis, dir);
            let pos = match axis {
                Axis::X => Vec3::new(-60.0 + k as f32 * 7.0, 0.0, lane),
                Axis::Z => Vec3::new(lane, 0.0, -60.0 + k as f32 * 7.0),
            };
            let w: [Entity; 4] =
                std::array::from_fn(|_| app.world_mut().spawn(Transform::default()).id());
            let e = app
                .world_mut()
                .spawn((
                    Transform::from_translation(pos)
                        .with_rotation(Quat::from_rotation_y(heading_yaw(axis, dir))),
                    Car {
                        axis,
                        dir,
                        speed: 6.0 + k as f32 * 0.5,
                        velocity: 0.0,
                        parked: false,
                        turn_cooldown: 0.0,
                        color_idx: 0,
                    },
                    CarWheels {
                        fl: w[0],
                        fr: w[1],
                        rl: w[2],
                        rr: w[3],
                        angle: 0.0,
                    },
                ))
                .id();
            cars.push((e, axis));
        }

        for _ in 0..(60.0 / DT) as usize {
            app.update();
        }

        let mut turned = 0;
        for (e, start_axis) in cars {
            let car = app.world().get::<Car>(e).unwrap();
            let pos = app.world().get::<Transform>(e).unwrap().translation;
            if car.axis != start_axis {
                turned += 1;
            }
            if car.turn_cooldown == 0.0 {
                let cross = if car.axis == Axis::X { pos.z } else { pos.x };
                let want = nearest_road_center(cross) + lane_offset(car.axis, car.dir);
                assert!(
                    (cross - want).abs() < 0.3,
                    "car left its lane: cross={cross} want={want}"
                );
            }
        }
        assert!(turned > 0, "no car turned in a whole minute");
    }

    #[test]
    fn standing_car_stays_put() {
        assert_eq!(run(0.0, 0.0, 5.0), 0.0);
    }
}
