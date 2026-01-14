use clap::Parser;
use std::path::{Path, PathBuf};

use crate::config::{
    load_config, load_tile_config, output_path_for_config, resolve_path, tilesheet_entries,
    ConfigFile, TileConfig, DEFAULT_OUT_DIR, TILESET_CONFIG_DIR,
};
use crate::render::{parse_hex_color, render_tile, render_tilesheet};

mod config;
mod render;

#[derive(Parser, Debug)]
#[command(name = "spriteforge", about = "Procedural sprite generator")]
pub struct Args {
    /// Output PNG path
    #[arg(long)]
    out: Option<PathBuf>,

    /// Image size in pixels (square)
    #[arg(long)]
    size: Option<u32>,

    /// Solid background color (hex)
    #[arg(long)]
    bg: Option<String>,

    /// Random seed for reproducibility
    #[arg(long)]
    seed: Option<u64>,

    /// Path to JSON config file
    #[arg(long)]
    config: Option<PathBuf>,
}

pub fn run() -> Result<(), String> {
    let args = Args::parse();
    if args.config.is_none()
        && args.out.is_none()
        && args.size.is_none()
        && args.bg.is_none()
        && args.seed.is_none()
    {
        build_all_tilesheets()?;
        return Ok(());
    }

    let config_path = args
        .config
        .as_ref()
        .ok_or("Config file is required unless running with no arguments")?;
    build_from_config_path(config_path, &args)?;
    Ok(())
}

fn build_all_tilesheets() -> Result<(), String> {
    let dir = Path::new(TILESET_CONFIG_DIR);
    if !dir.exists() {
        return Err(format!(
            "Tilesheet config directory not found: {TILESET_CONFIG_DIR}"
        ));
    }
    let mut found = false;
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("config") {
            continue;
        }
        found = true;
        let args = Args {
            out: None,
            size: None,
            bg: None,
            seed: None,
            config: None,
        };
        build_from_config_path(&path, &args)?;
    }
    if !found {
        return Err("No tilesheet configs found".to_string());
    }
    Ok(())
}

fn build_from_config_path(config_path: &Path, args: &Args) -> Result<(), String> {
    let config = load_config(config_path)?;
    let out_path = output_path_for_config(config_path, args.out.as_ref(), DEFAULT_OUT_DIR);

    let image = match config {
        ConfigFile::Tile(tile) => render_single_tile(&tile, config_path, args)?,
        ConfigFile::Tilesheet(sheet) => {
            let entries = tilesheet_entries(&sheet)?;
            if entries.is_empty() {
                return Err("Tilesheet must include seeds or variants".to_string());
            }
            let tile_path = resolve_path(config_path, &sheet.tile_config);
            let tile_config = load_tile_config(&tile_path)?;
            let size = args.size.or(tile_config.size).unwrap_or(256);
            let bg_hex = args
                .bg
                .clone()
                .or_else(|| tile_config.bg.clone())
                .unwrap_or_else(|| "#2b2f3a".to_string());
            let bg = parse_hex_color(&bg_hex)?;
            let columns = sheet.columns.unwrap_or(4).max(1);
            let padding = sheet.padding.unwrap_or(0);
            render_tilesheet(size, bg, &tile_config, &entries, columns, padding)?
        }
    };

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    image.save(&out_path).map_err(|e| e.to_string())?;
    println!("Saved sprite to {}", out_path.display());
    Ok(())
}

fn render_single_tile(
    tile: &TileConfig,
    _config_path: &Path,
    args: &Args,
) -> Result<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>, String> {
    let size = args.size.or(tile.size).unwrap_or(256);
    let bg_hex = args
        .bg
        .clone()
        .or_else(|| tile.bg.clone())
        .unwrap_or_else(|| "#2b2f3a".to_string());
    let bg = parse_hex_color(&bg_hex)?;
    let seed = args
        .seed
        .or(tile.seed)
        .unwrap_or_else(rand::random::<u64>);
    render_tile(size, bg, seed, tile, None)
}

