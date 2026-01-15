use clap::Parser;
use std::path::{Path, PathBuf};

use crate::config::{
    load_config, load_tile_config, output_path_for_config, resolve_path, tilesheet_entries,
    ConfigFile, TileConfig, TilesheetEntry, DEFAULT_OUT_DIR, TILESET_CONFIG_DIR,
};
use crate::render::{parse_hex_color, render_tile, render_tilesheet, render_tilesheet_mask};
use serde::Serialize;

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
                .unwrap_or_else(|| "transparent".to_string());
            let bg = parse_hex_color(&bg_hex)?;
            let columns = sheet.columns.unwrap_or(4).max(1);
            let padding = sheet.padding.unwrap_or(0);
            let image = render_tilesheet(size, bg, &tile_config, &entries, columns, padding)?;
            if tile_config.name == "water" {
                let mask = render_tilesheet_mask(size, &entries, columns, padding)?;
                let mask_path = mask_output_path(&out_path);
                if let Some(parent) = mask_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                mask.save(&mask_path).map_err(|e| e.to_string())?;
                println!("Saved tilesheet mask to {}", mask_path.display());
            }
            write_tilesheet_metadata(
                &out_path,
                &entries,
                size,
                columns,
                padding,
                config_path,
            )?;
            image
        }
    };

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    image.save(&out_path).map_err(|e| e.to_string())?;
    println!("Saved sprite to {}", out_path.display());
    Ok(())
}

fn mask_output_path(out_path: &Path) -> std::path::PathBuf {
    let stem = out_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("tilesheet");
    let file_name = format!("{stem}_mask.png");
    out_path.with_file_name(file_name)
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
        .unwrap_or_else(|| "transparent".to_string());
    let bg = parse_hex_color(&bg_hex)?;
    let seed = args
        .seed
        .or(tile.seed)
        .unwrap_or_else(rand::random::<u64>);
    render_tile(size, bg, seed, tile, None, None)
}

#[derive(Debug, Serialize)]
struct TilesheetMetadata {
    image: String,
    config: String,
    tile_size: u32,
    columns: u32,
    rows: u32,
    padding: u32,
    tile_count: usize,
    tiles: Vec<TileMetadata>,
}

#[derive(Debug, Serialize)]
struct TileMetadata {
    index: usize,
    row: u32,
    col: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    seed: u64,
    angles: Vec<f32>,
}

fn write_tilesheet_metadata(
    out_path: &Path,
    entries: &[TilesheetEntry],
    tile_size: u32,
    columns: u32,
    padding: u32,
    config_path: &Path,
) -> Result<(), String> {
    let cols = columns.max(1);
    let rows = ((entries.len() as u32) + cols - 1) / cols;
    let mut tiles = Vec::with_capacity(entries.len());
    for (i, entry) in entries.iter().enumerate() {
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = col * tile_size + padding * col;
        let y = row * tile_size + padding * row;
        tiles.push(TileMetadata {
            index: i,
            row,
            col,
            x,
            y,
            width: tile_size,
            height: tile_size,
            seed: entry.seed,
            angles: entry.angles.clone().unwrap_or_default(),
        });
    }

    let meta = TilesheetMetadata {
        image: out_path.to_string_lossy().to_string(),
        config: config_path.to_string_lossy().to_string(),
        tile_size,
        columns: cols,
        rows,
        padding,
        tile_count: entries.len(),
        tiles,
    };

    let json = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    let meta_path = out_path.with_extension("json");
    if let Some(parent) = meta_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&meta_path, json).map_err(|e| e.to_string())?;
    println!("Saved tilesheet metadata to {}", meta_path.display());
    Ok(())
}
