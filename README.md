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
cargo run -- --out out/tilesheet/tree.png --config configs/tile/tree.config
cargo run -- --out out/tilesheet/bush.png --config configs/tile/bush.config
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

## Notes
- Pass 1 renders the base isometric grass tile.
- Pass 2 adds simple grass blades with random height and color variation.

## Ubuntu WSL dependencies

These packages are required on Ubuntu WSL to build the Bevy example:

```bash
sudo apt-get update
sudo apt-get install -y pkg-config libasound2-dev libudev-dev
```

## Windows build tools

On Windows, you may need the Visual Studio Build Tools with the "Desktop development with C++" workload (MSVC) installed so native dependencies can compile.
