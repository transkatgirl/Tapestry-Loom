use eframe::egui::{self, Context, Modal};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq)]
pub struct EligibleNotices {
    #[serde(default = "default_true")]
    upgrade_format_0_to_1: bool,
}

fn default_true() -> bool {
    true
}

impl EligibleNotices {
    pub fn new() -> EligibleNotices {
        EligibleNotices {
            upgrade_format_0_to_1: true,
        }
    }
    pub fn display(&mut self, ctx: &Context) {
        if self.upgrade_format_0_to_1 {
            Modal::new("notice-upgrade_format_0_to_1".into())
                .show(ctx, |ui| {
                    ui.set_width(400.0);

                    ui.heading("\u{e127} Breaking Format Changes");

                    ui.label("This version introduces a breaking change to Tapestry Loom's format.");
                    ui.label("All weaves opened in this new version will automatically be converted into a new format, making them unreadable in older versions of Tapestry Loom.");
                    ui.label("This format conversion may result in data loss. Please report any bugs that you find.");
                    ui.colored_label(ctx.style().visuals.warn_fg_color, "It is strongly recommended that you create backups of all of your weaves before continuing.");

                    ui.add_space(ui.style().spacing.menu_spacing);

                    ui.collapsing("Actions", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Exit Application").clicked() {
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            if ui.button("Continue").clicked() {
                                self.upgrade_format_0_to_1 = false;
                            }
                        });
                    });
                });
        }
    }
}
