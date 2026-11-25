

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub tracks: Vec<PathBuf>,
    #[serde(default)]
    pub cover: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Persisted {
    pub folder: Option<PathBuf>,
    pub volume: f32,
    pub favorites: Vec<PathBuf>,
    pub last_track: Option<PathBuf>,
    pub shuffle: bool,
    pub repeat: u8,
    pub playlists: Vec<Playlist>,
}

impl Default for Persisted {
    fn default() -> Self {
        Self {
            folder: None,
            volume: 0.72,
            favorites: Vec::new(),
            last_track: None,
            shuffle: false,
            repeat: 0,
            playlists: Vec::new(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ResoRust")
}

pub fn covers_dir() -> PathBuf {
    config_dir().join("covers")
}

pub fn bin_dir() -> PathBuf {
    config_dir().join("bin")
}

pub fn default_music_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Music")
        .join("ResoRust")
}

fn state_path() -> PathBuf {
    config_dir().join("state.json")
}

pub fn load() -> Persisted {
    std::fs::read_to_string(state_path())
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save(state: &Persisted) {
    let dir = config_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(state_path(), json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_ida_e_volta() {
        let mut original = Persisted::default();
        original.volume = 0.42;
        original.shuffle = true;
        original.repeat = 2;
        original.favorites = vec![PathBuf::from(r"C:\Musicas\a.mp3")];
        original.last_track = Some(PathBuf::from(r"C:\Musicas\b.flac"));
        original.playlists = vec![Playlist {
            id: 7,
            name: "Para dirigir".to_owned(),
            tracks: vec![PathBuf::from(r"C:\Musicas\a.mp3")],
            cover: Some(PathBuf::from(r"C:\Covers\capa.jpg")),
        }];

        let json = serde_json::to_string(&original).unwrap();
        let decoded: Persisted = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.folder, original.folder);
        assert!((decoded.volume - 0.42).abs() < 1e-6);
        assert!(decoded.shuffle);
        assert_eq!(decoded.repeat, 2);
        assert_eq!(decoded.favorites, original.favorites);
        assert_eq!(decoded.last_track, original.last_track);
        assert_eq!(decoded.playlists.len(), 1);
        assert_eq!(decoded.playlists[0].name, "Para dirigir");
        assert_eq!(decoded.playlists[0].tracks.len(), 1);
    }

    #[test]
    fn json_incompleto_usa_padroes() {
        let decoded: Persisted = serde_json::from_str(r#"{"volume": 0.5}"#).unwrap();
        assert!((decoded.volume - 0.5).abs() < 1e-6);
        assert!(!decoded.shuffle);
        assert!(decoded.favorites.is_empty());
        assert!(decoded.playlists.is_empty());
        assert_eq!(decoded.repeat, 0);
    }

    #[test]
    fn pasta_padrao_termina_com_resorust() {
        assert_eq!(
            default_music_dir().file_name().and_then(|n| n.to_str()),
            Some("ResoRust")
        );
    }
}
