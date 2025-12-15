use std::{fmt::Display, path::PathBuf, time::Duration};

use eframe::egui::{
    Color32, ComboBox, Context, Frame, ScrollArea, Slider, SliderClamping, Style, TextStyle, Ui,
    Visuals,
    color_picker::{Alpha, color_edit_button_srgba},
};
use flagset::FlagSet;
use serde::{Deserialize, Serialize};

use crate::settings::{
    inference::InferenceSettings,
    shortcuts::{KeyboardShortcuts, Shortcuts},
};

pub mod inference;
pub mod shortcuts;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Settings {
    pub interface: UISettings,
    pub shortcuts: KeyboardShortcuts,
    pub documents: DocumentSettings,
    pub inference: InferenceSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct UISettings {
    pub ui_scale: f32,
    pub ui_theme: UITheme,
    pub ui_fonts: UIFonts,

    pub displayed_ui_scale: f32,
    pub show_model_colors: bool,

    #[serde(default)]
    pub override_model_colors: bool,

    pub model_color_override: Option<Color32>,
    pub show_token_probabilities: bool,

    #[serde(default = "default_show_token_confidence")]
    pub show_token_confidence: bool,

    pub minimum_token_opacity: f32,

    #[serde(default = "default_list_separator_opacity")]
    pub list_separator_opacity: f32,

    pub max_tree_depth: usize,
    pub auto_scroll: bool,
    pub optimize_tree: bool,

    #[serde(skip)]
    fonts_changed: bool,
}

fn default_list_separator_opacity() -> f32 {
    30.0
}

fn default_show_token_confidence() -> bool {
    true
}

impl Default for UISettings {
    fn default() -> Self {
        Self {
            ui_scale: 1.25,
            ui_theme: UITheme::Dark,
            ui_fonts: UIFonts::Default,

            displayed_ui_scale: 1.25,
            show_model_colors: true,
            override_model_colors: false,
            model_color_override: None,
            show_token_probabilities: true,
            show_token_confidence: true,
            minimum_token_opacity: 65.0,
            list_separator_opacity: 30.0,
            max_tree_depth: 10,
            auto_scroll: true,
            optimize_tree: false,

            fonts_changed: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UITheme {
    Dark,
    Light,
    SolarizedDark,
    SolarizedLight,
}

impl Display for UITheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dark => f.write_str("egui Dark"),
            Self::Light => f.write_str("egui Light"),
            Self::SolarizedDark => f.write_str("Solarized Dark"),
            Self::SolarizedLight => f.write_str("Solarized Light"),
        }
    }
}

impl UITheme {
    fn get_visuals(&self) -> Visuals {
        match &self {
            Self::Dark => Visuals::dark(),
            Self::Light => Visuals::light(),
            Self::SolarizedDark => egui_solarized::Theme::solarized_dark().into(),
            Self::SolarizedLight => egui_solarized::Theme::solarized_light().into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UIFonts {
    Default,
    System,
    UnifontEX,
}

impl Display for UIFonts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => f.write_str("egui Default"),
            Self::System => f.write_str("System"),
            Self::UnifontEX => f.write_str("UnifontEX"),
        }
    }
}

impl UISettings {
    pub fn apply(&self, ctx: &Context) {
        ctx.set_zoom_factor(self.ui_scale);
        ctx.set_visuals(self.ui_theme.get_visuals());
    }
    fn render(&mut self, ui: &mut Ui) {
        ComboBox::from_label("Theme")
            .selected_text(self.ui_theme.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.ui_theme, UITheme::Dark, UITheme::Dark.to_string());
                ui.selectable_value(
                    &mut self.ui_theme,
                    UITheme::Light,
                    UITheme::Light.to_string(),
                );
                ui.selectable_value(
                    &mut self.ui_theme,
                    UITheme::SolarizedDark,
                    UITheme::SolarizedDark.to_string(),
                );
                ui.selectable_value(
                    &mut self.ui_theme,
                    UITheme::SolarizedLight,
                    UITheme::SolarizedLight.to_string(),
                );
            })
            .response
            .on_hover_text("Changes the application-wide UI theme.");
        let ui_slider = ui
            .add(
                Slider::new(&mut self.displayed_ui_scale, 0.5..=4.0)
                    .logarithmic(true)
                    .clamping(SliderClamping::Never)
                    .text("Scale")
                    .suffix("x"),
            )
            .on_hover_text("Changes the application-wide UI scale.");
        if !(ui_slider.has_focus() || ui_slider.contains_pointer()) {
            self.ui_scale = self.displayed_ui_scale;
        }
        ComboBox::from_label("Primary Font")
            .selected_text(self.ui_fonts.to_string())
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(
                        &mut self.ui_fonts,
                        UIFonts::Default,
                        UIFonts::Default.to_string(),
                    )
                    .clicked()
                {
                    self.fonts_changed = true;
                };
                if ui
                    .selectable_value(
                        &mut self.ui_fonts,
                        UIFonts::System,
                        UIFonts::System.to_string(),
                    )
                    .clicked()
                {
                    self.fonts_changed = true;
                };
                if ui
                    .selectable_value(
                        &mut self.ui_fonts,
                        UIFonts::UnifontEX,
                        UIFonts::UnifontEX.to_string(),
                    )
                    .clicked()
                {
                    self.fonts_changed = true;
                };
            }).response.on_hover_text("Tapestry Loom has multiple built-in fonts in order to display the widest range of unicode characters possible. However, some of these fonts may be considered less visually appealing.\n\nThis setting allows you to change the primary fonts used when rendering the UI and weave text. If a character is not present in the primary font, the built-in fonts are is used as a fallback.");

        if self.fonts_changed {
            ui.colored_label(
                ui.style().visuals.warn_fg_color,
                "Font changes require the app to be restarted to take effect.",
            );
        }

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.75);

        ui.checkbox(
            &mut self.show_model_colors,
            "Color code editor using model label colors",
        );

        if self.show_model_colors {
            ui.checkbox(
                &mut self.override_model_colors,
                "Override model color coding",
            );

            if self.override_model_colors {
                if let Some(model_color_override) = &mut self.model_color_override {
                    ui.horizontal_wrapped(|ui| {
                        let label = ui.label("Model color:").id;
                        color_edit_button_srgba(ui, model_color_override, Alpha::Opaque)
                            .labelled_by(label);
                        if ui.button("\u{E148}").on_hover_text("Reset color").clicked() {
                            *model_color_override = ui.style().visuals.hyperlink_color;
                        }
                    });
                } else {
                    self.model_color_override = Some(ui.style().visuals.hyperlink_color);
                }
            }
        }

        ui.checkbox(
            &mut self.show_token_probabilities,
            "Shade tokens by probability",
        );
        if self.show_token_probabilities {
            ui.checkbox(
                &mut self.show_token_confidence,
                "Use token confidence when determining shading",
            );
        }
        if self.show_token_probabilities {
            ui.add(
                Slider::new(&mut self.minimum_token_opacity, 20.0..=80.0)
                    .suffix("%")
                    .text("Minimum token opacity"),
            );
        }
        ui.add(
            Slider::new(&mut self.list_separator_opacity, 0.0..=100.0)
                .suffix("%")
                .text("List item separator opacity"),
        );
        ui.add(
            Slider::new(&mut self.max_tree_depth, 3..=32)
                .clamping(SliderClamping::Never)
                .text("Maximum displayed tree list depth"),
        );
        ui.checkbox(
            &mut self.auto_scroll,
            "Automatically adjust scroll position (may reduce performance)",
        );
        if !self.auto_scroll {
            ui.checkbox(
                &mut self.optimize_tree,
                "Perform additional tree rendering optimizations (experimental)",
            );
        }

        // TODO: Add editor layout presets
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentSettings {
    pub location: PathBuf,
    pub save_interval: Duration,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            location: dirs_next::document_dir()
                .unwrap_or_default()
                .join("Tapestry Loom"),
            save_interval: Duration::from_secs(30),
        }
    }
}

