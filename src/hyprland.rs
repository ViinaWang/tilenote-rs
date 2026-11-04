//! Hyprland-specific integration. Window stacking/pinning are compositor
//! responsibilities on Wayland; a normal xdg-toplevel cannot demand global
//! always-on-top regardless of other clients.
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
fn dispatch(action: &str, argument: &str) -> Result<()> {
    let out = Command::new("hyprctl")
        .args(["dispatch", action, argument]).output()
        .with_context(|| format!("running hyprctl dispatch {action}"))?;
    anyhow::ensure!(out.status.success(), "Hyprland refused {action}: {}", String::from_utf8_lossy(&out.stderr));
    Ok(())
}

pub fn clients() -> Result<Vec<Client>> { json(&["clients", "-j"]) }
pub fn tile_clients() -> Result<Vec<Client>> {
    Ok(clients()?.into_iter().filter(|c| c.class == TILE_APP_ID).collect())
}
pub fn find_tile(id: &str) -> Result<Option<Client>> {
    Ok(tile_clients()?.into_iter().find(|c| c.title == title_for(id)))
}
fn target(c: &Client) -> String { format!("address:{}", c.address) }
fn hidden(c: &Client) -> bool {
    c.workspace.as_ref().is_some_and(|w| w.name == HIDDEN_WORKSPACE)
}
fn current_workspace() -> Result<i32> {
    Ok(json::<Workspace>(&["activeworkspace", "-j"])?.id)
}

pub fn make_tile_float_and_pin(id: &str, want_pin: bool) -> Result<()> {
    let c = find_tile(id)?.context("tile window not found")?;
    if !c.floating { dispatch("setfloating", &target(&c))?; }
    if c.pinned != want_pin { dispatch("pin", &target(&c))?; }
    let _ = dispatch("alterzorder", &format!("top,{}", target(&c)));
    Ok(())
}

pub fn pin_when_ready(id: String, want_pin: bool) {
    thread::spawn(move || {
        for _ in 0..30 {
            thread::sleep(Duration::from_millis(160));
            if matches!(find_tile(&id), Ok(Some(_))) {
                let _ = make_tile_float_and_pin(&id, want_pin);
                return;
            }
        }
    });
}

pub fn set_pin(id: &str, pinned: bool) -> Result<()> {
    make_tile_float_and_pin(id, pinned)
}

pub fn focus_tile(id: &str, desired_pin: bool) -> Result<bool> {
    if let Some(c) = find_tile(id)? {
        if hidden(&c) {
            show_client(&c, current_workspace()?, desired_pin)?;
        }
        dispatch("focuswindow", &target(&c))?;
        let _ = dispatch("alterzorder", &format!("top,{}", target(&c)));
        return Ok(true);
    }
    Ok(false)
}

fn hide_client(c: &Client) -> Result<()> {
    if hidden(c) { return Ok(()); }
    // Pinned windows ignore workspace changes, so unpin before hiding.
    if c.pinned { dispatch("pin", &target(c))?; }
    dispatch("movetoworkspacesilent", &format!("{HIDDEN_WORKSPACE},{}", target(c)))
}

fn show_client(c: &Client, ws: i32, pin: bool) -> Result<()> {
    dispatch("movetoworkspacesilent", &format!("{ws},{}", target(c)))?;
    if pin { dispatch("pin", &target(c))?; }
    let _ = dispatch("alterzorder", &format!("top,{}", target(c)));
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
        let _ = dispatch("focuswindow", &target(last));
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
