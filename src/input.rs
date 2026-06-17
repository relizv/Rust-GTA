//! Keyboard + mouse capture and cursor locking.
//!
//! On game start (or window click), we lock the cursor so the player can
//! look around freely. On Escape, we release it. While locked, `MouseMotion`
//! events drive `InputState.yaw` / `InputState.pitch`.

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;

use crate::resources::{GameState, InputState, KeysPressed};

pub fn capture_input(
    keys: Res<Input<KeyCode>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut input_state: ResMut<InputState>,
    mut keys_pressed: ResMut<KeysPressed>,
    game_state: Res<GameState>,
    mut last_f: Local<bool>,
    mut last_r: Local<bool>,
    mut last_e: Local<bool>,
) {
    // --- Edge-triggered keys (F, R, E) ---
    let f_now = keys.pressed(KeyCode::F);
    let r_now = keys.pressed(KeyCode::R);
    let e_now = keys.pressed(KeyCode::E);
    keys_pressed.f_pressed = f_now && !*last_f;
    keys_pressed.r_pressed = r_now && !*last_r;
    keys_pressed.e_pressed = e_now && !*last_e;
    *last_f = f_now;
    *last_r = r_now;
    *last_e = e_now;

    // --- Continuous keys ---
    keys_pressed.w = keys.pressed(KeyCode::W);
    keys_pressed.a = keys.pressed(KeyCode::A);
    keys_pressed.s = keys.pressed(KeyCode::S);
    keys_pressed.d = keys.pressed(KeyCode::D);
    keys_pressed.shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    keys_pressed.space = keys.pressed(KeyCode::Space);

    // --- Mouse look (only when game has started and cursor is locked) ---
    if game_state.started && input_state.cursor_locked {
        for ev in mouse_motion.iter() {
            input_state.yaw -= ev.delta.x * 0.003;
            input_state.pitch -= ev.delta.y * 0.003;
            input_state.pitch = input_state.pitch.clamp(0.1, 1.2);
        }
    }
}

pub fn manage_cursor_lock(
    mut windows: Query<&mut Window>,
    mouse_buttons: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    mut input_state: ResMut<InputState>,
    game_state: Res<GameState>,
) {
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    // Escape releases the cursor
    if keys.just_pressed(KeyCode::Escape) {
        input_state.cursor_locked = false;
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
    }

    // Click to (re-)lock the cursor (only if the game has started)
    if game_state.started
        && mouse_buttons.just_pressed(MouseButton::Left)
        && !input_state.cursor_locked
    {
        input_state.cursor_locked = true;
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
    }
}
