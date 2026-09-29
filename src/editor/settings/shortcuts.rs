use eframe::egui::{Context, Key, KeyboardShortcut, Modifiers, TextStyle, Ui};
use flagset::{FlagSet, flags};
use serde::{Deserialize, Serialize};

use crate::common::{
    ui::{shortcut_ui, update_shortcut_flag, update_shortcut_flag_held},
    view::Edit,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ShortcutSettings {
    #[serde(default)]
    generate_at_cursor: Option<KeyboardShortcut>,
    #[serde(default)]
    toggle_node_active: Option<KeyboardShortcut>,
    #[serde(default)]
    toggle_node_bookmarked: Option<KeyboardShortcut>,

    #[serde(default)]
    add_child: Option<KeyboardShortcut>,
    #[serde(default)]
    add_sibling: Option<KeyboardShortcut>,
    #[serde(default)]
    delete_current: Option<KeyboardShortcut>,
    #[serde(default)]
    delete_children: Option<KeyboardShortcut>,
    #[serde(default)]
    delete_siblings: Option<KeyboardShortcut>,
    #[serde(default)]
    delete_siblings_and_current: Option<KeyboardShortcut>,
    #[serde(default)]
    merge_with_parent: Option<KeyboardShortcut>,
    #[serde(default)]
    split_at_cursor: Option<KeyboardShortcut>,
    #[serde(default)]
    activate_hovered: Option<KeyboardShortcut>,

    #[serde(default)]
    move_to_parent: Option<KeyboardShortcut>,
    #[serde(default)]
    move_to_child: Option<KeyboardShortcut>,
    #[serde(default)]
    move_to_previous_sibling: Option<KeyboardShortcut>,
    #[serde(default)]
    move_to_next_sibling: Option<KeyboardShortcut>,

    #[serde(default)]
    reset_parameters: Option<KeyboardShortcut>,
    #[serde(default)]
    parameter_preset_1: Option<KeyboardShortcut>,
    #[serde(default)]
    parameter_preset_2: Option<KeyboardShortcut>,
    #[serde(default)]
    parameter_preset_3: Option<KeyboardShortcut>,
    #[serde(default)]
    parameter_preset_4: Option<KeyboardShortcut>,
    #[serde(default)]
    parameter_preset_5: Option<KeyboardShortcut>,
    #[serde(default)]
    parameter_preset_6: Option<KeyboardShortcut>,
    #[serde(default)]
    parameter_preset_7: Option<KeyboardShortcut>,
    #[serde(default)]
    parameter_preset_8: Option<KeyboardShortcut>,
    #[serde(default)]
    parameter_preset_9: Option<KeyboardShortcut>,
    #[serde(default)]
    parameter_preset_10: Option<KeyboardShortcut>,
    #[serde(default)]
    toggle_colors: Option<KeyboardShortcut>,
    #[serde(default)]
    toggle_color_override: Option<KeyboardShortcut>,
    #[serde(default)]
    toggle_probabilities: Option<KeyboardShortcut>,
    #[serde(default)]
    toggle_automatic_scrolling: Option<KeyboardShortcut>,

    #[serde(default)]
    toggle_node_collapsed: Option<KeyboardShortcut>,
    #[serde(default)]
    collapse_all_visible_inactive: Option<KeyboardShortcut>,
    #[serde(default)]
    collapse_children: Option<KeyboardShortcut>,
    #[serde(default)]
    expand_all_visible: Option<KeyboardShortcut>,
    #[serde(default)]
    expand_children: Option<KeyboardShortcut>,

    #[serde(default)]
    fit_to_cursor: Option<KeyboardShortcut>,
    #[serde(default)]
    fit_to_weave: Option<KeyboardShortcut>,

    #[serde(default)]
    close_focused_tab: Option<KeyboardShortcut>,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            generate_at_cursor: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::Space,
            }),
            toggle_node_active: None,
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

            close_focused_tab: Some(KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::W,
            }),
        }
    }
}

