use helpers::{bucket_to_freq, closest_bucket_to_freq, bucket_spread};
use nih_plug::prelude::*;
use nih_plug::util::midi_note_to_freq;
use realfft::{RealFftPlanner, RealToComplex, ComplexToReal, num_complex::Complex32};
use std::sync::Arc;
use std::num::NonZeroU32;

mod helpers;
mod editor;

/*
 * Note: some of the skeleton of this project was based on the stft example plugin in the nih-plug
 * library. (https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/stft/src/lib.rs)
 * However, besides this, all other bits of code were written by myself.
 */

const WINDOW_SIZE: usize = 8192;
const GAIN_COMPENSATION: f32 = 1.0 / WINDOW_SIZE as f32;

struct PitchQuantizer {
    params: Arc<PitchQuantizerParams>,

    stft: util::StftHelper,
    r2c_plan: Arc<dyn RealToComplex<f32>>,
    c2r_plan: Arc<dyn ComplexToReal<f32>>,
    convert_fft_buffer: Vec<Complex32>,
    process_fft_buffer: Vec<Complex32>,
    window: Vec<f32>,
    
    bucket_freq: Vec<f32>,
    note_on: Vec<bool>
}

#[derive(Params)]
struct PitchQuantizerParams {
    #[id = "pull_strength"]
    pull_strength: FloatParam,
    #[id = "note_spread"]
    note_spread: FloatParam,
    #[id = "spread_falloff"]
    spread_falloff: FloatParam,
    #[id = "use_exact"]
    use_exact: BoolParam
}

impl Default for PitchQuantizerParams {
    fn default() -> Self {
        Self {
            pull_strength:
                FloatParam::new(
                    "Pull Strength",
                    1.0,
                    FloatRange::Linear {
                        min: 1.0,
                        max: 5.0
                    }
                )
                .with_smoother(SmoothingStyle::Linear(1f32)),
            note_spread:
                FloatParam::new(
                    "Note Spread Degree",
                    1.0,
                    FloatRange::Linear {
                        min: 0.0,
                        max: 1.0
                    }
                )
                .with_smoother(SmoothingStyle::Linear(1f32)),

            spread_falloff:
                FloatParam::new(
                    "Spread Falloff Degree",
                    0.0,
                    FloatRange::Linear {
                        min: 0.0,
                        max: 0.999
                    }
                )
                .with_smoother(SmoothingStyle::Linear(1f32)),
            use_exact:
                BoolParam::new(
                    "Use Exact MIDI",
                    true
                )
        }
    }
}

impl Default for PitchQuantizer {
    fn default() -> Self {
        let mut planner = RealFftPlanner::new();
        let r2c_plan = planner.plan_fft_forward(WINDOW_SIZE);
        let c2r_plan = planner.plan_fft_inverse(WINDOW_SIZE);
        let mut _real_fft_buffer = r2c_plan.make_input_vec();
        let convert_fft_buffer = r2c_plan.make_output_vec();
        let process_fft_buffer = r2c_plan.make_output_vec();

        Self {
            params: Arc::new(PitchQuantizerParams::default()),
            stft: util::StftHelper::new(2, WINDOW_SIZE, 0),
            r2c_plan,
            c2r_plan,
            convert_fft_buffer,
            process_fft_buffer,
            bucket_freq: (0..WINDOW_SIZE).map(|_| {0f32}).collect(),
            note_on: (0..128).map(|_| {false}).collect(),
            window: util::window::hann(WINDOW_SIZE)
        }
    }
}

