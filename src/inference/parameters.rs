use eframe::egui::{ComboBox, DragValue, Slider, SliderClamping, Ui};
use serde::{Deserialize, Serialize};
use tapestry_weave::universal_weave::indexmap::IndexMap;
use ulid::Ulid;

use super::settings::{InferenceEngineSettings, InferenceModel};
use crate::common::ui::{ListAction, colored_text, config_map};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct InferenceParameters {
    pub recursion_depth: usize,
    pub models: Vec<ModelInferenceParameters>,
}

impl InferenceParameters {
    pub fn reset(&mut self, settings: &InferenceEngineSettings) {
        *self = settings.default_parameters.clone();
    }
    pub fn switch_preset(&mut self, settings: &InferenceEngineSettings, index: usize) {
        if let Some(preset) = settings.parameter_presets.get(index) {
            *self = preset.parameters.clone();
        }
    }
    pub fn ui(&mut self, settings: &InferenceEngineSettings, ui: &mut Ui) {
        if !settings.parameter_presets.is_empty() {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button("\u{E148}")
                        .on_hover_text("Reset parameters")
                        .clicked()
                    {
                        self.reset(settings);
                    }

                    for preset in &settings.parameter_presets {
                        if ui
                            .button(colored_text(&preset.label, preset.color))
                            .clicked()
                        {
                            *self = preset.parameters.clone();
                        }
                    }
                });
            });
            ui.add_space(ui.spacing().icon_spacing);
        }

        self.ui_inner(&settings.models, ui);
    }
    pub(super) fn ui_inner(&mut self, models: &IndexMap<Ulid, InferenceModel>, ui: &mut Ui) {
        ui.add(
            Slider::new(&mut self.recursion_depth, 0..=3)
                .clamping(SliderClamping::Never)
                .text("Recursion")
                .suffix(" layers"),
        ).on_hover_text("The recursion depth used for generating nodes. If this is > 0, nodes will be recursively generated up to the set number of layers.");

        let mut action = None;

        let length = self.models.len();
        for (index, model) in self.models.iter_mut().enumerate() {
            ui.group(|ui| {
                model.ui(ui, models, index, length, &mut action);
            });
        }

        if let Some(action) = action {
            action.apply(&mut self.models);
        }

        ComboBox::from_id_salt(ui.next_auto_id())
            .selected_text("Choose model...")
            .show_ui(ui, |ui| {
                let _ = ui.selectable_label(true, "Choose model...");

                for (id, model) in models {
                    if ui.selectable_label(false, model.widget_text()).clicked() {
                        self.models.push(ModelInferenceParameters {
                            model: *id,
                            requests: 5,
                            parameters: model.endpoint.default_parameters(),
                        });
                    }
                }
            });
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelInferenceParameters {
    pub model: Ulid,
    pub requests: usize,
    pub parameters: Vec<(String, String)>,
}

impl ModelInferenceParameters {
    fn ui(
        &mut self,
        ui: &mut Ui,
        models: &IndexMap<Ulid, InferenceModel>,
        index: usize,
        length: usize,
        action: &mut Option<ListAction>,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.add(
                DragValue::new(&mut self.requests)
                    .suffix("x")
                    .range(1..=usize::MAX),
            )
            .on_hover_text("Request count");
            ComboBox::from_id_salt(ui.next_auto_id())
                .selected_text(
                    models
                        .get(&self.model)
                        .map_or_else(|| "Invalid model".into(), InferenceModel::widget_text),
                )
                .width(ui.spacing().text_edit_width * 0.6)
                .show_ui(ui, |ui| {
                    for (id, model) in models {
                        ui.selectable_value(&mut self.model, *id, model.widget_text());
                    }
                });
            ListAction::buttons(ui, "model", index, length, action);
        });

        ui.label("Request parameters:");
        config_map(ui, &mut self.parameters, 0.55, 0.45);
    }
}