impl ShortcutSettings {
    pub fn clear(shortcuts: &mut FlagSet<Shortcuts>) {
        *shortcuts = FlagSet::<Shortcuts>::empty();
    }
    pub fn update(&mut self, shortcuts: &mut FlagSet<Shortcuts>, ctx: &Context) {
        Self::clear(shortcuts);

        ctx.input_mut(|input| {
            update_shortcut_flag(
                input,
                shortcuts,
                &self.generate_at_cursor,
                Shortcuts::GenerateAtCursor,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.toggle_node_active,
                Shortcuts::ToggleNodeActive,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.toggle_node_bookmarked,
                Shortcuts::ToggleNodeBookmarked,
            );

            update_shortcut_flag(input, shortcuts, &self.add_child, Shortcuts::AddChild);
            update_shortcut_flag(input, shortcuts, &self.add_sibling, Shortcuts::AddSibling);
            update_shortcut_flag(
                input,
                shortcuts,
                &self.delete_current,
                Shortcuts::DeleteCurrent,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.delete_children,
                Shortcuts::DeleteChildren,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.delete_siblings,
                Shortcuts::DeleteSiblings,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.delete_siblings_and_current,
                Shortcuts::DeleteSiblingsAndCurrent,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.merge_with_parent,
                Shortcuts::MergeWithParent,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.split_at_cursor,
                Shortcuts::SplitAtCursor,
            );

            update_shortcut_flag(
                input,
                shortcuts,
                &self.move_to_parent,
                Shortcuts::MoveToParent,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.move_to_child,
                Shortcuts::MoveToChild,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.move_to_previous_sibling,
                Shortcuts::MoveToPreviousSibling,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.move_to_next_sibling,
                Shortcuts::MoveToNextSibling,
            );

            update_shortcut_flag(
                input,
                shortcuts,
                &self.reset_parameters,
                Shortcuts::ResetParameters,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.parameter_preset_1,
                Shortcuts::ParameterPreset1,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.parameter_preset_2,
                Shortcuts::ParameterPreset2,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.parameter_preset_3,
                Shortcuts::ParameterPreset3,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.parameter_preset_4,
                Shortcuts::ParameterPreset4,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.parameter_preset_5,
                Shortcuts::ParameterPreset5,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.parameter_preset_6,
                Shortcuts::ParameterPreset6,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.parameter_preset_7,
                Shortcuts::ParameterPreset7,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.parameter_preset_8,
                Shortcuts::ParameterPreset8,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.parameter_preset_9,
                Shortcuts::ParameterPreset9,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.parameter_preset_10,
                Shortcuts::ParameterPreset10,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.toggle_colors,
                Shortcuts::ToggleColors,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.toggle_color_override,
                Shortcuts::ToggleColorOverride,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.toggle_probabilities,
                Shortcuts::ToggleProbabilities,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.toggle_automatic_scrolling,
                Shortcuts::ToggleAutoScroll,
            );

            update_shortcut_flag(
                input,
                shortcuts,
                &self.toggle_node_collapsed,
                Shortcuts::ToggleNodeCollapsed,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.collapse_all_visible_inactive,
                Shortcuts::CollapseAllVisibleInactive,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.collapse_children,
                Shortcuts::CollapseChildren,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.expand_all_visible,
                Shortcuts::ExpandAllVisible,
            );
            update_shortcut_flag(
                input,
                shortcuts,
                &self.expand_children,
                Shortcuts::ExpandChildren,
            );

            update_shortcut_flag(
                input,
                shortcuts,
                &self.fit_to_cursor,
                Shortcuts::FitToCursor,
            );
            update_shortcut_flag(input, shortcuts, &self.fit_to_weave, Shortcuts::FitToWeave);

            update_shortcut_flag(
                input,
                shortcuts,
                &self.close_focused_tab,
                Shortcuts::CloseFocusedTab,
            );

            update_shortcut_flag_held(
                input,
                shortcuts,
                &self.activate_hovered,
                Shortcuts::ActivateHovered,
            );
        });
    }
}

