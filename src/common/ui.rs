use std::path::Path;

use eframe::{
    egui::{
        Event, InputState, Key, KeyboardShortcut, Modifiers, PointerButton, Response, Ui,
        response::Flags,
    },
    epaint::MarginF32,
};

pub fn clicked_rising_edge(response: &Response) -> bool {
    // egui default is falling-edge
    // See also: https://ux.stackexchange.com/questions/16066/what-to-consider-a-click
    response.flags.contains(Flags::FAKE_PRIMARY_CLICKED)
        || (response.flags.contains(Flags::CONTAINS_POINTER)
            && response
                .ctx
                .input(|i| i.pointer.button_pressed(PointerButton::Primary)))
}

// Copied from egui source code and modified to use Modifiers::matches_exact()
pub fn count_and_consume_key(
    input: &mut InputState,
    modifiers: Modifiers,
    logical_key: Key,
) -> usize {
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
pub fn consume_shortcut(input: &mut InputState, shortcut: &KeyboardShortcut) -> bool {
    let KeyboardShortcut {
        modifiers,
        logical_key,
    } = *shortcut;
    count_and_consume_key(input, modifiers, logical_key) > 0
}

pub fn is_shortcut_pressed(input: &mut InputState, shortcut: &KeyboardShortcut) -> bool {
    let KeyboardShortcut {
        modifiers,
        logical_key,
    } = *shortcut;
    input.modifiers.matches_exact(modifiers)
        && input.keys_down.len() == 1
        && input.keys_down.contains(&logical_key)
}

pub fn listing_margin(ui: &mut Ui) -> MarginF32 {
    MarginF32::same(ui.style().spacing.menu_spacing)
}

pub fn abbreviate_path<'a>(root: &Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}

pub fn format_large_number(number: usize, singular_suffix: &str, plural_suffix: &str) -> String {
    if number >= 100_000_000 {
        format!("{:.0}M {plural_suffix}", number as f32 / 1_000_000.0)
    } else if number >= 1_000_000 {
        format!("{:.1}M {plural_suffix}", number as f32 / 1_000_000.0)
    } else if number >= 100_000 {
        format!("{:.0}k {plural_suffix}", number as f32 / 1_000.0)
    } else if number >= 1_000 {
        format!("{:.1}k {plural_suffix}", number as f32 / 1_000.0)
    } else if number == 1 {
        format!("1 {singular_suffix}")
    } else {
        format!("{} {plural_suffix}", number)
    }
}

pub fn format_large_number_detailed(
    number: usize,
    singular_suffix: &str,
    plural_suffix: &str,
) -> String {
    if number >= 100_000_000 {
        format!("{:.0}M {plural_suffix}", number as f32 / 1_000_000.0)
    } else if number >= 10_000_000 {
        format!("{:.1}M {plural_suffix}", number as f32 / 1_000_000.0)
    } else if number >= 1_000_000 {
        format!("{:.2}M {plural_suffix}", number as f32 / 1_000_000.0)
    } else if number >= 10_000 {
        format!("{:.1}k {plural_suffix}", number as f32 / 1_000.0)
    } else if number == 1 {
        format!("1 {singular_suffix}")
    } else {
        format!("{} {plural_suffix}", number)
    }
}

pub fn format_file_size(size: usize) -> String {
    if size >= 100_000_000_000 {
        format!("{:.0} GB", size as f32 / 1_000_000_000.0)
    } else if size >= 1_000_000_000 {
        format!("{:.1} GB", size as f32 / 1_000_000_000.0)
    } else if size >= 100_000_000 {
        format!("{:.0} MB", size as f32 / 1_000_000.0)
    } else if size >= 1_000_000 {
        format!("{:.1} MB", size as f32 / 1_000_000.0)
    } else if size >= 100_000 {
        format!("{:.0} kB", size as f32 / 1_000.0)
    } else if size >= 1_000 {
        format!("{:.1} kB", size as f32 / 1_000.0)
    } else {
        format!("{} bytes", size)
    }
}
