# Spriteforge

Procedural isometric sprite generator (no AI). This project uses Rust and the `image`
crate to render pixel-art style sprites based on simple prompt rules.

## Quick start

```bash
cargo run -- --out out/grass.png --config configs/tile/grass.config
cargo run -- --out out/tilesheet/grass.png --config configs/tilesheet/grass.config
```

Build all tilesheets (no args):
```bash
cargo run
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

Running with no arguments will build every tilesheet config in `configs/tilesheet`
and write outputs to `out/tilesheet/<config-name>.png`.

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
