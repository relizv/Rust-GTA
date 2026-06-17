//! Mini GTA — Rust Edition
//!
//! A browser-style mini-GTA ported from Three.js to Bevy 0.13 + wgpu.
//! Visual style is intentionally preserved: blocky low-poly characters,
//! windows on buildings, dashed road lines, fog, directional sun + shadows.

mod camera;
mod car;
mod city;
mod hud;
mod input;
mod pedestrian;
mod player;
mod resources;

use bevy::core_pipeline::fog::FogFalloff;
use bevy::core_pipeline::fog::FogSettings;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy_egui::EguiPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Mini GTA — Rust Edition".into(),
                        resolution: (1280.0, 720.0).into(),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    // No external assets; everything is procedural.
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin)
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
                camera::update_camera,
                player::update_wanted_decay,
                hud::update_hud,
            )
                .chain(),
        )
        .run();
}

/// Spawn camera, lights, fog, and the grass ground.
fn setup_world(mut commands: Commands) {
    // Camera with fog (matches the JS scene.background #87ceeb + fog 80..250)
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 10.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                hdr: false,
                ..default()
            },
            ..default()
        },
        FogSettings {
            color: Color::rgb(0.529, 0.808, 0.922),
            directional_light_color: Color::rgb(1.0, 0.957, 0.878),
            directional_light_exponent: 30.0,
            falloff: FogFalloff::Linear {
                start: 80.0,
                end: 250.0,
            },
        },
    ));

    // Ambient + hemisphere fill
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.55,
    });

    commands.spawn(HemisphereLightBundle {
        hemisphere_light: HemisphereLight {
            sky_color: Color::rgb(0.529, 0.808, 0.922),
            ground_color: Color::rgb(0.266, 0.266, 0.2),
            intensity: 0.4,
        },
        ..default()
    });

    // Sun (directional) with shadows — covers the whole city
    let mut shadow_projection = OrthographicProjection {
        scale: 1.0,
        ..default()
    };
    let half = resources::CITY_HALF + 20.0;
    shadow_projection.left = -half;
    shadow_projection.right = half;
    shadow_projection.top = half;
    shadow_projection.bottom = -half;
    shadow_projection.near = 10.0;
    shadow_projection.far = 250.0;

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::rgb(1.0, 0.957, 0.878),
            illuminance: 1.0,
            shadows_enabled: true,
            shadow_projection,
            shadow_depth_bias: -0.0005,
            ..default()
        },
        transform: Transform::from_xyz(60.0, 100.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Grass ground (will be replaced/superseded by city ground in city.rs)
    // — kept minimal here; city.rs spawns the actual grass plane.
}
