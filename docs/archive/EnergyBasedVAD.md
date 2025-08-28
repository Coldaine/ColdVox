<!-- Archived from docs/4_vad/EnergyBasedVAD.md on 2025-08-26 -->
<archived>
<summary>Practical energy-based VAD (RMS/STE + optional ZCR) for 16 kHz PCM</summary>

<img src="https://r2cdn.perplexity.ai/pplx-full-logo-primary-dark%402x.png" style="height:64px;margin-right:32px"/>

# Practical energy-based VAD (RMS/STE + optional ZCR) for 16 kHz PCM

## Quoted lines from docs

- librosa effects.split
> The threshold (in decibels) below reference to consider as silence … The reference amplitude. By default, it uses np.max and compares to the peak amplitude in the signal. Frame length … hop length … the number of samples between analysis frames.[^1]
- SoX “silence” effect (example)
> rec … silence 1 0.50 0.1% 1 10:00 0.1% | … sox … silence 1 0.50 0.1% 1 2.0 0.1% : newfile : restart — records … and splits … at points with 2 seconds of silence. Also, it does not start recording until it detects audio is playing and stops after it sees 10 minutes of silence.[^2]
- pyAudioAnalysis (frame norms)
> Widely accepted short-term window sizes are 20 to 100 ms. … short-term process can be conducted either using overlapping (frame step is shorter than the frame length).[^3]


## Minimal algorithm spec (step-by-step)

1) Framing and hop

- Use 20 ms frames at 16 kHz → 320 samples per frame; hop 10–20 ms for overlap and smooth decisions.[^3]

2) Preprocessing (recommended)

- Apply either a simple DC-removal/high-pass stage or a first-order HPF around ~100 Hz to reduce DC/rumble; a first-order HPF is standard and cheap to implement.[^4]
- Optionally apply pre-emphasis y[n] = x[n] − α x[n−1] with α ≈ 0.97 to tilt speech spectrum and stabilize energy measures; typical α values 0.95–0.97 in speech pipelines.[^5][^6]

3) Per-frame features

- Short-time energy (STE) or RMS: RMS = sqrt(mean(x^2)) over the frame; STE is equivalent before the square root for monotonic comparisons.[^1]
- Optional ZCR to guard against noise/music false positives: compute fraction of sign changes per frame; the API definition is “fraction of zero crossings in frame i.”[^7]

4) Adaptive floor with EMA

- Maintain an EMA noise floor of log-energy (dB) over frames believed to be non-speech; dynamic thresholds are common for silence removal and segmentation.[^3][^1]
- Update only when the detector is “inactive” (below off-threshold) to avoid biasing the floor with speech.[^3]

5) Thresholding with hysteresis (in dB)

- On-threshold = floor + K_on dB; Off-threshold = On − K_hyst dB, like “silence”/“not-silence” durations used in SoX to avoid reacting to brief bursts.[^2][^1]
- Combine an energy gate with an optional ZCR constraint to reduce false positives: require ZCR below a small normalized gate to accept “voiced-like” frames; ZCR is a standard voiced/unvoiced cue.[^7]

6) Debounce and hangover (durations)

- Require min_speech_ms before issuing SpeechStart; require min_silence_ms before SpeechEnd, mirroring silence effect’s duration semantics for robust segmentation.[^2]
- Add pre-roll and post-roll buffers around events to include leading/trailing context (queue frames while pending start/end).[^2]

7) Clipping guards (optional but practical)

- Ignore frames with peak |x| > 0.95 FS (likely clipped) when updating the noise floor to avoid skew; cap RMS using a winsorized or limiter-like cap to reduce clipping bias; this is an engineering safeguard consistent with headroom practice.[^2]

8) Outputs

- Per frame: boolean active flag. Emit SpeechStart/SpeechEnd when debounced state changes, with pre/post-roll applied to the boundaries.[^1][^2]


## Final recommended defaults

Recommended defaults reflect standard short-term settings, SoX-style hysteresis via duration, librosa’s dB-relative threshold concept, and common pre-emphasis practice.[^6][^5][^2][^1][^3]

