use clap::Parser;
use std::path::{Path, PathBuf};

use crate::config::{
    load_tile_config, output_path_for_config, require_field, TileConfig, TilesheetEntry,
    TransitionOverrides, DEFAULT_OUT_DIR, TILESET_CONFIG_DIR,
};
use crate::render::{parse_hex_color, render_tile, render_tilesheet, render_tilesheet_mask};
use spriteforge_assets::{TileMetadata, TilesheetMetadata};

mod config;
mod render;
mod tree;

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
            "Tile config directory not found: {TILESET_CONFIG_DIR}"
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
    let out_path = output_path_for_config(config_path, args.out.as_ref(), DEFAULT_OUT_DIR);
    let tile_config = load_tile_config(config_path)?;
    let image = build_from_tile_config(&tile_config, config_path, args, &out_path)?;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    image.save(&out_path).map_err(|e| e.to_string())?;
    println!("Saved sprite to {}", out_path.display());
    Ok(())
}

fn build_from_tile_config(
    tile_config: &TileConfig,
    config_path: &Path,
    args: &Args,
    out_path: &Path,
) -> Result<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>, String> {
    let size_override = args.size;
    let mut sprite_width = require_field(tile_config.sprite_width, "sprite_width")?;
    let mut sprite_height = require_field(tile_config.sprite_height, "sprite_height")?;
    if let Some(override_size) = size_override {
        sprite_width = override_size;
        sprite_height = override_size;
    }
    let mut bg_hex = require_field(tile_config.bg.clone(), "bg")?;
    if let Some(override_bg) = args.bg.clone() {
        bg_hex = override_bg;
    }
    let bg = parse_hex_color(&bg_hex)?;
    let is_transition = matches!(
        tile_config.name.as_str(),
        "grass_transition" | "water_transition" | "path_transition" | "debug_weight"
    );
    let has_tilesheet = tile_config.tilesheet_count.is_some()
        || tile_config.tilesheet_seed_start.is_some()
        || is_transition;
    if !has_tilesheet {
        let seed = require_field(tile_config.seed, "seed")?;
        return render_tile(
            sprite_width,
            sprite_height,
            bg,
            seed,
            tile_config,
            None,
            None,
        );
    }

    let columns = require_field(tile_config.tilesheet_columns, "tilesheet_columns")?.max(1);
    let padding = require_field(tile_config.tilesheet_padding, "tilesheet_padding")?;
    let entries = build_tilesheet_entries(tile_config)?;
    let image = render_tilesheet(
        sprite_width,
        sprite_height,
        bg,
        tile_config,
        &entries,
        columns,
        padding,
    )?;
    if tile_config.name == "water"
        || tile_config.name == "water_transition"
        || tile_config.name == "tree"
    {
        let mask = render_tilesheet_mask(
            sprite_width,
            sprite_height,
            tile_config,
            &entries,
            columns,
            padding,
        )?;
        let mask_path = mask_output_path(out_path);
        if let Some(parent) = mask_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        mask.save(&mask_path).map_err(|e| e.to_string())?;
        println!("Saved tilesheet mask to {}", mask_path.display());
    }
    write_tilesheet_metadata(
        out_path,
        &entries,
        sprite_width,
        sprite_height,
        columns,
        padding,
        config_path,
    )?;
    Ok(image)
}

fn build_tilesheet_entries(tile_config: &TileConfig) -> Result<Vec<TilesheetEntry>, String> {
    let seed_start = require_field(tile_config.tilesheet_seed_start, "tilesheet_seed_start")?;
    if matches!(
        tile_config.name.as_str(),
        "grass_transition" | "water_transition" | "path_transition" | "debug_weight"
    ) {
        let masks = spriteforge_assets::all_transition_masks();
        return Ok(masks
            .iter()
            .enumerate()
            .map(|(index, mask)| TilesheetEntry {
                seed: seed_start + index as u64,
                overrides: TransitionOverrides::default(),
                transition_mask: Some(*mask),
            })
            .collect());
    }
    let count = require_field(tile_config.tilesheet_count, "tilesheet_count")? as usize;
    Ok((0..count)
        .map(|index| TilesheetEntry {
            seed: seed_start + index as u64,
            overrides: TransitionOverrides::default(),
            transition_mask: None,
        })
        .collect())
}

fn mask_output_path(out_path: &Path) -> std::path::PathBuf {
    let stem = out_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("tilesheet");
    let file_name = format!("{stem}_mask.png");
    out_path.with_file_name(file_name)
}

fn write_tilesheet_metadata(
    out_path: &Path,
    entries: &[TilesheetEntry],
    sprite_width: u32,
    sprite_height: u32,
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
        let x = col * sprite_width + padding * col;
        let y = row * sprite_height + padding * row;
        tiles.push(TileMetadata {
            index: i,
            row,
            col,
            x,
            y,
            width: sprite_width,
            height: sprite_height,
            seed: entry.seed,
            transition_mask: entry.transition_mask,
        });
    }

    let meta = TilesheetMetadata {
        image: out_path.to_string_lossy().to_string(),
        config: config_path.to_string_lossy().to_string(),
        sprite_width: Some(sprite_width),
        sprite_height: Some(sprite_height),
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
