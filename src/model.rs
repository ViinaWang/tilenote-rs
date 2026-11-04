use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Configuration and notes intentionally use XDG locations and plain JSON/Markdown.
#[derive(Debug, Clone)]
pub struct Paths {
    pub config: PathBuf,
    pub notes: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self> {
        let home = env::var_os("HOME").map(PathBuf::from).context("HOME is not set")?;
        let config = xdg_home("XDG_CONFIG_HOME", &home, ".config").join("tilenote");
        let data = xdg_home("XDG_DATA_HOME", &home, ".local/share").join("tilenote");
        let notes = data.join("notes");
        fs::create_dir_all(&config)?;
        fs::create_dir_all(&notes)?;
        Ok(Self { config, notes })
    }

    pub fn settings_path(&self) -> PathBuf {
        self.config.join("settings.json")
    }
    pub fn meta_path(&self, id: &str) -> PathBuf {
        self.notes.join(format!("{id}.json"))
    }
    pub fn markdown_path(&self, id: &str) -> PathBuf {
        self.notes.join(format!("{id}.md"))
    }
}

fn xdg_home(var: &str, home: &Path, default: &str) -> PathBuf {
    env::var_os(var)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(default))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    OneDarkPro,
    GithubLight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    pub accent: usize,
    pub font_size: f32,
    pub corner_radius: u8,
    pub shadow: bool,
    /// Hyprland pin = visible on every workspace (not a privileged overlay layer).
    pub auto_pin: bool,
    pub default_preview: bool,
    pub tile_width: f32,
    pub tile_height: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::OneDarkPro,
            accent: 0,
            font_size: 14.0,
            corner_radius: 9,
            shadow: true,
            auto_pin: true,
            default_preview: false,
            tile_width: 440.0,
            tile_height: 350.0,
        }
    }
}

impl Settings {
    pub fn load(paths: &Paths) -> Self {
        fs::read(paths.settings_path())
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, paths: &Paths) -> Result<()> {
        atomic_write(&paths.settings_path(), &serde_json::to_vec_pretty(self)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NoteMeta {
    pub id: String,
    pub title: String,
    pub tag: String,
    pub accent: usize,
    pub pinned: bool,
    pub preview: bool,
}

impl Default for NoteMeta {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: "新笔记".to_owned(),
            tag: "灵感".to_owned(),
            accent: 0,
            pinned: true,
            preview: false,
        }
    }
}

impl NoteMeta {
    pub fn load(paths: &Paths, id: &str) -> Result<Self> {
        validate_id(id)?;
        let bytes = fs::read(paths.meta_path(id))
            .with_context(|| format!("note metadata does not exist: {id}"))?;
        let meta: Self = serde_json::from_slice(&bytes)?;
        anyhow::ensure!(meta.id == id, "note metadata ID does not match filename");
        Ok(meta)
    }
    pub fn save(&self, paths: &Paths) -> Result<()> {
        validate_id(&self.id)?;
        atomic_write(&paths.meta_path(&self.id), &serde_json::to_vec_pretty(self)?)
    }
}

pub fn validate_id(id: &str) -> Result<()> {
    anyhow::ensure!(
        !id.is_empty() && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "invalid note id"
    );
    Ok(())
}

pub fn list_notes(paths: &Paths) -> Vec<NoteMeta> {
    let Ok(entries) = fs::read_dir(&paths.notes) else {
        return Vec::new();
    };
    let mut notes: Vec<_> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()?.to_str()? != "json" {
                return None;
            }
            let id = path.file_stem()?.to_str()?;
            NoteMeta::load(paths, id).ok()
        })
        .collect();
    notes.sort_unstable_by(|a, b| b.id.cmp(&a.id));
    notes
}

pub fn create_note(paths: &Paths, title: &str, settings: &Settings) -> Result<NoteMeta> {
    let ticks = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let id = format!("n{ticks:x}-{:x}", std::process::id());
    let title = title.trim();
    let meta = NoteMeta {
        id,
        title: if title.is_empty() { "新笔记" } else { title }.to_owned(),
        accent: settings.accent % 6,
        pinned: settings.auto_pin,
        preview: settings.default_preview,
        ..Default::default()
    };
    // Write content before metadata: list_notes only discovers committed note metadata.
    write_note(paths, &meta.id, "# 记录此刻\n\n- [ ] 一个新想法\n\n")?;
    meta.save(paths)?;
    Ok(meta)
}

pub fn delete_note(paths: &Paths, id: &str) -> Result<()> {
    validate_id(id)?;
    for path in [paths.meta_path(id), paths.markdown_path(id)] {
        match fs::remove_file(&path) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e).with_context(|| format!("deleting {}", path.display())),
        }
    }
    Ok(())
}

pub fn read_note(paths: &Paths, id: &str) -> Result<String> {
    validate_id(id)?;
    fs::read_to_string(paths.markdown_path(id)).with_context(|| format!("reading note {id}"))
}

pub fn write_note(paths: &Paths, id: &str, text: &str) -> Result<()> {
    validate_id(id)?;
    atomic_write(&paths.markdown_path(id), text.as_bytes())
}

/// Atomic rename protects existing content from partial writes.
/// The one-process-per-note architecture avoids conflicting note editors by default.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_dark_and_floatable() {
        let cfg = Settings::default();
        assert_eq!(cfg.theme, Theme::OneDarkPro);
        assert!(cfg.auto_pin);
    }
    #[test]
    fn id_validation_disallows_path_traversal() {
        assert!(validate_id("../../oops").is_err());
        assert!(validate_id("notes/test").is_err());
        assert!(validate_id("n123-a9").is_ok());
    }
}
