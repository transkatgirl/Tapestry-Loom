use eframe::egui::{Context, Event, InputState, Key, KeyboardShortcut, Modifiers, TextStyle, Ui};
use egui_keybind::Keybind;
use flagset::{FlagSet, flags};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    activate_hovered: Option<KeyboardShortcut>,

    move_to_parent: Option<KeyboardShortcut>,
    move_to_child: Option<KeyboardShortcut>,
    move_to_previous_sibling: Option<KeyboardShortcut>,
    move_to_next_sibling: Option<KeyboardShortcut>,

    reset_parameters: Option<KeyboardShortcut>,
    parameter_preset_1: Option<KeyboardShortcut>,
    parameter_preset_2: Option<KeyboardShortcut>,
    parameter_preset_3: Option<KeyboardShortcut>,
    parameter_preset_4: Option<KeyboardShortcut>,
    parameter_preset_5: Option<KeyboardShortcut>,
    parameter_preset_6: Option<KeyboardShortcut>,
    parameter_preset_7: Option<KeyboardShortcut>,
    parameter_preset_8: Option<KeyboardShortcut>,
    parameter_preset_9: Option<KeyboardShortcut>,
    parameter_preset_10: Option<KeyboardShortcut>,
    toggle_colors: Option<KeyboardShortcut>,
    toggle_color_override: Option<KeyboardShortcut>,
    toggle_probabilities: Option<KeyboardShortcut>,
    toggle_automatic_scrolling: Option<KeyboardShortcut>,

    toggle_node_collapsed: Option<KeyboardShortcut>,
    collapse_all_visible_inactive: Option<KeyboardShortcut>,
    collapse_children: Option<KeyboardShortcut>,
    expand_all_visible: Option<KeyboardShortcut>,
    expand_children: Option<KeyboardShortcut>,

    fit_to_cursor: Option<KeyboardShortcut>,
    fit_to_weave: Option<KeyboardShortcut>,
}

impl Default for KeyboardShortcuts {
    fn default() -> Self {
        Self {
            generate_at_cursor: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::Space,
            }),
            toggle_node_bookmarked: None,
            add_child: None,
            add_sibling: None,
            delete_current: None,
            delete_children: None,
            delete_siblings: None,
            delete_siblings_and_current: None,
            merge_with_parent: None,
            split_at_cursor: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::S,
            }),
            activate_hovered: None,
            move_to_parent: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::ArrowLeft,
            }),
            move_to_child: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::ArrowRight,
            }),
            move_to_previous_sibling: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::ArrowUp,
            }),
            move_to_next_sibling: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::ArrowDown,
            }),
            reset_parameters: None,
            parameter_preset_1: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::Num1,
            }),
            parameter_preset_2: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::Num2,
            }),
            parameter_preset_3: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::Num3,
            }),
            parameter_preset_4: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::Num4,
            }),
            parameter_preset_5: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::Num5,
            }),
            parameter_preset_6: None,
            parameter_preset_7: None,
            parameter_preset_8: None,
            parameter_preset_9: None,
            parameter_preset_10: None,
            toggle_colors: None,
            toggle_color_override: None,
            toggle_probabilities: None,
            toggle_automatic_scrolling: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::D,
            }),
            toggle_node_collapsed: None,
            collapse_all_visible_inactive: None,
            collapse_children: None,
            expand_all_visible: None,
            expand_children: None,
            fit_to_cursor: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::Num9,
            }),
            fit_to_weave: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::Num0,
            }),
        }
    }
}

