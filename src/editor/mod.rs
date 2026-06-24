use std::path::PathBuf;

use eframe::egui::{Context, Ui, WidgetText};

use crate::{AppShared, shared::view::View};

pub struct Editor {
    path: Option<PathBuf>,
}

impl Editor {
    pub fn new(path: Option<PathBuf>, shared: &AppShared) -> Editor {
        Editor { path }
    }
}

impl View<AppShared> for Editor {
    fn title(&self, shared: &AppShared) -> WidgetText {
        WidgetText::Text(match &self.path {
            Some(path) => {
                if let Some(filename) = path.file_stem() {
                    filename.to_string_lossy().to_string()
                } else {
                    "Editor".to_string()
                }
            }
            None => "New Weave".to_string(),
        })
    }
    fn closable(&self, shared: &AppShared) -> bool {
        true
    }
    /*fn check_close(&mut self, shared: &mut AppShared) -> bool {
        todo!()
    }*/

    fn logic(&mut self, shared: &mut AppShared, ctx: &Context) {
        //todo!()
    }
    fn modals(&mut self, shared: &mut AppShared, ctx: &Context) -> bool {
        //todo!()
        false
    }
    fn ui(&mut self, shared: &mut AppShared, ui: &mut Ui) {
        //todo!()
    }

    fn close(&mut self, shared: &mut AppShared) -> bool {
        //todo!()
        true
    }
}
