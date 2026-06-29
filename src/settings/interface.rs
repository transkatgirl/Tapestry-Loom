use eframe::egui::{ComboBox, Context, Slider, SliderClamping, ThemePreference, Ui};
use serde::{Deserialize, Serialize};

use crate::settings::{Editable, SettingsView};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterfaceSettings {
    #[serde(default)]
    pub theme: ThemePreference,

    #[serde(default = "default_scale")]
    pub scale: f32,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            scale: 1.25,
        }
    }
}

fn default_scale() -> f32 {
    1.25
}

impl InterfaceSettings {
    pub(super) fn apply(&mut self, ctx: &Context) {
        ctx.options_mut(|options| {
            options.theme_preference = self.theme;
            options.zoom_factor = self.scale;
        });
    }
}

impl Editable<SettingsView> for InterfaceSettings {
    fn ui(&mut self, shared: &mut SettingsView, ui: &mut Ui) {
        ComboBox::from_label("Theme")
            .selected_text(match self.theme {
                ThemePreference::Dark => "Dark",
                ThemePreference::Light => "Light",
                ThemePreference::System => "Auto",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.theme, ThemePreference::Dark, "Dark");
                ui.selectable_value(&mut self.theme, ThemePreference::Light, "Light");
                ui.selectable_value(&mut self.theme, ThemePreference::System, "Auto");
            })
            .response
            .on_hover_text("Changes the application-wide UI theme.");

        ui.options_mut(|options| {
            options.theme_preference = self.theme;
        });

        let zoom_slider = ui
            .add(
                Slider::new(&mut self.scale, 0.5..=4.0)
                    .logarithmic(true)
                    .clamping(SliderClamping::Never)
                    .text("Scale")
                    .suffix("x"),
            )
            .on_hover_text("Changes the application-wide UI scale.");
        if !(zoom_slider.has_focus() || zoom_slider.contains_pointer()) {
            ui.set_zoom_factor(self.scale);
        }
    }
}
