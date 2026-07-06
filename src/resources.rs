//! Shared constants and resources for the game.

use bevy::prelude::*;
use bevy::render::mesh::Mesh;

// ----- City config (mirrors the JS version) -----
pub const BLOCK: f32 = 36.0;
pub const ROAD_W: f32 = 10.0;
pub const GRID: usize = 6;
pub const SIDEWALK_W: f32 = 2.0;
pub const CITY_HALF: f32 = (BLOCK + ROAD_W) * GRID as f32 / 2.0;
pub const STEP: f32 = BLOCK + ROAD_W;

// ----- Global game state -----
#[derive(Resource)]
pub struct GameState {
    pub hp: f32,
    pub cash: i32,
    pub wanted: u32,
    pub wanted_decay_timer: f32,
    pub in_vehicle: Option<Entity>,
    pub toast: Option<(String, f32)>,
    pub started: bool,
    pub last_speed_kmh: f32,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            hp: 100.0,
            cash: 0,
            wanted: 0,
            wanted_decay_timer: 0.0,
            in_vehicle: None,
            toast: None,
            started: false,
            last_speed_kmh: 0.0,
        }
    }
}

impl GameState {
    pub fn show_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), 1.8));
    }

    pub fn add_wanted(&mut self, n: u32) {
        self.wanted = (self.wanted + n).min(5);
        self.wanted_decay_timer = 0.0;
    }
}

#[derive(Resource, Default)]
pub struct InputState {
    pub yaw: f32,
    pub pitch: f32,
    pub cursor_locked: bool,
}

#[derive(Resource, Default, Debug)]
pub struct KeysPressed {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub shift: bool,
    pub space: bool,
    pub f_pressed: bool,
    pub r_pressed: bool,
    pub e_pressed: bool,
}

#[derive(Resource)]
pub struct GameAssets {
    // Meshes
    pub mesh_unit_box: Handle<Mesh>,
    pub mesh_unit_plane: Handle<Mesh>,
    pub mesh_cylinder_wheel: Handle<Mesh>,
    pub mesh_player_torso: Handle<Mesh>,
    pub mesh_player_head: Handle<Mesh>,
    pub mesh_player_arm: Handle<Mesh>,
    pub mesh_player_leg: Handle<Mesh>,
    pub mesh_car_body: Handle<Mesh>,
    pub mesh_car_cabin: Handle<Mesh>,
    pub mesh_car_windshield: Handle<Mesh>,
    pub mesh_car_headlight: Handle<Mesh>,
    pub mesh_window: Handle<Mesh>,
    pub mesh_pistol: Handle<Mesh>,

    // Static environment materials
    pub mat_ground: Handle<StandardMaterial>,
    pub mat_road: Handle<StandardMaterial>,
    pub mat_sidewalk: Handle<StandardMaterial>,
    pub mat_line_white: Handle<StandardMaterial>,
    pub mat_line_yellow: Handle<StandardMaterial>,
    pub mat_window_off: Handle<StandardMaterial>,
    pub mat_window_on: Handle<StandardMaterial>,
    pub mat_roof: Handle<StandardMaterial>,
    pub mat_windshield: Handle<StandardMaterial>,
    pub mat_headlight: Handle<StandardMaterial>,
    pub mat_taillight: Handle<StandardMaterial>,
    pub mat_wheel: Handle<StandardMaterial>,

    // Player materials
    pub mat_player_shirt: Handle<StandardMaterial>,
    pub mat_player_skin: Handle<StandardMaterial>,
    pub mat_player_pants: Handle<StandardMaterial>,
    pub mat_player_hair: Handle<StandardMaterial>,

    // Vehicle body material pool
    pub mat_car_colors: Vec<Handle<StandardMaterial>>,
    // Building body material pool
    pub mat_building_colors: Vec<Handle<StandardMaterial>>,

    // Police materials (body, cabin, light bar + flashing lamp variants)
    pub mat_police_body: Handle<StandardMaterial>,
    pub mat_police_cabin: Handle<StandardMaterial>,
    pub mat_lightbar: Handle<StandardMaterial>,
    pub mat_lamp_red_on: Handle<StandardMaterial>,
    pub mat_lamp_red_off: Handle<StandardMaterial>,
    pub mat_lamp_blue_on: Handle<StandardMaterial>,
    pub mat_lamp_blue_off: Handle<StandardMaterial>,

    // Street lamps (head emissive is driven by the day/night cycle)
    pub mat_lamp_pole: Handle<StandardMaterial>,
    pub mat_lamp_head: Handle<StandardMaterial>,

