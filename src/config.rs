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
    pub world: WorldConfig,
    pub player: PlayerConfig,
    pub driving: DrivingConfig,
    pub police: PoliceConfig,
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
    /// Chase speed, m/s. For reference: the player runs at 9 and drives at 28,
    /// so on foot you can't outrun them — steal a car.
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
    pub wanted_decay_secs: f32,
}

impl Default for PoliceConfig {
    fn default() -> Self {
        Self {
            cars_per_star: 1,
            max_cars: 4,
            chase_speed: 15.0,
            steer_rate: 2.2,
            spawn_distance: 50.0,
            contact_radius: 2.4,
            contact_damage_per_sec: 30.0,
            busted_fine_frac: 0.5,
            flash_hz: 5.0,
            wanted_decay_secs: 18.0,
        }
    }
}
