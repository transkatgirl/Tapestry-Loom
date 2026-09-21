use eframe::egui::{Context, Frame, ScrollArea, TextEdit, Ui, WidgetText};
use tapestry_weave::universal_weave::MetadataWeave;

use crate::{
    common::{ui::config_map, view::View},
    editor::EditorShared,
};

#[derive(Default, Debug)]
pub struct InfoView {}

impl View<EditorShared> for InfoView {
    fn title(&self, _shared: &EditorShared) -> WidgetText {
        WidgetText::Text("\u{E0F9} Info".to_string())
    }
    fn logic(&mut self, _shared: &mut EditorShared, _force_close: impl FnOnce(), _ctx: &Context) {}
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        let Some(weave) = &mut shared.weave else {
            return;
        };

        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .show(ui, |ui| {
                Frame::new()
                    .outer_margin(ui.style().spacing.menu_margin)
                    .show(ui, |ui| {
                        // TODO: add metadata logging
                        weave.weave.metadata_mut(|metadata| {
                            let width = ui.spacing().text_edit_width * 2.0;

                            ui.group(|ui| {
                                let label = ui.label("Title:").id;

                                let mut title = metadata.title.clone().unwrap_or_default();

                                ui.add(TextEdit::singleline(&mut title).desired_width(width))
                                    .labelled_by(label);

                                if title.is_empty() {
                                    metadata.title = None;
                                } else {
                                    metadata.title = Some(title);
                                }
                            });

                            ui.group(|ui| {
                                let label = ui.label("Description:").id;

                                let mut description =
                                    metadata.description.clone().unwrap_or_default();

                                ui.add(
                                    TextEdit::multiline(&mut description)
                                        .desired_width(width)
                                        .lock_focus(true),
                                )
                                .labelled_by(label);

                                if description.is_empty() {
                                    metadata.description = None;
                                } else {
                                    metadata.description = Some(description);
                                }
                            });

                            ui.group(|ui| {
                                let mut mapping: Vec<(String, String)> = metadata
                                    .metadata
                                    .iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect();

                                ui.label("Metadata:");

                                if config_map(ui, &mut mapping, 0.9, 1.1) {
                                    metadata.metadata.clear();
                                    metadata.metadata.extend(mapping);
                                };
                            });

                            ui.group(|ui| {
                                ui.weak(format!(
                                    "Created {}",
                                    metadata.created.strftime("%m/%d/%Y %I:%M:%S %p")
                                ));

                                if !metadata.converted_from.is_empty() {
                                    ui.weak("Converted from:");

                                    for conversion in &metadata.converted_from {
                                        ui.weak(format!(
                                            "\t- {}{} ({})\n\t\t- Using {}{}",
                                            conversion.source,
                                            conversion
                                                .source_version
                                                .as_ref()
                                                .map(|version| format!(" v{version}"))
                                                .unwrap_or_default(),
                                            conversion.timestamp.strftime("%m/%d/%Y %I:%M:%S %p"),
                                            if conversion.is_converter_native() {
                                                "Tapestry Loom"
                                            } else {
                                                &conversion.converter
                                            },
                                            conversion
                                                .converter_version
                                                .as_ref()
                                                .map(
                                                    |version| if conversion.is_converter_native() {
                                                        format!(
                                                            " ({} v{version})",
                                                            conversion.converter
                                                        )
                                                    } else {
                                                        format!(" v{version}")
                                                    }
                                                )
                                                .unwrap_or_default(),
                                        ));
                                    }
                                }
                            });
                        });
                    });
            });
    }
}

#[derive(Default, Debug)]
pub struct MenuView {}

impl View<EditorShared> for MenuView {
    fn title(&self, _shared: &EditorShared) -> WidgetText {
        WidgetText::Text("\u{E1B1} Menu".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, _force_close: impl FnOnce(), ctx: &Context) {}
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        if let Some(weave) = &mut shared.weave {
            // TODO
        }
    }
}