    // Weapons: pistol in hand, bullet tracer, cop-car explosion flash
    pub mat_pistol: Handle<StandardMaterial>,
    pub mat_tracer: Handle<StandardMaterial>,
    pub mat_explosion: Handle<StandardMaterial>,
}

pub fn setup_game_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // --- Meshes (Bevy 0.15 primitives in bevy::math) ---
    // Note: Plane3d::new(normal) gives a 1x1 plane; for sized planes we use
    // direct struct construction with `half_size` (half of full dimensions).
    let mesh_unit_box = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let mesh_unit_plane = meshes.add(Plane3d {
        normal: Dir3::Y,
        half_size: Vec2::splat(0.5),
    });
    let mesh_cylinder_wheel = meshes.add(Cylinder::new(0.35, 0.25));
    let mesh_player_torso = meshes.add(Cuboid::new(0.55, 0.7, 0.3));
    let mesh_player_head = meshes.add(Cuboid::new(0.32, 0.34, 0.32));
    let mesh_player_arm = meshes.add(Cuboid::new(0.16, 0.6, 0.18));
    let mesh_player_leg = meshes.add(Cuboid::new(0.2, 0.7, 0.22));
    let mesh_car_body = meshes.add(Cuboid::new(2.0, 0.7, 4.2));
    let mesh_car_cabin = meshes.add(Cuboid::new(1.7, 0.7, 2.0));
    let mesh_car_windshield = meshes.add(Rectangle::new(1.6, 0.6));
    let mesh_car_headlight = meshes.add(Cuboid::new(0.3, 0.15, 0.05));
    let mesh_window = meshes.add(Rectangle::new(0.9, 1.4));
    let mesh_pistol = meshes.add(Cuboid::new(0.09, 0.14, 0.38));

    // --- Materials ---
    let lambert = |mats: &mut Assets<StandardMaterial>, color: Color| -> Handle<StandardMaterial> {
        mats.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.85,
            metallic: 0.0,
            ..default()
        })
    };
    let emissive =
        |mats: &mut Assets<StandardMaterial>, color: Color| -> Handle<StandardMaterial> {
            mats.add(StandardMaterial {
                base_color: Color::BLACK,
                emissive: color.into(),
                ..default()
            })
        };
    let unlit = |mats: &mut Assets<StandardMaterial>, color: Color| -> Handle<StandardMaterial> {
        mats.add(StandardMaterial {
            base_color: color,
            unlit: true,
            ..default()
        })
    };

    let mat_ground = lambert(&mut materials, Color::srgb(0.29, 0.36, 0.23));
    let mat_road = lambert(&mut materials, Color::srgb(0.13, 0.13, 0.15));
    let mat_sidewalk = lambert(&mut materials, Color::srgb(0.53, 0.53, 0.53));
    let mat_line_white = unlit(&mut materials, Color::WHITE);
    let mat_line_yellow = unlit(&mut materials, Color::srgb(1.0, 1.0, 0.0));
    let mat_window_off = materials.add(StandardMaterial {
        base_color: Color::srgb(0.13, 0.20, 0.27),
        emissive: Color::srgb(0.05, 0.07, 0.10).into(),
        double_sided: true,
        ..default()
    });
    let mat_window_on = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.93, 0.73),
        emissive: Color::srgb(0.7, 0.6, 0.3).into(),
        double_sided: true,
        ..default()
    });
    let mat_roof = lambert(&mut materials, Color::srgb(0.20, 0.20, 0.20));
    let mat_windshield = materials.add(StandardMaterial {
        base_color: Color::srgb(0.53, 0.67, 0.80),
        metallic: 0.5,
        perceptual_roughness: 0.2,
        ..default()
    });
    let mat_headlight = emissive(&mut materials, Color::srgb(1.0, 1.0, 0.80));
    let mat_taillight = emissive(&mut materials, Color::srgb(1.0, 0.13, 0.13));
    let mat_wheel = lambert(&mut materials, Color::srgb(0.07, 0.07, 0.07));

    let mat_player_shirt = lambert(&mut materials, Color::srgb(0.16, 0.36, 1.0));
    let mat_player_skin = lambert(&mut materials, Color::srgb(1.0, 0.80, 0.67));
    let mat_player_pants = lambert(&mut materials, Color::srgb(0.13, 0.13, 0.13));
    let mat_player_hair = lambert(&mut materials, Color::srgb(0.20, 0.10, 0.04));

    let car_palette: [Color; 8] = [
        Color::srgb(1.0, 0.20, 0.20),
        Color::srgb(0.20, 0.40, 1.0),
        Color::srgb(0.20, 0.80, 0.20),
        Color::srgb(1.0, 0.80, 0.20),
        Color::srgb(1.0, 1.0, 1.0),
        Color::srgb(0.13, 0.13, 0.13),
        Color::srgb(1.0, 0.53, 0.0),
        Color::srgb(0.53, 0.27, 1.0),
    ];
    let car_colors: Vec<_> = car_palette
        .iter()
        .map(|c| lambert(&mut materials, *c))
        .collect();

    let building_palette: [Color; 8] = [
        Color::srgb(0.54, 0.54, 0.60),
        Color::srgb(0.42, 0.42, 0.48),
        Color::srgb(0.60, 0.55, 0.49),
        Color::srgb(0.36, 0.43, 0.49),
        Color::srgb(0.50, 0.55, 0.55),
        Color::srgb(0.69, 0.63, 0.56),
        Color::srgb(0.63, 0.35, 0.29),
        Color::srgb(0.29, 0.41, 0.54),
    ];
    let building_colors: Vec<_> = building_palette
        .iter()
        .map(|c| lambert(&mut materials, *c))
        .collect();

    // --- Police: white body, black cabin, roof light bar with red/blue
    // lamps. Each lamp has an "on" (bright emissive) and "off" (dim) variant;
    // `police::update_police` swaps them to make the bar flash. ---
    let mat_police_body = lambert(&mut materials, Color::srgb(0.95, 0.95, 0.97));
    let mat_police_cabin = lambert(&mut materials, Color::srgb(0.08, 0.08, 0.10));
    let mat_lightbar = lambert(&mut materials, Color::srgb(0.05, 0.05, 0.06));
    let mat_lamp_red_on = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.15, 0.15),
        emissive: LinearRgba::rgb(8.0, 0.3, 0.3),
        ..default()
    });
    let mat_lamp_red_off = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.04, 0.04),
        emissive: LinearRgba::rgb(0.1, 0.0, 0.0),
        ..default()
    });
    let mat_lamp_blue_on = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.45, 1.0),
        emissive: LinearRgba::rgb(0.4, 1.5, 8.0),
        ..default()
    });
    let mat_lamp_blue_off = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.08, 0.3),
        emissive: LinearRgba::rgb(0.0, 0.02, 0.1),
        ..default()
    });

    // --- Street lamps: dark pole + a head whose emissive is switched on at
    // night by `daynight::update_day_night` (one shared material for all). ---
    let mat_lamp_pole = lambert(&mut materials, Color::srgb(0.16, 0.17, 0.18));
    let mat_lamp_head = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.75, 0.70),
        emissive: LinearRgba::BLACK,
        ..default()
    });

    // --- Weapons: pistol in hand, bullet tracer, cop-car explosion flash ---
    let mat_pistol = lambert(&mut materials, Color::srgb(0.10, 0.10, 0.12));
    let mat_tracer = unlit(&mut materials, Color::srgb(1.0, 0.9, 0.45));
    let mat_explosion = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.55, 0.1),
        emissive: LinearRgba::rgb(8.0, 3.0, 0.6),
        ..default()
    });

    commands.insert_resource(GameAssets {
        mesh_unit_box,
        mesh_unit_plane,
        mesh_cylinder_wheel,
        mesh_player_torso,
        mesh_player_head,
        mesh_player_arm,
        mesh_player_leg,
        mesh_car_body,
        mesh_car_cabin,
        mesh_car_windshield,
        mesh_car_headlight,
        mesh_window,
        mesh_pistol,
        mat_ground,
        mat_road,
        mat_sidewalk,
        mat_line_white,
        mat_line_yellow,
        mat_window_off,
        mat_window_on,
        mat_roof,
        mat_windshield,
        mat_headlight,
        mat_taillight,
        mat_wheel,
        mat_player_shirt,
        mat_player_skin,
        mat_player_pants,
        mat_player_hair,
        mat_car_colors: car_colors,
        mat_building_colors: building_colors,
        mat_police_body,
        mat_police_cabin,
        mat_lightbar,
        mat_lamp_red_on,
        mat_lamp_red_off,
        mat_lamp_blue_on,
        mat_lamp_blue_off,
        mat_lamp_pole,
        mat_lamp_head,
        mat_pistol,
        mat_tracer,
        mat_explosion,
    });
}
