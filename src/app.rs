

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, Label, Layout, Pos2, Rect, RichText, Sense, Stroke,
    Vec2,
};

use crate::audio::AudioEngine;
use crate::demo;
use crate::library::{self, Track};
use crate::state::{self, Persisted, Playlist};
use crate::theme::{self, *};
use crate::widgets::{self, Icon};

const ROW_HEIGHT: f32 = 56.0;
const ALBUM_MIN_WIDTH: f32 = 820.0;
const PLAYER_BAR_HEIGHT: f32 = 106.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Library,
    Favorites,
    Playlists,
    PlaylistDetail(u64),
    NowPlaying,
    Settings,
    Downloads,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowContext {
    Library,
    Playlist(u64),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Repeat {
    Off,
    All,
    One,
}

impl Repeat {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::All,
            2 => Self::One,
            _ => Self::Off,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Off => "desligada",
            Self::All => "toda a biblioteca",
            Self::One => "uma faixa",
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            Self::Off => "Repetir: desligado",
            Self::All => "Repetir: toda a biblioteca",
            Self::One => "Repetir: uma faixa",
        }
    }
}

impl DownloadState {
    fn new() -> Self {
        Self {
            yt_dlp: find_yt_dlp(),
            ..Self::default()
        }
    }
}

fn find_yt_dlp() -> Option<PathBuf> {
    find_tool("yt-dlp")
}


fn find_tool(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };

    let local = state::bin_dir().join(&exe);
    if local.is_file() {
        return Some(local);
    }

    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(&exe);
        if candidate.is_file() {
            return Some(candidate);
        }
        let plain = dir.join(name);
        if plain.is_file() {
            return Some(plain);
        }
    }
    None
}

struct RowGeom {
    idx: Rect,
    art: Rect,
    title: Rect,
    artist: Rect,
    album: Rect,
    duration: Rect,
    heart: Rect,
}

struct PlaylistDialog {
    editing: Option<u64>,
    name: String,
    cover: Option<PathBuf>,
    cover_is_new: bool,
    cover_removed: bool,
    tracks: Vec<PathBuf>,
    error: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PickerTarget {
    
    Dialog,
    
    Playlist(u64),
}

struct TrackPicker {
    target: PickerTarget,
    search: String,
    selected: Vec<PathBuf>,
}

struct MetadataDialog {
    path: PathBuf,
    title: String,
    artist: String,
    album: String,
    cover: Option<PathBuf>,
    cover_is_new: bool,
    remove_cover: bool,
    error: Option<String>,
}

#[derive(Default)]
struct DownloadState {
    url: String,
    status: String,
    busy: bool,
    rx: Option<Receiver<DownloadEvent>>,
    yt_dlp: Option<PathBuf>,
}

struct DownloadedAudio {
    audio: PathBuf,
    thumb: Option<PathBuf>,
    info: Option<PathBuf>,
}

enum DownloadEvent {
    Status(String),
    Finished(Result<DownloadedAudio, String>),
    Installed(Result<PathBuf, String>),
}

pub struct ResoRustApp {
    ctx: egui::Context,

    library: Vec<Track>,
    folder: PathBuf,
    scan_rx: Option<Receiver<(PathBuf, Vec<Track>)>>,
    scanning: bool,
    pending_play: Option<PathBuf>,

    search: String,
    view: View,
    favorites: HashSet<PathBuf>,
    playlists: Vec<Playlist>,
    playlist_covers: HashMap<PathBuf, egui::TextureHandle>,
    queue: Vec<PathBuf>,

    current: Option<usize>,
    last_track_path: Option<PathBuf>,
    playing: bool,
    playback: Option<AudioEngine>,
    playback_error: Option<String>,
    volume: f32,
    last_volume: f32,
    repeat: Repeat,
    shuffle: bool,
    position: Duration,
    seek_preview: Option<f32>,

    thumb_cache: HashMap<PathBuf, egui::TextureHandle>,
    big_art: Option<egui::TextureHandle>,
    big_art_for: Option<PathBuf>,
    spin: f32,

    playlist_dialog: Option<PlaylistDialog>,
    track_picker: Option<TrackPicker>,
    metadata_dialog: Option<MetadataDialog>,
    confirm_delete_playlist: Option<u64>,
    download: DownloadState,

