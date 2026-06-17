//! Mini GTA — Rust Edition (Bevy 0.15)
//!
//! A browser-style mini-GTA ported from Three.js to Bevy + wgpu.
//! Visual style is preserved: blocky low-poly characters, windows on
//! buildings, dashed road lines, fog, directional sun + shadows.

mod camera;
mod car;
mod city;
mod hud;
mod input;
mod pedestrian;
mod player;
mod resources;

use bevy::core_pipeline::fog::DistanceFog;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Mini GTA — Rust Edition".into(),
                resolution: (1280.0, 720.0).into(),
                resizable: true,
                ..default()
            }),
            ..default()
        }))
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
        Camera3d::default(),
        Camera {
            hdr: false,
            ..default()
        },
        Transform::from_xyz(0.0, 10.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        DistanceFog {
            color: Color::srgb(0.529, 0.808, 0.922),
            directional_light_color: Color::srgb(1.0, 0.957, 0.878),
            directional_light_exponent: 30.0,
            falloff: bevy::core_pipeline::fog::FogFalloff::Linear {
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

    // Hemisphere light via PbrLightBundle / direct spawn.
    // In Bevy 0.15 `HemisphereLight` is in `bevy::pbr`.
    commands.spawn((
        // Skip HemisphereLight (its bundle was removed in 0.15).
        // HemisphereLight still exists in bevy::pbr but spawning it as a
        // component-only entity is awkward; ambient + directional suffices.
        SpatialBundle::default(),
    ));

    // Sun (directional) with cascaded shadows covering the whole city.
    let cascade_config = CascadeShadowConfigBuilder {
        num_cascades: 1,
        minimum_distance: 10.0,
        maximum_distance: 250.0,
        first_cascade_far_bound: 250.0,
        overlap_proportion: 0.0,
        ..default()
    }
    .build();

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.957, 0.878),
            illuminance: 1.0,
            shadows_enabled: true,
            shadow_depth_bias: -0.0005,
            ..default()
        },
        cascade_config,
        Transform::from_xyz(60.0, 100.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
