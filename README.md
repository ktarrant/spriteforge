# Spriteforge

Procedural isometric sprite generator (no AI). This project uses Rust and the `image`
crate to render pixel-art style sprites based on simple prompt rules.

## Quick start

```bash
cargo run -- --out out/grass.png --config configs/tile/grass.config
cargo run -- --out out/tilesheet/grass.png --config configs/tilesheet/grass.config
cargo run -- --out out/dirt.png --config configs/tile/dirt.config
cargo run -- --out out/grass_transition.png --config configs/tile/grass_transition.config
cargo run -- --out out/tilesheet/grass_transition.png --config configs/tilesheet/grass_transition.config
```

Build all tilesheets (no args):
```bash
cargo run
```

Bevy tilesheet viewer (workspace crate):
```bash
cargo run -p spriteforge_bevy --example view_tilesheet
```
The demo expects `out/tilesheet/grass.png` + `.json`, `out/tilesheet/dirt.png` + `.json`,
and `out/tilesheet/grass_transition.png` + `.json`.

## CLI

```bash
spriteforge --out out/grass.png --config configs/tile/grass.config
spriteforge --out out/tilesheet/grass.png --config configs/tilesheet/grass.config
```

## Notes
- Pass 1 renders the base isometric grass tile.
- Pass 2 adds simple grass blades with random height and color variation.

## Config

All tweakable settings can live in a JSON config file. The `type` field selects
`tile` or `tilesheet`.

```json
{
  "type": "tile",
  "name": "grass",
  "size": 256,
  "bg": "#2b2f3a",
  "seed": 1234,
  "blade_min": 1,
  "blade_max": 6,
  "grass_base": "#205c3e",
  "grass_shades": ["#2f6f4a", "#3f8f5e", "#58b174"]
}
```

CLI flags (like `--size` or `--bg`) override values in the config when provided.
Use `bg: "transparent"` for a transparent background.

Running with no arguments will build every tilesheet config in `configs/tilesheet`
and write outputs to `out/tilesheet/<config-name>.png`.
Tilesheet builds also write metadata JSON next to the image (same name, `.json`).

Debug weight visualization (manual run only):
```bash
cargo run -- --out out/tilesheet/debug_weight.png --config configs/debug/debug_weight_tileset.config
```

Tilesheet example:

```json
{
  "type": "tilesheet",
  "tile_config": "../tile/grass.config",
  "seeds": [11, 27, 43, 59, 71, 83, 97, 103, 127, 149, 173, 199],
  "columns": 4,
  "padding": 0
}
```

Dirt tile example:

```json
{
  "type": "tile",
  "name": "dirt",
  "size": 256,
  "bg": "#2b2f3a",
  "seed": 4242,
  "dirt_base": "#6b4a2b",
  "dirt_splotches": ["#6a4a2f", "#5c3f27"],
  "dirt_stones": ["#4b5057", "#3e4349"],
  "dirt_splotch_count": 18,
  "dirt_stone_count": 10
}
```

Transition tile example (grass overlay):

```json
{
  "type": "tile",
  "name": "grass_transition",
  "size": 256,
  "bg": "transparent",
  "seed": 777,
  "blade_min": 1,
  "blade_max": 6,
  "transition_angle": 333.435,
  "transition_density": 0.35,
  "transition_bias": 0.85,
  "transition_falloff": 2.2,
  "grass_base": "#205c3e",
  "grass_shades": ["#2f6f4a", "#3f8f5e", "#58b174"]
}
```

Transition tilesheet example (grass overlay):

```json
{
  "type": "tilesheet",
  "tile_config": "../tile/grass_transition.config",
  "variants": [
    {"seed": 101, "angles": [153.435]},
    {"seed": 102, "angles": [26.5]},
    {"seed": 103, "angles": [206.565]},
    {"seed": 104, "angles": [333.435]},
    {"seed": 105, "angles": [0, 90, 180, 270], "density": 0.3, "falloff": 2.2}
  ],
  "columns": 4,
  "padding": 0
}
```

