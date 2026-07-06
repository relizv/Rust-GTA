# Mini GTA — Rust Edition (Bevy 0.15)

A browser-style mini-GTA ported from Three.js to **Bevy 0.15 + wgpu**. Visual style is intentionally preserved: blocky low-poly characters, windows on buildings, dashed road lines, fog, directional sun + cascaded shadows.

Gameplay mirrors the JS version:

- 6×6 grid of city blocks with roads, sidewalks, and ~50 procedurally-generated buildings (with window grids on every facade)
- Player character (third-person) with limb animation
- 14 AI cars driving on the road grid, turning at intersections
- 22 pedestrians walking on the sidewalks
- Walk up to a car and press **F** to steal it
- HUD: top-left info, top-right wanted stars, bottom-left minimap (rotates with view), bottom-right speedometer
- Wanted system: rises when you steal a car, decays after ~18s of good behavior
- Police: cop cars with flashing red/blue light bars spawn at 1+ stars and chase you; if they corner you — busted (fine + respawn). Stars don't decay while a cop is near you
- Day/night cycle (10-min day): moving sun, sunrise/sunset skies, windows glow and street lamps switch on at night; HUD clock
- FPS counter (top-left) and a central config file (`src/config.rs`) with all gameplay/graphics tunables
- Pause/settings menu on **ESC**: a true pause (the virtual clock freezes), live graphics & gameplay sliders, and a quit button
- Weapons & combat: a pistol (key **2**) with hitscan bullets, glowing tracers, RMB aiming (camera zoom + tight spread) and auto-reload; peds and cop cars have HP — kills pay cash but raise your wanted level, destroyed cop cars explode, and fresh peds respawn away from the player so the city never empties

## Controls

| Key | Action |
|-----|--------|
| WASD | Move / drive |
| Mouse | Look around |
| SHIFT | Sprint |
| SPACE | Jump |
| F | Enter / exit nearest car |
| LMB | Punch / fire the pistol |
| 1 / 2 | Switch to fists / pistol |
| RMB (hold) | Aim (zoom + tight spread) |
| R | Reset position |
| ESC | Pause & settings menu |

ESC is a true pause — the whole simulation freezes until you resume.

## Requirements

- Rust toolchain (1.80+). Install via [rustup](https://rustup.rs/).
- System graphics drivers (Vulkan on Linux/Windows, Metal on macOS).
- Linux additional packages:
  ```bash
  # Ubuntu / Debian
  sudo apt install libasound2-dev libudev-dev pkg-config \
                   libwayland-dev libxkbcommon-dev

  # Fedora
  sudo dnf install alsa-lib-devel systemd-devel wayland-devel \
                   libxkbcommon-devel pkg-config
  ```

## Build & Run

```bash
# Debug build (slower compile, fast iteration)
cargo run

# Release build (slower compile, smooth FPS)
cargo run --release
```

First build downloads and compiles Bevy + its dependency tree. Expect ~5–10 minutes for the first build; subsequent builds are much faster thanks to incremental compilation.

## Project Structure

```
mini-gta-rust/
├── Cargo.toml          # Pinned to bevy 0.15, bevy_egui 0.31, rand 0.8
├── .gitignore
├── README.md
└── src/
    ├── main.rs         # App entry: plugins, lights, fog, cascade shadows
    ├── config.rs       # ALL gameplay/graphics tunables in one place
    ├── daynight.rs     # Day/night cycle: sun, sky, night windows, street lamps
    ├── resources.rs    # Constants, GameState, InputState, GameAssets
    ├── input.rs        # Keyboard/mouse capture, pointer lock
    ├── city.rs         # Road grid, sidewalks, buildings + windows
    ├── player.rs       # Player spawn, movement, limbs, enter/exit car, punch
    ├── car.rs          # Car spawn, AI navigation, player driving
    ├── pedestrian.rs   # Ped spawn + sidewalk AI
    ├── police.rs       # Cop cars: spawn on wanted, chase AI, flashing light bar
    ├── weapons.rs      # Pistol: hitscan shots, tracers, HP, corpses, explosions
    ├── settings.rs     # Applies pause-menu graphics changes to live engine state
    ├── camera.rs       # Smooth third-person follow camera
    └── hud.rs          # egui HUD (info, minimap, speedo, stars, start overlay)
```

## Bevy 0.15 API notes (vs. older versions)

This codebase targets Bevy 0.15. Key API differences from older versions that are reflected in the code:

| Concept | Bevy 0.13 / 0.14 | Bevy 0.15 (this code) |
|---|---|---|
| Input state | `Res<Input<KeyCode>>` | `Res<ButtonInput<KeyCode>>` |
| Key codes | `KeyCode::W` | `KeyCode::KeyW` |
| Event reading | `EventReader::iter()` | `EventReader::read()` |
| Fog | `FogSettings` | `DistanceFog` |
| Directional shadows | `DirectionalLight::shadow_projection` | `CascadeShadowConfigBuilder` (separate component) |
| Mesh primitives | `shape::Box`, `shape::Plane`, `shape::Quad`, `shape::Cylinder` (deprecated) | `Cuboid`, `Plane3d`, `Rectangle`, `bevy::math::Cylinder` |
| egui access | `ResMut<EguiContext>` + `ctx_mut()` | `EguiContexts` system param + `contexts.ctx_mut()` |
| Vec3 with_y | `Vec3::with_y(y)` | `Vec3::new(v.x, y, v.z)` (manual) |
| Hemisphere light | `HemisphereLightBundle` | Removed in 0.15 — replaced with ambient + directional |

## Known Limitations / TODO

- **Pedestrian clothing colors are shared** with the player's. To restore per-ped variety, add an `Assets<StandardMaterial>` parameter to `spawn_peds` and create per-ped materials.
- **No audio** (Bevy has `bevy_audio` if you want engine/siren sounds).
- **AI cars don't avoid each other** — they can clip through one another at intersections.
- **No save/load** of game state.

## License

MIT — do whatever you want with this code.
