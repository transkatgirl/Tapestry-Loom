use eframe::egui::{Context, Ui, WidgetText};

use crate::{AppShared, shared::view::View};

pub struct Editor {}

impl View<AppShared> for Editor {
    fn title(&self, shared: &AppShared) -> WidgetText {
        todo!()
    }
    fn closable(&self, shared: &AppShared) -> bool {
        true
    }
    fn check_close(&mut self, shared: &mut AppShared) -> bool {
        todo!()
    }

    fn logic(&mut self, shared: &mut AppShared, ctx: &Context) {
        todo!()
    }
    fn modals(&mut self, shared: &mut AppShared, ctx: &Context) -> bool {
        todo!()
    }
    fn ui(&mut self, shared: &mut AppShared, ui: &mut Ui) {
        todo!()
    }

    fn close(&mut self, shared: &mut AppShared) -> bool {
        todo!()
    }
}
