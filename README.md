# Spriteforge

Procedural isometric sprite generator (no AI). This project uses Rust and the `image`
crate to render pixel-art style sprites based on simple prompt rules.

## Quick start

```bash
cargo run -- --out out/tilesheet/grass.png --config configs/tile/grass.config
cargo run -- --out out/tilesheet/grass_transition.png --config configs/tile/grass_transition.config
cargo run -- --out out/tilesheet/water.png --config configs/tile/water.config
cargo run -- --out out/tilesheet/water_transition.png --config configs/tile/water_transition.config
cargo run -- --out out/tilesheet/path.png --config configs/tile/path.config
cargo run -- --out out/tilesheet/path_transition.png --config configs/tile/path_transition.config
cargo run -- --out out/tilesheet/dirt.png --config configs/tile/dirt.config
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
`out/tilesheet/grass_transition.png` + `.json`, `out/tilesheet/water.png` + `.json`,
`out/tilesheet/water_transition.png` + `.json`, `out/tilesheet/path.png` + `.json`,
and `out/tilesheet/path_transition.png` + `.json`.

## CLI

```bash
spriteforge --out out/grass.png --config configs/tile/grass.config
spriteforge --out out/tilesheet/grass.png --config configs/tile/grass.config
```

## Notes
- Pass 1 renders the base isometric grass tile.
- Pass 2 adds simple grass blades with random height and color variation.

## Config

All tweakable settings live in a JSON config file. Tilesheets are generated from
the same tile config using `tilesheet_*` fields.

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

Running with no arguments will build every tile config in `configs/tile`
and write outputs to `out/tilesheet/<config-name>.png`.
Tilesheet builds also write metadata JSON next to the image (same name, `.json`).
Water tilesheets (including water transitions) also emit a mask PNG named `<config-name>_mask.png`.

Debug weight visualization (manual run only):
```bash
cargo run -- --out out/tilesheet/debug_weight.png --config configs/debug/debug_weight.config
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
  "transition_density": 0.35,
  "transition_bias": 0.85,
  "transition_falloff": 2.2,
  "grass_base": "#205c3e",
  "grass_shades": ["#2f6f4a", "#3f8f5e", "#58b174"],
  "tilesheet_seed_start": 101,
  "tilesheet_columns": 4,
  "tilesheet_padding": 0
}
```

Water transition tile example (transparent edge cutout):

```json
{
  "type": "tile",
  "name": "water_transition",
  "size": 256,
  "bg": "transparent",
  "seed": 5555,
  "water_base": "#1c3f66",
  "water_edge_cutoff": 0.78,
  "tilesheet_seed_start": 201,
  "tilesheet_columns": 4,
  "tilesheet_padding": 0
}
```

Path transition tile example (transparent edge cutout):

```json
{
  "type": "tile",
  "name": "path_transition",
  "size": 256,
  "bg": "transparent",
  "seed": 6060,
  "path_base": "#6b6b6b",
  "path_edge_cutoff": 1,
  "path_brick_count": 8,
  "path_brick_crack": 0.1,
  "tilesheet_seed_start": 401,
  "tilesheet_columns": 4,
  "tilesheet_padding": 0
}
```