impl Edit for ShortcutSettings {
    fn ui(&mut self, ui: &mut Ui) {
        shortcut_ui(
            ui,
            &mut self.generate_at_cursor,
            "keybind-generate_at_cursor",
            "Generate completions at cursor",
        );
        shortcut_ui(
            ui,
            &mut self.toggle_node_active,
            "keybind-toggle_node_active",
            "Toggle active",
        );
        shortcut_ui(
            ui,
            &mut self.toggle_node_bookmarked,
            "keybind-toggle_node_bookmarked",
            "Toggle bookmarked",
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        shortcut_ui(ui, &mut self.add_child, "keybind-add_child", "Create child");
        shortcut_ui(
            ui,
            &mut self.add_sibling,
            "keybind-add_sibling",
            "Create sibling",
        );
        shortcut_ui(
            ui,
            &mut self.delete_current,
            "keybind-delete_current",
            "Delete current node",
        );
        shortcut_ui(
            ui,
            &mut self.delete_children,
            "keybind-delete_children",
            "Delete all children",
        );
        shortcut_ui(
            ui,
            &mut self.delete_siblings,
            "keybind-delete_siblings",
            "Delete all siblings",
        );
        shortcut_ui(
            ui,
            &mut self.delete_siblings_and_current,
            "keybind-delete_siblings_and_current",
            "Delete current node & all siblings",
        );
        shortcut_ui(
            ui,
            &mut self.merge_with_parent,
            "keybind-merge_with_parent",
            "Merge with parent",
        );
        shortcut_ui(
            ui,
            &mut self.split_at_cursor,
            "keybind-split_at_cursor",
            "Split node at cursor",
        );
        shortcut_ui(
            ui,
            &mut self.activate_hovered,
            "keybind-activate_hovered",
            "Activate hovered node",
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        shortcut_ui(
            ui,
            &mut self.move_to_parent,
            "keybind-move_to_parent",
            "Move to parent",
        );
        shortcut_ui(
            ui,
            &mut self.move_to_child,
            "keybind-move_to_child",
            "Move to child",
        );
        shortcut_ui(
            ui,
            &mut self.move_to_previous_sibling,
            "keybind-move_to_previous_sibling",
            "Move to previous sibling",
        );
        shortcut_ui(
            ui,
            &mut self.move_to_next_sibling,
            "keybind-move_to_next_sibling",
            "Move to next sibling",
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        shortcut_ui(
            ui,
            &mut self.reset_parameters,
            "keybind-reset_parameters",
            "Reset editor parameters",
        );
        shortcut_ui(
            ui,
            &mut self.parameter_preset_1,
            "keybind-parameter_preset_1",
            "Parameter preset 1",
        );
        shortcut_ui(
            ui,
            &mut self.parameter_preset_2,
            "keybind-parameter_preset_2",
            "Parameter preset 2",
        );
        shortcut_ui(
            ui,
            &mut self.parameter_preset_3,
            "keybind-parameter_preset_3",
            "Parameter preset 3",
        );
        shortcut_ui(
            ui,
            &mut self.parameter_preset_4,
            "keybind-parameter_preset_4",
            "Parameter preset 4",
        );
        shortcut_ui(
            ui,
            &mut self.parameter_preset_5,
            "keybind-parameter_preset_5",
            "Parameter preset 5",
        );
        shortcut_ui(
            ui,
            &mut self.parameter_preset_6,
            "keybind-parameter_preset_6",
            "Parameter preset 6",
        );
        shortcut_ui(
            ui,
            &mut self.parameter_preset_7,
            "keybind-parameter_preset_7",
            "Parameter preset 7",
        );
        shortcut_ui(
            ui,
            &mut self.parameter_preset_8,
            "keybind-parameter_preset_8",
            "Parameter preset 8",
        );
        shortcut_ui(
            ui,
            &mut self.parameter_preset_9,
            "keybind-parameter_preset_9",
            "Parameter preset 9",
        );
        shortcut_ui(
            ui,
            &mut self.parameter_preset_10,
            "keybind-parameter_preset_10",
            "Parameter preset 10",
        );
        shortcut_ui(
            ui,
            &mut self.toggle_colors,
            "keybind-toggle_colors",
            "Toggle color coding",
        );
        shortcut_ui(
            ui,
            &mut self.toggle_color_override,
            "keybind-toggle_color_override",
            "Toggle model color override",
        );
        shortcut_ui(
            ui,
            &mut self.toggle_probabilities,
            "keybind-toggle_probabilities",
            "Toggle token shading",
        );
        shortcut_ui(
            ui,
            &mut self.toggle_automatic_scrolling,
            "keybind-toggle_automatic_scrolling",
            "Toggle automatic scrolling",
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        shortcut_ui(
            ui,
            &mut self.toggle_node_collapsed,
            "keybind-toggle_node_collapsed",
            "Toggle collapsed",
        );
        shortcut_ui(
            ui,
            &mut self.collapse_all_visible_inactive,
            "keybind-collapse_all_visible_inactive",
            "Collapse all inactive + visible",
        );
        shortcut_ui(
            ui,
            &mut self.collapse_children,
            "keybind-collapse_children",
            "Collapse all children",
        );
        shortcut_ui(
            ui,
            &mut self.expand_all_visible,
            "keybind-expand_all_visible",
            "Expand all visible",
        );
        shortcut_ui(
            ui,
            &mut self.expand_children,
            "keybind-expand_children",
            "Expand all children",
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        shortcut_ui(
            ui,
            &mut self.fit_to_cursor,
            "keybind-fit_to_cursor",
            "Fit view to cursor",
        );
        shortcut_ui(
            ui,
            &mut self.fit_to_weave,
            "keybind-fit_to_weave",
            "Fit view to weave",
        );

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.5);

        shortcut_ui(
            ui,
            &mut self.close_focused_tab,
            "keybind-close_focused_tab",
            "Close focused tab",
        );
    }
}

flags! {
    pub enum Shortcuts: u64 {
        GenerateAtCursor,
        ToggleNodeBookmarked,
        ToggleNodeActive,

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

        CloseFocusedTab,
    }
}
