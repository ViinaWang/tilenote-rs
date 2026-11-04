mod hyprland;
mod markdown;
mod model;
mod ui;

use anyhow::{Context, Result};
use eframe::egui;
use std::{env, process::Command};
use model::{Paths, Settings};

fn main() {
    if let Err(e) = run() {
        eprintln!("TileNote: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let paths = Paths::discover()?;
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str).unwrap_or("settings") {
        "settings" => run_settings(paths),
        "tile" => {
            let id = args.get(1).context("Usage: tilenote tile <note-id>")?;
            model::validate_id(id)?;
            run_tile(paths, id)
        }
        "new" => {
            let title = args.get(1).map(String::as_str).unwrap_or("新笔记");
            let meta = model::create_note(&paths, title, &Settings::load(&paths))?;
            start_tile(&meta.id)?;
            println!("Created note: {}", meta.id);
            Ok(())
        }
        "quick" => {
            // A single existing quick note is reopened instead of creating endless duplicates.
            let existing = model::list_notes(&paths).into_iter().find(|m| m.tag == "速记");
            if let Some(meta) = existing {
                if !hyprland::focus_tile(&meta.id, meta.pinned).unwrap_or(false) { start_tile(&meta.id)?; }
            } else {
                let mut meta = model::create_note(&paths, "快速记录", &Settings::load(&paths))?;
                meta.tag = "速记".into();
                meta.preview = false;
                meta.save(&paths)?;
                start_tile(&meta.id)?;
            }
            Ok(())
        }
        "show" => {
            let count = hyprland::show_all(|id| {
                model::NoteMeta::load(&paths, id).map(|m| m.pinned).unwrap_or(true)
            })?;
            if count == 0 { open_latest_or_new(&paths)?; }
            Ok(())
        }
        "hide" => { hyprland::hide_all()?; Ok(()) }
        "toggle" => {
            let running = hyprland::tile_clients()?;
            if running.is_empty() { open_latest_or_new(&paths)?; }
            else if hyprland::any_visible()? { hyprland::hide_all()?; }
            else {
                hyprland::show_all(|id| model::NoteMeta::load(&paths, id)
                    .map(|m| m.pinned).unwrap_or(true))?;
            }
            Ok(())
        }
        "list" => {
            for note in model::list_notes(&paths) {
                println!("{}\t{}\t{}", note.id, note.tag, note.title);
            }
            Ok(())
        }
        "help" | "--help" | "-h" => { usage(); Ok(()) }
        other => anyhow::bail!("Unknown command: {other}. Run `tilenote help`."),
    }
}

fn usage() {
    println!("\
TileNote — floating Markdown tiles for Hyprland

  tilenote [settings]        Open standard preferences window
  tilenote new [title]       Create and open a new tile
  tilenote quick             Open/reuse a quick-capture tile
  tilenote tile <note-id>    Open an existing note tile
  tilenote toggle            Hide/show all running note tiles
  tilenote show | hide       Explicitly show/hide running tiles
  tilenote list              Print stored note IDs and titles
  tilenote help              Display this message

Data: ${{XDG_DATA_HOME:-~/.local/share}}/tilenote/notes
Config: ${{XDG_CONFIG_HOME:-~/.config}}/tilenote/settings.json
");
}

fn open_latest_or_new(paths: &Paths) -> Result<()> {
    if let Some(note) = model::list_notes(paths).into_iter().next() {
        start_tile(&note.id)
    } else {
        let meta = model::create_note(paths, "快速记录", &Settings::load(paths))?;
        start_tile(&meta.id)
    }
}

/// Launch tile in its own process to give Hyprland an independent native
/// toplevel surface. The settings window may exit without closing tile windows.
pub fn start_tile(id: &str) -> Result<()> {
    model::validate_id(id)?;
    let pinned = Paths::discover().ok()
        .and_then(|p| model::NoteMeta::load(&p, id).ok())
        .map(|m| m.pinned).unwrap_or(true);
    if hyprland::focus_tile(id, pinned).unwrap_or(false) { return Ok(()); }
    let exe = env::current_exe().context("resolving tilenote executable")?;
    Command::new(exe)
        .arg("tile").arg(id)
        .spawn()
        .context("starting tile window")?;
    Ok(())
}

fn run_settings(paths: Paths) -> Result<()> {
    // Focus an already open preferences window rather than open duplicates.
    if let Ok(clients) = hyprland::clients() {
        if let Some(existing) = clients.iter().find(|c| c.class == hyprland::SETTINGS_APP_ID) {
            let _ = Command::new("hyprctl")
                .args(["dispatch", "focuswindow", &format!("address:{}", existing.address)])
                .status();
            return Ok(());
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TileNote — 设置")
            .with_app_id(hyprland::SETTINGS_APP_ID)
            .with_inner_size(egui::vec2(960.0, 690.0))
            .with_min_inner_size(egui::vec2(700.0, 500.0)),
        ..Default::default()
    };
    eframe::run_native(hyprland::SETTINGS_APP_ID, options,
        Box::new(move |cc| Ok(Box::new(ui::settings::SettingsApp::new(cc, paths)))))
        .map_err(|e| anyhow::anyhow!("starting settings UI: {e}"))
}

fn run_tile(paths: Paths, id: &str) -> Result<()> {
    let config = Settings::load(&paths);
    let id_owned = id.to_owned();
    // A separate process has a separate Wayland app_id; compositor rules can
    // distinguish tiles from the regular settings window by class.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(hyprland::title_for(id))
            .with_app_id(hyprland::TILE_APP_ID)
            .with_inner_size(egui::vec2(config.tile_width, config.tile_height))
            .with_min_inner_size(egui::vec2(300.0, 190.0))
            .with_decorations(false)
            .with_resizable(true)
            .with_always_on_top(),
        ..Default::default()
    };
    eframe::run_native(hyprland::TILE_APP_ID, options,
        Box::new(move |cc| {
            let app = ui::tile::TileApp::new(cc, paths, &id_owned)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    Box::new(std::io::Error::other(e.to_string()))
                })?;
            Ok(Box::new(app))
        }))
        .map_err(|e| anyhow::anyhow!("starting note window: {e}"))
}
