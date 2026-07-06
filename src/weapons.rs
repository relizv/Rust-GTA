//! Weapons & combat: pistol (hitscan + tracers), ped/cop health, corpses,
//! explosions.
//!
//! Keys: **1** — fists, **2** — pistol. **LMB** — fire (hold for rapid fire),
//! **RMB (hold)** — aim: the camera zooms in and spread tightens.
//!
//! All tunables live in `config.rs` (`WeaponConfig`).

use bevy::input::mouse::MouseButton;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy::transform::components::GlobalTransform;
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::car::collides_buildings_at;
use crate::city::Building;
use crate::config::GameConfig;
use crate::pedestrian::Pedestrian;
use crate::player::{Player, PlayerLimb, PlayerLimbs, PlayerState};
use crate::police::{PoliceCar, PoliceHealth};
use crate::resources::{GameAssets, GameState, InputState};

/// Current weapon + ammo / reload / aim state.
#[derive(Resource)]
pub struct WeaponState {
    pub pistol_equipped: bool,
    pub ammo: u32,
    pub reloading: bool,
    pub reload_timer: f32,
    pub fire_cooldown: f32,
    pub aiming: bool,
}

impl Default for WeaponState {
    fn default() -> Self {
        Self {
            pistol_equipped: false,
            ammo: 12,
            reloading: false,
            reload_timer: 0.0,
            fire_cooldown: 0.0,
            aiming: false,
        }
    }
}

/// Marker for the pistol mesh in the player's right hand (child of `arm_r`).
#[derive(Component)]
pub struct PistolMesh;

/// A dead ped lying on the ground. Despawned after a timeout.
#[derive(Component)]
pub struct Corpse {
    pub timer: f32,
}

/// Bullet tracer line, despawned after `life` seconds.
#[derive(Component)]
pub struct Tracer {
    pub life: f32,
}

/// Expanding orange flash where a cop car was destroyed.
#[derive(Component)]
pub struct Explosion {
    pub life: f32,
}

/// Weapon switching (1/2), aim state, cooldown + reload timers.
pub fn weapon_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    config: Res<GameConfig>,
    input_state: Res<InputState>,
    mut game_state: ResMut<GameState>,
    mut weapon: ResMut<WeaponState>,
) {
    // Timers tick on the virtual clock, so the pause menu freezes them too.
    let dt = time.delta_secs();
    if weapon.fire_cooldown > 0.0 {
        weapon.fire_cooldown -= dt;
    }
    if weapon.reloading {
        weapon.reload_timer -= dt;
        if weapon.reload_timer <= 0.0 {
            weapon.reloading = false;
            weapon.ammo = config.weapons.magazine;
        }
    }

    if !game_state.started || !input_state.cursor_locked {
        weapon.aiming = false;
        return;
    }

    if keys.just_pressed(KeyCode::Digit1) && weapon.pistol_equipped {
        weapon.pistol_equipped = false;
        game_state.show_toast("👊 Кулаки");
    }
    if keys.just_pressed(KeyCode::Digit2) && !weapon.pistol_equipped {
        weapon.pistol_equipped = true;
        game_state.show_toast("🔫 Пистолет");
    }

    weapon.aiming = weapon.pistol_equipped
        && game_state.in_vehicle.is_none()
        && mouse.pressed(MouseButton::Right);
}

