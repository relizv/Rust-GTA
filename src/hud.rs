//! HUD overlay via egui.
//!
//! - Top-left:  HP / cash / mode
//! - Top-right: wanted stars
//! - Bottom-left: minimap (drawn with egui::Painter)
//! - Bottom-right: speedometer (only when driving)
//! - Center-top: toast notification
//! - Center:    start overlay (when not started) + pause overlay (cursor unlocked)

use bevy::app::AppExit;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::transform::components::GlobalTransform;
use bevy::window::CursorGrabMode;
use bevy_egui::{egui, EguiContexts};

use crate::car::Car;
use crate::config::GameConfig;
use crate::daynight::DayNight;
use crate::pedestrian::Pedestrian;
use crate::player::Player;
use crate::police::PoliceCar;
use crate::resources::{GameState, InputState, CITY_HALF, GRID, STEP};
use crate::weapons::WeaponState;

#[allow(clippy::too_many_arguments)]
pub fn update_hud(
    mut contexts: EguiContexts,
    diagnostics: Res<DiagnosticsStore>,
    mut config: ResMut<GameConfig>,
    mut day_night: ResMut<DayNight>,
    weapon: Res<WeaponState>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut exit: EventWriter<AppExit>,
    mut game_state: ResMut<GameState>,
    mut input_state: ResMut<InputState>,
    mut windows: Query<&mut Window>,
    // Read positions via GlobalTransform to avoid B0001 conflicts with the
    // movement systems that hold `&mut Transform` on the same entities.
    player_q: Query<&GlobalTransform, With<Player>>,
    cars: Query<&GlobalTransform, With<Car>>,
    peds: Query<&GlobalTransform, With<Pedestrian>>,
    police: Query<&GlobalTransform, With<PoliceCar>>,
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
                            window.cursor_options.visible = false;
                            window.cursor_options.grab_mode = CursorGrabMode::Locked;
                        }
                    }
                    ui.add_space(36.0);
                    ui.label(
                        egui::RichText::new(
                            "WASD — движение   |   Мышь — камера   |   SHIFT — бег   |   ПРОБЕЛ — прыжок\n\
                             F — войти/выйти из машины   |   ЛКМ — удар / огонь   |   R — сброс позиции\n\
                             1 — кулаки   |   2 — пистолет   |   ПКМ (держать) — прицел   |   ESC — пауза и настройки"
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
                    if config.graphics.fps_counter {
                        if let Some(fps) = diagnostics
                            .get(&FrameTimeDiagnosticsPlugin::FPS)
                            .and_then(|d| d.smoothed())
                        {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("FPS")
                                        .color(egui::Color32::from_rgb(170, 170, 187)),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{fps:.0}"))
                                        .color(egui::Color32::from_rgb(120, 255, 120))
                                        .strong(),
                                );
                            });
                        }
                    }
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Время")
                                .color(egui::Color32::from_rgb(170, 170, 187)),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{:02}:{:02}",
                                day_night.hour as u32,
                                ((day_night.hour % 1.0) * 60.0) as u32
                            ))
                            .color(egui::Color32::from_rgb(255, 220, 130))
                            .strong(),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Здоровье")
                                .color(egui::Color32::from_rgb(170, 170, 187)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}", game_state.hp as i32))
                                .color(egui::Color32::WHITE)
                                .strong(),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Деньги")
                                .color(egui::Color32::from_rgb(170, 170, 187)),
                        );
                        ui.label(
                            egui::RichText::new(format!("${}", game_state.cash))
                                .color(egui::Color32::WHITE)
                                .strong(),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Режим")
                                .color(egui::Color32::from_rgb(170, 170, 187)),
                        );
                        let mode = if game_state.in_vehicle.is_some() {
                            "ЗА РУЛЁМ"
                        } else {
                            "ПЕШКОМ"
                        };
                        ui.label(
                            egui::RichText::new(mode)
                                .color(egui::Color32::from_rgb(255, 204, 51))
                                .strong(),
                        );
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
        .map(|gt| gt.translation())
        .unwrap_or(Vec3::ZERO);
    let yaw = if let Some(car_entity) = game_state.in_vehicle {
        cars.get(car_entity)
            .map(|gt| gt.rotation().to_euler(EulerRot::YXZ).0)
            .unwrap_or(0.0)
    } else {
        input_state.yaw
    };

    egui::Area::new(egui::Id::new("minimap"))
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(18.0, -18.0))
        .show(ctx, |ui| {
            let size = 190.0;
            let (rect, _resp) =
                ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
            let painter = ui.painter().with_clip_rect(rect);
            painter.circle_filled(
                rect.center(),
                size / 2.0,
                egui::Color32::from_rgb(26, 42, 26),
            );
            let center = rect.center();
            let rot = -yaw + std::f32::consts::PI;
            let scale = 0.32;

            // Roads
            for i in 0..=GRID {
                let c = -CITY_HALF + i as f32 * STEP;
                let (rx, ry) = ((c - player_pos.x) * scale, (c - player_pos.z) * scale);
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
            for car_gt in cars.iter() {
                let cp = car_gt.translation();
                let rx = (cp.x - player_pos.x) * scale;
                let ry = (cp.z - player_pos.z) * scale;
                let p = rotate2d(rx, ry, rot);
                painter.circle_filled(center + p, 2.2, egui::Color32::from_rgb(255, 255, 80));
            }

            // Police (blue dots)
            for cop_gt in police.iter() {
                let cp = cop_gt.translation();
                let rx = (cp.x - player_pos.x) * scale;
                let ry = (cp.z - player_pos.z) * scale;
                let p = rotate2d(rx, ry, rot);
                painter.circle_filled(center + p, 2.6, egui::Color32::from_rgb(70, 130, 255));
            }

            // Peds (white dots)
            for ped_gt in peds.iter() {
                let pp = ped_gt.translation();
                let rx = (pp.x - player_pos.x) * scale;
                let ry = (pp.z - player_pos.z) * scale;
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
                                egui::RichText::new(format!(
                                    "{}",
                                    game_state.last_speed_kmh as i32
                                ))
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

    // ----- Ammo (bottom-right, on foot with the pistol out) -----
    if game_state.in_vehicle.is_none() && weapon.pistol_equipped {
        egui::Area::new(egui::Id::new("ammo"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -20.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_black_alpha(153))
                    .show(ui, |ui| {
                        ui.set_min_width(150.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("ПИСТОЛЕТ")
                                    .color(egui::Color32::from_rgb(170, 170, 170))
                                    .size(11.0),
                            );
                            let ammo_text = if weapon.reloading {
                                "ПЕРЕЗАРЯДКА…".to_string()
                            } else {
                                format!("{} / ∞", weapon.ammo)
                            };
                            ui.label(
                                egui::RichText::new(ammo_text)
                                    .color(egui::Color32::from_rgb(255, 204, 51))
                                    .size(26.0)
                                    .strong(),
                            );
                        });
                    });
            });
    }

    // ----- Crosshair (pistol out, on foot, not paused) -----
    if game_state.in_vehicle.is_none() && weapon.pistol_equipped && input_state.cursor_locked {
        egui::Area::new(egui::Id::new("crosshair"))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .interactable(false)
            .show(ctx, |ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                let painter = ui.painter();
                let c = rect.center();
                let (gap, len, col) = if weapon.aiming {
                    (3.0, 6.0, egui::Color32::from_rgb(255, 90, 90))
                } else {
                    (5.0, 6.0, egui::Color32::WHITE)
                };
                let stroke = egui::Stroke::new(1.6, col);
                painter.line_segment(
                    [c + egui::vec2(gap, 0.0), c + egui::vec2(gap + len, 0.0)],
                    stroke,
                );
                painter.line_segment(
                    [c - egui::vec2(gap, 0.0), c - egui::vec2(gap + len, 0.0)],
                    stroke,
                );
                painter.line_segment(
                    [c + egui::vec2(0.0, gap), c + egui::vec2(0.0, gap + len)],
                    stroke,
                );
                painter.line_segment(
                    [c - egui::vec2(0.0, gap), c - egui::vec2(0.0, gap + len)],
                    stroke,
                );
                painter.circle_filled(c, 1.2, col);
            });
    }

    // ----- Toast (top-center) -----
    if let Some((msg, _)) = &game_state.toast {
        egui::Area::new(egui::Id::new("toast"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 24.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_black_alpha(217))
                    .stroke(egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgb(255, 204, 51),
                    ))
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

    // ----- Pause + settings menu (ESC) -----
    if game_state.started && !input_state.cursor_locked {
        // Dim the game behind the menu.
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_black_alpha(150)))
            .show(ctx, |_ui| {});

        let section = |ui: &mut egui::Ui, title: &str| {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(title)
                    .color(egui::Color32::from_rgb(255, 204, 51))
                    .size(13.0)
                    .strong(),
            );
            ui.separator();
        };

        egui::Window::new("pause_menu")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(
                egui::Frame::popup(&ctx.style())
                    .fill(egui::Color32::from_rgb(16, 16, 26))
                    .stroke(egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgb(255, 204, 51),
                    )),
            )
            .show(ctx, |ui| {
                ui.set_width(360.0);
                ui.vertical_centered(|ui| {
                    ui.heading(
                        egui::RichText::new("ПАУЗА")
                            .color(egui::Color32::from_rgb(255, 204, 51))
                            .size(30.0)
                            .strong(),
                    );
                });

                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        section(ui, "ГРАФИКА");
                        ui.checkbox(&mut config.graphics.fps_counter, "Счётчик FPS");
                        ui.checkbox(&mut config.graphics.msaa, "Сглаживание (MSAA x4)");
                        ui.checkbox(&mut config.graphics.vsync, "VSync");
                        ui.checkbox(&mut config.graphics.shadows_enabled, "Тени");
                        ui.add(
                            egui::Slider::new(&mut config.graphics.shadow_distance, 40.0..=250.0)
                                .text("Дальность теней, м"),
                        );

                        section(ui, "ДЕНЬ И НОЧЬ");
                        ui.checkbox(&mut config.day_night.enabled, "Цикл дня и ночи");
                        ui.add(
                            egui::Slider::new(&mut day_night.hour, 0.0..=23.99)
                                .text("Время суток, ч"),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.day_night.day_length_secs, 60.0..=1800.0)
                                .text("Длина суток, сек"),
                        );

                        section(ui, "ИГРОК");
                        ui.add(
                            egui::Slider::new(&mut config.world.gravity, 1.0..=40.0)
                                .text("Гравитация (Луна = 3.7)"),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.player.run_speed, 4.0..=25.0)
                                .text("Скорость бега"),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.player.jump_velocity, 3.0..=20.0)
                                .text("Сила прыжка"),
                        );

                        section(ui, "МАШИНЫ");
                        ui.add(
                            egui::Slider::new(&mut config.driving.max_speed, 10.0..=60.0)
                                .text("Макс. скорость"),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.driving.accel, 6.0..=40.0).text("Разгон"),
                        );

                        section(ui, "ОРУЖИЕ");
                        ui.add(
                            egui::Slider::new(&mut config.weapons.pistol_damage, 5.0..=100.0)
                                .text("Урон пистолета"),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.weapons.fire_cooldown, 0.05..=1.0)
                                .text("Задержка выстрела, сек"),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.weapons.magazine, 4..=40)
                                .text("Магазин, патронов"),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.weapons.ped_hp, 10.0..=200.0)
                                .text("HP прохожих"),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.weapons.police_car_hp, 25.0..=300.0)
                                .text("HP машин полиции"),
                        );

                        section(ui, "ПОЛИЦИЯ");
                        ui.add(
                            egui::Slider::new(&mut config.police.chase_speed, 8.0..=30.0)
                                .text("Скорость погони"),
                        );
                        ui.add(
                            egui::Slider::new(&mut config.police.max_cars, 1..=8)
                                .text("Макс. машин копов"),
                        );
                    });

                ui.add_space(14.0);
                ui.vertical_centered_justified(|ui| {
                    let resume = egui::Button::new(
                        egui::RichText::new("▶  ПРОДОЛЖИТЬ")
                            .color(egui::Color32::BLACK)
                            .size(16.0)
                            .strong(),
                    )
                    .fill(egui::Color32::from_rgb(255, 204, 51));
                    if ui.add(resume).clicked() {
                        if let Ok(mut window) = windows.get_single_mut() {
                            input_state.cursor_locked = true;
                            window.cursor_options.visible = false;
                            window.cursor_options.grab_mode = CursorGrabMode::Locked;
                        }
                        virtual_time.unpause();
                    }
                    ui.add_space(6.0);
                    let quit = egui::Button::new(
                        egui::RichText::new("Выйти из игры")
                            .color(egui::Color32::WHITE)
                            .size(14.0),
                    )
                    .fill(egui::Color32::from_rgb(120, 30, 30));
                    if ui.add(quit).clicked() {
                        exit.send(AppExit::Success);
                    }
                });
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("ESC — вернуться в игру")
                            .color(egui::Color32::from_rgb(140, 140, 150))
                            .size(11.0),
                    );
                });
            });
    }
}

fn rotate2d(x: f32, y: f32, angle: f32) -> egui::Vec2 {
    let (s, c) = (angle.sin(), angle.cos());
    egui::vec2(x * c - y * s, x * s + y * c)
}