```
frame_ms = 20
hop_ms = 10
hpf_fc_hz = 100
pre_emphasis = 0.97

ema_alpha = 0.02
on_db_above_floor = 9.0
off_db_below_on = 3.0

min_speech_ms = 200
min_silence_ms = 400
pre_roll_ms = 150
post_roll_ms = 200

zcr_gate = 0.10  // normalized fraction per frame; optional
```

Why these are typical: 20 ms frames with 10 ms hop are standard for short-term speech analysis; 0.95–0.97 pre-emphasis is widely used; dB thresholds relative to a reference (peak or EMA’d floor) are conventional; hysteresis and hangover mirror SoX’s “silence” durations to avoid chatter.[^6][^5][^2][^1][^3]

## Compact Rust-style pseudocode

```rust
struct VadConfig {
	sample_rate: usize,            // 16000
	frame_len: usize,              // 320 (20 ms)
	hop_len: usize,                // 160 (10 ms)
	hpf_fc_hz: f32,                // 100
	pre_emphasis: f32,             // 0.97
	ema_alpha: f32,                // 0.02
	on_db_above_floor: f32,        // 9.0
	off_db_below_on: f32,          // 3.0
	min_speech_ms: u32,            // 200
	min_silence_ms: u32,           // 400
	pre_roll_ms: u32,              // 150
	post_roll_ms: u32,             // 200
	zcr_gate: Option<f32>,         // Some(0.10) or None
}

struct VadState {
	floor_db: f32,                 // EMA noise floor (dB)
	active: bool,                  // current VAD state
	speech_run_ms: u32,            // accumulated active ms
	silence_run_ms: u32,           // accumulated inactive ms
	preroll_buf: Vec<Vec<f32>>,    // circular buffer of frames
	postroll_ms_left: u32,         // countdown after SpeechEnd
}

fn preprocess_frame(x: &[f32], cfg: &VadConfig, mem: &mut (f32, f32)) -> Vec<f32> {
	// mem: (preemph_last, hpf_state)
	let (mut z1, mut hp) = *mem;
	// pre-emphasis y[n] = x[n] - a*x[n-1]
	let mut y: Vec<f32> = x.iter().map(|&s| {
		let out = s - cfg.pre_emphasis * z1;
		z1 = s;
		out
	}).collect();
	// simple first-order HPF via DC-leak (one-pole): y_hp[n] = y[n] - y[n-1] + beta*y_hp[n-1]
	// choose beta from fc≈100 Hz; precompute beta = exp(-2π fc / fs)
	let beta = (-2.0 * std::f32::consts::PI * cfg.hpf_fc_hz / cfg.sample_rate as f32).exp();
	for v in &mut y {
		hp = *v - hp + beta * hp;
		*v = hp;
	}
	*mem = (z1, hp);
	y
}

fn frame_rms_db(frame: &[f32]) -> (f32, bool) {
	let mut peak = 0.0f32;
	let mut e = 0.0f32;
	for &s in frame {
		let a = s.abs();
		if a > peak { peak = a; }
		e += s * s;
	}
	let clipped = peak > 0.95;            // guard
	let rms = (e / frame.len() as f32).sqrt();
	let db = 20.0 * (rms.max(1e-8)).log10();
	(db, clipped)
}

fn frame_zcr(frame: &[f32]) -> f32 {
	let mut crossings = 0usize;
	let mut prev = frame[^0].signum();
	for &s in &frame[1..] {
		let cur = s.signum();
		if cur != 0.0 && cur != prev { crossings += 1; }
		if cur != 0.0 { prev = cur; }
	}
	crossings as f32 / frame.len() as f32
}

fn vad_step(
	cfg: &VadConfig,
	st: &mut VadState,
	frame: &[f32],
	frame_ms: u32,
) -> (bool, Option<&'static str>) {
	let (db, clipped) = frame_rms_db(frame);
	let use_zcr = cfg.zcr_gate.is_some();
	let zcr_ok = if let Some(g) = cfg.zcr_gate { frame_zcr(frame) <= g } else { true };

	// Update floor only on inactive frames and non-clipped frames
	if !st.active && !clipped {
		st.floor_db = (1.0 - cfg.ema_alpha) * st.floor_db + cfg.ema_alpha * db;
	}

	let on_db = st.floor_db + cfg.on_db_above_floor;
	let off_db = on_db - cfg.off_db_below_on;

	let energy_gate = if st.active { db >= off_db } else { db >= on_db };
	let gate = energy_gate && (!use_zcr || zcr_ok);

	let mut event = None;

	if st.active {
		if gate {
			st.silence_run_ms = 0;
			st.speech_run_ms += frame_ms;
		} else {
			st.silence_run_ms += frame_ms;
			if st.silence_run_ms >= cfg.min_silence_ms {
				st.active = false;
				st.speech_run_ms = 0;
				st.postroll_ms_left = cfg.post_roll_ms;
				event = Some("SpeechEnd");
			}
		}
	} else {
		// inactive
		if gate {
			st.speech_run_ms += frame_ms;
			if st.speech_run_ms >= cfg.min_speech_ms {
				st.active = true;
				st.silence_run_ms = 0;
				event = Some("SpeechStart");
			}
		} else {
			st.speech_run_ms = 0;
		}
	}

	// pre/post roll buffering would be handled by queueing frames when pending start/end

	(st.active, event)
}
```

