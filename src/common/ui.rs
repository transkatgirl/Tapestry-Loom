use std::{borrow::Cow, path::Path};

use color::{AlphaColor, Oklch, PremulColor, PremulRgba8, Srgb};
use eframe::egui::{
    self, Color32, Event, Frame, InnerResponse, InputState, Key, KeyboardShortcut, Modifiers,
    PointerButton, Response, Sense, Ui, UiBuilder, Vec2, response::Flags, vec2,
};
use egui_keybind::Keybind;

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

pub fn shortcut_ui(
    ui: &mut Ui,
    bind: &mut Option<KeyboardShortcut>,
    id: impl Into<egui::Id>,
    label: &str,
) {
    ui.add(
        Keybind::new(bind, id)
            .with_text(label)
            .with_reset(None)
            .with_reset_key(Some(Key::Escape)),
    );
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

pub fn listing<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    Frame::new()
        .outer_margin(ui.style().spacing.menu_spacing)
        .show(ui, add_contents)
}

pub fn with_context_menu<R>(
    ui: &mut Ui,
    add_contents: impl FnOnce(&mut Ui) -> R,
    add_context_contents: impl FnOnce(&mut Ui),
) -> InnerResponse<R> {
    let response = ui.scope_builder(UiBuilder::new().sense(Sense::CLICK), add_contents);
    response.response.context_menu(add_context_contents);
    response
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

// Modified version of String::from_utf8_lossy() which uses the ASCII substitution character
// Because the ASCII substitution character is 1 byte long, the converted string is always the same length as the input bytes
pub fn from_utf8_lossy(v: &[u8]) -> Cow<'_, str> {
    let mut iter = v.utf8_chunks();

    let (first_valid, first_invalid) = if let Some(chunk) = iter.next() {
        let valid = chunk.valid();
        let invalid = chunk.invalid();
        if invalid.is_empty() {
            return Cow::Borrowed(valid);
        }
        (valid, invalid)
    } else {
        return Cow::Borrowed("");
    };

    const REPLACEMENT: &str = "\u{1A}";

    debug_assert_eq!(REPLACEMENT.len(), 1);

    let mut res = String::with_capacity(v.len());
    res.push_str(first_valid);
    for _ in first_invalid {
        res.push_str(REPLACEMENT);
    }

    for chunk in iter {
        res.push_str(chunk.valid());
        for _ in chunk.invalid() {
            res.push_str(REPLACEMENT);
        }
    }

    debug_assert_eq!(v.len(), res.len());

    Cow::Owned(res)
}

pub fn change_color_alpha(color: Color32, alpha: f32) -> Color32 {
    let color = PremulColor::from(PremulRgba8::from_u8_array(color.to_array()))
        .un_premultiply()
        .with_alpha(alpha)
        .premultiply()
        .to_rgba8()
        .to_u8_array();

    Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3])
}

pub fn multiply_color_alpha(color: Color32, rhs: f32) -> Color32 {
    let color = PremulColor::from(PremulRgba8::from_u8_array(color.to_array()))
        .un_premultiply()
        .multiply_alpha(rhs)
        .premultiply()
        .to_rgba8()
        .to_u8_array();

    Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3])
}

pub fn into_oklch(color: Color32) -> AlphaColor<Oklch> {
    PremulColor::from(PremulRgba8::from_u8_array(color.to_array()))
        .un_premultiply()
        .convert::<Oklch>()
}

pub fn from_oklch(color: AlphaColor<Oklch>) -> Color32 {
    let color = color
        .convert::<Srgb>()
        .premultiply()
        .to_rgba8()
        .to_u8_array();

    Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3])
}

// Based on egui::widgets::Separator
pub fn label_separator(ui: &mut Ui, opacity: f32) {
    if opacity < f32::EPSILON {
        return;
    }

    let available_space = if ui.is_sizing_pass() {
        Vec2::ZERO
    } else {
        ui.available_size_before_wrap()
    };

    let size = vec2(available_space.x, 0.0);

    let (rect, response) = ui.allocate_at_least(size, Sense::empty());

    if ui.is_rect_visible(response.rect) {
        let mut stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        stroke.color = multiply_color_alpha(stroke.color, opacity);
        let painter = ui.painter();

        painter.hline(rect.left()..=rect.right(), rect.center().y, stroke);
    }
}
