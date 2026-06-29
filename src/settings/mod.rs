use eframe::egui::{Context, Frame, ScrollArea, Ui, WidgetText};
#[cfg(feature = "donation-link")]
use eframe::egui::{Layout, OpenUrl, Sides};
use serde::{Deserialize, Serialize};

use crate::{
    AppShared,
    common::{task::BACKGROUND_REFRESH_INTERVAL, view::View},
    settings::{document::DocumentSettings, interface::InterfaceSettings},
};

mod document;
mod interface;

#[derive(Default, Debug)]
pub struct SettingsView {}

impl View<AppShared> for SettingsView {
    fn title(&self, _shared: &AppShared) -> WidgetText {
        WidgetText::Text("\u{E154} Settings".to_string())
    }
    fn logic(&mut self, _shared: &mut AppShared, _force_close: impl FnOnce(), _ctx: &Context) {}
    fn ui(&mut self, shared: &mut AppShared, ui: &mut Ui) {
        ScrollArea::both()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(ui.style().spacing.menu_margin)
                    .show(ui, |ui| {
                        shared.settings.ui(self, ui);
                    })
            });

        ui.request_repaint_after(BACKGROUND_REFRESH_INTERVAL);
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Settings {
    pub interface: InterfaceSettings,
    pub documents: DocumentSettings,
}

impl Settings {
    pub fn deserialize(data: &str) -> ron::error::SpannedResult<Self> {
        ron::from_str(data)
    }
    pub fn serialize(&self) -> ron::error::Result<String> {
        ron::to_string(self)
    }
}

impl Editable<SettingsView> for Settings {
    fn ui(&mut self, shared: &mut SettingsView, ui: &mut Ui) {
        #[cfg(feature = "donation-link")]
        Sides::new().show(
            ui,
            |ui| {
                ui.with_layout(Layout::default(), |ui| {
                    ui.heading("Interface");
                    self.interface.ui(shared, ui);
                });
            },
            |ui| {
                let response = ui.button("\u{E2D7} Donate").on_hover_text("Tapestry Loom is free to use, but it isn't free to make. Please consider donating to help make further development possible.");

                if response.clicked() {
                    ui.ctx().open_url(OpenUrl {
                        url: env!("DONATION_LINK").to_string(),
                        new_tab: response.clicked_with_open_in_background(),
                    });
                }
            },
        );

        #[cfg(not(feature = "donation-link"))]
        {
            ui.heading("Interface");
            self.interface.ui(shared, ui);
        }

        ui.separator();
        ui.heading("Document");
        self.documents.ui(shared, ui);
    }
}

trait Editable<T> {
    fn ui(&mut self, shared: &mut T, ui: &mut Ui);
}