    rng: u64,
    status: Option<(String, f64)>,
    dirty: bool,
    last_save: Instant,
}

impl ResoRustApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);

        let persisted = state::load();
        let folder = persisted
            .folder
            .clone()
            .filter(|path| path.exists())
            .unwrap_or_else(state::default_music_dir);
        let _ = std::fs::create_dir_all(&folder);

        let volume = persisted.volume.clamp(0.0, 1.0);
        let audio = AudioEngine::new(volume);
        let playback_error = audio.as_ref().err().cloned();
        let playback = audio.ok();

        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);

        let mut app = Self {
            ctx: cc.egui_ctx.clone(),
            library: Vec::new(),
            folder,
            scan_rx: None,
            scanning: false,
            pending_play: None,
            search: String::new(),
            view: View::Library,
            favorites: persisted.favorites.into_iter().collect(),
            playlists: persisted.playlists,
            playlist_covers: HashMap::new(),
            queue: Vec::new(),
            current: None,
            last_track_path: persisted.last_track,
            playing: false,
            playback,
            playback_error,
            volume,
            last_volume: if volume > 0.001 { volume } else { 0.72 },
            repeat: Repeat::from_u8(persisted.repeat),
            shuffle: persisted.shuffle,
            position: Duration::ZERO,
            seek_preview: None,
            thumb_cache: HashMap::new(),
            big_art: None,
            big_art_for: None,
            spin: 0.0,
            playlist_dialog: None,
            track_picker: None,
            metadata_dialog: None,
            confirm_delete_playlist: None,
            download: DownloadState::new(),
            rng: seed | 1,
            status: None,
            dirty: false,
            last_save: Instant::now(),
        };

        app.start_scan();
        app
    }

    
    
    

    fn start_scan(&mut self) {
        if self.scanning {
            return;
        }
        self.scanning = true;

        let (tx, rx) = mpsc::channel();
        self.scan_rx = Some(rx);
        let folder = self.folder.clone();
        let ctx = self.ctx.clone();

        std::thread::spawn(move || {
            let tracks = library::scan(&folder);
            let _ = tx.send((folder, tracks));
            ctx.request_repaint();
        });
    }

    
    fn restart_scan(&mut self) {
        self.scan_rx = None;
        self.scanning = false;
        self.start_scan();
    }

    fn poll_scan(&mut self) {
        let received = self
            .scan_rx
            .as_ref()
            .map(|rx| rx.try_recv())
            .unwrap_or(Err(TryRecvError::Disconnected));

        match received {
            Ok((folder, tracks)) => {
                self.scanning = false;
                self.scan_rx = None;
                if folder != self.folder {
                    return;
                }

                self.library = tracks;
                self.thumb_cache.clear();
                self.big_art = None;
                self.big_art_for = None;

                if let Some(path) = self.last_track_path.clone() {
                    self.current = self.library.iter().position(|track| track.path == path);
                }

                if let Some(path) = self.pending_play.take() {
                    if let Some(index) = self.library.iter().position(|track| track.path == path) {
                        self.queue = self.library.iter().map(|track| track.path.clone()).collect();
                        self.play_index(index);
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.scanning = false;
                self.scan_rx = None;
            }
        }
    }

    fn visible_indices(&self) -> Vec<usize> {
        let query = self.search.trim().to_lowercase();
        self.library
            .iter()
            .enumerate()
            .filter_map(|(index, track)| {
                if self.view == View::Favorites && !self.favorites.contains(&track.path) {
                    return None;
                }
                if !query.is_empty() {
                    let haystack =
                        format!("{} {} {}", track.title, track.artist, track.album).to_lowercase();
                    if !haystack.contains(&query) {
                        return None;
                    }
                }
                Some(index)
            })
            .collect()
    }

    fn thumb_texture(&mut self, index: usize) -> Option<egui::TextureHandle> {
        let (path, thumb) = {
            let track = self.library.get(index)?;
            (track.path.clone(), track.thumb.clone()?)
        };

        if let Some(texture) = self.thumb_cache.get(&path) {
            return Some(texture.clone());
        }
        if self.thumb_cache.len() > 400 {
            self.thumb_cache.clear();
        }

        let image = egui::ColorImage::from_rgba_unmultiplied(
            [library::THUMB_SIZE as usize; 2],
            thumb.as_slice(),
        );
        let texture = self.ctx.load_texture(
            format!("thumb:{}", path.display()),
            image,
            egui::TextureOptions::LINEAR,
        );
        self.thumb_cache.insert(path, texture.clone());
        Some(texture)
    }

    fn ensure_big_art(&mut self) {
        let Some(index) = self.current else {
            self.big_art = None;
            self.big_art_for = None;
            return;
        };
        let Some(track) = self.library.get(index) else {
            self.current = None;
            self.big_art = None;
            self.big_art_for = None;
            return;
        };
        let path = track.path.clone();
        if self.big_art_for.as_ref() == Some(&path) {
            return;
        }

        self.big_art_for = Some(path.clone());
        self.big_art = None;

        let Some(bytes) = library::load_full_art(&path) else {
            return;
        };
        let Ok(image) = image::load_from_memory(&bytes) else {
            return;
        };
        let image = image
            .resize_to_fill(560, 560, image::imageops::FilterType::Triangle)
            .to_rgba8();
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        );
        self.big_art = Some(self.ctx.load_texture(
            format!("cover:{}", path.display()),
            image,
            egui::TextureOptions::LINEAR,
        ));
    }

    
    
    

    fn new_playlist_id(&self) -> u64 {
        self.playlists
            .iter()
            .map(|playlist| playlist.id)
            .max()
            .unwrap_or(0)
            + 1
    }

    fn open_new_playlist(&mut self, seed: Option<PathBuf>) {
        self.playlist_dialog = Some(PlaylistDialog {
            editing: None,
            name: String::new(),
            cover: None,
            cover_is_new: false,
            cover_removed: false,
            tracks: seed.into_iter().collect(),
            error: None,
        });
    }

    fn open_edit_playlist(&mut self, id: u64) {
        let Some(playlist) = self.playlists.iter().find(|item| item.id == id) else {
            return;
        };
        self.playlist_dialog = Some(PlaylistDialog {
            editing: Some(id),
            name: playlist.name.clone(),
            cover: playlist.cover.clone(),
            cover_is_new: false,
            cover_removed: false,
            tracks: playlist.tracks.clone(),
            error: None,
        });
    }

    fn open_metadata(&mut self, path: PathBuf) {
        let Some(track) = self.library.iter().find(|item| item.path == path) else {
            self.toast("Esta música não está mais na biblioteca.".to_owned());
            return;
        };
        self.metadata_dialog = Some(MetadataDialog {
            path,
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            cover: None,
            cover_is_new: false,
            remove_cover: false,
            error: None,
        });
    }

    fn add_to_playlist(&mut self, id: u64, path: PathBuf) {
        let mut name = None;
        let mut duplicate = false;
        if let Some(playlist) = self.playlists.iter_mut().find(|item| item.id == id) {
            if playlist.tracks.contains(&path) {
                duplicate = true;
            } else {
                playlist.tracks.push(path);
                name = Some(playlist.name.clone());
            }
        }
        if duplicate {
            self.toast("Essa música já está na playlist.".to_owned());
        } else if let Some(name) = name {
            self.mark_dirty();
            self.toast(format!("Adicionada a “{name}”."));
        }
    }

    fn save_playlist_dialog(&mut self) {
        let Some(dialog) = self.playlist_dialog.as_ref() else {
            return;
        };
        let name = dialog.name.trim().to_owned();
        if name.is_empty() {
            if let Some(dialog) = &mut self.playlist_dialog {
                dialog.error = Some("Dê um nome para a playlist.".to_owned());
            }
            return;
        }

        let editing = dialog.editing;
        let cover_is_new = dialog.cover_is_new;
        let cover_removed = dialog.cover_removed;
        let cover = dialog.cover.clone();
        let tracks = dialog.tracks.clone();
        let id = editing.unwrap_or_else(|| self.new_playlist_id());

        let stored = if cover_is_new {
            let Some(source) = cover else {
                return;
            };
            match self.store_playlist_cover(&source, id) {
                Some(path) => Some(path),
                None => {
                    if let Some(dialog) = &mut self.playlist_dialog {
                        dialog.error =
                            Some("Não foi possível copiar a imagem escolhida.".to_owned());
                    }
                    return;
                }
            }
        } else if cover_removed {
            None
        } else {
            cover
        };

        if let Some(id) = editing {
            if let Some(position) = self.playlists.iter().position(|item| item.id == id) {
                let old_cover = self.playlists[position].cover.clone();
                if cover_is_new || cover_removed {
                    if let Some(old) = old_cover {
                        if stored.as_ref() != Some(&old) {
                            self.discard_managed_cover(&old);
                        }
                    }
                }
                let playlist = &mut self.playlists[position];
                playlist.name = name.clone();
                playlist.tracks = tracks;
                playlist.cover = stored;
            }
        } else {
            self.playlists.push(Playlist {
                id,
                name: name.clone(),
                tracks,
                cover: stored,
            });
        }

        self.playlist_dialog = None;
        self.mark_dirty();
        self.toast(format!("Playlist “{name}” salva."));
    }

    fn store_playlist_cover(&self, source: &std::path::Path, playlist_id: u64) -> Option<PathBuf> {
        let extension = source
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .filter(|ext| matches!(ext.as_str(), "jpg" | "jpeg" | "png"))
            .unwrap_or_else(|| "jpg".to_owned());

        let directory = state::covers_dir();
        std::fs::create_dir_all(&directory).ok()?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let destination = directory.join(format!("playlist-{playlist_id}-{stamp}.{extension}"));
        std::fs::copy(source, &destination).ok()?;
        Some(destination)
    }

    fn discard_managed_cover(&self, path: &std::path::Path) {
        if path.starts_with(state::covers_dir()) {
            let _ = std::fs::remove_file(path);
        }
    }

    fn playlist_cover_texture(&mut self, cover: &std::path::Path) -> Option<egui::TextureHandle> {
        if let Some(texture) = self.playlist_covers.get(cover) {
            return Some(texture.clone());
        }
        let bytes = std::fs::read(cover).ok()?;
        let image = image::load_from_memory(&bytes).ok()?;
        let image = image
            .resize_to_fill(360, 360, image::imageops::FilterType::Triangle)
            .to_rgba8();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        );
        let texture = self.ctx.load_texture(
            format!("playlist:{}", cover.display()),
            color_image,
            egui::TextureOptions::LINEAR,
        );
        self.playlist_covers
            .insert(cover.to_path_buf(), texture.clone());
        Some(texture)
    }

    fn apply_picker(&mut self) {
        let Some(picker) = self.track_picker.take() else {
            return;
        };
        let selected = picker.selected;

        match picker.target {
            PickerTarget::Dialog => {
                if let Some(dialog) = &mut self.playlist_dialog {
                    for path in selected {
                        if !dialog.tracks.contains(&path) {
                            dialog.tracks.push(path);
                        }
                    }
                }
            }
            PickerTarget::Playlist(id) => {
                let mut added = 0usize;
                if let Some(playlist) = self.playlists.iter_mut().find(|item| item.id == id) {
                    for path in selected {
                        if !playlist.tracks.contains(&path) {
                            playlist.tracks.push(path);
                            added += 1;
                        }
                    }
                }
                if added > 0 {
                    self.mark_dirty();
                    self.toast(format!("{added} música(s) adicionada(s)."));
                }
            }
        }
    }

    
    
    

    fn duration(&self) -> Duration {
        if let Some(index) = self.current {
            if let Some(track) = self.library.get(index) {
                if !track.duration.is_zero() {
                    return track.duration;
                }
            }
        }
        self.playback
            .as_ref()
            .map(AudioEngine::track_duration)
            .unwrap_or_default()
    }

    
    fn play_library_index(&mut self, index: usize) {
        let queue: Vec<PathBuf> = self
            .visible_indices()
            .iter()
            .filter_map(|visible| self.library.get(*visible).map(|track| track.path.clone()))
            .collect();
        if !queue.is_empty() {
            self.queue = queue;
        }
        self.play_index(index);
    }

    fn play_index(&mut self, index: usize) {        if index >= self.library.len() {
            return;
        }
        let path = self.library[index].path.clone();
        if !self.queue.iter().any(|queued| queued == &path) {
            self.queue = self.library.iter().map(|track| track.path.clone()).collect();
        }

        let mut error = None;
        let mut started = false;
        if let Some(player) = &mut self.playback {
            match player.play(&path) {
                Ok(()) => started = true,
                Err(message) => error = Some(message),
            }
        } else {
            error = Some(
                self.playback_error
                    .clone()
                    .unwrap_or_else(|| "Saída de áudio indisponível.".to_owned()),
            );
        }

        self.current = Some(index);
        self.last_track_path = Some(path);
        self.position = Duration::ZERO;
        self.seek_preview = None;
        self.playing = started;
        self.big_art = None;
        self.big_art_for = None;

        if let Some(message) = error {
            self.toast(message);
        }
        self.mark_dirty();
    }

    fn toggle_play(&mut self) {
        if self.playing {
            if let Some(player) = &mut self.playback {
                player.pause();
            }
            self.playing = false;
            return;
        }

        if self.playback.is_none() {
            self.toast("Saída de áudio indisponível. Verifique seus alto-falantes.".to_owned());
            return;
        }

        if let Some(index) = self.current {
            let has_track = self
                .playback
                .as_ref()
                .map(AudioEngine::has_track)
                .unwrap_or(false);
            if has_track {
                if let Some(player) = &mut self.playback {
                    player.resume();
                }
                self.playing = true;
            } else {
                self.play_index(index);
            }
            return;
        }

        if let Some(index) = self.visible_indices().first().copied() {
            self.play_library_index(index);
        }
    }

    fn next_track(&mut self, auto: bool) {
        if self.library.is_empty() {
            return;
        }

        if auto && self.repeat == Repeat::One {
            if let Some(index) = self.current {
                self.play_index(index);
            }
            return;
        }

        let len = self.library.len();

        if !self.shuffle {
            if let Some(position) = self.queue_position() {
                if position + 1 < self.queue.len() {
                    let path = self.queue[position + 1].clone();
                    if let Some(index) = self.library_index_of(&path) {
                        self.play_index(index);
                        return;
                    }
                } else if auto && self.repeat == Repeat::Off {
                    self.finish_playback();
                    return;
                } else if let Some(path) = self.queue.first().cloned() {
                    if let Some(index) = self.library_index_of(&path) {
                        self.play_index(index);
                        return;
                    }
                }
            }
        }

        let next = if self.shuffle && len > 1 {
            let mut candidate = self.random_index(len);
            for _ in 0..8 {
                if Some(candidate) != self.current {
                    break;
                }
                candidate = self.random_index(len);
            }
            candidate
        } else {
            match self.current {
                Some(index) if index + 1 < len => index + 1,
                Some(_) => {
                    if auto && self.repeat == Repeat::Off {
                        self.finish_playback();
                        return;
                    }
                    0
                }
                None => 0,
            }
        };

        self.play_index(next);
    }

    fn prev_track(&mut self) {
        if self.position.as_secs_f32() > 3.0 {
            if let Some(player) = &mut self.playback {
                let _ = player.seek(Duration::ZERO);
            }
            self.position = Duration::ZERO;
            self.seek_preview = None;
            return;
        }

        if self.library.is_empty() {
            return;
        }

        if !self.shuffle {
            if let Some(position) = self.queue_position() {
                let previous = if position == 0 {
                    self.queue.len().saturating_sub(1)
                } else {
                    position - 1
                };
                if let Some(path) = self.queue.get(previous).cloned() {
                    if let Some(index) = self.library_index_of(&path) {
                        self.play_index(index);
                        return;
                    }
                }
            }
        }

        let len = self.library.len();
        let previous = if self.shuffle && len > 1 {
            self.random_index(len)
        } else {
            match self.current {
                Some(0) | None => len - 1,
                Some(index) => index - 1,
            }
        };
        self.play_index(previous);
    }

    fn queue_position(&self) -> Option<usize> {
        let current = self.current.and_then(|index| self.library.get(index))?;
        self.queue.iter().position(|path| path == &current.path)
    }

    fn library_index_of(&self, path: &std::path::Path) -> Option<usize> {
        self.library.iter().position(|track| track.path == path)
    }

    fn play_from_playlist(&mut self, playlist_id: u64, path: Option<PathBuf>) {
        let Some(playlist) = self.playlists.iter().find(|item| item.id == playlist_id) else {
            return;
        };
        let queue: Vec<PathBuf> = playlist
            .tracks
            .iter()
            .filter(|track| self.library.iter().any(|item| &item.path == *track))
            .cloned()
            .collect();
        if queue.is_empty() {
            self.toast("Esta playlist não tem músicas disponíveis.".to_owned());
            return;
        }

        let index = path
            .and_then(|wanted| {
                queue
                    .iter()
                    .position(|item| *item == wanted)
                    .and_then(|position| self.library_index_of(&queue[position]))
            })
            .or_else(|| self.library_index_of(&queue[0]));

        if let Some(index) = index {
            self.queue = queue;
            self.play_index(index);
        }
    }

    fn finish_playback(&mut self) {
        if let Some(player) = &mut self.playback {
            player.stop();
        }
        self.playing = false;
        self.position = Duration::ZERO;
        self.seek_preview = None;
    }

    fn random_index(&mut self, len: usize) -> usize {
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.rng >> 33) as usize) % len.max(1)
    }

    fn apply_volume(&mut self, value: f32) {
        self.volume = value.clamp(0.0, 1.0);
        if self.volume > 0.001 {
            self.last_volume = self.volume;
        }
        if let Some(player) = &mut self.playback {
            player.set_volume(self.volume);
        }
        self.mark_dirty();
    }

    fn toggle_mute(&mut self) {
        if self.volume > 0.001 {
            self.last_volume = self.volume;
            self.apply_volume(0.0);
        } else {
            self.apply_volume(self.last_volume.max(0.2));
        }
    }

    fn adjust_volume(&mut self, delta: f32) {
        self.apply_volume(self.volume + delta);
    }

    fn tick(&mut self) {
        let dt = self.ctx.input(|input| input.stable_dt).min(0.1);
        if self.playing {
            self.spin = (self.spin + dt * 1.1) % std::f32::consts::TAU;
        }

        if self.playing {
            let finished = self
                .playback
                .as_ref()
                .map(AudioEngine::finished)
                .unwrap_or(false);

            if finished {
                self.next_track(true);
            } else if self.seek_preview.is_none() {
                if let Some(player) = &self.playback {
                    self.position = player.position();
                }
            }
            self.ctx.request_repaint_after(Duration::from_millis(33));
        }

        if self.status.is_some() {
            self.ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn shortcuts(&mut self) {
        if self.ctx.egui_wants_keyboard_input() {
            return;
        }

        let (space, next, previous, louder, quieter, mute) = self.ctx.input(|input| {
            (
                input.key_pressed(egui::Key::Space),
                input.key_pressed(egui::Key::ArrowRight) && input.modifiers.ctrl,
                input.key_pressed(egui::Key::ArrowLeft) && input.modifiers.ctrl,
                input.key_pressed(egui::Key::ArrowUp),
                input.key_pressed(egui::Key::ArrowDown),
                input.key_pressed(egui::Key::M),
            )
        });

        if space {
            self.toggle_play();
        }
        if next {
            self.next_track(false);
        }
        if previous {
            self.prev_track();
        }
        if louder {
            self.adjust_volume(0.05);
        }
        if quieter {
            self.adjust_volume(-0.05);
        }
        if mute {
            self.toggle_mute();
        }
    }

    
    
    

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn save_state(&mut self) {
        let mut favorites: Vec<PathBuf> = self.favorites.iter().cloned().collect();
        favorites.sort();

        let snapshot = Persisted {
            folder: Some(self.folder.clone()),
            volume: self.volume,
            favorites,
            last_track: self.last_track_path.clone(),
            shuffle: self.shuffle,
            repeat: self.repeat as u8,
            playlists: self.playlists.clone(),
        };
        state::save(&snapshot);
        self.dirty = false;
        self.last_save = Instant::now();
    }

    fn maybe_save(&mut self) {
        if self.dirty && self.last_save.elapsed() > Duration::from_millis(700) {
            self.save_state();
        }
    }

    
    
    

    fn download_track(&mut self) {
        let url = self.download.url.trim().to_owned();
        if url.is_empty() {
            self.toast("Cole o link do YouTube ou YouTube Music.".to_owned());
            return;
        }
        if url.to_lowercase().contains("spotify.") {
            self.toast(
                "Spotify usa DRM e não permite download. Use YouTube ou YouTube Music."
                    .to_owned(),
            );
            return;
        }
        let Some(yt_dlp) = self.download.yt_dlp.clone() else {
            self.toast("yt-dlp não encontrado. Instale na tela de Downloads.".to_owned());
            return;
        };

        let folder = self.folder.clone();
        let _ = std::fs::create_dir_all(&folder);
        let (tx, rx) = mpsc::channel();
        self.download.rx = Some(rx);
        self.download.busy = true;
        self.download.status = "Iniciando download…".to_owned();

        std::thread::spawn(move || {
            let _ = tx.send(DownloadEvent::Status("Baixando áudio…".to_owned()));

            let mut command = Command::new(&yt_dlp);
            command
                .arg("--no-playlist")
                .arg("--newline")
                .arg("--no-warnings")
                .arg("-P")
                .arg(&folder)
                .arg("-o")
                .arg("%(artist,uploader)s - %(title)s.%(ext)s")
                
                .args([
                    "-f",
                    "bestaudio[ext=m4a]/bestaudio",
                    "--write-thumbnail",
                    "--write-info-json",
                ]);
            command.arg(&url);

            match command.output() {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let audio = stdout.lines().rev().find_map(|line| {
                        line.split("Destination:")
                            .nth(1)
                            .map(|value| PathBuf::from(value.trim()))
                    });
                    let Some(audio) = audio else {
                        let _ = tx.send(DownloadEvent::Finished(Err(
                            "Não foi possível localizar o arquivo baixado.".to_owned(),
                        )));
                        return;
                    };

                    let stem = audio.with_extension("");
                    let candidates = [
                        stem.with_extension("jpg"),
                        stem.with_extension("png"),
                        stem.with_extension("webp"),
                    ];
                    let thumb = candidates
                        .into_iter()
                        .find(|candidate| candidate.is_file());
                    let info = stem.with_extension("info.json");
                    let info = if info.is_file() { Some(info) } else { None };

                    let _ = tx.send(DownloadEvent::Finished(Ok(DownloadedAudio {
                        audio,
                        thumb,
                        info,
                    })));
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let message = stderr
                        .lines()
                        .rev()
                        .find(|line| !line.trim().is_empty())
                        .unwrap_or("O download falhou.")
                        .trim()
                        .to_owned();
                    let _ = tx.send(DownloadEvent::Finished(Err(message)));
                }
                Err(error) => {
                    let _ = tx.send(DownloadEvent::Finished(Err(error.to_string())));
                }
            }
        });
    }

    fn install_yt_dlp(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.download.rx = Some(rx);
        self.download.busy = true;
        self.download.status = "Baixando yt-dlp…".to_owned();

        std::thread::spawn(move || {
            let directory = state::bin_dir();
            let _ = std::fs::create_dir_all(&directory);
            let target = directory.join(if cfg!(windows) {
                "yt-dlp.exe"
            } else {
                "yt-dlp"
            });

            #[cfg(target_os = "windows")]
            let result = {
                let script = format!(
                    "$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -Uri \
                     'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe' \
                     -OutFile '{}'",
                    target.display()
                );
                Command::new("powershell")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                    .output()
            };

            #[cfg(not(target_os = "windows"))]
            let result = {
                let url = if cfg!(target_os = "macos") {
                    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
                } else {
                    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
                };
                Command::new("curl")
                    .args(["-L", "-o"])
                    .arg(&target)
                    .arg(url)
                    .output()
            };

            match result {
                Ok(output) if output.status.success() && target.is_file() => {
                    let _ = tx.send(DownloadEvent::Installed(Ok(target)));
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let message = stderr
                        .lines()
                        .rev()
                        .find(|line| !line.trim().is_empty())
                        .unwrap_or("A instalação falhou.")
                        .trim()
                        .to_owned();
                    let _ = tx.send(DownloadEvent::Installed(Err(message)));
                }
                Err(error) => {
                    let _ = tx.send(DownloadEvent::Installed(Err(error.to_string())));
                }
            }
        });
    }

    fn poll_download(&mut self) {
        let mut events = Vec::new();
        let mut disconnected = false;
        if let Some(rx) = &self.download.rx {
            loop {
                match rx.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        for event in events {
            match event {
                DownloadEvent::Status(status) => self.download.status = status,
                DownloadEvent::Installed(Ok(path)) => {
                    self.download.yt_dlp = Some(path);
                    self.download.busy = false;
                    self.download.rx = None;
                    self.download.status =
                        "yt-dlp instalado. Cole um link para baixar.".to_owned();
                    self.toast("yt-dlp instalado com sucesso.".to_owned());
                }
                DownloadEvent::Installed(Err(error)) => {
                    self.download.busy = false;
                    self.download.rx = None;
                    self.download.status = format!("Falha ao instalar: {error}");
                    self.toast("Não foi possível instalar o yt-dlp.".to_owned());
                }
                DownloadEvent::Finished(Ok(result)) => {
                    self.download.busy = false;
                    self.download.rx = None;

                    let path = result.audio;
                    let stem = path
                        .file_stem()
                        .map(|value| value.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let mut title = stem.clone();
                    let mut artist = String::new();
                    let mut album = String::new();

                    if let Some(info) = &result.info
                        && let Ok(raw) = std::fs::read_to_string(info)
                        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw)
                    {
                        if let Some(value) = json.get("title").and_then(|v| v.as_str()) {
                            title = value.to_owned();
                        }
                        artist = json
                            .get("artist")
                            .or_else(|| json.get("uploader"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_owned();
                        album = json
                            .get("album")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_owned();
                    }

                    let is_audio = library::is_supported(&path);
                    if is_audio {
                        if let Err(error) = library::write_metadata(
                            &path,
                            &title,
                            &artist,
                            &album,
                            result.thumb.as_deref(),
                            false,
                        ) {
                            self.toast(format!("Capas/tags: {error}"));
                        }
                    }

                    if let Some(thumb) = &result.thumb {
                        let _ = std::fs::remove_file(thumb);
                    }
                    if let Some(info) = &result.info {
                        let _ = std::fs::remove_file(info);
                    }

                    if is_audio {
                        let name = path
                            .file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_default();
                        self.download.status = format!("Baixado: {name}");
                        self.pending_play = Some(path);
                        self.toast("Download concluído.".to_owned());
                    } else {
                        self.download.status = "Download concluído.".to_owned();
                        self.toast("Download concluído.".to_owned());
                    }
                    self.restart_scan();
                }
                DownloadEvent::Finished(Err(error)) => {
                    self.download.busy = false;
                    self.download.rx = None;
                    self.download.status = format!("Falhou: {error}");
                    self.toast("O download falhou.".to_owned());
                }
            }
        }

        if disconnected && self.download.busy && self.download.rx.is_some() {
            self.download.busy = false;
            self.download.rx = None;
            self.download.status = "Download interrompido.".to_owned();
        }

        if self.download.busy {
            self.ctx.request_repaint_after(Duration::from_millis(120));
        }
    }

    fn toast(&mut self, message: String) {
        let now = self.ctx.input(|input| input.time);
        self.status = Some((message, now + 3.6));
    }

    fn open_folder(&mut self) {
        let _ = std::fs::create_dir_all(&self.folder);

        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer")
                .arg(&self.folder)
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&self.folder).spawn();
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(&self.folder)
                .spawn();
        }
    }

    fn choose_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Escolha a pasta de músicas")
            .set_directory(&self.folder)
            .pick_folder()
        {
            if folder != self.folder {
                if let Some(player) = &mut self.playback {
                    player.stop();
                }
                self.playing = false;
                self.current = None;
                self.last_track_path = None;
                self.position = Duration::ZERO;
                self.seek_preview = None;
                self.library.clear();
                self.thumb_cache.clear();
                self.big_art = None;
                self.big_art_for = None;
                self.folder = folder;
                self.restart_scan();
                self.mark_dirty();
            }
        }
    }

    fn create_demo(&mut self) {
        match demo::generate(&self.folder) {
            Ok(path) => {
                self.pending_play = Some(path);
                self.restart_scan();
                self.toast("Faixa de demonstração criada!".to_owned());
            }
            Err(message) => self.toast(format!("Não foi possível gerar a demo: {message}")),
        }
    }

    
    
    

    fn header(&mut self, ui: &mut egui::Ui, title: &str, subtitle: &str, show_search: bool) {
        let width = ui.available_width();
        ui.horizontal(|ui| {
            let title_width = if show_search {
                (width * 0.46).max(120.0)
            } else {
                width
            };

            ui.allocate_ui_with_layout(
                Vec2::new(title_width, 50.0),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.label(RichText::new(title).font(theme::font_semi(25.0)).color(TEXT));
                    ui.add(
                        Label::new(
                            RichText::new(subtitle)
                                .font(theme::font(11.5))
                                .color(TEXT_MUTED),
                        )
                        .truncate(),
                    );
                },
            );

            if show_search {
                let search_w = (width * 0.27).clamp(170.0, 260.0);
                let used = title_width + 8.0 + search_w + 40.0;
                ui.add_space((width - used).max(4.0));
                self.search_field(ui, search_w);
                if widgets::icon_button(
                    ui,
                    Icon::Refresh,
                    32.0,
                    self.scanning,
                    "Atualizar biblioteca",
                )
                .clicked()
                {
                    self.restart_scan();
                }
            }
        });
    }

    fn search_field(&mut self, ui: &mut egui::Ui, width: f32) {
        let has_text = !self.search.is_empty();
        let text_width =
            (width - 20.0 - 14.0 - 8.0 - if has_text { 26.0 } else { 0.0 }).max(80.0);

        egui::Frame::NONE
            .fill(SURFACE)
            .corner_radius(CornerRadius::same(9))
            .stroke(Stroke::new(1.0, LINE))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                    widgets::paint_icon(ui.painter(), rect, Icon::Search, TEXT_FAINT, 1.4);

                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Pesquisar músicas…")
                            .font(theme::font(12.5))
                            .text_color(TEXT)
                            .desired_width(text_width)
                            .margin(egui::Margin::ZERO)
                            .frame(egui::Frame::NONE)
                            .align(Align2::LEFT_CENTER),
                    );

                    if has_text
                        && widgets::icon_button(ui, Icon::Close, 18.0, false, "Limpar busca")
                            .clicked()
                    {
                        self.search.clear();
                    }
                });
            });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        
        
        ui.set_max_height(
            (ui.available_height() - PLAYER_BAR_HEIGHT + 2.0).max(220.0),
        );

        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .show(ui, |ui| {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
            widgets::paint_logo(ui.painter(), rect);
            ui.add_space(2.0);
            ui.vertical(|ui| {
                ui.add_space(2.0);
                ui.label(
                    RichText::new("ResoRust")
                        .font(theme::font_semi(16.5))
                        .color(TEXT),
                );
                ui.label(
                    RichText::new("OFFLINE MUSIC")
                        .font(theme::font_medium(8.0))
                        .color(TEXT_FAINT),
                );
            });
        });

        ui.add_space(24.0);
        section_label(ui, "NAVEGAR");

        let favorites = self.favorites.len();
        if widgets::nav_button(
            ui,
            Icon::Music,
            "Biblioteca",
            Some(self.library.len()),
            self.view == View::Library,
        )
        .clicked()
        {
            self.view = View::Library;
        }
        if widgets::nav_button(
            ui,
            Icon::Heart,
            "Favoritos",
            Some(favorites),
            self.view == View::Favorites,
        )
        .clicked()
        {
            self.view = View::Favorites;
        }
        if widgets::nav_button(
            ui,
            Icon::Playlist,
            "Playlists",
            Some(self.playlists.len()),
            matches!(self.view, View::Playlists | View::PlaylistDetail(_)),
        )
        .clicked()
        {
            self.view = View::Playlists;
        }
        if widgets::nav_button(
            ui,
            Icon::Disc,
            "Tocando agora",
            None,
            self.view == View::NowPlaying,
        )
        .clicked()
        {
            self.view = View::NowPlaying;
        }

        ui.add_space(18.0);
        section_label(ui, "SISTEMA");
        if widgets::nav_button(
            ui,
            Icon::Download,
            "Downloads",
            None,
            self.view == View::Downloads,
        )
        .clicked()
        {
            self.view = View::Downloads;
        }
        if widgets::nav_button(
            ui,
            Icon::Sliders,
            "Configurações",
            None,
            self.view == View::Settings,
        )
        .clicked()
        {
            self.view = View::Settings;
        }

        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            self.folder_card(ui);
        });
            });
    }

    fn folder_card(&mut self, ui: &mut egui::Ui) {
        let name = library::folder_name(&self.folder);
        let path = library::pretty_path(&self.folder);

        let response = egui::Frame::NONE
            .fill(SURFACE)
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, LINE))
            .inner_margin(egui::Margin::same(11))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.with_layout(Layout::top_down(Align::Min), |ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                        widgets::paint_icon(
                            ui.painter(),
                            icon_rect,
                            Icon::Folder,
                            ACCENT,
                            1.5,
                        );
                        ui.add_space(2.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("PASTA DE MÚSICAS")
                                    .font(theme::font_medium(8.0))
                                    .color(TEXT_FAINT),
                            );
                            ui.label(
                                RichText::new(&name)
                                    .font(theme::font_semi(12.5))
                                    .color(TEXT),
                            );
                            ui.add(
                                Label::new(
                                    RichText::new(&path)
                                        .font(theme::font(9.0))
                                        .color(TEXT_FAINT),
                                )
                                .truncate(),
                            );
                        });
                    });
                });
            })
            .response;

        let response = ui.interact(
            response.rect,
            response.id.with("abrir-pasta"),
            Sense::click(),
        );
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Abrir pasta de músicas")
            .clicked()
        {
            self.open_folder();
        }
    }

    fn main_content(&mut self, ui: &mut egui::Ui) {
        match self.view {
            View::Library | View::Favorites => self.library_view(ui),
            View::Playlists => self.playlists_view(ui),
            View::PlaylistDetail(id) => self.playlist_detail_view(ui, id),
            View::NowPlaying => self.now_playing_view(ui),
            View::Settings => self.settings_view(ui),
            View::Downloads => self.downloads_view(ui),
        }

        self.show_dialogs(ui.ctx());
    }

    fn library_view(&mut self, ui: &mut egui::Ui) {
        let is_favorites = self.view == View::Favorites;
        let visible = self.visible_indices();
        let count = visible.len();
        let total = self.library.len();

        let title = if is_favorites { "Favoritos" } else { "Biblioteca" };
        let subtitle = if self.scanning {
            format!("Escaneando {}…", library::folder_name(&self.folder))
        } else if is_favorites {
            if count == 1 {
                format!("1 de {total} música favorita")
            } else {
                format!("{count} de {total} músicas favoritas")
            }
        } else if count == 1 {
            format!("1 música · {}", library::folder_name(&self.folder))
        } else {
            format!("{count} músicas · {}", library::folder_name(&self.folder))
        };

        self.header(ui, title, &subtitle, true);
        ui.add_space(16.0);

        if self.library.is_empty() && !self.scanning {
            self.empty_library(ui, is_favorites);
            return;
        }

        if count == 0 {
            if is_favorites && self.search.trim().is_empty() {
                self.empty_library(ui, true);
            } else {
                self.empty_search(ui);
            }
            return;
        }

        let width = ui.available_width();
        self.list_header(ui, width);
        ui.add_space(2.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, ROW_HEIGHT, count, |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (offset, index) in visible[range.clone()].iter().enumerate() {
                    self.track_row(ui, *index, range.start + offset + 1, RowContext::Library);
                }
            });
    }

    fn list_header(&self, ui: &mut egui::Ui, width: f32) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 26.0), Sense::hover());
        let geom = row_geom(rect, width >= ALBUM_MIN_WIDTH);
        let font = theme::font_medium(8.5);
        let painter = ui.painter();

        painter.text(
            Pos2::new(geom.idx.center().x, rect.center().y),
            Align2::CENTER_CENTER,
            "#",
            font.clone(),
            TEXT_FAINT,
        );
        painter.text(
            Pos2::new(geom.title.left(), rect.center().y),
            Align2::LEFT_CENTER,
            "TÍTULO",
            font.clone(),
            TEXT_FAINT,
        );
        painter.text(
            Pos2::new(geom.artist.left(), rect.center().y),
            Align2::LEFT_CENTER,
            "ARTISTA",
            font.clone(),
            TEXT_FAINT,
        );
        if width >= ALBUM_MIN_WIDTH {
            painter.text(
                Pos2::new(geom.album.left(), rect.center().y),
                Align2::LEFT_CENTER,
                "ÁLBUM",
                font.clone(),
                TEXT_FAINT,
            );
        }
        painter.text(
            Pos2::new(geom.duration.right(), rect.center().y),
            Align2::RIGHT_CENTER,
            "TEMPO",
            font,
            TEXT_FAINT,
        );

        painter.line_segment(
            [
                Pos2::new(rect.left(), rect.bottom()),
                Pos2::new(rect.right(), rect.bottom()),
            ],
            Stroke::new(1.0, LINE),
        );
    }

    fn track_row(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        number: usize,
        context: RowContext,
    ) {
        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::click());
        if index >= self.library.len() {
            return;
        }

        let (title, artist, album, path) = {
            let track = &self.library[index];
            (
                track.title.clone(),
                track.artist.clone(),
                track.album.clone(),
                track.path.clone(),
            )
        };
        let is_current = self.current == Some(index);
        let is_favorite = self.favorites.contains(&path);
        let show_album = width >= ALBUM_MIN_WIDTH;
        let geom = row_geom(rect, show_album);
        let painter = ui.painter_at(rect.shrink(1.0));

        if is_current {
            painter.rect_filled(
                rect.shrink(1.0),
                CornerRadius::same(8),
                alpha(ACCENT, 0.10),
            );
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.left() + 1.0, rect.top() + 14.0),
                    Vec2::new(3.0, rect.height() - 28.0),
                ),
                CornerRadius::same(2),
                ACCENT,
            );
        } else if response.hovered() {
            painter.rect_filled(
                rect.shrink(1.0),
                CornerRadius::same(8),
                alpha(SURFACE, 0.72),
            );
        }

        let art = geom.art.shrink(1.0);
        if let Some(texture) = self.thumb_texture(index) {
            let sized = egui::load::SizedTexture::new(texture.id(), art.size());
            ui.put(
                art,
                egui::Image::new(sized).corner_radius(CornerRadius::same(8)),
            );
        } else {
            let initial = title
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "M".to_owned());
            widgets::paint_placeholder_art(&painter, art, &initial, is_current, 15.0);
        }

        if response.hovered() && !is_current {
            painter.rect_filled(art, CornerRadius::same(8), alpha(Color32::BLACK, 0.45));
            widgets::paint_icon(
                &painter,
                Rect::from_center_size(art.center(), Vec2::splat(16.0)),
                Icon::Play,
                TEXT,
                1.5,
            );
        }

        if is_current && self.playing {
            let time = ui.ctx().input(|input| input.time) as f32;
            for bar in 0..3 {
                let height =
                    4.0 + 10.0 * (0.5 + 0.5 * (time * 4.0 + bar as f32 * 1.8).sin()).abs();
                let x = geom.idx.center().x + (bar as f32 - 1.0) * 5.0;
                painter.rect_filled(
                    Rect::from_center_size(
                        Pos2::new(x, geom.idx.center().y),
                        Vec2::new(2.5, height),
                    ),
                    CornerRadius::same(1),
                    ACCENT_BRIGHT,
                );
            }
        } else if response.hovered() {
            widgets::paint_icon(
                &painter,
                Rect::from_center_size(geom.idx.center(), Vec2::splat(12.0)),
                Icon::Play,
                TEXT,
                1.3,
            );
        } else {
            painter.text(
                geom.idx.center(),
                Align2::CENTER_CENTER,
                number.to_string(),
                theme::font(10.5),
                TEXT_FAINT,
            );
        }

        let title_text = widgets::ellipsize(ui, &title, theme::font_semi(13.0), geom.title.width());
        painter.text(
            Pos2::new(geom.title.left(), rect.center().y),
            Align2::LEFT_CENTER,
            title_text,
            if is_current {
                theme::font_semi(13.0)
            } else {
                theme::font_medium(13.0)
            },
            if is_current { ACCENT_BRIGHT } else { TEXT },
        );

        let artist_text =
            widgets::ellipsize(ui, &artist, theme::font(11.5), geom.artist.width());
        painter.text(
            Pos2::new(geom.artist.left(), rect.center().y),
            Align2::LEFT_CENTER,
            artist_text,
            theme::font(11.5),
            TEXT_MUTED,
        );

        if show_album {
            let album_text = widgets::ellipsize(ui, &album, theme::font(11.5), geom.album.width());
            painter.text(
                Pos2::new(geom.album.left(), rect.center().y),
                Align2::LEFT_CENTER,
                album_text,
                theme::font(11.5),
                TEXT_FAINT,
            );
        }

        painter.text(
            Pos2::new(geom.duration.right(), rect.center().y),
            Align2::RIGHT_CENTER,
            fmt_time(self.library[index].duration),
            theme::font(10.5),
            TEXT_FAINT,
        );

        let heart = widgets::heart_button(
            ui,
            geom.heart,
            egui::Id::unique(("favorite", index)),
            is_favorite,
        );
        if heart.clicked() {
            if is_favorite {
                self.favorites.remove(&path);
            } else {
                self.favorites.insert(path.clone());
            }
            self.mark_dirty();
        }

        let mut play_now = false;
        let mut edit_metadata = false;
        let mut remove_from_playlist = false;
        let mut add_target = None;
        let mut new_with_track = false;

        response.context_menu(|ui| {
            if ui.button("Tocar agora").clicked() {
                play_now = true;
                ui.close();
            }
            ui.separator();

            match context {
                RowContext::Library => {
                    ui.menu_button("Adicionar à playlist", |ui| {
                        for playlist in &self.playlists {
                            if ui.button(&playlist.name).clicked() {
                                add_target = Some(playlist.id);
                                ui.close();
                            }
                        }
                        if !self.playlists.is_empty() {
                            ui.separator();
                        }
                        if ui.button("Nova playlist com esta música…").clicked() {
                            new_with_track = true;
                            ui.close();
                        }
                    });
                }
                RowContext::Playlist(_) => {
                    if ui.button("Remover da playlist").clicked() {
                        remove_from_playlist = true;
                        ui.close();
                    }
                }
            }

            if ui.button("Editar metadados…").clicked() {
                edit_metadata = true;
                ui.close();
            }
        });

        if play_now {
            match context {
                RowContext::Library => self.play_library_index(index),
                RowContext::Playlist(id) => {
                    self.play_from_playlist(id, Some(path.clone()));
                }
            }
        }
        if let Some(id) = add_target {
            self.add_to_playlist(id, path.clone());
        }
        if new_with_track {
            self.open_new_playlist(Some(path.clone()));
        }
        if remove_from_playlist {
            if let RowContext::Playlist(id) = context {
                if let Some(playlist) = self.playlists.iter_mut().find(|item| item.id == id) {
                    playlist.tracks.retain(|track| track != &path);
                }
                self.mark_dirty();
            }
        }
        if edit_metadata {
            self.open_metadata(path.clone());
        }

        if response.clicked() && !heart.hovered() && !heart.clicked() {
            match context {
                RowContext::Library => self.play_library_index(index),
                RowContext::Playlist(id) => {
                    self.play_from_playlist(id, Some(path.clone()));
                }
            }
        }
    }

    fn empty_library(&mut self, ui: &mut egui::Ui, favorites: bool) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);

            if favorites {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(64.0), Sense::hover());
                widgets::paint_icon(ui.painter(), rect, Icon::Heart, TEXT_FAINT, 2.0);
                ui.add_space(14.0);
                ui.label(
                    RichText::new("Nenhum favorito ainda")
                        .font(theme::font_semi(20.0))
                        .color(TEXT),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Toque no coração de uma música para guardá-la aqui.")
                        .font(theme::font(12.5))
                        .color(TEXT_MUTED),
                );
                ui.add_space(18.0);
                if widgets::action_button(ui, "Ir para a biblioteca", true, 220.0).clicked() {
                    self.view = View::Library;
                }
            } else {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(104.0), Sense::hover());
                widgets::paint_vinyl(ui.painter(), rect, 0.6);
                ui.add_space(18.0);
                ui.label(
                    RichText::new("Sua biblioteca está vazia")
                        .font(theme::font_semi(22.0))
                        .color(TEXT),
                );
                ui.add_space(6.0);
                ui.add(
                    Label::new(
                        RichText::new(
                            "Coloque arquivos MP3, FLAC, WAV, OGG ou M4A na pasta abaixo.",
                        )
                        .font(theme::font(12.5))
                        .color(TEXT_MUTED),
                    )
                    .truncate(),
                );
                ui.add_space(2.0);
                ui.add(
                    Label::new(
                        RichText::new(library::pretty_path(&self.folder))
                            .font(theme::font(10.5))
                            .color(TEXT_FAINT),
                    )
                    .truncate(),
                );
                ui.add_space(18.0);
                if widgets::action_button(ui, "Abrir pasta", true, 230.0).clicked() {
                    self.open_folder();
                }
                ui.add_space(7.0);
                if widgets::action_button(ui, "Gerar faixa de demonstração", false, 230.0)
                    .clicked()
                {
                    self.create_demo();
                }
                ui.add_space(7.0);
                if widgets::action_button(ui, "Atualizar biblioteca", false, 230.0).clicked() {
                    self.restart_scan();
                }

                if let Some(error) = &self.playback_error {
                    ui.add_space(14.0);
                    ui.add(
                        Label::new(
                            RichText::new(error)
                                .font(theme::font(11.0))
                                .color(ACCENT_BRIGHT),
                        )
                        .truncate(),
                    );
                }
            }
        });
    }

    fn empty_search(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(70.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
            widgets::paint_icon(ui.painter(), rect, Icon::Search, TEXT_FAINT, 2.0);
            ui.add_space(14.0);
            ui.label(
                RichText::new("Nada encontrado")
                    .font(theme::font_semi(18.0))
                    .color(TEXT),
            );
            ui.add_space(4.0);
            ui.add(
                Label::new(
                    RichText::new(format!("Nenhum resultado para “{}”.", self.search.trim()))
                        .font(theme::font(12.0))
                        .color(TEXT_MUTED),
                )
                .truncate(),
            );
            ui.add_space(16.0);
            if widgets::action_button(ui, "Limpar busca", false, 160.0).clicked() {
                self.search.clear();
            }
        });
    }

    
    
    

    fn playlists_view(&mut self, ui: &mut egui::Ui) {
        let count = self.playlists.len();
        let subtitle = if count == 1 {
            "1 playlist · capas personalizadas".to_owned()
        } else {
            format!("{count} playlists · capas personalizadas")
        };
        self.header(ui, "Playlists", &subtitle, false);
        ui.add_space(12.0);

        let mut create = false;
        ui.horizontal(|ui| {
            if widgets::action_button(ui, "+ Nova playlist", true, 150.0).clicked() {
                create = true;
            }
        });
        ui.add_space(14.0);

        if self.playlists.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(54.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(60.0), Sense::hover());
                widgets::paint_icon(ui.painter(), rect, Icon::Playlist, TEXT_FAINT, 2.2);
                ui.add_space(14.0);
                ui.label(
                    RichText::new("Crie sua primeira playlist")
                        .font(theme::font_semi(20.0))
                        .color(TEXT),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "Junte músicas, escolha uma capa e toque tudo na ordem que quiser.",
                    )
                    .font(theme::font(12.5))
                    .color(TEXT_MUTED),
                );
                ui.add_space(18.0);
                if widgets::action_button(ui, "Criar playlist", true, 200.0).clicked() {
                    create = true;
                }
            });
        } else {
            let available = ui.available_width();
            let columns = ((available + 14.0) / (160.0 + 14.0)).floor().max(1.0) as usize;
            let card_width =
                ((available - (columns as f32 - 1.0) * 14.0) / columns as f32).clamp(132.0, 200.0);

            let mut open = None;
            let mut edit = None;
            let mut delete = None;

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(14.0, 18.0);
                    ui.horizontal_wrapped(|ui| {
                        for index in 0..self.playlists.len() {
                            let playlist = self.playlists[index].clone();
                            let response = self.playlist_card(ui, &playlist, card_width);
                            if response.clicked() {
                                open = Some(playlist.id);
                            }
                            response.context_menu(|ui| {
                                if ui.button("Abrir").clicked() {
                                    open = Some(playlist.id);
                                    ui.close();
                                }
                                if ui.button("Editar").clicked() {
                                    edit = Some(playlist.id);
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button("Excluir").clicked() {
                                    delete = Some(playlist.id);
                                    ui.close();
                                }
                            });
                        }
                    });
                    ui.add_space(10.0);
                });

            if let Some(id) = open {
                self.view = View::PlaylistDetail(id);
            }
            if let Some(id) = edit {
                self.open_edit_playlist(id);
            }
            if let Some(id) = delete {
                self.confirm_delete_playlist = Some(id);
            }
        }

        if create {
            self.open_new_playlist(None);
        }
    }

    fn playlist_card(
        &mut self,
        ui: &mut egui::Ui,
        playlist: &Playlist,
        width: f32,
    ) -> egui::Response {
        let height = width + 46.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
        let painter = ui.painter_at(rect);
        let cover = Rect::from_min_size(rect.min, Vec2::splat(width));

        if response.hovered() {
            painter.rect_filled(
                rect.expand(2.0),
                CornerRadius::same(14),
                alpha(SURFACE, 0.55),
            );
        }
        painter.rect_filled(
            cover.translate(Vec2::new(0.0, 4.0)),
            CornerRadius::same(12),
            alpha(Color32::BLACK, 0.32),
        );

        let mut painted = false;
        if let Some(path) = playlist.cover.clone() {
            if let Some(texture) = self.playlist_cover_texture(&path) {
                let sized = egui::load::SizedTexture::new(texture.id(), cover.size());
                ui.put(
                    cover,
                    egui::Image::new(sized).corner_radius(CornerRadius::same(12)),
                );
                painted = true;
            }
        }
        if !painted {
            let initial = playlist
                .name
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "P".to_owned());
            widgets::paint_placeholder_art(&painter, cover, &initial, false, 34.0);
        }

        if response.hovered() {
            painter.rect_filled(cover, CornerRadius::same(12), alpha(Color32::BLACK, 0.42));
            let button = Rect::from_center_size(cover.center(), Vec2::splat(44.0));
            painter.circle_filled(button.center(), 22.0, alpha(ACCENT, 0.92));
            widgets::paint_icon(
                &painter,
                Rect::from_center_size(button.center(), Vec2::splat(18.0)),
                Icon::Play,
                TEXT,
                1.6,
            );
        }

        let name = widgets::ellipsize(
            ui,
            &playlist.name,
            theme::font_semi(13.0),
            width - 4.0,
        );
        painter.text(
            Pos2::new(rect.left() + 2.0, cover.bottom() + 10.0),
            Align2::LEFT_TOP,
            name,
            theme::font_semi(13.0),
            TEXT,
        );
        let count = playlist.tracks.len();
        let label = if count == 1 {
            "1 música".to_owned()
        } else {
            format!("{count} músicas")
        };
        painter.text(
            Pos2::new(rect.left() + 2.0, cover.bottom() + 27.0),
            Align2::LEFT_TOP,
            label,
            theme::font(10.5),
            TEXT_FAINT,
        );

        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn playlist_detail_view(&mut self, ui: &mut egui::Ui, id: u64) {
        let Some(playlist) = self.playlists.iter().find(|item| item.id == id).cloned() else {
            self.view = View::Playlists;
            return;
        };

        let mut back = false;
        let mut play = false;
        let mut edit = false;
        let mut add = false;
        let mut delete = false;

        ui.horizontal(|ui| {
            if widgets::action_button(ui, "Voltar", false, 0.0).clicked() {
                back = true;
            }
        });
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            let size = 148.0;
            let (cover_rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
            let painter = ui.painter_at(cover_rect);
            let mut painted = false;
            if let Some(path) = playlist.cover.clone() {
                if let Some(texture) = self.playlist_cover_texture(&path) {
                    let sized = egui::load::SizedTexture::new(texture.id(), cover_rect.size());
                    ui.put(
                        cover_rect,
                        egui::Image::new(sized).corner_radius(CornerRadius::same(14)),
                    );
                    painted = true;
                }
            }
            if !painted {
                let initial = playlist
                    .name
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string())
                    .unwrap_or_else(|| "P".to_owned());
                widgets::paint_placeholder_art(&painter, cover_rect, &initial, false, 40.0);
            }

            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("PLAYLIST")
                        .font(theme::font_medium(8.5))
                        .color(ACCENT),
                );
                ui.add_space(2.0);
                ui.add(
                    Label::new(
                        RichText::new(&playlist.name)
                            .font(theme::font_semi(25.0))
                            .color(TEXT),
                    )
                    .truncate(),
                );
                let count = playlist.tracks.len();
                let label = if count == 1 {
                    "1 música".to_owned()
                } else {
                    format!("{count} músicas")
                };
                ui.label(
                    RichText::new(label)
                        .font(theme::font(11.5))
                        .color(TEXT_MUTED),
                );
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    if widgets::action_button(ui, "Tocar", true, 104.0).clicked() {
                        play = true;
                    }
                    if widgets::action_button(ui, "Editar", false, 92.0).clicked() {
                        edit = true;
                    }
                    if widgets::action_button(ui, "+ Músicas", false, 108.0).clicked() {
                        add = true;
                    }
                    if widgets::action_button(ui, "Excluir", false, 92.0).clicked() {
                        delete = true;
                    }
                });
            });
        });

        ui.add_space(18.0);

        let entries: Vec<(usize, PathBuf)> = playlist
            .tracks
            .iter()
            .filter_map(|path| {
                self.library
                    .iter()
                    .position(|track| &track.path == path)
                    .map(|index| (index, path.clone()))
            })
            .collect();

        if entries.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(36.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
                widgets::paint_icon(ui.painter(), rect, Icon::Music, TEXT_FAINT, 2.0);
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Playlist vazia")
                        .font(theme::font_semi(18.0))
                        .color(TEXT),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Adicione músicas da sua biblioteca para começar.")
                        .font(theme::font(12.0))
                        .color(TEXT_MUTED),
                );
                ui.add_space(14.0);
                if widgets::action_button(ui, "Adicionar músicas", true, 190.0).clicked() {
                    add = true;
                }
            });
        } else {
            let width = ui.available_width();
            self.list_header(ui, width);
            ui.add_space(2.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show_rows(ui, ROW_HEIGHT, entries.len(), |ui, range| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (offset, (index, _path)) in entries[range.clone()].iter().enumerate() {
                        self.track_row(
                            ui,
                            *index,
                            range.start + offset + 1,
                            RowContext::Playlist(id),
                        );
                    }
                });
        }

        if back {
            self.view = View::Playlists;
        }
        if play {
            self.play_from_playlist(id, None);
        }
        if edit {
            self.open_edit_playlist(id);
        }
        if add {
            self.track_picker = Some(TrackPicker {
                target: PickerTarget::Playlist(id),
                search: String::new(),
                selected: Vec::new(),
            });
        }
        if delete {
            self.confirm_delete_playlist = Some(id);
        }
    }

    
    
    

    fn downloads_view(&mut self, ui: &mut egui::Ui) {
        self.header(
            ui,
            "Downloads",
            "Baixe áudios do YouTube e YouTube Music direto para sua biblioteca",
            false,
        );
        ui.add_space(14.0);

        let mut download = false;
        let mut install = false;
        let mut open_folder = false;


        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                settings_card(ui, "Baixar música", |ui| {
                    ui.label(
                        RichText::new("Cole o link do vídeo ou da música")
                            .font(theme::font(10.5))
                            .color(TEXT_FAINT),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let field_width = (ui.available_width() - 160.0).max(160.0);
                        egui::Frame::NONE
                            .fill(BG)
                            .corner_radius(CornerRadius::same(9))
                            .stroke(Stroke::new(1.0, LINE))
                            .inner_margin(egui::Margin::symmetric(10, 7))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.download.url)
                                        .hint_text("Pressione Colar para colar o link da sua música.")
                                        .font(theme::font(11.5))
                                        .text_color(TEXT)
                                        .desired_width(field_width)
                                        .frame(egui::Frame::NONE)
                                        .margin(egui::Margin::ZERO),
                                );
                            });
                        if widgets::action_button(ui, "Colar", false, 0.0).clicked() {
                            if let Some(link) = read_clipboard() {
                                self.download.url = link.trim().to_owned();
                            }
                        }
                        let label = if self.download.busy {
                            "Baixando…"
                        } else {
                            "Baixar"
                        };
                        if widgets::action_button(ui, label, true, 0.0).clicked()
                            && !self.download.busy
                        {
                            download = true;
                        }
                    });

                    if !self.download.status.is_empty() {
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if self.download.busy {
                                ui.spinner();
                            }
                            ui.add(
                                Label::new(
                                    RichText::new(&self.download.status)
                                        .font(theme::font(10.5))
                                        .color(if self.download.status.starts_with("Falhou")
                                            || self.download.status.starts_with("Falha")
                                        {
                                            ACCENT_BRIGHT
                                        } else {
                                            TEXT_MUTED
                                        }),
                                )
                                .truncate(),
                            );
                        });
                    }

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!(
                            "Destino: {}",
                            library::pretty_path(&self.folder)
                        ))
                        .font(theme::font(9.5))
                        .color(TEXT_FAINT),
                    );
                    if widgets::action_button(ui, "Abrir pasta", false, 0.0).clicked() {
                        open_folder = true;
                    }
                });

                ui.add_space(12.0);

                settings_card(ui, "Ferramenta de download", |ui| {
                    match &self.download.yt_dlp {
                        Some(path) => {
                            ui.horizontal(|ui| {
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                                widgets::paint_icon(
                                    ui.painter(),
                                    rect,
                                    Icon::Download,
                                    ACCENT,
                                    1.5,
                                );
                                ui.add(
                                    Label::new(
                                        RichText::new("yt-dlp pronto")
                                            .font(theme::font_semi(12.0))
                                            .color(TEXT),
                                    )
                                    .truncate(),
                                );
                            });
                            ui.add_space(3.0);
                            ui.add(
                                Label::new(
                                    RichText::new(path.display().to_string())
                                        .font(theme::font(9.5))
                                        .color(TEXT_FAINT),
                                )
                                .truncate(),
                            );
                        }
                        None => {
                            ui.label(
                                RichText::new(
                                    "O yt-dlp não foi encontrado. Ele é o componente que faz \
                                     o download — leve e sem instalação pesada.",
                                )
                                .font(theme::font(11.0))
                                .color(TEXT_MUTED),
                            );
                            ui.add_space(8.0);
                            if widgets::action_button(
                                ui,
                                "Instalar yt-dlp automaticamente",
                                true,
                                0.0,
                            )
                            .clicked()
                                && !self.download.busy
                            {
                                install = true;
                            }
                        }
                    }
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(
                            "YouTube e YouTube Music funcionam. Spotify não é suportado porque \
                             usa DRM — o áudio não pode ser baixado por meios legais sem o app \
                             oficial rodando.",
                        )
                        .font(theme::font(10.0))
                        .color(TEXT_FAINT),
                    );
                });

                ui.add_space(4.0);
            });

        if download {
            self.download_track();
        }
        if install {
            self.install_yt_dlp();
        }
        if open_folder {
            self.open_folder();
        }
    }

    fn now_playing_view(&mut self, ui: &mut egui::Ui) {
        let Some(index) = self.current else {
            self.empty_now_playing(ui);
            return;
        };
        if index >= self.library.len() {
            self.empty_now_playing(ui);
            return;
        }

        let (title, artist, album, path) = {
            let track = &self.library[index];
            (
                track.title.clone(),
                track.artist.clone(),
                track.album.clone(),
                track.path.clone(),
            )
        };
        let is_favorite = self.favorites.contains(&path);
        let available = ui.available_size();
        let art_size = (available.y - 255.0)
            .clamp(160.0, 340.0)
            .min((available.x - 70.0).max(140.0));

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width((available.x - 20.0).max(300.0));
                    ui.add_space(8.0);

                    let (box_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(art_size + 44.0), Sense::hover());
                    let inner = Rect::from_center_size(box_rect.center(), Vec2::splat(art_size));
                    let painter = ui.painter();

                    for layer in 0..3 {
                        painter.circle_filled(
                            inner.center(),
                            art_size * 0.5 * (1.0 + layer as f32 * 0.05),
                            alpha(ACCENT, 0.028),
                        );
                    }
                    painter.rect_filled(
                        inner.translate(Vec2::new(0.0, 10.0)),
                        CornerRadius::same(18),
                        alpha(Color32::BLACK, 0.35),
                    );

                    if let Some(texture) = self.big_art.clone() {
                        let sized =
                            egui::load::SizedTexture::new(texture.id(), inner.size());
                        ui.put(
                            inner,
                            egui::Image::new(sized).corner_radius(CornerRadius::same(18)),
                        );
                    } else {
                        widgets::paint_vinyl(painter, inner, self.spin);
                    }

                    ui.add_space(22.0);
                    ui.label(
                        RichText::new("TOCANDO AGORA")
                            .font(theme::font_medium(8.5))
                            .color(ACCENT),
                    );
                    ui.add_space(4.0);
                    ui.add(
                        Label::new(
                            RichText::new(title)
                                .font(theme::font_semi(24.0))
                                .color(TEXT),
                        )
                        .truncate(),
                    );
                    ui.add_space(2.0);
                    ui.add(
                        Label::new(
                            RichText::new(artist)
                                .font(theme::font(13.5))
                                .color(TEXT_MUTED),
                        )
                        .truncate(),
                    );
                    ui.add(
                        Label::new(
                            RichText::new(album)
                                .font(theme::font(11.0))
                                .color(TEXT_FAINT),
                        )
                        .truncate(),
                    );

                    ui.add_space(8.0);
                    if widgets::icon_button(
                        ui,
                        if is_favorite {
                            Icon::HeartFilled
                        } else {
                            Icon::Heart
                        },
                        26.0,
                        is_favorite,
                        "Favorito",
                    )
                    .clicked()
                    {
                        if is_favorite {
                            self.favorites.remove(&path);
                        } else {
                            self.favorites.insert(path.clone());
                        }
                        self.mark_dirty();
                    }

                    ui.add_space(4.0);
                    self.transport_controls(ui, 52.0);
                    ui.add_space(4.0);
                    self.timeline(ui, 440.0);
                });
            });
    }

    fn empty_now_playing(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(84.0), Sense::hover());
            widgets::paint_vinyl(ui.painter(), rect, 0.3);
            ui.add_space(16.0);
            ui.label(
                RichText::new("Nada tocando agora")
                    .font(theme::font_semi(21.0))
                    .color(TEXT),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new("Escolha uma música na biblioteca para começar.")
                    .font(theme::font(12.5))
                    .color(TEXT_MUTED),
            );
            ui.add_space(18.0);
            if widgets::action_button(ui, "Ir para a biblioteca", true, 210.0).clicked() {
                self.view = View::Library;
            }
        });
    }

    fn settings_view(&mut self, ui: &mut egui::Ui) {
        self.header(ui, "Configurações", "Preferências do ResoRust", false);
        ui.add_space(16.0);

        let folder_full = self.folder.display().to_string();
        let folder_short = library::pretty_path(&self.folder);
        let track_count = self.library.len();
        let favorite_count = self.favorites.len();
        let mut choose = false;
        let mut open = false;
        let mut rescan = false;
        let mut demo = false;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                settings_card(ui, "Biblioteca", |ui| {
                    small_label(ui, "Pasta de músicas");
                    egui::Frame::NONE
                        .fill(BG)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::symmetric(12, 9))
                        .stroke(Stroke::new(1.0, LINE))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.add(
                                Label::new(
                                    RichText::new(&folder_short)
                                        .font(theme::font(11.0))
                                        .color(TEXT_MUTED),
                                )
                                .truncate(),
                            );
                        });
                    ui.add_space(3.0);
                    ui.add(
                        Label::new(
                            RichText::new(&folder_full)
                                .font(theme::font(9.5))
                                .color(TEXT_FAINT),
                        )
                        .truncate(),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if widgets::action_button(ui, "Escolher pasta", false, 0.0).clicked() {
                            choose = true;
                        }
                        if widgets::action_button(ui, "Abrir pasta", false, 0.0).clicked() {
                            open = true;
                        }
                        if widgets::action_button(ui, "Atualizar", false, 0.0).clicked() {
                            rescan = true;
                        }
                    });
                    ui.add_space(10.0);
                    let tracks_label = if track_count == 1 {
                        "1 música indexada".to_owned()
                    } else {
                        format!("{track_count} músicas indexadas")
                    };
                    let favorites_label = if favorite_count == 1 {
                        "1 favorita".to_owned()
                    } else {
                        format!("{favorite_count} favoritas")
                    };
                    ui.label(
                        RichText::new(format!("{tracks_label} · {favorites_label}"))
                            .font(theme::font(10.5))
                            .color(TEXT_FAINT),
                    );
                    ui.label(
                        RichText::new("Formatos: MP3 · FLAC · WAV · OGG · M4A")
                            .font(theme::font(10.5))
                            .color(TEXT_FAINT),
                    );
                });

                ui.add_space(12.0);

                let mut volume = self.volume;
                let mut shuffle = self.shuffle;
                let mut repeat = self.repeat;
                settings_card(ui, "Reprodução", |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Volume")
                                .font(theme::font(11.5))
                                .color(TEXT_MUTED),
                        );
                        if let Some(value) = widgets::volume_bar(ui, volume, 190.0) {
                            volume = value;
                        }
                        ui.label(
                            RichText::new(format!("{}%", (volume * 100.0).round() as i32))
                                .font(theme::font(10.5))
                                .color(TEXT_FAINT),
                        );
                    });
                    ui.add_space(6.0);
                    ui.checkbox(&mut shuffle, "Modo aleatório");
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("Repetição: {}", repeat.label()))
                                .font(theme::font(11.0))
                                .color(TEXT_MUTED),
                        );
                        if widgets::action_button(ui, "Alterar", false, 0.0).clicked() {
                            repeat = repeat.next();
                        }
                    });
                });

                if (volume - self.volume).abs() > f32::EPSILON {
                    self.apply_volume(volume);
                }
                if shuffle != self.shuffle {
                    self.shuffle = shuffle;
                    self.mark_dirty();
                }
                if repeat != self.repeat {
                    self.repeat = repeat;
                    self.mark_dirty();
                }

                ui.add_space(12.0);
                settings_card(ui, "Sobre", |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
                        widgets::paint_logo(ui.painter(), rect);
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("ResoRust 0.1.0")
                                    .font(theme::font_semi(14.0))
                                    .color(TEXT),
                            );
                            ui.label(
                                RichText::new("Player de música offline")
                                    .font(theme::font(10.5))
                                    .color(TEXT_MUTED),
                            );
                            ui.label(
                                RichText::new("By Briff")
                                    .font(theme::font(9.0))
                                    .color(TEXT_FAINT),
                            );
                        });
                    });
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(
                            "Feito em Rust, com egui para a interface e rodio para o áudio. \
                             Suas músicas ficam apenas no seu computador — nada é enviado para a internet.",
                        )
                        .font(theme::font(11.0))
                        .color(TEXT_MUTED),
                    );
                    ui.add_space(12.0);
                    if widgets::action_button(
                        ui,
                        "Gerar faixa de demonstração",
                        false,
                        0.0,
                    )
                    .clicked()
                    {
                        demo = true;
                    }
                });

                ui.add_space(4.0);
            });

        if choose {
            self.choose_folder();
        }
        if open {
            self.open_folder();
        }
        if rescan {
            self.restart_scan();
        }
        if demo {
            self.create_demo();
        }
    }

    
    
    

    
    
    

    fn show_dialogs(&mut self, ctx: &egui::Context) {
        self.playlist_dialog_window(ctx);
        self.track_picker_window(ctx);
        self.metadata_dialog_window(ctx);
        self.delete_confirm_window(ctx);
    }

    fn playlist_dialog_window(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.playlist_dialog.take() else {
            return;
        };

        let cover_texture = dialog
            .cover
            .as_ref()
            .and_then(|path| self.playlist_cover_texture(path));
        let track_titles: Vec<(String, String)> = dialog
            .tracks
            .iter()
            .map(|path| {
                self.library
                    .iter()
                    .find(|track| &track.path == path)
                    .map(|track| (track.title.clone(), track.artist.clone()))
                    .unwrap_or_else(|| {
                        let name = path
                            .file_stem()
                            .map(|stem| stem.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Arquivo ausente".to_owned());
                        (name, "não encontrado na biblioteca".to_owned())
                    })
            })
            .collect();

        let mut save = false;
        let mut cancel = false;
        let mut choose_cover = false;
        let mut clear_cover = false;
        let mut add_tracks = false;
        let mut remove_track = None;
        let title = if dialog.editing.is_some() {
            "Editar playlist"
        } else {
            "Nova playlist"
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(dialog_frame())
            .show(ctx, |ui| {
                ui.set_width(410.0);
                small_label(ui, "Nome");
                egui::Frame::NONE
                    .fill(BG)
                    .corner_radius(CornerRadius::same(9))
                    .stroke(Stroke::new(1.0, LINE))
                    .inner_margin(egui::Margin::symmetric(10, 7))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut dialog.name)
                                .hint_text("Ex.: Para dirigir")
                                .font(theme::font(12.0))
                                .text_color(TEXT)
                                .desired_width(f32::INFINITY)
                                .frame(egui::Frame::NONE)
                                .margin(egui::Margin::ZERO),
                        );
                    });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let size = 118.0;
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(
                        rect.translate(Vec2::new(0.0, 3.0)),
                        CornerRadius::same(12),
                        alpha(Color32::BLACK, 0.3),
                    );
                    if let Some(texture) = &cover_texture {
                        let sized = egui::load::SizedTexture::new(texture.id(), rect.size());
                        ui.put(
                            rect,
                            egui::Image::new(sized).corner_radius(CornerRadius::same(12)),
                        );
                    } else {
                        let initial = dialog
                            .name
                            .chars()
                            .next()
                            .map(|c| c.to_uppercase().to_string())
                            .unwrap_or_else(|| "P".to_owned());
                        widgets::paint_placeholder_art(&painter, rect, &initial, false, 28.0);
                    }

                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        small_label(ui, "Capa da playlist");
                        ui.label(
                            RichText::new("JPG ou PNG · quadrada fica melhor")
                                .font(theme::font(9.5))
                                .color(TEXT_FAINT),
                        );
                        ui.add_space(8.0);
                        if widgets::action_button(ui, "Escolher capa", false, 0.0).clicked() {
                            choose_cover = true;
                        }
                        if dialog.cover.is_some()
                            && widgets::action_button(ui, "Remover capa", false, 0.0).clicked()
                        {
                            clear_cover = true;
                        }
                    });
                });

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    small_label(ui, &format!("Músicas ({})", dialog.tracks.len()));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::action_button(ui, "+ Adicionar", false, 0.0).clicked() {
                            add_tracks = true;
                        }
                    });
                });

                if track_titles.is_empty() {
                    ui.label(
                        RichText::new("Nenhuma música ainda. Adicione da sua biblioteca.")
                            .font(theme::font(10.5))
                            .color(TEXT_FAINT),
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (index, (title, artist)) in track_titles.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    if widgets::icon_button(
                                        ui,
                                        Icon::Close,
                                        15.0,
                                        false,
                                        "Remover da playlist",
                                    )
                                    .clicked()
                                    {
                                        remove_track = Some(index);
                                    }
                                    ui.add(
                                        Label::new(
                                            RichText::new(format!("{title} · {artist}"))
                                                .font(theme::font(10.5))
                                                .color(TEXT_MUTED),
                                        )
                                        .truncate(),
                                    );
                                });
                            }
                        });
                }

                if let Some(error) = &dialog.error {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(error)
                            .font(theme::font(10.5))
                            .color(ACCENT_BRIGHT),
                    );
                }

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if widgets::action_button(ui, "Cancelar", false, 96.0).clicked() {
                        cancel = true;
                    }
                    if widgets::action_button(ui, "Salvar playlist", true, 150.0).clicked() {
                        save = true;
                    }
                });
            });

        if let Some(index) = remove_track {
            if index < dialog.tracks.len() {
                dialog.tracks.remove(index);
            }
        }
        if choose_cover {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Escolha a capa da playlist")
                .add_filter("Imagens", &["jpg", "jpeg", "png"])
                .pick_file()
            {
                dialog.cover = Some(path);
                dialog.cover_is_new = true;
                dialog.cover_removed = false;
            }
        }
        if clear_cover {
            dialog.cover = None;
            dialog.cover_is_new = false;
            dialog.cover_removed = true;
        }
        if add_tracks {
            self.track_picker = Some(TrackPicker {
                target: PickerTarget::Dialog,
                search: String::new(),
                selected: Vec::new(),
            });
        }

        if cancel {
            return;
        }
        if save {
            self.playlist_dialog = Some(dialog);
            self.save_playlist_dialog();
        } else {
            self.playlist_dialog = Some(dialog);
        }
    }

    fn track_picker_window(&mut self, ctx: &egui::Context) {
        let Some(mut picker) = self.track_picker.take() else {
            return;
        };

        let query = picker.search.trim().to_lowercase();
        let candidates: Vec<(PathBuf, String, String)> = self
            .library
            .iter()
            .filter(|track| {
                if query.is_empty() {
                    return true;
                }
                format!("{} {} {}", track.title, track.artist, track.album)
                    .to_lowercase()
                    .contains(&query)
            })
            .map(|track| {
                (
                    track.path.clone(),
                    track.title.clone(),
                    track.artist.clone(),
                )
            })
            .collect();

        let mut apply = false;
        let mut cancel = false;

        egui::Window::new("Adicionar músicas")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(dialog_frame())
            .show(ctx, |ui| {
                ui.set_width(440.0);
                egui::Frame::NONE
                    .fill(BG)
                    .corner_radius(CornerRadius::same(9))
                    .stroke(Stroke::new(1.0, LINE))
                    .inner_margin(egui::Margin::symmetric(10, 7))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut picker.search)
                                .hint_text("Pesquisar na biblioteca…")
                                .font(theme::font(11.5))
                                .text_color(TEXT)
                                .desired_width(f32::INFINITY)
                                .frame(egui::Frame::NONE)
                                .margin(egui::Margin::ZERO),
                        );
                    });
                ui.add_space(8.0);

                if candidates.is_empty() {
                    ui.label(
                        RichText::new("Nenhuma música encontrada.")
                            .font(theme::font(11.0))
                            .color(TEXT_FAINT),
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(280.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (path, title, artist) in &candidates {
                                let selected = picker.selected.contains(path);
                                let label = format!("{title}  ·  {artist}");
                                if ui.selectable_label(selected, label).clicked() {
                                    if selected {
                                        picker.selected.retain(|item| item != path);
                                    } else {
                                        picker.selected.push(path.clone());
                                    }
                                }
                            }
                        });
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if widgets::action_button(ui, "Cancelar", false, 96.0).clicked() {
                        cancel = true;
                    }
                    let label = format!("Adicionar ({})", picker.selected.len());
                    if widgets::action_button(ui, &label, true, 150.0).clicked() {
                        apply = true;
                    }
                });
            });

        if cancel {
            return;
        }
        self.track_picker = Some(picker);
        if apply {
            self.apply_picker();
        }
    }

    fn metadata_dialog_window(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.metadata_dialog.take() else {
            return;
        };

        let cover_texture = dialog
            .cover
            .as_ref()
            .and_then(|path| self.playlist_cover_texture(path));
        let file_name = dialog
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut save = false;
        let mut cancel = false;
        let mut choose_cover = false;
        let mut clear_cover = false;

        egui::Window::new("Editar metadados")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(dialog_frame())
            .show(ctx, |ui| {
                ui.set_width(410.0);
                ui.label(
                    RichText::new(&file_name)
                        .font(theme::font(9.5))
                        .color(TEXT_FAINT),
                );
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let size = 108.0;
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(
                        rect.translate(Vec2::new(0.0, 3.0)),
                        CornerRadius::same(12),
                        alpha(Color32::BLACK, 0.3),
                    );
                    if let Some(texture) = &cover_texture {
                        let sized = egui::load::SizedTexture::new(texture.id(), rect.size());
                        ui.put(
                            rect,
                            egui::Image::new(sized).corner_radius(CornerRadius::same(12)),
                        );
                    } else {
                        let initial = dialog
                            .title
                            .chars()
                            .next()
                            .map(|c| c.to_uppercase().to_string())
                            .unwrap_or_else(|| "M".to_owned());
                        widgets::paint_placeholder_art(&painter, rect, &initial, false, 26.0);
                    }

                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        small_label(ui, "Capa embutida no arquivo");
                        ui.label(
                            RichText::new("A capa fica salva dentro do MP3/FLAC/M4A.")
                                .font(theme::font(9.5))
                                .color(TEXT_FAINT),
                        );
                        ui.add_space(8.0);
                        if widgets::action_button(ui, "Escolher capa", false, 0.0).clicked() {
                            choose_cover = true;
                        }
                        if (dialog.cover.is_some() || !dialog.remove_cover)
                            && widgets::action_button(ui, "Remover capa", false, 0.0).clicked()
                        {
                            clear_cover = true;
                        }
                    });
                });

                ui.add_space(12.0);
                metadata_field(ui, "Título", &mut dialog.title);
                metadata_field(ui, "Artista", &mut dialog.artist);
                metadata_field(ui, "Álbum", &mut dialog.album);

                if let Some(error) = &dialog.error {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(error)
                            .font(theme::font(10.5))
                            .color(ACCENT_BRIGHT),
                    );
                }

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if widgets::action_button(ui, "Cancelar", false, 96.0).clicked() {
                        cancel = true;
                    }
                    if widgets::action_button(ui, "Salvar metadados", true, 160.0).clicked() {
                        save = true;
                    }
                });
            });

        if choose_cover {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Escolha a capa")
                .add_filter("Imagens", &["jpg", "jpeg", "png"])
                .pick_file()
            {
                dialog.cover = Some(path);
                dialog.cover_is_new = true;
                dialog.remove_cover = false;
            }
        }
        if clear_cover {
            dialog.cover = None;
            dialog.cover_is_new = false;
            dialog.remove_cover = true;
        }

        if cancel {
            return;
        }
        if save {
            if self.last_track_path.as_ref() == Some(&dialog.path) {
                if let Some(player) = &mut self.playback {
                    player.stop();
                }
                self.playing = false;
            }
            let cover = if dialog.cover_is_new {
                dialog.cover.as_deref()
            } else {
                None
            };
            match library::write_metadata(
                &dialog.path,
                &dialog.title,
                &dialog.artist,
                &dialog.album,
                cover,
                dialog.remove_cover && !dialog.cover_is_new,
            ) {
                Ok(()) => {
                    self.toast("Metadados salvos.".to_owned());
                    self.restart_scan();
                }
                Err(error) => {
                    dialog.error = Some(error);
                    self.metadata_dialog = Some(dialog);
                }
            }
        } else {
            self.metadata_dialog = Some(dialog);
        }
    }

    fn delete_confirm_window(&mut self, ctx: &egui::Context) {
        let Some(id) = self.confirm_delete_playlist else {
            return;
        };
        let Some(playlist) = self.playlists.iter().find(|item| item.id == id).cloned() else {
            self.confirm_delete_playlist = None;
            return;
        };

        let mut confirm = false;
        let mut cancel = false;
        egui::Window::new("Excluir playlist")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(dialog_frame())
            .show(ctx, |ui| {
                ui.set_width(340.0);
                ui.label(
                    RichText::new(format!("Excluir “{}”?", playlist.name))
                        .font(theme::font_semi(14.0))
                        .color(TEXT),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("As músicas continuam na sua biblioteca. Apenas a playlist é removida.")
                        .font(theme::font(11.0))
                        .color(TEXT_MUTED),
                );
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if widgets::action_button(ui, "Cancelar", false, 96.0).clicked() {
                        cancel = true;
                    }
                    if widgets::action_button(ui, "Excluir", true, 100.0).clicked() {
                        confirm = true;
                    }
                });
            });

        if cancel {
            self.confirm_delete_playlist = None;
        }
        if confirm {
            if let Some(cover) = playlist.cover.clone() {
                self.discard_managed_cover(&cover);
            }
            self.playlists.retain(|item| item.id != id);
            if self.view == View::PlaylistDetail(id) {
                self.view = View::Playlists;
            }
            self.confirm_delete_playlist = None;
            self.mark_dirty();
            self.toast("Playlist excluída.".to_owned());
        }
    }

    fn player_bar(&mut self, ui: &mut egui::Ui) {
        let full = ui.available_width();
        let height = ui.available_height();
        let show_volume_bar = full > 720.0;
        let left_w = (full * 0.27).clamp(210.0, 310.0);
        let right_w = if show_volume_bar { 200.0 } else { 62.0 };
        let center_w = (full - left_w - right_w - 24.0).max(240.0);

        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(left_w, height),
                Layout::left_to_right(Align::Center),
                |ui| self.now_playing_mini(ui, left_w),
            );

            ui.allocate_ui_with_layout(
                Vec2::new(center_w, height),
                Layout::top_down(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.add_space(12.0);
                    self.transport_controls(ui, 40.0);
                    ui.add_space(4.0);
                    self.timeline(ui, center_w - 10.0);
                },
            );

            ui.allocate_ui_with_layout(
                Vec2::new(right_w, height),
                Layout::right_to_left(Align::Center),
                |ui| self.player_extras(ui, show_volume_bar),
            );
        });
    }

    fn now_playing_mini(&mut self, ui: &mut egui::Ui, width: f32) {
        let art_size = 56.0;
        let heart_size = 26.0;
        let text_width = (width - art_size - heart_size - 40.0).max(70.0);

        let (art_rect, art_response) =
            ui.allocate_exact_size(Vec2::splat(art_size), Sense::click());

        if let Some(index) = self.current {
            if let Some(texture) = self.big_art.clone() {
                let sized = egui::load::SizedTexture::new(texture.id(), art_rect.size());
                ui.put(
                    art_rect,
                    egui::Image::new(sized).corner_radius(CornerRadius::same(9)),
                );
            } else if let Some(texture) = self.thumb_texture(index) {
                let sized = egui::load::SizedTexture::new(texture.id(), art_rect.size());
                ui.put(
                    art_rect,
                    egui::Image::new(sized).corner_radius(CornerRadius::same(9)),
                );
            } else {
                widgets::paint_vinyl(ui.painter(), art_rect.shrink(2.0), self.spin);
            }
        } else {
            widgets::paint_vinyl(ui.painter(), art_rect.shrink(2.0), self.spin);
        }

        if art_response.hovered() {
            ui.painter().rect_filled(
                art_rect,
                CornerRadius::same(9),
                alpha(Color32::BLACK, 0.35),
            );
        }

        ui.add_space(10.0);
        ui.allocate_ui_with_layout(
            Vec2::new(text_width, art_size),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_max_width(text_width);
                ui.add_space(8.0);
                if let Some(track) = self.current.and_then(|index| self.library.get(index)) {
                    ui.add(
                        Label::new(
                            RichText::new(track.title.clone())
                                .font(theme::font_semi(13.0))
                                .color(TEXT),
                        )
                        .truncate(),
                    );
                    ui.add(
                        Label::new(
                            RichText::new(track.artist.clone())
                                .font(theme::font(11.0))
                                .color(TEXT_MUTED),
                        )
                        .truncate(),
                    );
                } else {
                    ui.add(
                        Label::new(
                            RichText::new("Nada tocando")
                                .font(theme::font_semi(12.5))
                                .color(TEXT_MUTED),
                        )
                        .truncate(),
                    );
                    ui.add(
                        Label::new(
                            RichText::new("Escolha uma música")
                                .font(theme::font(11.0))
                                .color(TEXT_FAINT),
                        )
                        .truncate(),
                    );
                }
            },
        );

        if let Some(path) = self
            .current
            .and_then(|index| self.library.get(index))
            .map(|track| track.path.clone())
        {
            let is_favorite = self.favorites.contains(&path);
            if widgets::icon_button(
                ui,
                if is_favorite {
                    Icon::HeartFilled
                } else {
                    Icon::Heart
                },
                heart_size,
                is_favorite,
                "Favorito",
            )
            .clicked()
            {
                if is_favorite {
                    self.favorites.remove(&path);
                } else {
                    self.favorites.insert(path);
                }
                self.mark_dirty();
            }
        }

        if art_response.clicked() {
            self.view = View::NowPlaying;
        }
    }

    fn transport_controls(&mut self, ui: &mut egui::Ui, play_size: f32) {
        let small = 30.0;
        let side = 34.0;
        let gap = 8.0;
        let total = small * 2.0 + side * 2.0 + play_size + gap * 4.0;
        let available = ui.available_width();

        ui.horizontal(|ui| {
            ui.add_space(((available - total) * 0.5).max(0.0));

            if widgets::icon_button(ui, Icon::Shuffle, small, self.shuffle, "Modo aleatório")
                .clicked()
            {
                self.shuffle = !self.shuffle;
                self.mark_dirty();
            }
            if widgets::icon_button(ui, Icon::Prev, side, false, "Faixa anterior").clicked() {
                self.prev_track();
            }
            if widgets::play_button(ui, self.playing, play_size).clicked() {
                self.toggle_play();
            }
            if widgets::icon_button(ui, Icon::Next, side, false, "Próxima faixa").clicked() {
                self.next_track(false);
            }
            let repeat_resp = widgets::icon_button(
                ui,
                Icon::Repeat,
                small,
                self.repeat != Repeat::Off,
                self.repeat.tooltip(),
            );
            if repeat_resp.clicked() {
                self.repeat = self.repeat.next();
                self.mark_dirty();
            }
            if self.repeat == Repeat::One {
                let badge = repeat_resp.rect.center() + Vec2::new(5.5, -5.5);
                ui.painter()
                    .circle_filled(badge, 6.5, alpha(Color32::BLACK, 0.85));
                ui.painter().circle_stroke(
                    badge,
                    6.5,
                    Stroke::new(1.0, ACCENT_BRIGHT),
                );
                ui.painter().text(
                    badge,
                    Align2::CENTER_CENTER,
                    "1",
                    theme::font_semi(9.0),
                    ACCENT_BRIGHT,
                );
            }
        });
    }

    fn timeline(&mut self, ui: &mut egui::Ui, max_width: f32) {
        let duration = self.duration();
        let total_seconds = duration.as_secs_f32();
        let preview_position = self
            .seek_preview
            .map(|fraction| duration.mul_f32(fraction))
            .unwrap_or(self.position);
        let fraction = if total_seconds > 0.0 {
            (preview_position.as_secs_f32() / total_seconds).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let time_width = 38.0;
        let available = ui.available_width();
        let width = max_width.min(available).max(120.0);

        ui.horizontal(|ui| {
            ui.add_space(((available - width) * 0.5).max(0.0));
            ui.allocate_ui_with_layout(
                Vec2::new(width, 18.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(time_width, 18.0),
                        Layout::right_to_left(Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(fmt_time(preview_position))
                                    .font(theme::font(10.5))
                                    .color(TEXT_FAINT),
                            );
                        },
                    );

                    let bar_width = (width - time_width * 2.0 - 14.0).max(40.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(bar_width, 18.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            let result =
                                widgets::seek_bar(ui, fraction, total_seconds > 0.0);

                            if let Some(preview) = result.preview {
                                self.seek_preview = Some(preview);
                            }
                            if let Some(commit) = result.commit {
                                let target = duration.mul_f32(commit);
                                if let Some(player) = &mut self.playback {
                                    if let Err(message) = player.seek(target) {
                                        self.toast(message);
                                    }
                                }
                                self.position = target;
                                self.seek_preview = None;
                            }
                        },
                    );

                    ui.allocate_ui_with_layout(
                        Vec2::new(time_width, 18.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(fmt_time(duration))
                                    .font(theme::font(10.5))
                                    .color(TEXT_FAINT),
                            );
                        },
                    );
                },
            );
        });
    }

    fn player_extras(&mut self, ui: &mut egui::Ui, show_volume_bar: bool) {
        
        
        let muted = self.volume <= 0.001;
        if widgets::icon_button(
            ui,
            if muted { Icon::Muted } else { Icon::Volume },
            28.0,
            false,
            if muted { "Ativar som" } else { "Silenciar" },
        )
        .clicked()
        {
            self.toggle_mute();
        }

        if show_volume_bar {
            if let Some(value) = widgets::volume_bar(ui, self.volume, 92.0) {
                self.apply_volume(value);
            }
        }
    }

    fn draw_toast(&mut self, ui: &mut egui::Ui) {
        let now = ui.ctx().input(|input| input.time);
        let Some((message, until)) = self.status.clone() else {
            return;
        };
        if now >= until {
            self.status = None;
            return;
        }

        let painter = ui.painter();
        let font = theme::font_medium(12.0);
        let galley = painter.layout_no_wrap(message, font, TEXT);
        let padding = Vec2::new(16.0, 10.0);
        let size = galley.size() + padding * 2.0;
        let screen = ui.max_rect();
        let center = Pos2::new(screen.center().x, screen.bottom() - 132.0);
        let rect = Rect::from_center_size(center, size);

        painter.rect_filled(
            rect.translate(Vec2::new(0.0, 3.0)),
            CornerRadius::same(10),
            alpha(Color32::BLACK, 0.35),
        );
        painter.rect_filled(rect, CornerRadius::same(10), Color32::from_rgb(34, 27, 23));
        painter.rect_stroke(
            rect,
            CornerRadius::same(10),
            Stroke::new(1.0, LINE),
            egui::StrokeKind::Inside,
        );
        painter.circle_filled(
            Pos2::new(rect.left() + 12.0, rect.center().y),
            3.0,
            ACCENT_BRIGHT,
        );
        painter.galley(rect.min + padding, galley, TEXT);

        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}