impl Plugin for PitchQuantizer {
    // Metadata
    const NAME: &'static str = "Pitch Quantizer";
    const VENDOR: &'static str = "VendorCo GmbH Ltd.";
    const URL: &'static str = "URL";
    const EMAIL: &'static str = "EMAIL";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [nih_plug::prelude::AudioIOLayout] = &[
        AudioIOLayout {
            // two input + output channels (stereo pair).
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            // no sidechain inputs.
            aux_input_ports: &[],
            aux_output_ports: &[],
            // default port names.
            names: PortNames::const_default(),
        }
    ];

    // allows MIDI in, no MIDI out.
    const MIDI_INPUT: nih_plug::prelude::MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    // smaller buffers for automation.
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    // can be run offline.
    const HARD_REALTIME_ONLY: bool = false;

    // does not use SysEx messages.
    type SysExMessage = ();

    // does not have background processing.
    type BackgroundTask = ();

    // called when plugin is created.
    fn task_executor(&mut self) -> nih_plug::prelude::TaskExecutor<Self> {
        // In the default implementation we can simply ignore the value
        Box::new(|_| ())
    }

    // plugin parameters. called after a plugin is instantiated.
    fn params(&self) -> Arc<dyn nih_plug::prelude::Params> {
        self.params.clone()
    }

    // loads the plugin UI editor.
    fn editor(&mut self, _async_executor: nih_plug::prelude::AsyncExecutor<Self>) -> Option<Box<dyn nih_plug::prelude::Editor>> {
        None
    }

    // called just before a PluginState object is loaded, allowing for preset compatibility.
    fn filter_state(_state: &mut nih_plug::prelude::PluginState) {}

    // initialize the plugin.
    // do expensive initialization here.
    // reset() called immediately afterwards.
    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &nih_plug::prelude::BufferConfig,
        context: &mut impl nih_plug::prelude::InitContext<Self>,
    ) -> bool {
        self.bucket_freq = (0..WINDOW_SIZE)
            .map(|k| { bucket_to_freq(k as i32, buffer_config.sample_rate, WINDOW_SIZE) })
            .collect();
        context.set_latency_samples(self.stft.latency_samples());
        true
    }

    // clear internal state.
    // do not alloc here.
    // host always calls before resuming audio processing.
    fn reset(&mut self) {
    }

    // do audio processing.
    fn process(
        &mut self,
        buffer: &mut nih_plug::prelude::Buffer,
        _aux: &mut nih_plug::prelude::AuxiliaryBuffers,
        context: &mut impl nih_plug::prelude::ProcessContext<Self>,
    ) -> nih_plug::prelude::ProcessStatus {
        // midi processing
        let mut next_event = context.next_event();
        while let Some(event) = next_event {
            match event {
                NoteEvent::NoteOn { note, velocity, .. } => {
                    if velocity == 0f32 { self.note_on[note as usize] = false; }
                    else { self.note_on[note as usize] = true; }
                },
                NoteEvent::NoteOff { note, .. } => {
                    self.note_on[note as usize] = false;
                },
                _ => ()
            }
            next_event = context.next_event();
        }

        // get the frequencies of the input midi.
        let mut note_spread_sum: Vec<f32> = (0..WINDOW_SIZE).map(|_| {0f32}).collect();
        let mut seen_letter_note: Vec<bool> = (0..12).map(|_| {false}).collect();
        for (idx, on) in self.note_on.iter().enumerate() {
            if *on {
                if self.params.use_exact.value() {
                    // use exact midi
                    let midi_bucket = closest_bucket_to_freq(midi_note_to_freq(idx as u8), context.transport().sample_rate, WINDOW_SIZE);
                    for (idx2, spread) in bucket_spread(midi_bucket, self.params.note_spread.value(), WINDOW_SIZE as i32, self.params.spread_falloff.value()).iter().enumerate() {
                        note_spread_sum[idx2] += spread;
                    }
                } else {
                    // clone across octaves
                    let note_idx = (idx-21) % 12; // 0th note is A, 11th note is G#
                    if !seen_letter_note[note_idx] {
                        seen_letter_note[note_idx] = true;
                        let fundamental = midi_note_to_freq((note_idx + 21) as u8);
                        let mut harmonic = fundamental;
                        while harmonic <= 20000f32 {
                            let harmonic_bucket = closest_bucket_to_freq(harmonic, context.transport().sample_rate, WINDOW_SIZE);
                            for (idx2, spread) in bucket_spread(harmonic_bucket, self.params.note_spread.value(), WINDOW_SIZE as i32, self.params.spread_falloff.value()).iter().enumerate() {
                                note_spread_sum[idx2] += spread;
                            }
                            harmonic += fundamental;
                        }
                    }
                }
            }
        }

        // audio processing
        self.stft.process_overlap_add(buffer, 1, |_channel_idx, real_fft_buffer: &mut [f32]| {
            // fft from time domain to complex domain.
            self.r2c_plan.process_with_scratch(real_fft_buffer, &mut self.convert_fft_buffer, &mut []).unwrap();
            // window the input
            for (sample, window_sample) in real_fft_buffer.iter_mut().zip(&mut self.window) {
                *sample = *sample * *window_sample;
            }

            // clear output buffer
            for fft_bin in self.process_fft_buffer.iter_mut() {
                fft_bin.re = 0f32;
                fft_bin.im = 0f32;
            }

            for (idx, fft_bin) in self.convert_fft_buffer.iter().enumerate() {
                let re: f32 = fft_bin.re;
                let im: f32 = fft_bin.im;

                // move gain compensated amplitude to bucket in working buffer.
                let bucket_shift = (note_spread_sum[(idx+1).min(note_spread_sum.len() - 1)] - note_spread_sum[idx]).floor() as isize;
                let modbucket = (bucket_shift as f32 * self.params.pull_strength.value()).floor() as isize;
                self.process_fft_buffer[(modbucket + idx as isize).min(WINDOW_SIZE as isize - 1).max(0) as usize].re += re * note_spread_sum[idx as usize] * GAIN_COMPENSATION;
                self.process_fft_buffer[(modbucket + idx as isize).min(WINDOW_SIZE as isize - 1).max(0) as usize].im += im * note_spread_sum[idx as usize] * GAIN_COMPENSATION;
            }

            // clear the DC bucket to get rid of subharmonics.
            self.process_fft_buffer[0].re = 0f32;
            self.process_fft_buffer[0].im = 0f32;

            // inverse fft from complex freq to time.
            self.c2r_plan.process_with_scratch(&mut self.process_fft_buffer, real_fft_buffer, &mut []).unwrap();
        });
        ProcessStatus::Normal
    }

    // dealloc + clean up resources here.
    // not 1-to-1 to activate()
    fn deactivate(&mut self) {}
}

impl Vst3Plugin for PitchQuantizer {
    const VST3_CLASS_ID: [u8; 16] = *b"1000100010001000";
    const VST3_SUBCATEGORIES: &'static [nih_plug::prelude::Vst3SubCategory] = &[
        Vst3SubCategory::Fx
    ];
}

nih_export_vst3!(PitchQuantizer);
