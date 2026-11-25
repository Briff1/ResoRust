




use std::f32::consts::TAU;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

const SAMPLE_RATE: u32 = 44_100;

pub fn generate(dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("Não foi possível criar a pasta: {e}"))?;

    let path = dir.join("ResoRust - Faixa Demo.wav");
    if path.exists() {
        return Ok(path);
    }

    let duration = 28.0f32;
    let total = (SAMPLE_RATE as f32 * duration) as usize;
    let mut samples: Vec<f32> = Vec::with_capacity(total);

    
    let chords: [[f32; 3]; 8] = [
        [110.00, 130.81, 164.81],
        [87.31, 110.00, 130.81],
        [130.81, 164.81, 196.00],
        [98.00, 123.47, 146.83],
        [110.00, 130.81, 164.81],
        [87.31, 110.00, 130.81],
        [98.00, 123.47, 146.83],
        [130.81, 164.81, 196.00],
    ];

    let chord_len = duration / chords.len() as f32;

    for n in 0..total {
        let t = n as f32 / SAMPLE_RATE as f32;
        let chord_index = ((t / chord_len) as usize).min(chords.len() - 1);
        let local = t - chord_index as f32 * chord_len;
        let chord = chords[chord_index];

        let attack = (local / 0.55).min(1.0);
        let release = ((chord_len - local) / 0.9).min(1.0);
        let env = smooth(attack) * smooth(release);

        
        let mut pad = 0.0;
        for (k, freq) in chord.iter().enumerate() {
            let detune = 1.0 + (k as f32 - 1.0) * 0.0018;
            pad += 0.055 * (TAU * freq * detune * t).sin();
            pad += 0.022 * (TAU * freq * 2.0 * detune * t + 0.7).sin();
            pad += 0.010 * (TAU * freq * 3.0 * t + 1.4).sin();
        }

        
        let bass = 0.16 * (TAU * chord[0] * 0.5 * t).sin() * env;

        
        let step = 0.4375;
        let step_index = (local / step) as usize;
        let note_time = local - step_index as f32 * step;
        let note_index = [0usize, 1, 2, 1][step_index % 4];
        let note_freq = chord[note_index] * 4.0;
        let pluck_env = (-note_time * 5.5).exp();
        let arp = 0.085
            * pluck_env
            * ((TAU * note_freq * t).sin() + 0.35 * (TAU * note_freq * 2.0 * t).sin());

        
        let beat = local % 1.75;
        let kick_env = (-beat * 16.0).exp();
        let kick = 0.11 * kick_env * (TAU * (52.0 - beat * 14.0) * t).sin();

        let tremolo = 1.0 + 0.04 * (TAU * 0.25 * t).sin();
        let dry = (pad * env + bass + arp + kick) * tremolo * 0.9;

        let master = smooth((t / 1.8).min(1.0)) * smooth(((duration - t) / 2.6).min(1.0));
        samples.push(dry * master);
    }

    
    let delay = (SAMPLE_RATE as f32 * 0.26) as usize;
    for i in delay..samples.len() {
        let echo = samples[i - delay] * 0.26;
        samples[i] += echo;
    }

    
    let peak = samples
        .iter()
        .fold(0.0f32, |acc, s| acc.max(s.abs()))
        .max(0.001);
    let gain = 0.82 / peak;
    let pcm: Vec<i16> = samples
        .iter()
        .map(|s| (soft_clip(s * gain) * i16::MAX as f32) as i16)
        .collect();

    write_wav(&path, &pcm).map_err(|e| format!("Falha ao escrever o WAV: {e}"))?;
    Ok(path)
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn soft_clip(x: f32) -> f32 {
    let x = x.clamp(-2.0, 2.0);
    x / (1.0 + x.abs() * 0.35)
}

fn write_wav(path: &Path, pcm: &[i16]) -> std::io::Result<()> {
    let data_len = (pcm.len() * 2) as u32;
    let mut out = BufWriter::new(fs::File::create(path)?);

    out.write_all(b"RIFF")?;
    out.write_all(&(36 + data_len).to_le_bytes())?;
    out.write_all(b"WAVE")?;
    out.write_all(b"fmt ")?;
    out.write_all(&16u32.to_le_bytes())?;
    out.write_all(&1u16.to_le_bytes())?; 
    out.write_all(&1u16.to_le_bytes())?; 
    out.write_all(&SAMPLE_RATE.to_le_bytes())?;
    out.write_all(&(SAMPLE_RATE * 2).to_le_bytes())?; 
    out.write_all(&2u16.to_le_bytes())?; 
    out.write_all(&16u16.to_le_bytes())?; 
    out.write_all(b"data")?;
    out.write_all(&data_len.to_le_bytes())?;
    for sample in pcm {
        out.write_all(&sample.to_le_bytes())?;
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("resorust-test-{name}"))
    }

    #[test]
    fn gera_wav_valido_e_visivel_na_biblioteca() {
        let dir = temp_dir("demo-biblioteca");
        let _ = fs::remove_dir_all(&dir);

        let path = generate(&dir).expect("gerar demo");
        let bytes = fs::read(&path).expect("ler demo");

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 1, "mono");
        assert_eq!(
            u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            SAMPLE_RATE
        );
        assert!(bytes.len() > 2_000_000, "28 s de áudio");

        let tracks = crate::library::scan(&dir);
        assert_eq!(tracks.len(), 1);
        let track = &tracks[0];
        assert!(track.duration > Duration::from_secs(25));
        assert!(track.duration <= Duration::from_secs(29));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn arquivo_existente_e_reaproveitado() {
        let dir = temp_dir("demo-reuso");
        let _ = fs::remove_dir_all(&dir);

        let first = generate(&dir).expect("primeira geração");
        let len = fs::metadata(&first).unwrap().len();
        let second = generate(&dir).expect("segunda geração");

        assert_eq!(first, second);
        assert_eq!(fs::metadata(&second).unwrap().len(), len);

        let _ = fs::remove_dir_all(&dir);
    }
}
