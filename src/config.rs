//! Central game configuration.
//!
//! Every gameplay/graphics tunable lives here, grouped by topic. `GameConfig`
//! is inserted as a resource at startup, so:
//! - tweak the defaults below and recompile, or
//! - later, an in-game settings menu can simply mutate `ResMut<GameConfig>`.
//!
//! NOTE: a few graphics values (window size, vsync, shadow map size) are
//! applied once at startup and need a restart to take effect.
//!
//! City layout constants (grid size, block size, ...) stay in `resources.rs`
//! because they are compile-time `const`s used in const expressions.

use bevy::prelude::*;

#[derive(Resource, Clone, Default)]
pub struct GameConfig {
    pub graphics: GraphicsConfig,
    pub camera: CameraConfig,
    pub world: WorldConfig,
    pub player: PlayerConfig,
    pub driving: DrivingConfig,
    pub police: PoliceConfig,
    pub weapons: WeaponConfig,
    pub day_night: DayNightConfig,
}

#[derive(Clone)]
pub struct GraphicsConfig {
    /// Anti-aliasing (MSAA x4). Off = big FPS win on integrated GPUs;
    /// the low-poly style barely changes visually.
    pub msaa: bool,
    /// Sun shadows on/off — the single biggest graphics cost.
    pub shadows_enabled: bool,
    /// Shadow map resolution. 1024 is plenty for this style
    /// (Bevy's default is 2048). Needs restart.
    pub shadow_map_size: usize,
    /// How far (meters) shadows are drawn. Beyond ~120 m the fog hides
    /// everything anyway.
    pub shadow_distance: f32,
    /// VSync: caps FPS to the monitor refresh rate, avoids tearing.
    /// Needs restart.
    pub vsync: bool,
    /// Show the FPS counter in the top-left HUD panel.
    pub fps_counter: bool,
    /// Needs restart.
    pub window_width: f32,
    /// Needs restart.
    pub window_height: f32,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            msaa: false,
            shadows_enabled: true,
            shadow_map_size: 1024,
            shadow_distance: 120.0,
            vsync: true,
            fps_counter: true,
            window_width: 1280.0,
            window_height: 720.0,
        }
    }
}

#[derive(Clone)]
pub struct CameraConfig {
    /// Mouse look sensitivity, radians per pixel of mouse movement.
    pub mouse_sensitivity: f32,
    /// Lowest camera pitch, radians.
    ///
    /// The camera orbits the player at `offset.y = pitch.sin() * distance`,
    /// so a POSITIVE pitch lifts the camera above the player and looks down,
    /// and a NEGATIVE pitch drops it below the player and looks UP at the sky.
    /// -0.6 rad ≈ 34° above the horizon.
    pub pitch_min: f32,
    /// Highest camera pitch, radians. 1.35 rad ≈ 77°, just short of a full
    /// top-down view (at exactly π/2 the `look_at` up-vector degenerates).
    pub pitch_max: f32,
    /// The camera never drops below this height above the target, meters.
    /// Without it, looking up would push the camera through the pavement.
    pub min_height: f32,
    /// Third-person camera distance, meters. Aiming uses
    /// `weapons.aim_zoom_dist` instead.
    pub distance: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 0.003,
            pitch_min: -0.6,
            pitch_max: 1.35,
            min_height: 0.5,
            distance: 7.0,
        }
    }
}

#[derive(Clone)]
pub struct WorldConfig {
    /// Gravity, m/s². ≈22 feels right for this arcade scale.
    /// For the Moon set ≈3.7 🌕
    pub gravity: f32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self { gravity: 22.0 }
    }
}

#[derive(Clone)]
pub struct PlayerConfig {
    /// m/s
    pub walk_speed: f32,
    /// m/s (SHIFT)
    pub run_speed: f32,
    /// Initial upward velocity of a jump, m/s.
    pub jump_velocity: f32,
    /// Punch hit radius, m.
    pub punch_range: f32,
    /// Cash for each ped hit.
    pub cash_per_punch: i32,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            walk_speed: 4.5,
            run_speed: 9.0,
            jump_velocity: 7.5,
            punch_range: 1.4,
            cash_per_punch: 5,
        }
    }
}

#[derive(Clone)]
pub struct DrivingConfig {
    /// Max speed of a player-driven car, m/s.
    pub max_speed: f32,
    /// Acceleration / brake rate, m/s².
    pub accel: f32,
    /// Steering rate, rad/s at full speed factor.
    pub steer_rate: f32,
}