- 20 ms/10 ms framing is standard for short-term speech analysis.[^3]
- dB-threshold relative to a reference is consistent with common APIs like librosa; hysteresis mirrors SoX’s “silence” durations.[^2][^1]
- ZCR as a per-frame fraction and voiced/unvoiced cue is standard.[^7]
- Pre-emphasis and simple HPF are conventional preprocessing stages.[^4][^5][^6]


## Tuning-free defaults and rationale

- Frame/hop: 20 ms frames, 10 ms hop balance temporal resolution and stationarity assumptions; these are standard speech settings.[^3]
- Thresholds: On = floor + 9 dB; Off = On − 3 dB provide robust separation with modest hysteresis, paralleling “silence” effect practices that rely on duration and thresholds to avoid reacting to bursts.[^1][^2]
- EMA alpha = 0.02 gives a slowly adapting floor similar to dynamic thresholding used for silence removal.[^3]
- Debounce: min_speech ≈ 200 ms and min_silence ≈ 400 ms prevent choppy segmentation and align with SoX-style silence durations for start/stop stability.[^2]
- Pre/post-roll: 150/200 ms capture onsets and tails common in conversational speech boundaries.[^2]
- Pre-emphasis α = 0.97 is a widely used value in speech pipelines; include if desired to stabilize energy across bands.[^5][^6]
- ZCR gate ≈ 0.10 (normalized fraction) helps avoid high-ZCR unvoiced noise/music being flagged as speech; ZCR is computed as the fraction of zero crossings per frame.[^7]


## Edge cases and default behavior

- Stationary fan/HVAC noise: EMA sets a conservative floor; 9 dB margin plus 3 dB hysteresis and min_speech/min_silence avoid toggling on minor fluctuations.[^1][^3]
- Music with pauses: Energy alone can misfire; optional ZCR gate suppresses high-ZCR unvoiced-like frames, reducing false positives during percussive/noisy segments.[^7]
- Bursts and clicks: Hysteresis plus min_speech_ms prevents short transients from producing SpeechStart; SoX-like duration logic avoids reacting to brief noise bursts.[^2]
- Very quiet speech: Relative-to-floor thresholding (not absolute) allows detection if speech exceeds local noise by ≈9 dB; if environment SNR is extremely low, EMA and hysteresis still prevent chattering while enabling speech when sustained energy rises above floor.[^1][^3]


## License notes

- pyAudioAnalysis is open-source under the Apache License (per the paper), suitable as a permissive reference for feature extraction logic and silence removal concepts.[^3]
- The pseudocode above is original and provided for unrestricted use; it paraphrases documented methods (energy/ZCR, EMA thresholds, hysteresis) rather than copying library code.[^7][^1][^3]

---

Notes on citations used:

