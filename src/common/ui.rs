use std::path::Path;

use eframe::{
    egui::{Context, PointerButton, Response, Ui, response::Flags},
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

pub fn after_ui_interaction(ctx: &Context) {
    // Discard the frame to allow same-frame feedback (saving potentially 16ms)
    // This is separated into a wrapper function to allow this behavior to be changed in the future
    ctx.request_discard("UI Interaction");
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
