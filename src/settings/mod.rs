use std::{fmt::Display, path::PathBuf, time::Duration};

use eframe::egui::{
    ComboBox, Context, Frame, ScrollArea, Slider, SliderClamping, TextStyle, Ui, Visuals,
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
    pub show_token_probabilities: bool,
    pub minimum_token_opacity: f32,
    pub max_tree_depth: usize,
    pub auto_scroll: bool,

    #[serde(skip)]
    fonts_changed: bool,
}

impl Default for UISettings {
    fn default() -> Self {
        Self {
            ui_scale: 1.25,
            ui_theme: UITheme::Dark,
            ui_fonts: UIFonts::Default,

            displayed_ui_scale: 1.25,
            show_model_colors: true,
            show_token_probabilities: true,
            minimum_token_opacity: 65.0,
            max_tree_depth: 10,
            auto_scroll: true,

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
            });
        let ui_slider = ui.add(
            Slider::new(&mut self.displayed_ui_scale, 0.5..=4.0)
                .logarithmic(true)
                .clamping(SliderClamping::Never)
                .text("Scale")
                .suffix("x"),
        );
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
            });

        if self.fonts_changed {
            ui.colored_label(
                ui.style().visuals.warn_fg_color,
                "Font changes require the app to be restarted to take effect.",
            );
        }

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.75);

        ui.checkbox(&mut self.show_model_colors, "Show model colors");
        ui.checkbox(
            &mut self.show_token_probabilities,
            "Show token probabilities",
        );
        if self.show_token_probabilities {
            ui.add(
                Slider::new(&mut self.minimum_token_opacity, 20.0..=80.0)
                    .suffix("%")
                    .text("Minimum token opacity"),
            );
        }
        ui.add(
            Slider::new(&mut self.max_tree_depth, 3..=32)
                .clamping(SliderClamping::Never)
                .text("Maximum tree list depth"),
        );
        ui.checkbox(
            &mut self.auto_scroll,
            "Automatically adjust scroll position",
        );

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
        let location_label = ui.label("Root location:");
        let mut document_location = self.location.to_string_lossy().to_string();

        if ui
            .text_edit_singleline(&mut document_location)
            .labelled_by(location_label.id)
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

                        #[cfg(debug_assertions)]
                        {
                            ui.separator();
                            ui.collapsing("Debug", |ui| {
                                ui.ctx().clone().settings_ui(ui);
                                //ui.ctx().clone().inspection_ui(ui);
                                ui.ctx().clone().texture_ui(ui);
                                ui.ctx().clone().memory_ui(ui);
                            });
                        }
                    });
            });
    }
    pub fn handle_shortcuts(&mut self, shortcuts: FlagSet<Shortcuts>) {
        if shortcuts.contains(Shortcuts::ToggleColors) {
            self.interface.show_model_colors = !self.interface.show_model_colors;
        }
    }
}
