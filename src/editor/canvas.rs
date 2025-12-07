use eframe::egui::{Rect, Scene, Ui};
use egui_notify::Toasts;
use flagset::FlagSet;

use crate::{
    editor::shared::{SharedState, weave::WeaveWrapper},
    settings::{Settings, shortcuts::Shortcuts},
};

#[derive(Debug)]
pub struct CanvasView {
    rect: Rect,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self { rect: Rect::ZERO }
    }
}

impl CanvasView {
    pub fn reset(&mut self) {
        self.rect = Rect::ZERO;
    }
    pub fn update(
        &mut self,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
        shortcuts: FlagSet<Shortcuts>,
    ) {
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
        shortcuts: FlagSet<Shortcuts>,
    ) {
        Scene::new().show(ui, &mut self.rect, |ui| {
            ui.heading("Unimplemented");
        });

        if shortcuts.contains(Shortcuts::FitToCursor) {
            // TODO
        }

        if shortcuts.contains(Shortcuts::FitToWeave) {
            self.rect = Rect::ZERO;
        }

        /*ScrollArea::both()
        .animated(false)
        .auto_shrink(false)
        .show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(
                Vec2 {
                    x: self.arranged.width as f32,
                    y: self.arranged.height as f32,
                },
                Sense::click_and_drag(),
            );
            let rect = response.rect;

            for (item, (x, y)) in self.arranged.positions.iter() {
                let node = weave.get_node(item).unwrap();

                if let Some((p_x, p_y)) = node
                    .from
                    .and_then(|id| self.arranged.positions.get(&Ulid(id)))
                {
                    painter.line(
                        vec![
                            Pos2 {
                                x: *p_x as f32 + rect.min.x,
                                y: *p_y as f32 + rect.min.y,
                            },
                            Pos2 {
                                x: *x as f32 + rect.min.x,
                                y: *y as f32 + rect.min.y,
                            },
                        ],
                        Stroke {
                            width: 1.0,
                            color: default_color,
                        },
                    );
                }
            }

            for (item, (x, y)) in self.arranged.positions.iter() {
                let node = weave.get_node(item).unwrap();

                painter.circle(
                    Pos2 {
                        x: *x as f32 + rect.min.x,
                        y: *y as f32 + rect.min.y,
                    },
                    2.5,
                    get_node_color(node, settings).unwrap_or(default_color),
                    Stroke::NONE,
                );
            }
        });*/

        //println!("{:#?}", self.arranged);
    }
}