impl eframe::App for ResoRustApp {
    
    
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ctx = ctx.clone();
        self.poll_scan();
        self.poll_download();
        self.tick();

        
        
        if self.view == View::Downloads
            && ctx.input(|input| {
                input.key_pressed(egui::Key::V) && input.modifiers.ctrl
                    || input.events.iter().any(|event| {
                        matches!(event, egui::Event::Paste(_))
                    })
            })
            && let Some(link) = read_clipboard()
        {
            self.download.url = link.trim().to_owned();
        }

        self.maybe_save();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ctx = ui.ctx().clone();
        self.ensure_big_art();
        self.shortcuts();

        egui::Panel::bottom("resorust_player")
            .exact_size(PLAYER_BAR_HEIGHT)
            .frame(
                egui::Frame::NONE
                    .fill(PLAYER_BG)
                    .inner_margin(egui::Margin::symmetric(22, 12))
                    .stroke(Stroke::new(1.0, LINE)),
            )
            .show(ui, |ui| self.player_bar(ui));

        let sidebar = egui::Panel::left("resorust_sidebar")
            .exact_size(238.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::NONE
                    .fill(SIDEBAR_BG)
                    .inner_margin(egui::Margin::symmetric(14, 18)),
            )
            .show(ui, |ui| self.sidebar(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG).inner_margin(egui::Margin {
                left: 26,
                right: 26,
                top: 22,
                bottom: 8,
            }))
            .show(ui, |ui| self.main_content(ui));

        
        