impl KeyboardShortcuts {
    pub(super) fn render(&mut self, ui: &mut Ui) {
        ui.add(
            Keybind::new(&mut self.generate_at_cursor, "keybind-generate_at_cursor")
                .with_text("Generate completions at cursor")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.toggle_node_bookmarked,
                "keybind-toggle_node_bookmarked",
            )
            .with_text("Toggle bookmarked")
            .with_reset(None)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        ui.add(
            Keybind::new(&mut self.add_child, "keybind-add_child")
                .with_text("Create child")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.add_sibling, "keybind-add_sibling")
                .with_text("Create sibling")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.delete_current, "keybind-delete_current")
                .with_text("Delete current node")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.delete_children, "keybind-delete_children")
                .with_text("Delete all children")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.delete_siblings, "keybind-delete_siblings")
                .with_text("Delete all siblings")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.delete_siblings_and_current,
                "keybind-delete_siblings_and_current",
            )
            .with_text("Delete current node & all siblings")
            .with_reset(None)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.merge_with_parent, "keybind-merge_with_parent")
                .with_text("Merge with parent")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.split_at_cursor, "keybind-split_at_cursor")
                .with_text("Split node at cursor")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.activate_hovered, "keybind-activate_hovered")
                .with_text("Activate hovered node")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        ui.add(
            Keybind::new(&mut self.move_to_parent, "keybind-move_to_parent")
                .with_text("Move to parent")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.move_to_child, "keybind-move_to_child")
                .with_text("Move to child")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.move_to_previous_sibling,
                "keybind-move_to_previous_sibling",
            )
            .with_text("Move to previous sibling")
            .with_reset(None)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.move_to_next_sibling,
                "keybind-move_to_next_sibling",
            )
            .with_text("Move to next sibling")
            .with_reset(None)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        ui.add(
            Keybind::new(&mut self.reset_parameters, "keybind-reset_parameters")
                .with_text("Reset editor parameters")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.parameter_preset_1, "keybind-parameter_preset_1")
                .with_text("Parameter preset 1")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.parameter_preset_2, "keybind-parameter_preset_2")
                .with_text("Parameter preset 2")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.parameter_preset_3, "keybind-parameter_preset_3")
                .with_text("Parameter preset 3")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.parameter_preset_4, "keybind-parameter_preset_4")
                .with_text("Parameter preset 4")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.parameter_preset_5, "keybind-parameter_preset_5")
                .with_text("Parameter preset 5")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.parameter_preset_6, "keybind-parameter_preset_6")
                .with_text("Parameter preset 6")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.parameter_preset_7, "keybind-parameter_preset_7")
                .with_text("Parameter preset 7")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.parameter_preset_8, "keybind-parameter_preset_8")
                .with_text("Parameter preset 8")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.parameter_preset_9, "keybind-parameter_preset_9")
                .with_text("Parameter preset 9")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.parameter_preset_10, "keybind-parameter_preset_10")
                .with_text("Parameter preset 10")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.toggle_colors, "keybind-toggle_colors")
                .with_text("Toggle model colors")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.toggle_color_override,
                "keybind-toggle_color_override",
            )
            .with_text("Toggle model color override")
            .with_reset(None)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.toggle_probabilities,
                "keybind-toggle_probabilities",
            )
            .with_text("Toggle token probabilities")
            .with_reset(None)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.toggle_automatic_scrolling,
                "keybind-toggle_automatic_scrolling",
            )
            .with_text("Toggle automatic scrolling")
            .with_reset(None)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        ui.add(
            Keybind::new(
                &mut self.toggle_node_collapsed,
                "keybind-toggle_node_collapsed",
            )
            .with_text("Toggle collapsed")
            .with_reset(None)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(
                &mut self.collapse_all_visible_inactive,
                "keybind-collapse_all_visible_inactive",
            )
            .with_text("Collapse all inactive + visible")
            .with_reset(None)
            .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.collapse_children, "keybind-collapse_children")
                .with_text("Collapse all children")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.expand_all_visible, "keybind-expand_all_visible")
                .with_text("Expand all visible")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.expand_children, "keybind-expand_children")
                .with_text("Expand all children")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        ui.add(
            Keybind::new(&mut self.fit_to_cursor, "keybind-fit_to_cursor")
                .with_text("Fit view to cursor")
                .with_reset(None)
                .with_reset_key(Some(Key::Escape)),
        );

        ui.add(
            Keybind::new(&mut self.fit_to_weave, "keybind-fit_to_weave")
                .with_text("Fit view to weave")
                .with_reset(None)
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

            if let Some(shortcut) = &self.activate_hovered
                && is_shortcut_pressed(input, shortcut)
            {
                flags |= Shortcuts::ActivateHovered;
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

            if let Some(shortcut) = &self.parameter_preset_1
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ParameterPreset1;
            }

            if let Some(shortcut) = &self.parameter_preset_2
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ParameterPreset2;
            }

            if let Some(shortcut) = &self.parameter_preset_3
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ParameterPreset3;
            }

            if let Some(shortcut) = &self.parameter_preset_4
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ParameterPreset4;
            }

            if let Some(shortcut) = &self.parameter_preset_5
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ParameterPreset5;
            }

            if let Some(shortcut) = &self.parameter_preset_6
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ParameterPreset6;
            }

            if let Some(shortcut) = &self.parameter_preset_7
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ParameterPreset7;
            }

            if let Some(shortcut) = &self.parameter_preset_8
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ParameterPreset8;
            }

            if let Some(shortcut) = &self.parameter_preset_9
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ParameterPreset9;
            }

            if let Some(shortcut) = &self.parameter_preset_10
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ParameterPreset10;
            }

            if let Some(shortcut) = &self.toggle_colors
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ToggleColors;
            }

            if let Some(shortcut) = &self.toggle_color_override
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ToggleColorOverride;
            }

            if let Some(shortcut) = &self.toggle_probabilities
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ToggleProbabilities;
            }

            if let Some(shortcut) = &self.toggle_automatic_scrolling
                && consume_shortcut(input, shortcut)
            {
                flags |= Shortcuts::ToggleAutoScroll;
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
    pub enum Shortcuts: u64 {
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
        ActivateHovered,

        MoveToParent,
        MoveToChild,
        MoveToPreviousSibling,
        MoveToNextSibling,

        ResetParameters,
        ParameterPreset1,
        ParameterPreset2,
        ParameterPreset3,
        ParameterPreset4,
        ParameterPreset5,
        ParameterPreset6,
        ParameterPreset7,
        ParameterPreset8,
        ParameterPreset9,
        ParameterPreset10,
        ToggleColors,
        ToggleColorOverride,
        ToggleProbabilities,
        ToggleAutoScroll,

        ToggleNodeCollapsed,
        CollapseAllVisibleInactive,
        CollapseChildren,
        ExpandAllVisible,
        ExpandChildren,

        FitToCursor,
        FitToWeave,
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

fn is_shortcut_pressed(input: &mut InputState, shortcut: &KeyboardShortcut) -> bool {
    let KeyboardShortcut {
        modifiers,
        logical_key,
    } = *shortcut;
    input.modifiers.matches_exact(modifiers)
        && input.keys_down.len() == 1
        && input.keys_down.contains(&logical_key)
}
