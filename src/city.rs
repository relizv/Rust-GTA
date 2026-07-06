//! City generation: ground, roads, dashed lane lines, sidewalks, buildings.
//!
//! Performance notes — this file used to spawn ~10 000 entities (every single
//! window quad and every road dash was its own entity + draw call, and each
//! of them was also rendered into the shadow pass). Now:
//!
//! - ALL window quads across the whole city are merged into just TWO meshes
//!   (lit windows / dark windows) → 2 draw calls instead of thousands.
//! - ALL road dashes are merged into ONE mesh (they also used to allocate a
//!   brand-new mesh asset per dash, and were standing upright — Rectangle
//!   lies in the XY plane — so they are now rotated flat onto the road).
//! - Buildings, roads, ground and sidewalks reuse the shared unit meshes
//!   (scaled per entity), letting Bevy batch draws by mesh+material.
//! - Flat decals (ground, roads, dashes, windows) are excluded from the
//!   shadow pass with `NotShadowCaster`.

use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::{
    mesh::{Indices, Mesh},
    render_asset::RenderAssetUsages,
    render_resource::PrimitiveTopology,
};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, PI};

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

/// Accumulates many quads into a single mesh (positions/normals/uvs/indices).
#[derive(Default)]
struct QuadBatch {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl QuadBatch {
    /// Append a `w`×`h` quad (XY plane, +Z normal, centered at origin)
    /// transformed by `tf` into the batch.
    fn push_quad(&mut self, tf: Transform, w: f32, h: f32) {
        let m = tf.compute_matrix();
        let normal = m.transform_vector3(Vec3::Z).normalize().to_array();
        let base = self.positions.len() as u32;
        let (hw, hh) = (w / 2.0, h / 2.0);
        let corners = [
            Vec3::new(-hw, -hh, 0.0),
            Vec3::new(hw, -hh, 0.0),
            Vec3::new(hw, hh, 0.0),
            Vec3::new(-hw, hh, 0.0),
        ];
        let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        for (corner, uv) in corners.iter().zip(uvs) {
            self.positions.push(m.transform_point3(*corner).to_array());
            self.normals.push(normal);
            self.uvs.push(uv);
        }
        self.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    fn build(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

pub fn build_city(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut rng = rand::thread_rng();

    // Merged-geometry batches for the whole city.
    let mut windows_on = QuadBatch::default();
    let mut windows_off = QuadBatch::default();
    let mut dashes = QuadBatch::default();

    // --- Ground (grass) — shared unit plane, scaled ---
    commands.spawn((
        Mesh3d(assets.mesh_unit_plane.clone()),
        MeshMaterial3d(assets.mat_ground.clone()),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(800.0, 1.0, 800.0)),
        NotShadowCaster,
    ));

    // --- Roads along X and Z for each grid line ---
    let road_len = CITY_HALF * 2.0 + STEP;
    // Rotates an XY-plane quad flat onto the ground (normal becomes +Y).
    let flat = Quat::from_rotation_x(-FRAC_PI_2);

    for i in 0..=GRID {
        let coord = -CITY_HALF + i as f32 * STEP;

        // Road along X (varies X, fixed Z)
        commands.spawn((
            Mesh3d(assets.mesh_unit_plane.clone()),
            MeshMaterial3d(assets.mat_road.clone()),
            Transform::from_xyz(0.0, 0.02, coord).with_scale(Vec3::new(road_len, 1.0, ROAD_W)),
            NotShadowCaster,
        ));

        // Road along Z (varies Z, fixed X)
        commands.spawn((
            Mesh3d(assets.mesh_unit_plane.clone()),
            MeshMaterial3d(assets.mat_road.clone()),
            Transform::from_xyz(coord, 0.02, 0.0).with_scale(Vec3::new(ROAD_W, 1.0, road_len)),
            NotShadowCaster,
        ));

        // --- Dashed center line on each road (merged, lying flat) ---
        let mut x = -CITY_HALF;
        while x < CITY_HALF {
            dashes.push_quad(
                Transform::from_xyz(x + 1.5, 0.03, coord).with_rotation(flat),
                3.0,
                0.25,
            );
            dashes.push_quad(
                Transform::from_xyz(coord, 0.03, x + 1.5).with_rotation(flat),
                0.25,
                3.0,
            );
            x += 6.0;
        }
    }

    // --- Blocks (sidewalks + buildings) ---
    for ix in 0..GRID {
        for iz in 0..GRID {
            let cx = -CITY_HALF + ROAD_W / 2.0 + ix as f32 * STEP + BLOCK / 2.0;
            let cz = -CITY_HALF + ROAD_W / 2.0 + iz as f32 * STEP + BLOCK / 2.0;

            // Sidewalk pad — shared unit box, scaled
            commands.spawn((
                Mesh3d(assets.mesh_unit_box.clone()),
                MeshMaterial3d(assets.mat_sidewalk.clone()),
                Transform::from_xyz(cx, 0.15, cz).with_scale(Vec3::new(
                    BLOCK + SIDEWALK_W * 2.0,
                    0.3,
                    BLOCK + SIDEWALK_W * 2.0,
                )),
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
                        &assets,
                        &mut windows_on,
                        &mut windows_off,
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

    // --- Spawn the merged batches (3 entities for the entire city) ---
    if !windows_on.is_empty() {
        commands.spawn((
            Mesh3d(meshes.add(windows_on.build())),
            MeshMaterial3d(assets.mat_window_on.clone()),
            Transform::IDENTITY,
            NotShadowCaster,
        ));
    }
    if !windows_off.is_empty() {
        commands.spawn((
            Mesh3d(meshes.add(windows_off.build())),
            MeshMaterial3d(assets.mat_window_off.clone()),
            Transform::IDENTITY,
            NotShadowCaster,
        ));
    }
    if !dashes.is_empty() {
        commands.spawn((
            Mesh3d(meshes.add(dashes.build())),
            MeshMaterial3d(assets.mat_line_white.clone()),
            Transform::IDENTITY,
            NotShadowCaster,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_building<R: Rng>(
    commands: &mut Commands,
    assets: &GameAssets,
    windows_on: &mut QuadBatch,
    windows_off: &mut QuadBatch,
    cx: f32,
    cz: f32,
    w: f32,
    d: f32,
    h: f32,
    color_idx: usize,
    rng: &mut R,
) {
    // Body — shared unit box, scaled (batches with other buildings that use
    // the same material).
    commands.spawn((
        Mesh3d(assets.mesh_unit_box.clone()),
        MeshMaterial3d(assets.mat_building_colors[color_idx].clone()),
        Transform::from_xyz(cx, h / 2.0 + 0.3, cz).with_scale(Vec3::new(w, h, d)),
        Building { cx, cz, w, d, h },
    ));

    // Windows on each facade — appended to the city-wide merged batches.
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
            let batch: &mut QuadBatch = if rng.gen_bool(0.35) {
                &mut *windows_on
            } else {
                &mut *windows_off
            };

            batch.push_quad(Transform::from_xyz(cx + x, y, cz + d / 2.0 + 0.01), 0.9, 1.4);
            batch.push_quad(
                Transform::from_xyz(cx + x, y, cz - d / 2.0 - 0.01)
                    .with_rotation(Quat::from_rotation_y(PI)),
                0.9,
                1.4,
            );
        }

        // +X and -X facades
        for c in 0..cols_z {
            let z = -d / 2.0 + (c as f32 + 0.5) * (d / cols_z as f32);
            let batch: &mut QuadBatch = if rng.gen_bool(0.35) {
                &mut *windows_on
            } else {
                &mut *windows_off
            };

            batch.push_quad(
                Transform::from_xyz(cx + w / 2.0 + 0.01, y, cz + z)
                    .with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
                0.9,
                1.4,
            );
            batch.push_quad(
                Transform::from_xyz(cx - w / 2.0 - 0.01, y, cz + z)
                    .with_rotation(Quat::from_rotation_y(-FRAC_PI_2)),
                0.9,
                1.4,
            );
        }
    }

    // Roof accent
    commands.spawn((
        Mesh3d(assets.mesh_unit_box.clone()),
        MeshMaterial3d(assets.mat_roof.clone()),
        Transform::from_xyz(cx + w * 0.2, h + 0.6 + 0.3, cz - d * 0.15).with_scale(Vec3::new(
            w * 0.4,
            0.6,
            d * 0.4,
        )),
    ));
}