- Frame/hop norms and dynamic thresholding concepts come from pyAudioAnalysis.[^3]
- dB-relative thresholds and frame/hop definitions for splitting non-silence are from librosa.effects.split.[^1]
- Hysteresis/debounce via duration is aligned with SoX “silence” effect usage and examples.[^2]
- ZCR definition and use as a per-frame fraction comes from librosa ZCR docs.[^7]
- Pre-emphasis typical α values (0.95–0.97) are standard in speech processing tutorials and studies.[^6][^5]
- First-order HPF is a conventional DC/rumble removal approach in signal processing.[^4]
<span style="display:none">[^10][^11][^12][^13][^14][^15][^16][^17][^18][^19][^20][^21][^22][^23][^24][^25][^26][^27][^28][^29][^30][^31][^32][^8][^9]</span>

<div style="text-align: center">⁂</div>

[^1]: https://librosa.org/doc/latest/generated/librosa.effects.split.html

[^2]: https://audionyq.com/sox_man/sox.html

[^3]: https://pmc.ncbi.nlm.nih.gov/articles/PMC4676707/

[^4]: https://en.wikipedia.org/wiki/High-pass_filter

[^5]: https://haythamfayek.com/2016/04/21/speech-processing-for-machine-learning.html

[^6]: https://citeseerx.ist.psu.edu/document?repid=rep1\&type=pdf\&doi=07518a670153cb809ad965a651f7aff7171ddae3

[^7]: https://librosa.org/doc/latest/generated/librosa.feature.zero_crossing_rate.html

[^8]: https://digitalcardboard.com/blog/2009/08/25/the-sox-of-silence/

[^9]: https://stackoverflow.com/questions/36998949/sox-effect-retriggerable-silence

[^10]: https://ankitshah009.blogspot.com/2017/03/sox-of-silence-original-post.html

[^11]: https://stackoverflow.com/questions/36998949/sox-effect-retriggerable-silence/38285915

[^12]: https://digitalcardboard.com/blog/2009/08/25/the-sox-of-silence/comment-page-1/

[^13]: https://librosa.org/doc-playground/0.8.1/generated/librosa.effects.split.html

[^14]: https://arxiv.org/pdf/2502.17579.pdf

[^15]: https://stackoverflow.com/questions/70311228/find-the-best-decibel-threshold-to-split-an-audio-into-segments-with-and-without

[^16]: https://stackoverflow.com/questions/6071432/issue-implementing-energy-threshold-algorithm-for-voice-activity-detection

[^17]: https://github.com/linan2/Voice-activity-detection-VAD-paper-and-code

[^18]: https://github.com/rymshasaeed/Voice-Activity-Detection

[^19]: https://stackoverflow.com/questions/30409539/scipy-numpy-audio-classifier-voice-speech-activity-detection

[^20]: https://jad.shahroodut.ac.ir/article_2411_868275c6886e1a2dcf439785b0f313ee.pdf

[^21]: https://support.projectnaomi.com/docs/3.0.M7/configuration/vad.html

[^22]: https://bastian.rieck.me/blog/2014/simple_experiments_speech_detection/

[^23]: https://librosa.org/doc/0.11.0/generated/librosa.zero_crossings.html

[^24]: https://stackoverflow.com/questions/55656626/googles-webrtc-vad-algorithm-esp-aggressiveness

[^25]: https://librosa.org/doc-playground/main/generated/librosa.feature.zero_crossing_rate.html

[^26]: https://arxiv.org/pdf/2401.09315.pdf

[^27]: https://librosa.org/doc/main/generated/librosa.effects.preemphasis.html

[^28]: https://speechprocessingbook.aalto.fi/Preprocessing/Pre-emphasis.html

[^29]: https://apxml.com/courses/introduction-to-speech-recognition/chapter-2-processing-audio-signals/pre-emphasis-and-framing

[^30]: https://www.controlpaths.com/2023/12/09/dc-remover-fpga/

[^31]: https://www.sciencedirect.com/topics/engineering/emphasis-filter

[^32]: https://www.dsprelated.com/showarticle/58.php


</archived>
