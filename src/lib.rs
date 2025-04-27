use nih_plug::{
    formatters::{s2v_f32_gain_to_db, v2s_f32_gain_to_db},
    prelude::*,
    util::db_to_gain,
};
use nih_plug_iced::{
    Alignment, Column, Command, Element, IcedEditor, IcedState, Length, Text, WindowQueue,
    create_iced_editor, executor,
    widgets::{ParamMessage, ParamSlider, param_slider},
};
use std::sync::Arc;

#[derive(Default)]
struct Dcc {
    params: Arc<DccParams>,
}

#[derive(Params)]
struct DccParams {
    #[persist = "editor-state"]
    editor_state: Arc<IcedState>,
    #[id = "pregain"]
    pub pregain: FloatParam,
    #[id = "offset"]
    pub offset: FloatParam,
    #[id = "skew"]
    pub skew: FloatParam,
    #[id = "postgain"]
    pub postgain: FloatParam,
}

impl Default for DccParams {
    fn default() -> Self {
        Self {
            editor_state: IcedState::from_size(200, 250),
            pregain: FloatParam::new(
                "Pre-Gain",
                db_to_gain(0.0),
                FloatRange::Skewed {
                    min: db_to_gain(-12.0),
                    max: db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-12.0, 12.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit("dB")
            .with_value_to_string(v2s_f32_gain_to_db(2))
            .with_string_to_value(s2v_f32_gain_to_db()),
            offset: FloatParam::new(
                "Offset",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_step_size(0.01),
            skew: FloatParam::new(
                "Skew",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_step_size(0.01),
            postgain: FloatParam::new(
                "Post-Gain",
                db_to_gain(0.0),
                FloatRange::Skewed {
                    min: db_to_gain(-12.0),
                    max: db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-12.0, 12.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit("dB")
            .with_value_to_string(v2s_f32_gain_to_db(2))
            .with_string_to_value(s2v_f32_gain_to_db()),
        }
    }
}

impl Plugin for Dcc {
    const NAME: &'static str = "DCC";
    const VENDOR: &'static str = "edwloef";
    const URL: &'static str = "https://github.com/edwloef/dcc";
    const EMAIL: &'static str = "edwin.frank.loeffler@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        names: PortNames::const_default(),
    }];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        create_iced_editor::<DccEditor>(self.params.editor_state.clone(), self.params.clone())
    }

    fn process(
        &mut self,
        buffer: &mut Buffer<'_>,
        _aux: &mut AuxiliaryBuffers<'_>,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            let pregain = self.params.pregain.smoothed.next();
            let offset = self.params.offset.smoothed.next();
            let mut skew = self.params.skew.smoothed.next();
            let postgain = self.params.postgain.smoothed.next();

            for sample in channel_samples {
                *sample = (((*sample).mul_add(pregain, offset - skew).clamp(-1.0, 1.0)
                    - (offset - skew).clamp(-1.0, 1.0))
                    * postgain)
                    .clamp(-1.0, 1.0);

                skew = -skew;
            }
        }

        ProcessStatus::Normal
    }
}

struct DccEditor {
    params: Arc<DccParams>,
    context: Arc<dyn GuiContext>,

    pregain: param_slider::State,
    offset: param_slider::State,
    skew: param_slider::State,
    postgain: param_slider::State,
}

impl IcedEditor for DccEditor {
    type Executor = executor::Default;
    type Message = ParamMessage;
    type InitializationFlags = Arc<DccParams>;

    fn new(
        params: Self::InitializationFlags,
        context: Arc<dyn GuiContext>,
    ) -> (Self, Command<Self::Message>) {
        let editor = Self {
            params,
            context,

            pregain: param_slider::State::default(),
            offset: param_slider::State::default(),
            skew: param_slider::State::default(),
            postgain: param_slider::State::default(),
        };

        (editor, Command::none())
    }

    fn context(&self) -> &dyn GuiContext {
        &*self.context
    }

    fn update(
        &mut self,
        _window: &mut WindowQueue,
        message: Self::Message,
    ) -> Command<Self::Message> {
        self.handle_param_message(message);

        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        Column::new()
            .push(
                Column::new()
                    .push(
                        ParamSlider::new(&mut self.pregain, &self.params.pregain)
                            .width(Length::Fill),
                    )
                    .push(Text::new("Pre-Gain"))
                    .align_items(Alignment::Center),
            )
            .push(
                Column::new()
                    .push(
                        ParamSlider::new(&mut self.offset, &self.params.offset).width(Length::Fill),
                    )
                    .push(Text::new("Offset"))
                    .align_items(Alignment::Center),
            )
            .push(
                Column::new()
                    .push(ParamSlider::new(&mut self.skew, &self.params.skew).width(Length::Fill))
                    .push(Text::new("Skew"))
                    .align_items(Alignment::Center),
            )
            .push(
                Column::new()
                    .push(
                        ParamSlider::new(&mut self.postgain, &self.params.postgain)
                            .width(Length::Fill),
                    )
                    .push(Text::new("Post-Gain"))
                    .align_items(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(10)
            .padding(10)
            .into()
    }
}

impl ClapPlugin for Dcc {
    const CLAP_ID: &'static str = "com.edwloef.dcc";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A hard clipping distortion plugin with DC offset manipulation.");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect];
}

impl Vst3Plugin for Dcc {
    const VST3_CLASS_ID: [u8; 16] = *b" com.edwloef.dcc";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx];
}

nih_export_clap!(Dcc);
nih_export_vst3!(Dcc);
