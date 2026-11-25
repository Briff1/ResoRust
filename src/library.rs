

use lofty::config::WriteOptions;
use lofty::picture::{Picture, PictureType};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{ItemKey, Tag};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use walkdir::WalkDir;


pub const THUMB_SIZE: u32 = 96;

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Duration,
    
    pub thumb: Option<Arc<Vec<u8>>>,
}

pub const EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a"];

pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            EXTENSIONS.contains(&e.as_str())
        })
        .unwrap_or(false)
}


pub fn scan(dir: &Path) -> Vec<Track> {
    let mut tracks = Vec::new();
    let mut folder_covers: HashMap<PathBuf, Option<Arc<Vec<u8>>>> = HashMap::new();

    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if is_supported(path) {
            let parent = path.parent().unwrap_or(dir).to_path_buf();
            let cover = folder_covers
                .entry(parent.clone())
                .or_insert_with(|| {
                    find_folder_cover(&parent)
                        .and_then(|bytes| make_thumb(&bytes))
                        .map(Arc::new)
                })
                .clone();
            tracks.push(read_track(path, cover));
        }
    }

    tracks.sort_by(|a, b| {
        a.title
            .to_lowercase()
            .cmp(&b.title.to_lowercase())
            .then_with(|| a.artist.to_lowercase().cmp(&b.artist.to_lowercase()))
    });
    tracks
}


const FOLDER_COVER_NAMES: &[&str] = &[
    "cover.jpg",
    "cover.jpeg",
    "cover.png",
    "folder.jpg",
    "folder.jpeg",
    "folder.png",
    "front.jpg",
    "front.jpeg",
    "front.png",
    "album.jpg",
    "album.jpeg",
    "album.png",
    "albumart.jpg",
    "albumart.png",
    "artwork.jpg",
    "artwork.png",
];

fn find_folder_cover(dir: &Path) -> Option<Vec<u8>> {
    let mut candidates: Vec<(usize, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if let Some(position) = FOLDER_COVER_NAMES.iter().position(|c| *c == name) {
            candidates.push((position, entry.path()));
        }
    }
    candidates.sort_by_key(|(position, _)| *position);
    let (_, path) = candidates.into_iter().next()?;
    std::fs::read(path).ok()
}

fn make_thumb(bytes: &[u8]) -> Option<Vec<u8>> {
    let image = image::load_from_memory(bytes).ok()?;
    let thumb = image
        .resize_to_fill(
            THUMB_SIZE,
            THUMB_SIZE,
            image::imageops::FilterType::Triangle,
        )
        .to_rgba8();
    Some(thumb.into_raw())
}

fn read_track(path: &Path, folder_thumb: Option<Arc<Vec<u8>>>) -> Track {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Faixa sem nome".to_owned());

    let mut track = Track {
        path: path.to_path_buf(),
        title: stem.clone(),
        artist: "Artista desconhecido".to_owned(),
        album: "Álbum desconhecido".to_owned(),
        duration: Duration::ZERO,
        thumb: folder_thumb,
    };

    let Ok(tagged) = Probe::open(path).and_then(|probe| probe.read()) else {
        apply_stem_fallback(&mut track, &stem);
        return track;
    };

    track.duration = tagged.properties().duration();

    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        apply_stem_fallback(&mut track, &stem);
        return track;
    };

    let mut has_title = false;
    let mut has_artist = false;

    if let Some(title) = tag.title() {
        let title = title.trim();
        if !title.is_empty() {
            track.title = title.to_owned();
            has_title = true;
        }
    }
    if let Some(artist) = tag.artist() {
        let artist = artist.trim();
        if !artist.is_empty() {
            track.artist = artist.to_owned();
            has_artist = true;
        }
    }
    if let Some(album) = tag.album() {
        let album = album.trim();
        if !album.is_empty() {
            track.album = album.to_owned();
        }
    }

    if let Some(picture) = tag.pictures().first() {
        if let Some(thumb) = make_thumb(picture.data()) {
            track.thumb = Some(Arc::new(thumb));
        }
    }

    if !has_title || !has_artist {
        apply_stem_fallback(&mut track, &stem);
    }

    track
}


fn apply_stem_fallback(track: &mut Track, stem: &str) {
    let Some((artist, title)) = stem.split_once(" - ") else {
        return;
    };
    let artist = artist.trim();
    let title = title.trim();
    if artist.is_empty() || title.is_empty() {
        return;
    }
    if track.title == stem {
        track.title = title.to_owned();
    }
    if track.artist == "Artista desconhecido" {
        track.artist = artist.to_owned();
    }
}


pub fn load_full_art(path: &Path) -> Option<Vec<u8>> {
    if let Ok(tagged) = Probe::open(path).and_then(|probe| probe.read()) {
        if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
            if let Some(picture) = tag.pictures().first() {
                return Some(picture.data().to_vec());
            }
        }
    }
    path.parent().and_then(find_folder_cover)
}


pub fn write_metadata(
    path: &Path,
    title: &str,
    artist: &str,
    album: &str,
    cover: Option<&Path>,
    remove_cover: bool,
) -> Result<(), String> {
    let mut tagged = Probe::open(path)
        .and_then(|probe| probe.read())
        .map_err(|error| format!("Não foi possível abrir o arquivo: {error}"))?;

    if tagged.primary_tag().is_none() {
        let tag_type = tagged.primary_tag_type();
        tagged.insert_tag(Tag::new(tag_type));
    }

    let tag = tagged
        .primary_tag_mut()
        .ok_or_else(|| "Este formato não aceita metadados.".to_owned())?;

    if title.trim().is_empty() {
        tag.remove_key(ItemKey::TrackTitle);
    } else {
        tag.set_title(title.trim().to_owned());
    }
    if artist.trim().is_empty() {
        tag.remove_key(ItemKey::TrackArtist);
    } else {
        tag.set_artist(artist.trim().to_owned());
    }
    if album.trim().is_empty() {
        tag.remove_key(ItemKey::AlbumTitle);
    } else {
        tag.set_album(album.trim().to_owned());
    }

    if remove_cover {
        tag.remove_picture_type(PictureType::CoverFront);
    }
    if let Some(cover_path) = cover {
        let bytes = std::fs::read(cover_path)
            .map_err(|error| format!("Não foi possível ler a capa: {error}"))?;
        let mut cursor = &bytes[..];
        let mut picture = Picture::from_reader(&mut cursor)
            .map_err(|error| format!("Imagem de capa inválida: {error}"))?;
        picture.set_pic_type(PictureType::CoverFront);
        tag.remove_picture_type(PictureType::CoverFront);
        tag.push_picture(picture);
    }

    tagged
        .save_to_path(path, WriteOptions::default())
        .map_err(|error| format!("Não foi possível gravar os metadados: {error}"))
}


pub fn pretty_path(path: &Path) -> String {
    let components: Vec<String> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str().map(ToOwned::to_owned))
        .collect();

    if components.len() <= 2 {
        path.display().to_string()
    } else {
        let tail = &components[components.len() - 2..];
        format!("…\\{}", tail.join("\\"))
    }
}

pub fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}