/// LMB fires the pistol: instant hitscan ray from the crosshair with a
/// glowing tracer. Runs after `update_peds`, so `ped.pos` is fresh.
#[allow(clippy::too_many_arguments)]
pub fn player_shoot(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    config: Res<GameConfig>,
    assets: Res<GameAssets>,
    input_state: Res<InputState>,
    mut game_state: ResMut<GameState>,
    mut weapon: ResMut<WeaponState>,
    // Camera pose = exactly what the crosshair looks at. GlobalTransform is
    // one frame stale, which is invisible at any playable FPS.
    cam_q: Query<&GlobalTransform, With<Camera3d>>,
    player_q: Query<&GlobalTransform, With<Player>>,
    // Corpse posing needs `&mut Transform`; there is no other Transform
    // access in this system, so no B0001. Hit testing uses `ped.pos`.
    mut peds: Query<(Entity, &mut Pedestrian, &mut Transform)>,
    mut cops: Query<(Entity, &GlobalTransform, &mut PoliceHealth), With<PoliceCar>>,
    buildings: Query<&Building>,
) {
    if !game_state.started
        || !input_state.cursor_locked
        || !weapon.pistol_equipped
        || game_state.in_vehicle.is_some()
        || weapon.reloading
        || weapon.fire_cooldown > 0.0
        || !mouse.pressed(MouseButton::Left)
    {
        return;
    }
    if weapon.ammo == 0 {
        // Shouldn't happen (reload starts right after the last shot), but be safe.
        weapon.reloading = true;
        weapon.reload_timer = config.weapons.reload_secs;
        return;
    }
    let Ok(cam_gt) = cam_q.get_single() else {
        return;
    };
    let Ok(player_gt) = player_q.get_single() else {
        return;
    };

    // --- Consume ammo / start auto-reload ---
    weapon.ammo -= 1;
    weapon.fire_cooldown = config.weapons.fire_cooldown;
    if weapon.ammo == 0 {
        weapon.reloading = true;
        weapon.reload_timer = config.weapons.reload_secs;
    }

    // --- Build the shot ray ---
    let cam_pos = cam_gt.translation();
    let mut dir = cam_gt.rotation() * -Vec3::Z;
    let spread = if weapon.aiming {
        config.weapons.aim_spread_deg
    } else {
        config.weapons.hip_spread_deg
    }
    .to_radians();
    if spread > 0.0 {
        let mut rng = rand::thread_rng();
        dir = Quat::from_euler(
            EulerRot::YXZ,
            rng.gen_range(-spread..=spread),
            rng.gen_range(-spread..=spread),
            0.0,
        ) * dir;
    }
    let player_pos = player_gt.translation();
    // Start at the player's depth along the ray so nothing BETWEEN the camera
    // and the player (i.e. behind the character) can be hit.
    let t0 = (player_pos + Vec3::Y * 1.2 - cam_pos).dot(dir).max(0.0);
    let origin = cam_pos + dir * t0;
    let range = config.weapons.pistol_range;

    // --- Closest target along the ray ---
    enum Target {
        Ped(Entity),
        Cop(Entity),
        Wall,
    }
    let mut best: Option<(f32, Target)> = None;

    for (e, ped, _) in peds.iter() {
        let center = ped.pos + Vec3::Y * 0.7;
        let t = (center - origin).dot(dir);
        if t > 0.0
            && t < range
            && (origin + dir * t).distance(center) < 0.75
            && best.as_ref().map_or(true, |(bt, _)| t < *bt)
        {
            best = Some((t, Target::Ped(e)));
        }
    }
    for (e, gt, _) in cops.iter() {
        let center = gt.translation() + Vec3::Y * 0.8;
        let t = (center - origin).dot(dir);
        if t > 0.0
            && t < range
            && (origin + dir * t).distance(center) < 1.8
            && best.as_ref().map_or(true, |(bt, _)| t < *bt)
        {
            best = Some((t, Target::Cop(e)));
        }
    }

    // --- Buildings block bullets (cheap ray-march over footprints) ---
    let max_t = best.as_ref().map_or(range, |(bt, _)| *bt);
    let mut t = 1.0;
    while t < max_t {
        let p = origin + dir * t;
        if p.y < 8.0 && collides_buildings_at(p.x, p.z, 0.1, &buildings) {
            best = Some((t, Target::Wall));
            break;
        }
        t += 1.2;
    }

    let (hit_t, target) = match best {
        Some((t, tgt)) => (t, Some(tgt)),
        None => (range, None),
    };
    let hit_point = origin + dir * hit_t;

    // --- Apply damage ---
    match target {
        Some(Target::Ped(e)) => {
            if let Ok((entity, mut ped, mut tf)) = peds.get_mut(e) {
                ped.hp -= config.weapons.pistol_damage;
                if ped.hp <= 0.0 {
                    // Down: stop simulating this ped (remove `Pedestrian`)
                    // and leave a corpse lying on the ground for a while.
                    let side = if rand::random::<bool>() { 1.0 } else { -1.0 };
                    tf.rotation *= Quat::from_rotation_x(FRAC_PI_2 * side);
                    tf.translation.y = 0.2;
                    commands
                        .entity(entity)
                        .remove::<Pedestrian>()
                        .insert(Corpse {
                            timer: config.weapons.corpse_despawn_secs,
                        });
                    game_state.cash += config.weapons.cash_per_kill;
                    game_state.add_wanted(1);
                    game_state.show_toast(format!("💀 +${}", config.weapons.cash_per_kill));
                } else {
                    // Stagger the survivor a bit.
                    ped.knockback += Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero() * 0.6;
                }
            }
        }
        Some(Target::Cop(e)) => {
            if let Ok((entity, gt, mut health)) = cops.get_mut(e) {
                health.hp -= config.weapons.pistol_damage;
                if health.hp <= 0.0 {
                    commands.spawn((
                        Mesh3d(assets.mesh_unit_box.clone()),
                        MeshMaterial3d(assets.mat_explosion.clone()),
                        Transform::from_translation(gt.translation() + Vec3::Y)
                            .with_scale(Vec3::splat(1.2)),
                        NotShadowCaster,
                        Explosion { life: 0.35 },
                    ));
                    commands.entity(entity).despawn_recursive();
                    game_state.add_wanted(1);
                    game_state.show_toast("💥 Патруль уничтожен!");
                }
            }
        }
        _ => {}
    }

    // --- Tracer ---
    let muzzle = player_pos + Vec3::Y * 1.15 + dir * 0.6;
    let len = (hit_point - muzzle).length().max(0.1);
    let mid = (muzzle + hit_point) / 2.0;
    commands.spawn((
        Mesh3d(assets.mesh_unit_box.clone()),
        MeshMaterial3d(assets.mat_tracer.clone()),
        Transform::from_translation(mid)
            .looking_at(hit_point, Vec3::Y)
            .with_scale(Vec3::new(0.035, 0.035, len)),
        NotShadowCaster,
        Tracer { life: 0.07 },
    ));
}

