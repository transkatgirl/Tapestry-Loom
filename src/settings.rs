use std::{fmt::Display, path::PathBuf, time::Duration};

use eframe::egui::{
    ComboBox, Context, Event, Frame, InputState, Key, KeyboardShortcut, Modifiers, ScrollArea,
    Slider, SliderClamping, TextStyle, Ui, Visuals,
};
use egui_keybind::Keybind;
use flagset::{FlagSet, flags};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Settings {
    pub interface: UISettings,
    pub shortcuts: KeyboardShortcuts,
    pub documents: DocumentSettings,
    pub inference: InferenceSettings,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KeyboardShortcuts {
    generate_at_cursor: Option<KeyboardShortcut>,
    toggle_node_bookmarked: Option<KeyboardShortcut>,

    add_child: Option<KeyboardShortcut>,
    add_sibling: Option<KeyboardShortcut>,
    delete_current: Option<KeyboardShortcut>,
    delete_children: Option<KeyboardShortcut>,
    delete_siblings: Option<KeyboardShortcut>,
    delete_siblings_and_current: Option<KeyboardShortcut>,
    merge_with_parent: Option<KeyboardShortcut>,
    split_at_cursor: Option<KeyboardShortcut>,

    move_to_parent: Option<KeyboardShortcut>,
    move_to_child: Option<KeyboardShortcut>,
    move_to_previous_sibling: Option<KeyboardShortcut>,
    move_to_next_sibling: Option<KeyboardShortcut>,

    reset_parameters: Option<KeyboardShortcut>,
    toggle_colors: Option<KeyboardShortcut>,

    toggle_node_collapsed: Option<KeyboardShortcut>,
    collapse_all_visible_inactive: Option<KeyboardShortcut>,
    collapse_children: Option<KeyboardShortcut>,
    expand_all_visible: Option<KeyboardShortcut>,
    expand_children: Option<KeyboardShortcut>,

    fit_to_cursor: Option<KeyboardShortcut>,
    fit_to_weave: Option<KeyboardShortcut>,
}

impl KeyboardShortcuts {
    fn render(&mut self, ui: &mut Ui) {
        ui.label("Pressing escape while modifying a keybind resets it to its default value.");

        ui.add(
            Keybind::new(&mut self.generate_at_cursor, "keybind-generate_at_cursor")
                .with_text("Generate completions at cursor")
                .with_reset(KeyboardShortcuts::default().generate_at_cursor)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.toggle_node_bookmarked,
                "keybind-toggle_node_bookmarked",
            )
            .with_text("Toggle bookmarked")
            .with_reset(KeyboardShortcuts::default().toggle_node_bookmarked)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        ui.add(
            Keybind::new(&mut self.add_child, "keybind-add_child")
                .with_text("Create child")
                .with_reset(KeyboardShortcuts::default().add_child)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.add_sibling, "keybind-add_sibling")
                .with_text("Create sibling")
                .with_reset(KeyboardShortcuts::default().add_sibling)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.delete_current, "keybind-delete_current")
                .with_text("Delete current node")
                .with_reset(KeyboardShortcuts::default().delete_current)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.delete_children, "keybind-delete_children")
                .with_text("Delete all children")
                .with_reset(KeyboardShortcuts::default().delete_children)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.delete_siblings, "keybind-delete_siblings")
                .with_text("Delete all siblings")
                .with_reset(KeyboardShortcuts::default().delete_siblings)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.delete_siblings_and_current,
                "keybind-delete_siblings_and_current",
            )
            .with_text("Delete current node & all siblings")
            .with_reset(KeyboardShortcuts::default().delete_siblings_and_current)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.merge_with_parent, "keybind-merge_with_parent")
                .with_text("Merge with parent")
                .with_reset(KeyboardShortcuts::default().merge_with_parent)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.split_at_cursor, "keybind-split_at_cursor")
                .with_text("Split node at cursor")
                .with_reset(KeyboardShortcuts::default().split_at_cursor)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        ui.add(
            Keybind::new(&mut self.move_to_parent, "keybind-move_to_parent")
                .with_text("Move to parent")
                .with_reset(KeyboardShortcuts::default().move_to_parent)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.move_to_child, "keybind-move_to_child")
                .with_text("Move to child")
                .with_reset(KeyboardShortcuts::default().move_to_child)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.move_to_previous_sibling,
                "keybind-move_to_previous_sibling",
            )
            .with_text("Move to previous sibling")
            .with_reset(KeyboardShortcuts::default().move_to_previous_sibling)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.move_to_next_sibling,
                "keybind-move_to_next_sibling",
            )
            .with_text("Move to next sibling")
            .with_reset(KeyboardShortcuts::default().move_to_next_sibling)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        ui.add(
            Keybind::new(&mut self.reset_parameters, "keybind-reset_parameters")
                .with_text("Reset editor parameters")
                .with_reset(KeyboardShortcuts::default().reset_parameters)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.toggle_colors, "keybind-toggle_colors")
                .with_text("Toggle editor colors")
                .with_reset(KeyboardShortcuts::default().toggle_colors)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        ui.add(
            Keybind::new(
                &mut self.toggle_node_collapsed,
                "keybind-toggle_node_collapsed",
            )
            .with_text("Toggle collapsed")
            .with_reset(KeyboardShortcuts::default().toggle_node_collapsed)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.collapse_all_visible_inactive,
                "keybind-collapse_all_visible_inactive",
            )
            .with_text("Collapse all inactive + visible")
            .with_reset(KeyboardShortcuts::default().collapse_all_visible_inactive)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.collapse_children, "keybind-collapse_children")
                .with_text("Collapse all children")
                .with_reset(KeyboardShortcuts::default().collapse_children)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.expand_all_visible, "keybind-expand_all_visible")
                .with_text("Expand all visible")
                .with_reset(KeyboardShortcuts::default().expand_all_visible)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.expand_children, "keybind-expand_children")
                .with_text("Expand all children")
                .with_reset(KeyboardShortcuts::default().expand_children)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        ui.add(
            Keybind::new(&mut self.fit_to_cursor, "keybind-fit_to_cursor")
                .with_text("Fit view automatically")
                .with_reset(KeyboardShortcuts::default().fit_to_cursor)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.fit_to_weave, "keybind-fit_to_weave")
                .with_text("Fit view to weave")
                .with_reset(KeyboardShortcuts::default().fit_to_weave)
                .with_reset_key(Some(Key::Escape)),
        );
    }
    pub fn get_pressed(&self, ctx: &Context) -> FlagSet<Shortcuts> {
        let mut flags = FlagSet::<Shortcuts>::empty();

        ctx.input_mut(|input| {
            if let Some(shortcut) = &self.generate_at_cursor
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::GenerateAtCursor;
            }

            if let Some(shortcut) = &self.toggle_node_bookmarked
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ToggleNodeBookmarked;
            }

            if let Some(shortcut) = &self.add_child
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::AddChild;
            }

            if let Some(shortcut) = &self.add_sibling
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::AddSibling;
            }

            if let Some(shortcut) = &self.delete_current
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::DeleteCurrent;
            }

            if let Some(shortcut) = &self.delete_children
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::DeleteChildren;
            }

            if let Some(shortcut) = &self.delete_siblings
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::DeleteSiblings;
            }

            if let Some(shortcut) = &self.delete_siblings_and_current
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::DeleteSiblingsAndCurrent;
            }

            if let Some(shortcut) = &self.merge_with_parent
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::MergeWithParent;
            }

            if let Some(shortcut) = &self.split_at_cursor
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::SplitAtCursor;
            }

            if let Some(shortcut) = &self.move_to_parent
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::MoveToParent;
            }

            if let Some(shortcut) = &self.move_to_child
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::MoveToChild;
            }

            if let Some(shortcut) = &self.move_to_previous_sibling
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::MoveToPreviousSibling;
            }

            if let Some(shortcut) = &self.move_to_next_sibling
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::MoveToNextSibling;
            }

            if let Some(shortcut) = &self.reset_parameters
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ResetParameters;
            }

            if let Some(shortcut) = &self.toggle_colors
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ToggleColors;
            }

            if let Some(shortcut) = &self.toggle_node_collapsed
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ToggleNodeCollapsed;
            }

            if let Some(shortcut) = &self.collapse_all_visible_inactive
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::CollapseAllVisibleInactive;
            }

            if let Some(shortcut) = &self.collapse_children
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::CollapseChildren;
            }

            if let Some(shortcut) = &self.expand_all_visible
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ExpandAllVisible;
            }

            if let Some(shortcut) = &self.expand_children
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ExpandChildren;
            }

            if let Some(shortcut) = &self.fit_to_cursor
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::FitToCursor;
            }

            if let Some(shortcut) = &self.fit_to_weave
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::FitToWeave;
            }
        });

        flags
    }
}