impl Default for DrivingConfig {
    fn default() -> Self {
        Self {
            max_speed: 28.0,
            accel: 18.0,
            steer_rate: 1.6,
        }
    }
}

#[derive(Clone)]
pub struct PoliceConfig {
    /// Cop cars per wanted star.
    pub cars_per_star: usize,
    /// Hard cap on simultaneous cop cars.
    pub max_cars: usize,
    /// Chase speed, m/s. For reference: the player runs at 9 and drives at
    /// 28 — on foot you can't outrun them, in a car it's a proper chase.
    pub chase_speed: f32,
    /// How quickly cops turn toward you (steering lerp rate, 1/s).
    pub steer_rate: f32,
    /// How far from the player new cops spawn, m.
    pub spawn_distance: f32,
    /// Distance at which a cop "grabs" you and drains HP, m.
    pub contact_radius: f32,
    /// HP drained per second while a cop is on you.
    pub contact_damage_per_sec: f32,
    /// Fraction of your cash you lose when busted.
    pub busted_fine_frac: f32,
    /// Light bar flash rate: red/blue swaps per second.
    pub flash_hz: f32,
    /// Seconds of good behavior for one wanted star to decay.
    /// The timer doesn't tick while a cop is within `escape_distance` of you.
    pub wanted_decay_secs: f32,
    /// Wanted stars only start decaying once every cop is farther away
    /// than this, m.
    pub escape_distance: f32,
}

impl Default for PoliceConfig {
    fn default() -> Self {
        Self {
            cars_per_star: 1,
            max_cars: 4,
            chase_speed: 19.0,
            steer_rate: 2.2,
            spawn_distance: 50.0,
            contact_radius: 2.4,
            contact_damage_per_sec: 30.0,
            busted_fine_frac: 0.5,
            flash_hz: 5.0,
            wanted_decay_secs: 18.0,
            escape_distance: 45.0,
        }
    }
}

#[derive(Clone)]
pub struct WeaponConfig {
    /// Damage per pistol bullet.
    pub pistol_damage: f32,
    /// Max bullet travel distance, m.
    pub pistol_range: f32,
    /// Seconds between shots (hold LMB for rapid fire).
    pub fire_cooldown: f32,
    /// Magazine size; the pistol auto-reloads when it runs dry.
    pub magazine: u32,
    /// Reload time, seconds.
    pub reload_secs: f32,
    /// Bullet spread from the hip, degrees.
    pub hip_spread_deg: f32,
    /// Bullet spread while aiming (hold RMB), degrees.
    pub aim_spread_deg: f32,
    /// Camera distance while aiming (normal third-person is `camera.distance`).
    pub aim_zoom_dist: f32,
    /// Pedestrian health (two default bullets = down).
    pub ped_hp: f32,
    /// Police car health (three default bullets = boom).
    pub police_car_hp: f32,
    /// Cash per kill. This city has questionable morals.
    pub cash_per_kill: i32,
    /// Seconds before a body disappears.
    pub corpse_despawn_secs: f32,
}

impl Default for WeaponConfig {
    fn default() -> Self {
        Self {
            pistol_damage: 25.0,
            pistol_range: 60.0,
            fire_cooldown: 0.22,
            magazine: 12,
            reload_secs: 1.1,
            hip_spread_deg: 2.2,
            aim_spread_deg: 0.35,
            aim_zoom_dist: 3.4,
            ped_hp: 50.0,
            police_car_hp: 75.0,
            cash_per_kill: 20,
            corpse_despawn_secs: 20.0,
        }
    }
}

#[derive(Clone)]
pub struct DayNightConfig {
    /// Master switch. `false` = permanent noon, like before.
    pub enabled: bool,
    /// Real seconds for a full in-game 24h cycle. 600 = 10 minutes.
    pub day_length_secs: f32,
    /// In-game hour at startup (0.0..24.0). 9.5 = 09:30.
    pub start_hour: f32,
    /// Sun brightness at noon, lux (~10 000 = bright daylight).
    pub sun_lux: f32,
    /// Moonlight brightness at night, lux.
    pub moon_lux: f32,
    /// Ambient fill at noon.
    pub day_ambient: f32,
    /// Ambient fill at midnight.
    pub night_ambient: f32,
}

impl Default for DayNightConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            day_length_secs: 600.0,
            start_hour: 9.5,
            sun_lux: 10_000.0,
            moon_lux: 25.0,
            day_ambient: 300.0,
            night_ambient: 50.0,
        }
    }
}