/// Show/hide the pistol in the hand, hold the "gun ready" arm pose, and turn
/// the player toward the camera while aiming. Runs after `update_player`
/// (which animates limbs), so our arm pose wins the frame.
pub fn update_pistol_visuals(
    weapon: Res<WeaponState>,
    input_state: Res<InputState>,
    game_state: Res<GameState>,
    mut pistol_q: Query<&mut Visibility, With<PistolMesh>>,
    mut player_q: Query<
        (&mut Transform, &mut PlayerState, &PlayerLimbs),
        (With<Player>, Without<PlayerLimb>),
    >,
    mut limb_q: Query<&mut Transform, With<PlayerLimb>>,
) {
    let on_foot = game_state.in_vehicle.is_none();
    let show = weapon.pistol_equipped && on_foot && game_state.started;
    for mut vis in pistol_q.iter_mut() {
        let target = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != target {
            *vis = target;
        }
    }

    if !game_state.started {
        return;
    }
    let Ok((mut tf, mut state, limbs)) = player_q.get_single_mut() else {
        return;
    };

    if weapon.aiming && on_foot && input_state.cursor_locked {
        // Face where the camera looks, raise the shooting arm.
        state.yaw = input_state.yaw + PI;
        tf.rotation = Quat::from_rotation_y(state.yaw);
        if let Ok(mut arm) = limb_q.get_mut(limbs.arm_r) {
            arm.rotation = Quat::from_rotation_x(-FRAC_PI_2 + (input_state.pitch - 0.35) * 0.8);
        }
    } else if show {
        // Relaxed "gun ready" pose for the right arm.
        if let Ok(mut arm) = limb_q.get_mut(limbs.arm_r) {
            arm.rotation = Quat::from_rotation_x(-0.55);
        }
    }
}

/// Tick tracers, explosions, and corpses. Uses the virtual clock, so the
/// pause menu freezes all of it.
pub fn update_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut tracers: Query<(Entity, &mut Tracer)>,
    mut explosions: Query<(Entity, &mut Explosion, &mut Transform)>,
    mut corpses: Query<(Entity, &mut Corpse)>,
) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }
    for (e, mut tr) in tracers.iter_mut() {
        tr.life -= dt;
        if tr.life <= 0.0 {
            commands.entity(e).despawn_recursive();
        }
    }
    for (e, mut ex, mut tf) in explosions.iter_mut() {
        ex.life -= dt;
        tf.scale = Vec3::splat(1.2 + (0.35 - ex.life) * 10.0);
        tf.rotate_y(dt * 6.0);
        if ex.life <= 0.0 {
            commands.entity(e).despawn_recursive();
        }
    }
    for (e, mut c) in corpses.iter_mut() {
        c.timer -= dt;
        if c.timer <= 0.0 {
            commands.entity(e).despawn_recursive();
        }
    }
}
