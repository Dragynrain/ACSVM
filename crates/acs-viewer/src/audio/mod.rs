// SPDX-License-Identifier: GPL-3.0-or-later

//! Play Music's cues (client 6): the engine's `Event::Music` choices,
//! played one at a time through `cpal` on the desktop and in the browser.

pub mod cues;
pub mod synth;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use cues::Player;
use std::sync::{Arc, Mutex};

/// The sound output and its controls. The device opens at the first cue
/// after an input (a browser plays nothing before one).
pub struct Audio {
    /// Sound on (the Sound checkbox).
    pub on: bool,
    /// 0..=1 (the volume slider).
    pub volume: f32,
    /// An input has been seen.
    unlocked: bool,
    /// The device could not be opened: don't try again.
    failed: bool,
    out: Option<(cpal::Stream, Arc<Mutex<Player>>)>,
}

impl Default for Audio {
    fn default() -> Self {
        Audio {
            on: true,
            volume: 0.7,
            unlocked: false,
            failed: false,
            out: None,
        }
    }
}

impl Audio {
    /// The player saw an input: from now on a cue may open the device.
    pub fn unlock(&mut self) {
        self.unlocked = true;
    }

    /// Plays cue `n`, replacing what plays (client 6).
    pub fn play(&mut self, n: u8) {
        if !self.on || !self.unlocked {
            return;
        }
        self.open();
        if let Some((_, p)) = &self.out {
            let mut p = p.lock().unwrap();
            p.volume = self.volume;
            p.start(n);
        }
    }

    /// Applies the controls: the volume at once, and muting stops what
    /// plays.
    pub fn apply_controls(&mut self) {
        if let Some((_, p)) = &self.out {
            let mut p = p.lock().unwrap();
            p.volume = self.volume;
            if !self.on {
                p.stop();
            }
        }
    }

    fn open(&mut self) {
        if self.out.is_some() || self.failed {
            return;
        }
        match open_output() {
            Ok(out) => self.out = Some(out),
            Err(e) => {
                log::warn!("no sound: {e}");
                self.failed = true;
            }
        }
    }
}

/// The default output device, playing a fresh player's samples.
fn open_output() -> Result<(cpal::Stream, Arc<Mutex<Player>>), String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no output device")?;
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    let rate = config.sample_rate();
    let player = Arc::new(Mutex::new(Player::new(synth::bank(rate))));
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build::<f32>(&device, config.into(), player.clone()),
        cpal::SampleFormat::I16 => build::<i16>(&device, config.into(), player.clone()),
        cpal::SampleFormat::U16 => build::<u16>(&device, config.into(), player.clone()),
        other => return Err(format!("sample format {other} not supported")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, player))
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    player: Arc<Mutex<Player>>,
) -> Result<cpal::Stream, String> {
    let channels = config.channels as usize;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut p = player.lock().unwrap();
                for frame in data.chunks_mut(channels) {
                    let v = T::from_sample(p.next_sample());
                    frame.iter_mut().for_each(|s| *s = v);
                }
            },
            |e| log::warn!("sound: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Opens this machine's output and plays theme 13 for two seconds:
    /// `cargo test -p acs-viewer -- --ignored plays_a_cue`.
    #[test]
    #[ignore = "needs a sound device"]
    fn plays_a_cue_on_the_default_output() {
        let mut a = Audio::default();
        a.unlock();
        a.play(13);
        assert!(a.out.is_some(), "the output opened");
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    /// Writes every cue as `cue-NN.wav` (16-bit, 22,050 Hz) into
    /// `ACSVM_CUES_DIR`, to listen to them.
    #[test]
    #[ignore = "writes files"]
    fn write_cues_as_wav() {
        let Ok(dir) = std::env::var("ACSVM_CUES_DIR") else {
            return;
        };
        const RATE: u32 = 22_050;
        for (i, c) in synth::bank(RATE).iter().enumerate() {
            let data: Vec<u8> = c
                .iter()
                .flat_map(|v| ((v.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
                .collect();
            let mut w = Vec::new();
            w.extend_from_slice(b"RIFF");
            w.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
            w.extend_from_slice(b"WAVEfmt ");
            w.extend_from_slice(&16u32.to_le_bytes());
            w.extend_from_slice(&1u16.to_le_bytes());
            w.extend_from_slice(&1u16.to_le_bytes());
            w.extend_from_slice(&RATE.to_le_bytes());
            w.extend_from_slice(&(RATE * 2).to_le_bytes());
            w.extend_from_slice(&2u16.to_le_bytes());
            w.extend_from_slice(&16u16.to_le_bytes());
            w.extend_from_slice(b"data");
            w.extend_from_slice(&(data.len() as u32).to_le_bytes());
            w.extend_from_slice(&data);
            std::fs::write(format!("{dir}/cue-{:02}.wav", i + 1), w).unwrap();
        }
    }
}
