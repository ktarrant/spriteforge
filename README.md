# Spriteforge

Procedural isometric sprite generator (no AI). This project uses Rust and the `image`
crate to render pixel-art style sprites based on simple prompt rules.

## Quick start

```bash
cargo run -- --out out/grass.png --config configs/tile/grass.config
cargo run -- --out out/tilesheet/grass.png --config configs/tilesheet/grass.config
cargo run -- --out out/dirt.png --config configs/tile/dirt.config
cargo run -- --out out/dirt_to_grass.png --config configs/tile/dirt_to_grass.config
cargo run -- --out out/tilesheet/dirt_to_grass.png --config configs/tilesheet/dirt_to_grass.config
```

Build all tilesheets (no args):
```bash
cargo run
```

Bevy tilesheet viewer (workspace crate):
```bash
cargo run -p spriteforge_bevy --example view_tilesheet
```

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

Transition tile example (dirt to grass):

```json
{
  "type": "tile",
  "name": "transition",
  "size": 256,
  "bg": "#2b2f3a",
  "seed": 777,
  "dirt_base": "#765234",
  "dirt_splotches": ["#896548", "#7b583d"],
  "dirt_stones": ["#4b5057", "#3e4349"],
  "dirt_splotch_count": 28,
  "dirt_stone_count": 10,
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

Transition tilesheet example (dirt to grass):

```json
{
  "type": "tilesheet",
  "tile_config": "../tile/dirt_to_grass.config",
  "variants": [
    {"seed": 101, "angles": [153.435]},
    {"seed": 102, "angles": [153.435]},
    {"seed": 103, "angles": [153.435]},
    {"seed": 104, "angles": [153.435]},
    {"seed": 105, "angles": [26.5]},
    {"seed": 106, "angles": [26.5]},
    {"seed": 107, "angles": [26.5]},
    {"seed": 108, "angles": [26.5]},
    {"seed": 109, "angles": [206.565]},
    {"seed": 110, "angles": [206.565]},
    {"seed": 111, "angles": [206.565]},
    {"seed": 112, "angles": [206.565]},
    {"seed": 113, "angles": [333.435]},
    {"seed": 114, "angles": [333.435]},
    {"seed": 115, "angles": [333.435]},
    {"seed": 116, "angles": [333.435]},
    {"seed": 201, "angles": [153.435, 26.5]},
    {"seed": 202, "angles": [153.435, 26.5]},
    {"seed": 203, "angles": [153.435, 26.5]},
    {"seed": 204, "angles": [153.435, 26.5]},
    {"seed": 205, "angles": [153.435, 206.565]},
    {"seed": 206, "angles": [153.435, 206.565]},
    {"seed": 207, "angles": [153.435, 206.565]},
    {"seed": 208, "angles": [153.435, 206.565]},
    {"seed": 209, "angles": [153.435, 333.435]},
    {"seed": 210, "angles": [153.435, 333.435]},
    {"seed": 211, "angles": [153.435, 333.435]},
    {"seed": 212, "angles": [153.435, 333.435]},
    {"seed": 213, "angles": [26.5, 206.565]},
    {"seed": 214, "angles": [26.5, 206.565]},
    {"seed": 215, "angles": [26.5, 206.565]},
    {"seed": 216, "angles": [26.5, 206.565]},
    {"seed": 217, "angles": [26.5, 333.435]},
    {"seed": 218, "angles": [26.5, 333.435]},
    {"seed": 219, "angles": [26.5, 333.435]},
    {"seed": 220, "angles": [26.5, 333.435]},
    {"seed": 221, "angles": [206.565, 333.435]},
    {"seed": 222, "angles": [206.565, 333.435]},
    {"seed": 223, "angles": [206.565, 333.435]},
    {"seed": 224, "angles": [206.565, 333.435]},
    {"seed": 301, "angles": [153.435, 26.5, 206.565]},
    {"seed": 302, "angles": [153.435, 26.5, 206.565]},
    {"seed": 303, "angles": [153.435, 26.5, 206.565]},
    {"seed": 304, "angles": [153.435, 26.5, 206.565]},
    {"seed": 305, "angles": [153.435, 26.5, 333.435]},
    {"seed": 306, "angles": [153.435, 26.5, 333.435]},
    {"seed": 307, "angles": [153.435, 26.5, 333.435]},
    {"seed": 308, "angles": [153.435, 26.5, 333.435]},
    {"seed": 309, "angles": [153.435, 206.565, 333.435]},
    {"seed": 310, "angles": [153.435, 206.565, 333.435]},
    {"seed": 311, "angles": [153.435, 206.565, 333.435]},
    {"seed": 312, "angles": [153.435, 206.565, 333.435]},
    {"seed": 313, "angles": [26.5, 206.565, 333.435]},
    {"seed": 314, "angles": [26.5, 206.565, 333.435]},
    {"seed": 315, "angles": [26.5, 206.565, 333.435]},
    {"seed": 316, "angles": [26.5, 206.565, 333.435]},
    {"seed": 401, "angles": [153.435, 26.5, 206.565, 333.435]},
    {"seed": 402, "angles": [153.435, 26.5, 206.565, 333.435]},
    {"seed": 403, "angles": [153.435, 26.5, 206.565, 333.435]},
    {"seed": 404, "angles": [153.435, 26.5, 206.565, 333.435]}
  ],
  "columns": 4,
  "padding": 0
}
```
