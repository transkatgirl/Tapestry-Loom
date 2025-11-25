use eframe::egui::{
    self, Align, Button, Color32, Layout, Margin, Sense, Ui, UiBuilder, Vec2,
    collapsing_header::CollapsingState,
};
use egui_notify::Toasts;
use egui_phosphor::regular;
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, NodeContent, TapestryWeave},
};

use crate::{editor::shared::SharedState, settings::Settings};

#[derive(Default, Debug)]
pub struct ListView {}

impl ListView {
    pub fn reset(&mut self) {}
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
    }
}

#[derive(Default, Debug)]
pub struct TreeListView {
    hoist: Option<Ulid>,
}

impl TreeListView {
    pub fn reset(&mut self) {
        self.hoist = None;
    }
    pub fn render(
        &mut self,
        ui: &mut Ui,
        weave: &mut TapestryWeave,
        settings: &Settings,
        toasts: &mut Toasts,
        state: &mut SharedState,
    ) {
        let roots: Vec<Ulid> = weave.get_roots().collect();
        render_weave_node_tree(weave, ui, state.identifier, roots, 4);

        if ui.button("test").clicked() {
            weave.add_node(DependentNode {
                id: Ulid::new().0,
                from: None,
                to: IndexSet::default(),
                active: false,
                bookmarked: false,
                contents: NodeContent {
                    content: InnerNodeContent::Snippet(Ulid::new().to_string().as_bytes().to_vec()),
                    metadata: IndexMap::new(),
                    model: None,
                },
            });
        }
    }
}

fn render_weave_node_tree(
    weave: &mut TapestryWeave,
    ui: &mut Ui,
    editor_id: Ulid,
    items: impl IntoIterator<Item = Ulid>,
    max_depth: usize,
) {
    if max_depth == 0 {
        return;
    }

    let indent_compensation = ui.spacing().icon_width + ui.spacing().icon_spacing;

    for item in items {
        if let Some(node) = weave.get_node(&item).cloned() {
            let mut render_label = |ui: &mut Ui| {
                ui.horizontal(|ui| {
                    if node.to.is_empty() {
                        ui.add_space(indent_compensation);
                    }

                    if ui
                        .add(
                            Button::new(String::from_utf8_lossy(
                                &node.contents.content.as_bytes().to_vec(),
                            ))
                            .fill(Color32::TRANSPARENT)
                            .frame(false),
                        )
                        .clicked()
                    {
                        weave.set_node_active_status(&Ulid(node.id), !node.active);
                        println!("{}", Ulid(node.id));
                    }

                    if ui.rect_contains_pointer(ui.max_rect()) {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button(regular::ERASER).clicked() {
                                weave.remove_node(&Ulid(node.id));
                            };
                            if ui.button(regular::BOOKMARK_SIMPLE).clicked() {
                                weave.set_node_bookmarked_status(&Ulid(node.id), !node.bookmarked);
                            };
                            if ui.button(regular::CHAT_TEXT).clicked() {
                                weave.add_node(DependentNode {
                                    id: Ulid::new().0,
                                    from: Some(node.id),
                                    to: IndexSet::default(),
                                    active: false,
                                    bookmarked: false,
                                    contents: NodeContent {
                                        content: InnerNodeContent::Snippet(vec![]),
                                        /*content: InnerNodeContent::Snippet(
                                            Ulid::new().to_string().as_bytes().to_vec(),
                                        ),*/
                                        metadata: IndexMap::new(),
                                        model: None,
                                    },
                                });
                            };
                            if ui.button(regular::SPARKLE).clicked() {
                                todo!()
                            };
                            if weave.is_mergeable_with_parent(&Ulid(node.id))
                                && ui.button(regular::GIT_MERGE).clicked()
                            {
                                weave.merge_with_parent(&Ulid(node.id));
                            };
                        });
                    }
                });
            };

            if node.to.is_empty() {
                render_label(ui);
            } else {
                let id = ui.make_persistent_id([editor_id.0, node.id, 0]);
                CollapsingState::load_with_default_open(ui.ctx(), id, false)
                    .show_header(ui, |ui| {
                        render_label(ui);
                    })
                    .body(|ui| {
                        render_weave_node_tree(
                            weave,
                            ui,
                            editor_id,
                            node.to.into_iter().map(Ulid),
                            max_depth - 1,
                        );
                    });
            }
        }
    }
}
