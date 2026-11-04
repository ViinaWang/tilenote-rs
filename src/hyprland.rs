//! Hyprland-specific integration. Window stacking/pinning are compositor
//! responsibilities on Wayland; a normal xdg-toplevel cannot demand global
//! always-on-top regardless of other clients.
//!
//! Hyprland 0.55+ parses `hyprctl dispatch` arguments as Lua dispatcher
//! expressions (`hl.dsp.…`); the legacy `dispatch <action> <args>` form is
//! rejected there. All window actions below use the Lua API with explicit
//! `address:0x…` selectors so they work without changing focus.
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{process::Command, thread, time::Duration};

pub const TILE_APP_ID: &str = "io.github.tilenote.tile";
pub const SETTINGS_APP_ID: &str = "io.github.tilenote.settings";
const HIDDEN_WORKSPACE: &str = "special:tilenote-hidden";

pub fn title_for(id: &str) -> String {
    format!("TileNote :: {id}")
}

#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub name: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Client {
    pub address: String,
    pub class: String,
    pub title: String,
    #[serde(default)]
    pub floating: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub workspace: Option<Workspace>,
}

fn json<T: serde::de::DeserializeOwned>(args: &[&str]) -> Result<T> {
    let out = Command::new("hyprctl").args(args).output().context("hyprctl not installed/running")?;
    anyhow::ensure!(out.status.success(), "hyprctl error: {}", String::from_utf8_lossy(&out.stderr));
    Ok(serde_json::from_slice(&out.stdout)?)
}

/// Run a Lua dispatcher expression through hyprctl, e.g.
/// `hl.dsp.window.float({ action = "set", window = "address:0x55…" })`.
fn dispatch_lua(expr: &str) -> Result<()> {
    let out = Command::new("hyprctl")
        .args(["dispatch", expr])
        .output()
        .with_context(|| format!("running hyprctl dispatch {expr}"))?;
    anyhow::ensure!(
        out.status.success(),
        "Hyprland refused: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(())
}

fn sel(address: &str) -> String {
    format!("address:{address}")
}

fn float_window(address: &str) -> Result<()> {
    dispatch_lua(&format!(
        "hl.dsp.window.float({{ action = \"set\", window = \"{}\" }})",
        sel(address)
    ))
}

/// Pin toggles: a pinned window is visible on every workspace.
fn pin_window(address: &str) -> Result<()> {
    dispatch_lua(&format!(
        "hl.dsp.window.pin({{ window = \"{}\" }})",
        sel(address)
    ))
}

fn raise_window(address: &str) {
    let _ = dispatch_lua(&format!(
        "hl.dsp.window.alter_zorder({{ mode = \"top\", window = \"{}\" }})",
        sel(address)
    ));
}

fn focus_window(address: &str) -> Result<()> {
    dispatch_lua(&format!(
        "hl.dsp.focus({{ window = \"{}\" }})",
        sel(address)
    ))
}

fn move_to_workspace(address: &str, workspace: &str) -> Result<()> {
    dispatch_lua(&format!(
        "hl.dsp.window.move({{ workspace = \"{workspace}\", follow = false, window = \"{}\" }})",
        sel(address)
    ))
}

pub fn clients() -> Result<Vec<Client>> { json(&["clients", "-j"]) }
pub fn tile_clients() -> Result<Vec<Client>> {
    Ok(clients()?.into_iter().filter(|c| c.class == TILE_APP_ID).collect())
}
pub fn find_tile(id: &str) -> Result<Option<Client>> {
    Ok(tile_clients()?.into_iter().find(|c| c.title == title_for(id)))
}
fn hidden(c: &Client) -> bool {
    c.workspace.as_ref().is_some_and(|w| w.name == HIDDEN_WORKSPACE)
}
fn current_workspace() -> Result<i32> {
    Ok(json::<Workspace>(&["activeworkspace", "-j"])?.id)
}

pub fn make_tile_float_and_pin(id: &str, want_pin: bool) -> Result<()> {
    let c = find_tile(id)?.context("tile window not found")?;
    // Always dispatch float: it is idempotent with action=set and removes the
    // tiled-then-floated flicker when the window is registered late.
    float_window(&c.address)?;
    if c.pinned != want_pin { pin_window(&c.address)?; }
    raise_window(&c.address);
    Ok(())
}

/// Newly spawned windows register their app_id some time after the first
/// frame; poll fast at first, then keep retrying for a long while so slow
/// starts (cold caches, heavy compositor load) still get floated.
pub fn pin_when_ready(id: String, want_pin: bool) {
    thread::spawn(move || {
        let start = std::time::Instant::now();
        loop {
            let fast = start.elapsed() < Duration::from_secs(3);
            thread::sleep(if fast { Duration::from_millis(80) } else { Duration::from_millis(400) });
            match find_tile(&id) {
                Ok(Some(_)) => {
                    let _ = make_tile_float_and_pin(&id, want_pin);
                    return;
                }
                _ if start.elapsed() > Duration::from_secs(20) => return,
                _ => (),
            }
        }
    });
}

pub fn set_pin(id: &str, pinned: bool) -> Result<()> {
    make_tile_float_and_pin(id, pinned)
}

/// Bring an existing window to the front and focus it (used to reuse a
/// running settings window instead of spawning a duplicate).
pub fn focus_address(address: &str) -> Result<()> {
    focus_window(address)?;
    raise_window(address);
    Ok(())
}

pub fn focus_tile(id: &str, desired_pin: bool) -> Result<bool> {
    if let Some(c) = find_tile(id)? {
        if hidden(&c) {
            show_client(&c, current_workspace()?, desired_pin)?;
        }
        focus_window(&c.address)?;
        raise_window(&c.address);
        return Ok(true);
    }
    Ok(false)
}

fn hide_client(c: &Client) -> Result<()> {
    if hidden(c) { return Ok(()); }
    // Pinned windows ignore workspace changes, so unpin before hiding.
    if c.pinned { pin_window(&c.address)?; }
    move_to_workspace(&c.address, HIDDEN_WORKSPACE)
}

fn show_client(c: &Client, ws: i32, pin: bool) -> Result<()> {
    move_to_workspace(&c.address, &ws.to_string())?;
    if pin { pin_window(&c.address)?; }
    raise_window(&c.address);
    Ok(())
}

/// The persistent pin preference is obtained from note metadata by the caller.
pub fn show_all(want_pin: impl Fn(&str) -> bool) -> Result<usize> {
    let ws = current_workspace()?;
    let tiles = tile_clients()?;
    for c in &tiles {
        if hidden(c) {
            let id = c.title.strip_prefix("TileNote :: ").unwrap_or_default();
            show_client(c, ws, want_pin(id))?;
        }
    }
    if let Some(last) = tiles.first() {
        let _ = focus_window(&last.address);
    }
    Ok(tiles.len())
}

pub fn hide_all() -> Result<usize> {
    let tiles = tile_clients()?;
    let mut visible = 0;
    for c in &tiles {
        if !hidden(c) {
            hide_client(c)?;
            visible += 1;
        }
    }
    Ok(visible)
}

pub fn any_visible() -> Result<bool> {
    Ok(tile_clients()?.iter().any(|c| !hidden(c)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn title_is_stable() { assert_eq!(title_for("n1"), "TileNote :: n1"); }
}