flags! {
    pub enum Shortcuts: u32 {
        GenerateAtCursor,
        ToggleNodeBookmarked,

        AddChild,
        AddSibling,
        DeleteCurrent,
        DeleteChildren,
        DeleteSiblings,
        DeleteSiblingsAndCurrent,
        MergeWithParent,
        SplitAtCursor,

        MoveToParent,
        MoveToChild,
        MoveToPreviousSibling,
        MoveToNextSibling,

        ResetParameters,
        ToggleColors,

        ToggleNodeCollapsed,
        CollapseAllVisibleInactive,
        CollapseChildren,
        ExpandAllVisible,
        ExpandChildren,

        FitToCursor,
        FitToWeave,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct UISettings {
    pub ui_scale: f32,
    pub ui_theme: UITheme,
    pub displayed_ui_scale: f32,
    pub show_model_colors: bool,
    pub show_token_probabilities: bool,
    pub max_tree_depth: usize,
}

impl Default for UISettings {
    fn default() -> Self {
        Self {
            ui_scale: 1.25,
            ui_theme: UITheme::Dark,
            displayed_ui_scale: 1.25,
            show_model_colors: true,
            show_token_probabilities: true,
            max_tree_depth: 8,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UITheme {
    Dark,
    Light,
}

impl Display for UITheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dark => f.write_str("Dark"),
            Self::Light => f.write_str("Light"),
        }
    }
}

impl UITheme {
    fn get_visuals(&self) -> Visuals {
        match &self {
            Self::Dark => Visuals::dark(),
            Self::Light => Visuals::light(),
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
            .selected_text(format!("{:?}", self.ui_theme))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.ui_theme, UITheme::Dark, UITheme::Dark.to_string());
                ui.selectable_value(
                    &mut self.ui_theme,
                    UITheme::Light,
                    UITheme::Light.to_string(),
                );
            });
        let ui_slider = ui.add(
            Slider::new(&mut self.displayed_ui_scale, 0.5..=4.0)
                .logarithmic(true)
                .clamping(SliderClamping::Never)
                .text("Scale")
                .suffix("x"),
        );
        if !(ui_slider.has_focus() || ui_slider.hovered()) {
            self.ui_scale = self.displayed_ui_scale;
        }

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.75);

        ui.checkbox(&mut self.show_model_colors, "Show model colors");
        ui.checkbox(
            &mut self.show_token_probabilities,
            "Show token probabilities in editor",
        );
        ui.add(
            Slider::new(&mut self.max_tree_depth, 1..=32)
                .clamping(SliderClamping::Never)
                .text("Maximum tree list depth"),
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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct InferenceSettings {}

impl InferenceSettings {
    fn render(&mut self, ui: &mut Ui) {}
}

impl Settings {
    pub fn render(&mut self, ui: &mut Ui) {
        ScrollArea::vertical()
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

// Copied from egui source code and modified to use Modifiers::matches_exact()
fn count_and_consume_key(input: &mut InputState, modifiers: Modifiers, logical_key: Key) -> usize {
    let mut count = 0usize;

    input.events.retain(|event| {
        let is_match = matches!(
            event,
            Event::Key {
                key: ev_key,
                modifiers: ev_mods,
                pressed: true,
                ..
            } if *ev_key == logical_key && ev_mods.matches_exact(modifiers)
        );

        count += is_match as usize;

        !is_match
    });

    count
}

// Copied from egui source code
fn consume_shortcut(input: &mut InputState, shortcut: &KeyboardShortcut) -> bool {
    let KeyboardShortcut {
        modifiers,
        logical_key,
    } = *shortcut;
    count_and_consume_key(input, modifiers, logical_key) > 0
}
