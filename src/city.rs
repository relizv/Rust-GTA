//! City generation: ground, roads, dashed lane lines, sidewalks, buildings.

use bevy::math::{Cuboid, Plane3d, Rectangle};
use bevy::prelude::*;
use rand::Rng;

use crate::resources::{GameAssets, BLOCK, CITY_HALF, GRID, ROAD_W, SIDEWALK_W, STEP};

#[derive(Component)]
pub struct Building {
    pub cx: f32,
    pub cz: f32,
    pub w: f32,
    pub d: f32,
    pub h: f32,
}

#[derive(Component)]
pub struct SidewalkPad;

pub fn build_city(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut rng = rand::thread_rng();

    // --- Ground (grass) ---
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d {
            normal: Vec3::Y,
            half_size: Vec2::splat(400.0),
        }),
        material: assets.mat_ground.clone(),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });

    // --- Roads along X and Z for each grid line ---
    for i in 0..=GRID {
        let coord = -CITY_HALF + i as f32 * STEP;

        // Road along X (varies X, fixed Z)
        commands.spawn(PbrBundle {
            mesh: meshes.add(Plane3d {
                normal: Vec3::Y,
                half_size: Vec2::new((CITY_HALF * 2.0 + STEP) / 2.0, ROAD_W / 2.0),
            }),
            material: assets.mat_road.clone(),
            transform: Transform::from_xyz(0.0, 0.02, coord),
            ..default()
        });

        // Road along Z (varies Z, fixed X)
        commands.spawn(PbrBundle {
            mesh: meshes.add(Plane3d {
                normal: Vec3::Y,
                half_size: Vec2::new(ROAD_W / 2.0, (CITY_HALF * 2.0 + STEP) / 2.0),
            }),
            material: assets.mat_road.clone(),
            transform: Transform::from_xyz(coord, 0.02, 0.0),
            ..default()
        });

        // --- Dashed center line on each road ---
        let mut x = -CITY_HALF;
        while x < CITY_HALF {
            commands.spawn(PbrBundle {
                mesh: meshes.add(Rectangle::new(3.0, 0.25)),
                material: assets.mat_line_white.clone(),
                transform: Transform::from_xyz(x + 1.5, 0.03, coord),
                ..default()
            });
            commands.spawn(PbrBundle {
                mesh: meshes.add(Rectangle::new(0.25, 3.0)),
                material: assets.mat_line_white.clone(),
                transform: Transform::from_xyz(coord, 0.03, x + 1.5),
                ..default()
            });
            x += 6.0;
        }
    }

    // --- Blocks (sidewalks + buildings) ---
    for ix in 0..GRID {
        for iz in 0..GRID {
            let cx = -CITY_HALF + ROAD_W / 2.0 + ix as f32 * STEP + BLOCK / 2.0;
            let cz = -CITY_HALF + ROAD_W / 2.0 + iz as f32 * STEP + BLOCK / 2.0;

            // Sidewalk pad
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Cuboid::new(
                        BLOCK + SIDEWALK_W * 2.0,
                        0.3,
                        BLOCK + SIDEWALK_W * 2.0,
                    )),
                    material: assets.mat_sidewalk.clone(),
                    transform: Transform::from_xyz(cx, 0.15, cz),
                    ..default()
                },
                SidewalkPad,
            ));

            // Buildings inside the block
            let subdivs = 1 + rng.gen_range(0..3); // 1..=3
            let sub = BLOCK / subdivs as f32;
            for bx in 0..subdivs {
                for bz in 0..subdivs {
                    if rng.gen_bool(0.15) {
                        continue; // empty lot
                    }
                    let bcx = cx - BLOCK / 2.0 + sub / 2.0 + bx as f32 * sub;
                    let bcz = cz - BLOCK / 2.0 + sub / 2.0 + bz as f32 * sub;
                    let w = sub * (0.55 + rng.gen::<f32>() * 0.3);
                    let d = sub * (0.55 + rng.gen::<f32>() * 0.3);
                    let h = 6.0 + rng.gen::<f32>() * 28.0;
                    let color_idx = rng.gen_range(0..assets.mat_building_colors.len());
                    spawn_building(
                        &mut commands,
                        assets,
                        &mut meshes,
                        bcx,
                        bcz,
                        w,
                        d,
                        h,
                        color_idx,
                        &mut rng,
                    );
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_building<R: Rng>(
    commands: &mut Commands,
    assets: &GameAssets,
    meshes: &mut ResMut<Assets<Mesh>>,
    cx: f32,
    cz: f32,
    w: f32,
    d: f32,
    h: f32,
    color_idx: usize,
    rng: &mut R,
) {
    // Body
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(w, h, d)),
                material: assets.mat_building_colors[color_idx].clone(),
                transform: Transform::from_xyz(cx, h / 2.0 + 0.3, cz),
                ..default()
            },
            Building { cx, cz, w, d, h },
        ))
        .id();

    // Windows on each facade.
    // The building body is positioned at y = h/2 + 0.3 (its center), so its
    // base is at y = 0.3. Windows are placed at world Y = 0.3 + 1.5 + f * 3.0.
    let floors = (h / 3.2).floor().max(1.0) as i32;
    let cols_x = (w / 2.2).floor().max(1.0) as i32;
    let cols_z = (d / 2.2).floor().max(1.0) as i32;

    for f in 0..floors {
        let y = 0.3 + 1.5 + f as f32 * 3.0;

        // +Z and -Z facades
        for c in 0..cols_x {
            let x = -w / 2.0 + (c as f32 + 0.5) * (w / cols_x as f32);
            let on = rng.gen_bool(0.35);
            let mat = if on {
                assets.mat_window_on.clone()
            } else {
                assets.mat_window_off.clone()
            };

            commands.spawn(PbrBundle {
                mesh: assets.mesh_window.clone(),
                material: mat.clone(),
                transform: Transform::from_xyz(cx + x, y, cz + d / 2.0 + 0.01),
                ..default()
            });
            commands.spawn(PbrBundle {
                mesh: assets.mesh_window.clone(),
                material: mat,
                transform: Transform::from_xyz(cx + x, y, cz - d / 2.0 - 0.01)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                ..default()
            });
        }

        // +X and -X facades
        for c in 0..cols_z {
            let z = -d / 2.0 + (c as f32 + 0.5) * (d / cols_z as f32);
            let on = rng.gen_bool(0.35);
            let mat = if on {
                assets.mat_window_on.clone()
            } else {
                assets.mat_window_off.clone()
            };

            commands.spawn(PbrBundle {
                mesh: assets.mesh_window.clone(),
                material: mat.clone(),
                transform: Transform::from_xyz(cx + w / 2.0 + 0.01, y, cz + z)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI / 2.0)),
                ..default()
            });
            commands.spawn(PbrBundle {
                mesh: assets.mesh_window.clone(),
                material: mat,
                transform: Transform::from_xyz(cx - w / 2.0 - 0.01, y, cz + z)
                    .with_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 2.0)),
                ..default()
            });
        }
    }

    // Roof accent
    commands.spawn(PbrBundle {
        mesh: assets.mesh_unit_box.clone(),
        material: assets.mat_roof.clone(),
        transform: Transform::from_xyz(cx + w * 0.2, h + 0.6 + 0.3, cz - d * 0.15)
            .with_scale(Vec3::new(w * 0.4, 0.6, d * 0.4)),
        ..default()
    });
}
