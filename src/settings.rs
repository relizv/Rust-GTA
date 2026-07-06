//! Applies settings from `GameConfig` to live engine state.
//!
//! The pause menu (`hud.rs`) mutates `GameConfig` directly; this system
//! watches for changes and pushes them to the places that need it:
//! - MSAA is a component on the camera,
//! - shadows on/off + shadow distance live on the directional light,
//! - vsync is the window's present mode.
//!
//! Gameplay values (speeds, gravity, police, day length) need no "apply"
//! step — the gameplay systems read `GameConfig` every frame anyway.

use bevy::pbr::{CascadeShadowConfig, CascadeShadowConfigBuilder};
use bevy::prelude::*;
use bevy::window::PresentMode;

use crate::config::GameConfig;
use crate::daynight::Sun;

pub fn apply_graphics_settings(
    config: Res<GameConfig>,
    mut msaa_q: Query<&mut Msaa, With<Camera3d>>,
    mut sun_q: Query<(&mut DirectionalLight, &mut CascadeShadowConfig), With<Sun>>,
    mut windows: Query<&mut Window>,
) {
    // Only react when the config was actually touched (menu open / startup).
    if !config.is_changed() {
        return;
    }

    // --- MSAA (runtime switch; guarded so we don't respecialize pipelines
    // every frame the menu is open) ---
    let target_msaa = if config.graphics.msaa {
        Msaa::Sample4
    } else {
        Msaa::Off
    };
    for mut msaa in msaa_q.iter_mut() {
        if *msaa != target_msaa {
            *msaa = target_msaa;
        }
    }

    // --- Shadows ---
    for (mut light, mut cascades) in sun_q.iter_mut() {
        if light.shadows_enabled != config.graphics.shadows_enabled {
            light.shadows_enabled = config.graphics.shadows_enabled;
        }
        if cascades.bounds.first().copied() != Some(config.graphics.shadow_distance) {
            *cascades = CascadeShadowConfigBuilder {
                num_cascades: 1,
                minimum_distance: 10.0,
                maximum_distance: config.graphics.shadow_distance,
                first_cascade_far_bound: config.graphics.shadow_distance,
                overlap_proportion: 0.0,
                ..default()
            }
            .build();
        }
    }

    // --- VSync (bevy_winit only applies real differences, but guard anyway) ---
    let target_present = if config.graphics.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
    for mut window in windows.iter_mut() {
        if window.present_mode != target_present {
            window.present_mode = target_present;
        }
    }
}
