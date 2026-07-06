//! Day/night cycle: a sun that moves across the sky, sunrise/sunset tinting,
//! night-time window glow and street lamps.
//!
//! Perf notes for weak GPUs: the whole night look is driven by mutating just
//! TWO shared materials per frame (lit windows + lamp heads) plus the global
//! sun/ambient/fog parameters. No extra point lights are spawned — real
//! shadow-casting lights would tank FPS on integrated graphics.
//!
//! All tunables live in `config.rs` (`DayNightConfig`).

use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use std::f32::consts::PI;

use crate::config::GameConfig;
use crate::resources::{GameAssets, GameState, BLOCK, CITY_HALF, GRID, ROAD_W, SIDEWALK_W, STEP};

/// Marker for the directional light spawned in `main.rs`.
#[derive(Component)]
pub struct Sun;

/// Global clock. `hour` is the in-game hour, 0.0..24.0.
#[derive(Resource)]
pub struct DayNight {
    pub hour: f32,
}

/// Spawn a lamp post on each corner of every block's sidewalk pad.
/// 36 blocks × 4 lamps = 144 lamps, but they all share one mesh and two
/// materials, so Bevy batches them into a couple of draw calls.
pub fn spawn_street_lamps(mut commands: Commands, assets: Res<GameAssets>) {
    let inset = BLOCK / 2.0 + SIDEWALK_W / 2.0;
    for ix in 0..GRID {
        for iz in 0..GRID {
            let cx = -CITY_HALF + ROAD_W / 2.0 + ix as f32 * STEP + BLOCK / 2.0;
            let cz = -CITY_HALF + ROAD_W / 2.0 + iz as f32 * STEP + BLOCK / 2.0;
            for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let (x, z) = (cx + sx * inset, cz + sz * inset);
                // Pole (the sidewalk pad top is at y = 0.3)
                commands.spawn((
                    Mesh3d(assets.mesh_unit_box.clone()),
                    MeshMaterial3d(assets.mat_lamp_pole.clone()),
                    Transform::from_xyz(x, 0.3 + 2.0, z).with_scale(Vec3::new(0.14, 4.0, 0.14)),
                    NotShadowCaster,
                ));
                // Head — shared emissive material, driven by `update_day_night`.
                commands.spawn((
                    Mesh3d(assets.mesh_unit_box.clone()),
                    MeshMaterial3d(assets.mat_lamp_head.clone()),
                    Transform::from_xyz(x, 0.3 + 4.15, z).with_scale(Vec3::new(0.45, 0.22, 0.45)),
                    NotShadowCaster,
                ));
            }
        }
    }
}

/// Advance the clock and drive sun, ambient, sky/fog, windows and lamps.
#[allow(clippy::too_many_arguments)]
pub fn update_day_night(
    time: Res<Time>,
    config: Res<GameConfig>,
    game_state: Res<GameState>,
    assets: Res<GameAssets>,
    mut day_night: ResMut<DayNight>,
    mut ambient: ResMut<AmbientLight>,
    mut clear_color: ResMut<ClearColor>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sun_q: Query<(&mut DirectionalLight, &mut Transform), With<Sun>>,
    mut fog_q: Query<&mut DistanceFog>,
) {
    let cfg = &config.day_night;
    if !cfg.enabled {
        return;
    }

    // The clock only ticks once the game has started (not on the title screen).
    if game_state.started {
        day_night.hour = (day_night.hour + 24.0 * time.delta_secs() / cfg.day_length_secs) % 24.0;
    }
    let hour = day_night.hour;

    // Sun elevation: 1.0 at noon, 0.0 at 6:00 / 18:00, negative at night.
    let elev = ((hour - 6.0) / 12.0 * PI).sin();
    let day = elev.clamp(0.0, 1.0);
    // City lights (windows, lamps) fade in as the sun goes down.
    let glow = (1.0 - day / 0.25).clamp(0.0, 1.0);

    // --- Sun (acts as the moon at night) ---
    let theta = (hour - 6.0) / 12.0 * PI;
    if let Ok((mut light, mut tf)) = sun_q.get_single_mut() {
        // Keep the light above the horizon so shadows never go sideways.
        let height = theta.sin().abs().max(0.15) * 110.0;
        *tf = Transform::from_xyz(theta.cos() * 80.0 + 30.0, height, 40.0)
            .looking_at(Vec3::ZERO, Vec3::Y);
        // Continuous ramp: full sun at noon → `moon_lux` at night, no popping.
        light.illuminance = cfg.moon_lux + (cfg.sun_lux - cfg.moon_lux) * day.powf(1.2);
        light.color = mix(
            Vec3::new(0.63, 0.72, 1.0),
            Vec3::new(1.0, 0.957, 0.878),
            day,
        );
    }

    // --- Ambient fill ---
    ambient.brightness = cfg.night_ambient + (cfg.day_ambient - cfg.night_ambient) * day;

    // --- Sky + fog (with an orange band around sunrise/sunset) ---
    let day_sky = Vec3::new(0.529, 0.808, 0.922);
    let night_sky = Vec3::new(0.015, 0.025, 0.06);
    let dusk = (1.0 - elev.abs() / 0.35).clamp(0.0, 1.0);
    let sky = night_sky
        .lerp(day_sky, day)
        .lerp(Vec3::new(0.85, 0.45, 0.25), dusk * 0.55);
    let sky_color = Color::srgb(sky.x, sky.y, sky.z);
    clear_color.0 = sky_color;
    for mut fog in fog_q.iter_mut() {
        fog.color = sky_color;
        // Fog closes in at night — moodier and cheaper to draw.
        fog.falloff = FogFalloff::Linear {
            start: 45.0 + 35.0 * day,
            end: 170.0 + 80.0 * day,
        };
    }

    // --- City lights: two shared material writes per frame, that's all ---
    if let Some(m) = materials.get_mut(&assets.mat_window_on) {
        let e = Vec3::new(0.35, 0.30, 0.15).lerp(Vec3::new(3.0, 2.4, 1.1), glow);
        m.emissive = LinearRgba::rgb(e.x, e.y, e.z);
    }
    if let Some(m) = materials.get_mut(&assets.mat_lamp_head) {
        let e = Vec3::ZERO.lerp(Vec3::new(4.0, 3.2, 1.4), glow);
        m.emissive = LinearRgba::rgb(e.x, e.y, e.z);
    }
}

fn mix(a: Vec3, b: Vec3, t: f32) -> Color {
    let v = a.lerp(b, t);
    Color::srgb(v.x, v.y, v.z)
}
