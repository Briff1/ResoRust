

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct AudioEngine {
    
    _device: MixerDeviceSink,
    player: Player,
    loaded: Option<PathBuf>,
    track_duration: Duration,
    paused: bool,
    volume: f32,
}

impl AudioEngine {
    pub fn new(volume: f32) -> Result<Self, String> {
        let device = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| format!("Não foi possível abrir a saída de áudio: {e}"))?;
        let player = Player::connect_new(device.mixer());
        player.set_volume(volume.clamp(0.0, 1.0));

        Ok(Self {
            _device: device,
            player,
            loaded: None,
            track_duration: Duration::ZERO,
            paused: false,
            volume: volume.clamp(0.0, 1.0),
        })
    }

    pub fn play(&mut self, path: &Path) -> Result<(), String> {
        let file = File::open(path).map_err(|e| format!("Não foi possível abrir o arquivo: {e}"))?;
        let decoder =
            Decoder::try_from(file).map_err(|e| format!("Formato não suportado: {e}"))?;

        self.track_duration = decoder.total_duration().unwrap_or(Duration::ZERO);
        self.player.clear();
        self.player.set_volume(self.volume);
        self.player.append(decoder);
        self.player.play();

        self.loaded = Some(path.to_path_buf());
        self.paused = false;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.player.clear();
        self.loaded = None;
        self.track_duration = Duration::ZERO;
        self.paused = true;
    }

    pub fn pause(&mut self) {
        self.player.pause();
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.player.play();
        self.paused = false;
    }

    pub fn has_track(&self) -> bool {
        self.loaded.is_some()
    }

    pub fn finished(&self) -> bool {
        self.loaded.is_some() && self.player.empty() && !self.player.is_paused()
    }

    pub fn position(&self) -> Duration {
        self.player.get_pos()
    }

    pub fn track_duration(&self) -> Duration {
        self.track_duration
    }

    pub fn seek(&mut self, position: Duration) -> Result<(), String> {
        self.player
            .try_seek(position)
            .map_err(|e| format!("Não foi possível avançar na faixa: {e}"))
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.player.set_volume(self.volume);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    
    #[test]
    fn decodifica_a_faixa_de_demonstracao() {
        let dir = std::env::temp_dir().join("resorust-test-audio");
        let _ = std::fs::remove_dir_all(&dir);

        let path = crate::demo::generate(&dir).expect("gerar demo");
        let file = File::open(&path).expect("abrir demo");
        let decoder = Decoder::try_from(file).expect("decodificar demo");

        assert_eq!(decoder.sample_rate().get(), 44_100);
        assert_eq!(decoder.channels().get(), 1);

        let total = decoder.total_duration().expect("duração conhecida");
        assert!(
            total > Duration::from_secs(25) && total <= Duration::from_secs(29),
            "duração inesperada: {total:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
