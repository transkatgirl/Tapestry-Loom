use std::collections::BTreeMap;

use eframe::egui::{
    ComboBox, Context, FontFamily, Slider, SliderClamping, TextStyle, ThemePreference, Ui,
};
use serde::{Deserialize, Serialize};

use crate::{
    common::view::Edit, editor::settings::interface::InterfaceSettings as EditorInterfaceSettings,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterfaceSettings {
    #[serde(default)]
    pub theme: ThemePreference,

    #[serde(default = "default_scale")]
    pub scale: f32,

    #[serde(default)]
    pub font: FontPreference,

    #[serde(flatten)]
    pub editor: EditorInterfaceSettings,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            scale: 1.25,
            font: FontPreference::Default,
            editor: EditorInterfaceSettings::default(),
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
        if self.font != FontPreference::Default {
            self.font.apply(ctx);
        }
    }
}

impl Edit for InterfaceSettings {
    fn ui(&mut self, ui: &mut Ui) {
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

        let last_font = self.font;

        ComboBox::from_label("Fontset")
            .selected_text(match self.font {
                FontPreference::Default => "Default",
                FontPreference::Monospace => "Monospace",
                FontPreference::UnifontEX => "UnifontEX",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.font, FontPreference::Default, "Default");
                ui.selectable_value(&mut self.font, FontPreference::Monospace, "Monospace");
                ui.selectable_value(&mut self.font, FontPreference::UnifontEX, "UnifontEX");
            })
            .response
            .on_hover_text("This application uses multiple built-in fonts in order to display the widest range of unicode characters possible. However, some of these fonts may be considered less visually appealing.\n\nThis setting allows you to change which built-in fontset is used for rendering.");

        if self.font != last_font {
            self.font.apply(ui);
        }

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.75);
        self.editor.ui(ui);
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FontPreference {
    #[default]
    Default,
    Monospace,
    UnifontEX,
}

impl FontPreference {
    fn apply(&self, ctx: &Context) {
        let mut fonts = ctx.fonts(|fonts| fonts.definitions().clone());

        fonts.families = match self {
            FontPreference::Default => BTreeMap::from([
                (
                    FontFamily::Proportional,
                    vec![
                        "ubuntu-light".to_owned(),
                        "lucide".to_owned(),
                        "noto-emoji".to_owned(),
                        "unifontex".to_owned(),
                    ],
                ),
                (
                    FontFamily::Monospace,
                    vec![
                        "hack".to_owned(),
                        "ubuntu-light".to_owned(),
                        "noto-emoji".to_owned(),
                        "unifontex".to_owned(),
                    ],
                ),
            ]),
            FontPreference::Monospace => BTreeMap::from([
                (
                    FontFamily::Proportional,
                    vec![
                        "hack".to_owned(),
                        "ubuntu-light".to_owned(),
                        "lucide".to_owned(),
                        "noto-emoji".to_owned(),
                        "unifontex".to_owned(),
                    ],
                ),
                (
                    FontFamily::Monospace,
                    vec![
                        "hack".to_owned(),
                        "ubuntu-light".to_owned(),
                        "noto-emoji".to_owned(),
                        "unifontex".to_owned(),
                    ],
                ),
            ]),
            FontPreference::UnifontEX => BTreeMap::from([
                (
                    FontFamily::Proportional,
                    vec!["unifontex".to_owned(), "lucide".to_owned()],
                ),
                (FontFamily::Monospace, vec!["unifontex".to_owned()]),
            ]),
        };

        ctx.set_fonts(fonts);
    }
}
