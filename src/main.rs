//! Mini GTA — Rust Edition (Bevy 0.15)
//!
//! A browser-style mini-GTA ported from Three.js to Bevy + wgpu.
//! Visual style is preserved: blocky low-poly characters, windows on
//! buildings, dashed road lines, fog, directional sun + shadows.
//!
//! All tunables (graphics, physics, player, driving, police) live in
//! `src/config.rs`.

mod camera;
mod car;
mod city;
mod config;
mod hud;
mod input;
mod pedestrian;
mod player;
mod police;
mod resources;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy_egui::EguiPlugin;

use config::GameConfig;

fn main() {
    // Everything you might want to tweak is in src/config.rs.
    let config = GameConfig::default();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Mini GTA — Rust Edition".into(),
                resolution: (config.graphics.window_width, config.graphics.window_height).into(),
                present_mode: if config.graphics.vsync {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                },
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        // Data source for the HUD FPS counter.
        .add_plugins(FrameTimeDiagnosticsPlugin)
        // Shadow map resolution: 1024 is plenty for this style and much
        // cheaper than Bevy's 2048 default on weak GPUs.
        .insert_resource(DirectionalLightShadowMap {
            size: config.graphics.shadow_map_size,
        })
        .insert_resource(config)
        // Resources
        .init_resource::<resources::GameState>()
        .init_resource::<resources::InputState>()
        .init_resource::<resources::KeysPressed>()
        // Startup systems — order matters
        .add_systems(Startup, setup_world)
        .add_systems(Startup, resources::setup_game_assets.after(setup_world))
        .add_systems(
            Startup,
            city::build_city.after(resources::setup_game_assets),
        )
        .add_systems(
            Startup,
            player::spawn_player.after(resources::setup_game_assets),
        )
        .add_systems(Startup, car::spawn_cars.after(resources::setup_game_assets))
        .add_systems(
            Startup,
            pedestrian::spawn_peds.after(resources::setup_game_assets),
        )
        // Update systems
        .add_systems(
            Update,
            (
                input::capture_input,
                input::manage_cursor_lock,
                player::update_player,
                car::update_ai_cars,
                pedestrian::update_peds,
                // `player_punch` runs after `update_peds` so that it can read
                // `ped.pos` (synced at the end of `update_peds`) and queue
                // knockback into `ped.knockback`, which `update_peds` will
                // apply on the next frame. This ordering avoids Bevy 0.15's
                // B0001 panic on conflicting `&mut Transform` accesses.
                player::player_punch,
                police::manage_police,
                police::update_police,
                camera::update_camera,
                player::update_wanted_decay,
                hud::update_hud,
            )
                .chain(),
        )
        .run();
}

/// Spawn camera, lights, fog, and the grass ground.
fn setup_world(mut commands: Commands, config: Res<GameConfig>) {
    // Camera with fog (matches the JS scene.background #87ceeb + fog 80..250)
    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: false,
            ..default()
        },
        // MSAA off is a large FPS win on integrated GPUs; the low-poly style
        // barely changes visually. (Msaa is a per-camera component in 0.15.)
        if config.graphics.msaa {
            Msaa::Sample4
        } else {
            Msaa::Off
        },
        Transform::from_xyz(0.0, 10.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        DistanceFog {
            color: Color::srgb(0.529, 0.808, 0.922),
            directional_light_color: Color::srgb(1.0, 0.957, 0.878),
            directional_light_exponent: 30.0,
            falloff: FogFalloff::Linear {
                start: 80.0,
                end: 250.0,
            },
        },
    ));

    // Ambient + hemisphere fill.
    // NOTE: Bevy 0.15 uses physical-ish units here (default brightness is
    // 80.0), NOT the 0..1 intensity scale of Three.js. 0.55 was pitch black.
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
    });

    // Sun (directional) with cascaded shadows.
    let cascade_config = CascadeShadowConfigBuilder {
        num_cascades: 1,
        minimum_distance: 10.0,
        maximum_distance: config.graphics.shadow_distance,
        first_cascade_far_bound: config.graphics.shadow_distance,
        overlap_proportion: 0.0,
        ..default()
    }
    .build();

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.957, 0.878),
            // Lux (physical units). 1.0 lux is moonlight — that's why the
            // scene was almost black. ~10 000 lux = bright daylight.
            illuminance: 10_000.0,
            shadows_enabled: config.graphics.shadows_enabled,
            ..default()
        },
        cascade_config,
        Transform::from_xyz(60.0, 100.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