        let player_top = ui.max_rect().max.y - PLAYER_BAR_HEIGHT;
        ui.painter().vline(
            sidebar.response.rect.right() - 0.5,
            0.0..=player_top,
            Stroke::new(1.0, LINE),
        );

        self.draw_toast(ui);

        if self.ctx.input(|input| input.viewport().close_requested()) && self.dirty {
            self.save_state();
        }
    }
}





#[cfg(target_os = "windows")]
fn read_clipboard() -> Option<String> {
    use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
    use windows::Win32::System::Ole::CF_UNICODETEXT;

    unsafe {
        OpenClipboard(None).ok()?;
        let handle = GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?;
        let pointer = handle.0 as *const u16;
        let mut text = String::new();
        if !pointer.is_null() {
            let mut index = 0usize;
            while *pointer.add(index) != 0 {
                if let Some(character) = char::from_u32(*pointer.add(index) as u32) {
                    text.push(character);
                }
                index += 1;
            }
        }
        let _ = CloseClipboard();
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(not(target_os = "windows"))]
fn read_clipboard() -> Option<String> {
    None
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(
        RichText::new(text)
            .font(theme::font_medium(8.5))
            .color(TEXT_FAINT),
    );
    ui.add_space(2.0);
}

fn small_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme::font_medium(10.0))
            .color(TEXT_FAINT),
    );
    ui.add_space(2.0);
}