impl DocumentSettings {
    fn render(&mut self, ui: &mut Ui) {
        let location_hover_text = "Changes the path used by the built in file manager.\n\nFile paths in the UI are abbreviated to be relative to the root location whenever possible.";

        let location_label = ui
            .label("Root location:")
            .on_hover_text(location_hover_text);
        let mut document_location = self.location.to_string_lossy().to_string();

        if ui
            .text_edit_singleline(&mut document_location)
            .labelled_by(location_label.id)
            .on_hover_text(location_hover_text)
            .changed()
        {
            self.location = PathBuf::from(document_location);
        }

        let mut save_interval = self.save_interval.as_secs_f32();
        if ui
            .add(
                Slider::new(&mut save_interval, 1.0..=600.0)
                    .clamping(SliderClamping::Never)
                    .logarithmic(true)
                    .suffix("s")
                    .text("Autosave interval"),
            )
            .on_hover_text("Weaves are automatically saved at fixed intervals based on this setting.\n\nIn addition to the autosave interval, weaves will be automatically saved on application close.")
            .changed()
        {
            self.save_interval = Duration::from_secs_f32(save_interval);
        }
    }
}

impl Settings {
    pub fn render(&mut self, ui: &mut Ui) {
        ScrollArea::both()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(ui.style().spacing.menu_margin)
                    .show(ui, |ui| {
                        ui.heading("Interface");
                        self.interface.render(ui);
                        ui.separator();
                        ui.heading("Shortcuts");
                        self.shortcuts.render(ui);
                        ui.separator();
                        ui.heading("Document");
                        self.documents.render(ui);
                        ui.separator();
                        ui.heading("Inference");
                        self.inference.render(ui);
                        ui.separator();
                        ui.hyperlink_to(
                            format!(
                                "Tapestry Loom v{} by transkatgirl",
                                env!("CARGO_PKG_VERSION")
                            ),
                            env!("CARGO_PKG_HOMEPAGE"),
                        );

                        /*#[cfg(debug_assertions)]
                        {
                            ui.separator();
                            ui.collapsing("Debug", |ui| {
                                ui.ctx().clone().settings_ui(ui);
                                //ui.ctx().clone().inspection_ui(ui);
                                ui.ctx().clone().texture_ui(ui);
                                ui.ctx().clone().memory_ui(ui);
                            });
                        }*/
                    });
            });
    }
    pub fn handle_shortcuts(&mut self, style: &Style, shortcuts: FlagSet<Shortcuts>) {
        if shortcuts.contains(Shortcuts::ToggleColors) {
            self.interface.show_model_colors = !self.interface.show_model_colors;
        }

        if shortcuts.contains(Shortcuts::ToggleColorOverride) {
            self.interface.override_model_colors = !self.interface.override_model_colors;
            if self.interface.override_model_colors && self.interface.model_color_override.is_none()
            {
                self.interface.model_color_override = Some(style.visuals.hyperlink_color);
            }
        }

        if shortcuts.contains(Shortcuts::ToggleProbabilities) {
            self.interface.show_token_probabilities = !self.interface.show_token_probabilities;
        }

        if shortcuts.contains(Shortcuts::ToggleAutoScroll) {
            self.interface.auto_scroll = !self.interface.auto_scroll;
        }
    }
}
