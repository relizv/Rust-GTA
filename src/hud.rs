//! HUD overlay via egui.
//!
//! - Top-left:  HP / cash / mode
//! - Top-right: wanted stars
//! - Bottom-left: minimap (drawn with egui::Painter)
//! - Bottom-right: speedometer (only when driving)
//! - Center-top: toast notification
//! - Center:    start overlay (when not started) + pause overlay (cursor unlocked)

use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy_egui::{egui, EguiContexts};

use crate::player::Player;
use crate::car::Car;
use crate::pedestrian::Pedestrian;
use crate::resources::{GameState, InputState, CITY_HALF, GRID, STEP};

pub fn update_hud(
    mut contexts: EguiContexts,
    mut game_state: ResMut<GameState>,
    mut input_state: ResMut<InputState>,
    mut windows: Query<&mut Window>,
    player_q: Query<&Transform, With<Player>>,
    cars: Query<&Transform, With<Car>>,
    peds: Query<&Transform, With<Pedestrian>>,
) {
    let ctx = contexts.ctx_mut();

    // ----- Start overlay -----
    if !game_state.started {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(10, 10, 30)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(120.0);
                    ui.heading(
                        egui::RichText::new("MINI GTA")
                            .color(egui::Color32::from_rgb(255, 204, 51))
                            .size(72.0)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Город открыт. Делай что хочешь.")
                            .color(egui::Color32::from_rgb(170, 170, 170))
                            .size(16.0),
                    );
                    ui.add_space(36.0);
                    if ui.button(
                        egui::RichText::new("ИГРАТЬ ▶")
                            .color(egui::Color32::BLACK)
                            .size(20.0)
                            .strong(),
                    ).clicked() {
                        game_state.started = true;
                        if let Ok(mut window) = windows.get_single_mut() {
                            input_state.cursor_locked = true;
                            window.cursor.visible = false;
                            window.cursor.grab_mode = CursorGrabMode::Locked;
                        }
                    }
                    ui.add_space(36.0);
                    ui.label(
                        egui::RichText::new(
                            "WASD — движение   |   Мышь — камера   |   SHIFT — бег   |   ПРОБЕЛ — прыжок\n\
                             F — войти/выйти из машины   |   ЛКМ — удар   |   R — сброс позиции   |   ESC — отпустить курсор"
                        )
                        .color(egui::Color32::from_rgb(200, 200, 200))
                        .size(13.0),
                    );
                });
            });
        return;
    }

    // ----- Info (top-left) -----
    egui::Area::new(egui::Id::new("info"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(14.0, 14.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_black_alpha(140))
                .show(ui, |ui| {
                    ui.set_min_width(180.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Здоровье").color(egui::Color32::from_rgb(170, 170, 187)));
                        ui.label(egui::RichText::new(format!("{}", game_state.hp as i32)).color(egui::Color32::WHITE).strong());
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Деньги").color(egui::Color32::from_rgb(170, 170, 187)));
                        ui.label(egui::RichText::new(format!("${}", game_state.cash)).color(egui::Color32::WHITE).strong());
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Режим").color(egui::Color32::from_rgb(170, 170, 187)));
                        let mode = if game_state.in_vehicle.is_some() { "ЗА РУЛЁМ" } else { "ПЕШКОМ" };
                        ui.label(egui::RichText::new(mode).color(egui::Color32::from_rgb(255, 204, 51)).strong());
                    });
                });
        });

    // ----- Wanted stars (top-right) -----
    if game_state.wanted > 0 {
        egui::Area::new(egui::Id::new("wanted"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-18.0, 18.0))
            .show(ctx, |ui| {
                let stars: String = "★".repeat(game_state.wanted as usize)
                    + &"☆".repeat(5 - game_state.wanted as usize);
                ui.label(
                    egui::RichText::new(stars)
                        .color(egui::Color32::from_rgb(255, 204, 51))
                        .size(24.0)
                        .strong(),
                );
            });
    }

    // ----- Minimap (bottom-left) -----
    let player_pos = player_q
        .get_single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);
    let yaw = if let Some(car_entity) = game_state.in_vehicle {
        cars.get(car_entity)
            .map(|t| t.rotation.to_euler(EulerRot::YXZ).0)
            .unwrap_or(0.0)
    } else {
        input_state.yaw
    };

    egui::Area::new(egui::Id::new("minimap"))
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(18.0, -18.0))
        .show(ctx, |ui| {
            let size = 190.0;
            let (rect, _resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
            let painter = ui.painter().with_clip_rect(rect);
            painter.circle_filled(rect.center(), size / 2.0, egui::Color32::from_rgb(26, 42, 26));
            let center = rect.center();
            let rot = -yaw + std::f32::consts::PI;
            let scale = 0.32;

            // Roads
            for i in 0..=GRID {
                let c = -CITY_HALF + i as f32 * STEP;
                let (rx, ry) = (
                    (c - player_pos.x) * scale,
                    (c - player_pos.z) * scale,
                );
                let p1 = rotate2d(-CITY_HALF * scale, ry, rot);
                let p2 = rotate2d(CITY_HALF * scale, ry, rot);
                painter.line_segment(
                    [center + p1, center + p2],
                    egui::Stroke::new(4.0, egui::Color32::from_rgb(68, 68, 68)),
                );
                let p1 = rotate2d(rx, -CITY_HALF * scale, rot);
                let p2 = rotate2d(rx, CITY_HALF * scale, rot);
                painter.line_segment(
                    [center + p1, center + p2],
                    egui::Stroke::new(4.0, egui::Color32::from_rgb(68, 68, 68)),
                );
            }

            // Cars (yellow dots)
            for car_t in cars.iter() {
                let rx = (car_t.translation.x - player_pos.x) * scale;
                let ry = (car_t.translation.z - player_pos.z) * scale;
                let p = rotate2d(rx, ry, rot);
                painter.circle_filled(center + p, 2.2, egui::Color32::from_rgb(255, 255, 80));
            }

            // Peds (white dots)
            for ped_t in peds.iter() {
                let rx = (ped_t.translation.x - player_pos.x) * scale;
                let ry = (ped_t.translation.z - player_pos.z) * scale;
                let p = rotate2d(rx, ry, rot);
                painter.circle_filled(center + p, 1.4, egui::Color32::WHITE);
            }

            // Player arrow
            let arrow = vec![
                center + egui::vec2(0.0, -6.0),
                center + egui::vec2(-4.0, 4.0),
                center + egui::vec2(4.0, 4.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                arrow,
                egui::Color32::from_rgb(0, 255, 0),
                egui::Stroke::new(1.2, egui::Color32::BLACK),
            ));
        });

    // ----- Speedometer (bottom-right, only when driving) -----
    if game_state.in_vehicle.is_some() {
        egui::Area::new(egui::Id::new("speedo"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -20.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_black_alpha(153))
                    .show(ui, |ui| {
                        ui.set_min_width(150.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{}", game_state.last_speed_kmh as i32))
                                    .color(egui::Color32::from_rgb(255, 204, 51))
                                    .size(34.0)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new("КМ/Ч")
                                    .color(egui::Color32::from_rgb(170, 170, 170))
                                    .size(11.0),
                            );
                            ui.label(
                                egui::RichText::new("Седан")
                                    .color(egui::Color32::WHITE)
                                    .size(12.0),
                            );
                        });
                    });
            });
    }

    // ----- Toast (top-center) -----
    if let Some((msg, _)) = &game_state.toast {
        egui::Area::new(egui::Id::new("toast"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 24.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_black_alpha(217))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 204, 51)))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(msg)
                                .color(egui::Color32::from_rgb(255, 204, 51))
                                .size(15.0)
                                .strong(),
                        );
                    });
            });
    }

    // ----- Pause overlay (cursor unlocked mid-game) -----
    if game_state.started && !input_state.cursor_locked {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_black_alpha(180)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(200.0);
                    ui.heading(
                        egui::RichText::new("ПАУЗА")
                            .color(egui::Color32::from_rgb(255, 204, 51))
                            .size(48.0)
                            .strong(),
                    );
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new("Кликни в окно, чтобы продолжить")
                            .color(egui::Color32::from_rgb(200, 200, 200))
                            .size(16.0),
                    );
                });
            });
    }
}

fn rotate2d(x: f32, y: f32, angle: f32) -> egui::Vec2 {
    let (s, c) = (angle.sin(), angle.cos());
    egui::vec2(x * c - y * s, x * s + y * c)
}