fn dialog_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(SURFACE)
        .corner_radius(CornerRadius::same(16))
        .inner_margin(egui::Margin::same(18))
        .stroke(Stroke::new(1.0, LINE))
        .shadow(egui::epaint::Shadow {
            offset: [0, 8],
            blur: 28,
            spread: 0,
            color: alpha(Color32::BLACK, 0.55),
        })
}

fn metadata_field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    small_label(ui, label);
    egui::Frame::NONE
        .fill(BG)
        .corner_radius(CornerRadius::same(9))
        .stroke(Stroke::new(1.0, LINE))
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .font(theme::font(12.0))
                    .text_color(TEXT)
                    .desired_width(f32::INFINITY)
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO),
            );
        });
    ui.add_space(6.0);
}

fn settings_card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::NONE
        .fill(SURFACE)
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(18))
        .stroke(Stroke::new(1.0, LINE))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(title)
                    .font(theme::font_semi(13.0))
                    .color(TEXT),
            );
            ui.add_space(10.0);
            add(ui);
        });
}

fn row_geom(rect: Rect, show_album: bool) -> RowGeom {
    let pad = 12.0;
    let index_width = 26.0;
    let art_size = 42.0;
    let artist_width = if rect.width() > 700.0 { 150.0 } else { 100.0 };
    let album_width = if show_album { 150.0 } else { 0.0 };
    let duration_width = 50.0;
    let heart_size = 26.0;

    let idx = Rect::from_min_size(
        Pos2::new(rect.left() + pad, rect.top()),
        Vec2::new(index_width, rect.height()),
    );
    let art = Rect::from_center_size(
        Pos2::new(
            rect.left() + pad + index_width + art_size * 0.5,
            rect.center().y,
        ),
        Vec2::splat(art_size),
    );
    let heart = Rect::from_center_size(
        Pos2::new(rect.right() - 20.0, rect.center().y),
        Vec2::splat(heart_size),
    );
    let duration = Rect::from_min_max(
        Pos2::new(heart.left() - 12.0 - duration_width, rect.top()),
        Pos2::new(heart.left() - 12.0, rect.bottom()),
    );
    let album_right = duration.left() - 10.0;
    let album = Rect::from_min_max(
        Pos2::new(album_right - album_width, rect.top()),
        Pos2::new(album_right, rect.bottom()),
    );
    let artist_right = if show_album {
        album.left() - 12.0
    } else {
        duration.left() - 12.0
    };
    let artist = Rect::from_min_max(
        Pos2::new(artist_right - artist_width, rect.top()),
        Pos2::new(artist_right, rect.bottom()),
    );
    let title = Rect::from_min_max(
        Pos2::new(art.right() + 12.0, rect.top()),
        Pos2::new(artist.left() - 12.0, rect.bottom()),
    );

    RowGeom {
        idx,
        art,
        title,
        artist,
        album,
        duration,
        heart,
    }
}

fn fmt_time(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}
