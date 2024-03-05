use helpers::{bucket_to_freq, closest_bucket_to_freq, amplitude_from_complex};
// use nih_plug::{nih_export_vst3, prelude::{Plugin, AudioIOLayout, MidiConfig, PortNames, Vst3Plugin, Vst3SubCategory, ProcessStatus}, util};
use nih_plug::prelude::*;
use realfft::{RealFftPlanner, RealToComplex, ComplexToReal, num_complex::Complex32};
use std::sync::Arc;
use std::num::NonZeroU32;

mod helpers;

const WINDOW_SIZE : usize = 1024;
const GAIN_COMPENSATION: f32 = 1.0 / WINDOW_SIZE as f32;

struct PitchQuantizer {
    params: Arc<PitchQuantizerParams>,

    stft: util::StftHelper,
    r2c_plan: Arc<dyn RealToComplex<f32>>,
    c2r_plan: Arc<dyn ComplexToReal<f32>>,
    convert_fft_buffer: Vec<Complex32>,
    process_fft_buffer: Vec<Complex32>,
    
    bucket_freq: Vec<f32>
}

#[derive(Params)]
struct PitchQuantizerParams{}

#[allow(clippy::derivable_impls)]
impl Default for PitchQuantizerParams {
    fn default() -> Self {
        Self {}
    }
}

impl Default for PitchQuantizer {
    fn default() -> Self {
        let mut planner = RealFftPlanner::new();
        let r2c_plan = planner.plan_fft_forward(WINDOW_SIZE);
        let c2r_plan = planner.plan_fft_inverse(WINDOW_SIZE);
        let mut real_fft_buffer = r2c_plan.make_input_vec();
        let mut convert_fft_buffer = r2c_plan.make_output_vec();
        let mut process_fft_buffer = r2c_plan.make_output_vec();

        Self {
            params: Arc::new(PitchQuantizerParams::default()),
            stft: util::StftHelper::new(2, WINDOW_SIZE, 0),
            r2c_plan,
            c2r_plan,
            convert_fft_buffer,
            process_fft_buffer,
            bucket_freq: (0..WINDOW_SIZE).map(|_| {0f32}).collect()
        }
    }
}

impl Plugin for PitchQuantizer {
    // Metadata
    const NAME: &'static str = "Pitch Quantizer";
    const VENDOR: &'static str = "VENDCO";
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
    const MIDI_INPUT: nih_plug::prelude::MidiConfig = MidiConfig::None;
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
    fn editor(&mut self, async_executor: nih_plug::prelude::AsyncExecutor<Self>) -> Option<Box<dyn nih_plug::prelude::Editor>> {
        None
    }

    // called just before a PluginState object is loaded, allowing for preset compatibility.
    fn filter_state(state: &mut nih_plug::prelude::PluginState) {}

    // initialize the plugin.
    // do expensive initialization here.
    // reset() called immediately afterwards.
    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &nih_plug::prelude::BufferConfig,
        context: &mut impl nih_plug::prelude::InitContext<Self>,
    ) -> bool {
        self.bucket_freq = (0..WINDOW_SIZE)
            .map(|k| { bucket_to_freq(k as i32, buffer_config.sample_rate, WINDOW_SIZE) })
            .collect();
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
        aux: &mut nih_plug::prelude::AuxiliaryBuffers,
        context: &mut impl nih_plug::prelude::ProcessContext<Self>,
    ) -> nih_plug::prelude::ProcessStatus {
        self.stft.process_overlap_add(buffer, 1, |_channel_idx, real_fft_buffer| {
            // fft from time domain to complex domain.
            self.r2c_plan.process_with_scratch(real_fft_buffer, &mut self.convert_fft_buffer, &mut []).unwrap();

            for fft_bin in self.process_fft_buffer.iter_mut() {
                fft_bin.re = 0f32;
                fft_bin.im = 0f32;
            }

            for (idx, fft_bin) in self.convert_fft_buffer.iter_mut().enumerate() {
                let re: f32 = fft_bin.re;
                let im: f32 = fft_bin.im;

                let frequency = bucket_to_freq(idx as i32, context.transport().sample_rate, WINDOW_SIZE);
                let bucket: i32 = closest_bucket_to_freq(440.0, context.transport().sample_rate, WINDOW_SIZE);
                let amp = amplitude_from_complex(re, im);
                self.process_fft_buffer[bucket as usize].re += amp * GAIN_COMPENSATION;
            }

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
