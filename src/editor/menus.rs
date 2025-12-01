use eframe::egui::{Spinner, Ui};
use egui_notify::Toasts;
use flagset::FlagSet;
use tapestry_weave::v0::TapestryWeave;

use crate::{
    editor::shared::SharedState,
    settings::{Settings, shortcuts::Shortcuts},
};

#[derive(Default, Debug)]
pub struct MenuView {}

impl MenuView {
    pub fn reset(&mut self) {}
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
        shortcuts: FlagSet<Shortcuts>,
    ) {
        ui.heading("Unimplemented");
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_rtl_panel(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
        shortcuts: FlagSet<Shortcuts>,
        file_size: usize,
    ) {
        let request_count = state.get_request_count();

        if request_count > 0 {
            ui.add(Spinner::new());
            if request_count > 1 {
                ui.label(format!("{request_count} requests"));
            } else {
                ui.label("1 request");
            }
        } else {
            let node_count = weave.len();
            let file_size_label = if file_size >= 1_000_000_000 {
                format!(", {:.1} GB", file_size as f32 / 1_000_000_000.0)
            } else if file_size >= 1_000_000 {
                format!(", {:.1} MB", file_size as f32 / 1_000_000.0)
            } else if file_size >= 1_000 {
                format!(", {:.1} kB", file_size as f32 / 1_000.0)
            } else if file_size > 0 {
                format!(", {} bytes", file_size)
            } else {
                String::new()
            };
            let node_count_label = if node_count >= 1_000_000 {
                format!("{:.1}M nodes", node_count as f32 / 1_000_000.0)
            } else if node_count >= 1_000 {
                format!("{:.1}k nodes", node_count as f32 / 1_000.0)
            } else if node_count == 1 {
                "1 node".to_string()
            } else {
                format!("{} nodes", node_count)
            };
            ui.label(format!("{node_count_label}{file_size_label}"));
        }

        if shortcuts.contains(Shortcuts::ResetParameters) {
            // TODO
        }
    }
}
