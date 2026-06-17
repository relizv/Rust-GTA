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

## Controls

| Key | Action |
|-----|--------|
| WASD | Move / drive |
| Mouse | Look around |
| SHIFT | Sprint |
| SPACE | Jump |
| F | Enter / exit nearest car |
| LMB | Punch (knock back peds, +$5 each) |
| R | Reset position |
| ESC | Release cursor (pause) |

Click anywhere in the window to re-lock the cursor and resume play.

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
    ├── resources.rs    # Constants, GameState, InputState, GameAssets
    ├── input.rs        # Keyboard/mouse capture, pointer lock
    ├── city.rs         # Road grid, sidewalks, buildings + windows
    ├── player.rs       # Player spawn, movement, limbs, enter/exit car, punch
    ├── car.rs          # Car spawn, AI navigation, player driving
    ├── pedestrian.rs   # Ped spawn + sidewalk AI
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
- **Performance**: hundreds of building-window quads are spawned individually. If FPS drops, consider baking windows into a single texture per building.

## License

MIT — do whatever you want with this code.
