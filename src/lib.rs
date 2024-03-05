use nih_plug::{nih_export_vst3, prelude::{Plugin, AudioIOLayout, MidiConfig, PortNames, Vst3Plugin, Vst3SubCategory, ProcessStatus}};
use realfft::RealFftPlanner;
use std::sync::Arc;
use std::num::NonZeroU32;

const WINDOW_SIZE : usize = 2048;
const GAIN_COMPENSATION: f32 = 1.0 / WINDOW_SIZE as f32;

struct PitchQuantizer {
}

impl Default for PitchQuantizer {
    fn default() -> Self {
        Self {
        }
    }
}

impl Plugin for PitchQuantizer {
    // Metadata
    const NAME: &'static str = "Pitch Quantizer";
    const VENDOR: &'static str = "";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
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
        todo!()
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
        // Initialize planner for FFT algorithm.
        let mut planner = RealFftPlanner::<f64>::new();
        let r2c = planner.plan_fft_forward(WINDOW_SIZE);
        let c2r = planner.plan_fft_inverse(WINDOW_SIZE);
        let mut real_buf = r2c.make_input_vec();
        let mut complex_buf = r2c.make_output_vec();
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
        ProcessStatus::Normal
    }

    // dealloc + clean up resources here.
    // not 1-to-1 to activate()
    fn deactivate(&mut self) {}
}

fn main() {
    println!("Hello, world!");
}

impl Vst3Plugin for PitchQuantizer {
    const VST3_CLASS_ID: [u8; 16] = *b"1000100010001000";
    const VST3_SUBCATEGORIES: &'static [nih_plug::prelude::Vst3SubCategory] = &[
        Vst3SubCategory::Fx
    ];
}

nih_export_vst3!(PitchQuantizer);
