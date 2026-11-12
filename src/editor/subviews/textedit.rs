use std::{collections::HashMap, hash::BuildHasherDefault, iter, mem, ops::Range};

use eframe::egui::{
    Color32, Context, FontId, Frame, Galley, Id, Margin, Mesh, Pos2, Rect, ScrollArea, Sense,
    Shape, Stroke, TextBuffer, TextEdit, TextFormat, TextStyle, Ui, WidgetText,
    text::{ByteIndex, CCursor, CCursorRange, LayoutJob, LayoutSection, TextWrapping},
    text_edit::TextEditOutput,
};
use tapestry_weave::{
    InnerNodeContent, ShortId, content::from_utf8_lossy, universal_weave::Weave,
    util::RandomIdHasher, weave::wrappers::LoggedTapestryWeave,
};

use crate::{
    common::{
        ui::{hover_ui, rect_contains_pointer, view_contains_pointer},
        view::View,
    },
    editor::{
        EditorShared,
        shared::ui::{AutoscrollData, TokenTooltipFlags, WeaveUi},
    },
};

// TODO: Implement a context menu on the TextEdit
// Currently stuck on lacking APIs in egui; see https://github.com/emilk/egui/issues/4393

// TODO: Implement Ctrl+F in TextEdit

#[derive(Debug, Default)]
pub struct TextEditView {
    snippets: Vec<Snippet>,
    node_ranges: HashMap<ShortId, Range<usize>, BuildHasherDefault<RandomIdHasher>>,

    built: bool,
    default_color: Color32,

    edited: Option<String>,
    edited_snippets: Vec<Snippet>,
    removal_marks: Vec<CCursor>,
    divergence: usize,

    last_cursor: Option<ShortId>,
    selection: Option<Range<usize>>,
    last_text_cursor: Option<CCursorRange>,

    view_rect: Option<Rect>,
    pointer_inside: bool,
    hovered_snippet: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct Snippet {
    end: usize,
    node: Option<ShortId>,
    color: Color32,
    token_index: Option<usize>,
}

impl Snippet {
    fn with_end(self, end: usize) -> Self {
        Self { end, ..self }
    }
}

impl TextEditView {
    fn rebuild(&mut self, weave: &mut LoggedTapestryWeave, weave_ui: &mut WeaveUi) {
        let mut text = Vec::with_capacity(weave_ui.text.buffer.len());

        self.snippets.clear();
        self.node_ranges.clear();

        for node in weave.active_nodes() {
            let node_color = weave_ui.node_color(node).unwrap_or(self.default_color);
            let start = text.len();

            let mut push = |bytes: &[u8], color, token_index| {
                if !bytes.is_empty() {
                    text.extend_from_slice(bytes);
                    self.snippets.push(Snippet {
                        end: text.len(),
                        node: Some(node.id),
                        color,
                        token_index,
                    });
                }
            };

            match &node.contents.content {
                InnerNodeContent::Tokens(tokens) => {
                    for (token_index, token) in tokens.iter().enumerate() {
                        push(
                            &token.bytes,
                            weave_ui.token_color(node_color, token),
                            Some(token_index),
                        );
                    }
                }
                InnerNodeContent::Snippet(snippet) => push(snippet, node_color, None),
                InnerNodeContent::MetadataOnly => {}
            }

            self.node_ranges.insert(node.id, start..text.len());
        }

        if !weave_ui.text.dirty {
            weave_ui.text.buffer.clear();
            weave_ui.text.buffer.push_str(&from_utf8_lossy(&text));
        }

        self.built = true;
        self.edited = None;
    }
    fn update_preview(&mut self, weave: &mut LoggedTapestryWeave, text: &str) {
        if self.edited.as_deref() == Some(text) {
            return;
        }

        self.edited = Some(text.to_owned());
        self.edited_snippets.clear();
        self.removal_marks.clear();

        let hunks = weave.diff_active_text_str(text);

        self.divergence = hunks.first().map_or(text.len(), |hunk| hunk.new.start);

        let shifted =
            |snippet: &Snippet, shift| snippet.with_end(snippet.end.wrapping_add_signed(shift));

        let mut snippets = self.snippets.iter().peekable();
        let mut shift = 0;

        for hunk in &hunks {
            while let Some(snippet) = snippets.next_if(|snippet| snippet.end <= hunk.old.start) {
                self.edited_snippets.push(shifted(snippet, shift));
            }

            if let Some(snippet) = snippets.peek() {
                self.edited_snippets.push(snippet.with_end(hunk.new.start));
            }

            if hunk.new.is_empty() {
                self.removal_marks.push(char_index(text, hunk.new.start));
            } else {
                self.edited_snippets.push(Snippet {
                    end: hunk.new.end,
                    node: None,
                    color: self.default_color,
                    token_index: None,
                });
            }

            while snippets
                .next_if(|snippet| snippet.end <= hunk.old.end)
                .is_some()
            {}

            shift = hunk.new.end.cast_signed() - hunk.old.end.cast_signed();
        }

        self.edited_snippets
            .extend(snippets.map(|snippet| shifted(snippet, shift)));

        let mut start = 0;

        self.edited_snippets.retain_mut(|snippet| {
            snippet.end = text.ceil_char_boundary(snippet.end);

            mem::replace(&mut start, snippet.end) < snippet.end
        });
    }
    fn highlight(&self, ui: &Ui, weave_ui: &WeaveUi, font_id: &FontId) -> Vec<LayoutSection> {
        let inserted = Stroke::new(
            ui.visuals().widgets.hovered.fg_stroke.width,
            ui.visuals().warn_fg_color,
        );

        let mut sections = Vec::with_capacity(self.edited_snippets.len());
        let mut start = 0;

        for (snippet_index, snippet) in self.edited_snippets.iter().enumerate() {
            let mut format = TextFormat {
                font_id: font_id.clone(),
                color: snippet.color,
                valign: ui.text_valign(),
                ..Default::default()
            };

            if let Some(node) = &snippet.node {
                if weave_ui.is_hovered(node) {
                    format.background = ui.visuals().widgets.hovered.weak_bg_fill;

                    if self.hovered_snippet == Some(snippet_index) && snippet.token_index.is_some()
                    {
                        format.underline = ui.visuals().widgets.hovered.bg_stroke;
                    }
                }
            } else {
                format.underline = inserted;
            }

            sections.push(LayoutSection {
                leading_space: 0.0,
                byte_range: ByteIndex(start)..ByteIndex(snippet.end),
                format,
            });
            start = snippet.end;
        }

        sections
    }
    fn interact_snippets(
        &mut self,
        weave: &LoggedTapestryWeave,
        weave_ui: &mut WeaveUi,
        ui: &Ui,
        output: &TextEditOutput,
    ) -> Mesh {
        let boundary_color = ui.visuals().widgets.inactive.bg_fill;
        let removed_color = ui.visuals().error_fg_color;
        let stroke_width = ui.visuals().widgets.hovered.fg_stroke.width;

        let mut mesh = Mesh::default();
        let mut paint = |rect: Rect, color: Color32| {
            if ui.is_rect_visible(rect) {
                mesh.add_colored_rect(rect, color);
            }
        };

        let mut last_node = None;

        for (row, snippet_index, snippet, bounds) in
            snippet_pieces(&self.edited_snippets, output.galley_pos, &output.galley)
        {
            if last_node
                .replace(snippet.node)
                .is_some_and(|last| last != snippet.node)
            {
                paint(
                    Rect::from_x_y_ranges(
                        bounds.left() - stroke_width / 2.0..=bounds.left() + stroke_width / 2.0,
                        bounds.y_range(),
                    ),
                    boundary_color,
                );
            }

            if let Some(id) = snippet.node
                && ui.is_rect_visible(bounds)
            {
                let response = ui.interact(
                    bounds,
                    output.response.id.with((snippet_index, row)),
                    Sense::hover(),
                );

                let tooltip = hover_ui(&response, true, |ui| {
                    if let Some(node) = weave.get(&id) {
                        weave_ui.node_hover_tooltip(
                            ui,
                            node,
                            snippet.token_index,
                            TokenTooltipFlags::Contents
                                | TokenTooltipFlags::Counterfactual
                                | TokenTooltipFlags::WarnModified,
                        )
                    }
                });

                if response.contains_pointer() || tooltip.is_some() {
                    weave_ui.set_hovered(id);
                    self.hovered_snippet = Some(snippet_index);
                }
            }
        }

        for mark in self.removal_marks.iter().copied() {
            let caret = cursor_rect(output, mark);
            let half_length = caret.height() / 4.0;

            paint(
                Rect::from_x_y_ranges(
                    caret.left() - half_length..=caret.left() + half_length,
                    caret.bottom() - stroke_width..=caret.bottom(),
                ),
                removed_color,
            );
        }

        mesh
    }
    fn sync_cursor(&mut self, weave_ui: &mut WeaveUi, ui: &Ui, output: &TextEditOutput) {
        if weave_ui.cursor != self.last_cursor {
            self.last_cursor = weave_ui.cursor;
            self.selection = None;

            if !weave_ui.text.dirty
                && let Some(cursor) = weave_ui.cursor
                && let Some(range) = self.node_ranges.get(&cursor)
            {
                let byte_index = weave_ui.text.buffer.floor_char_boundary(range.end);
                let text_cursor = char_index(&weave_ui.text.buffer, byte_index);
                let cursor_range = CCursorRange::one(text_cursor);

                let mut state = output.state.clone();
                state.cursor.set_char_range(Some(cursor_range));
                state.store(ui.ctx(), output.response.id);

                ui.scroll_to_rect(cursor_rect(output, text_cursor), None);

                self.last_text_cursor = Some(cursor_range);
                self.selection = Some(byte_index..byte_index);
            }
        } else if let Some(cursor_range) = output.cursor_range
            && Some(cursor_range) != self.last_text_cursor
        {
            let to_byte_index = |cursor: CCursor| {
                weave_ui
                    .text
                    .buffer
                    .byte_index_from_char_index(cursor.index)
                    .0
            };
            let byte_index = to_byte_index(cursor_range.primary);
            let [start, end] = cursor_range.sorted_cursors().map(to_byte_index);

            self.last_text_cursor = Some(cursor_range);
            self.selection = Some(start..end);

            let within_cursor = weave_ui
                .cursor
                .and_then(|cursor| self.node_ranges.get(&cursor))
                .is_some_and(|range| (range.start..=range.end).contains(&byte_index));

            if byte_index <= self.divergence
                && !within_cursor
                && let Some(node) = self
                    .snippets
                    .get(
                        self.snippets
                            .partition_point(|snippet| snippet.end <= byte_index),
                    )
                    .or(self.snippets.last())
                    .and_then(|snippet| snippet.node)
            {
                weave_ui.cursor = Some(node);
                self.last_cursor = Some(node);
            }
        }
    }
}

impl View<EditorShared> for TextEditView {
    fn title(&self, _shared: &EditorShared) -> WidgetText {
        WidgetText::Text("\u{E265} Editor".to_string())
    }
    fn logic(&mut self, shared: &mut EditorShared, _force_close: impl FnOnce(), ctx: &Context) {
        let pointer_inside = self
            .view_rect
            .take()
            .is_some_and(|rect| rect_contains_pointer(ctx, rect));
        let pointer_left =
            mem::replace(&mut self.pointer_inside, pointer_inside) && !pointer_inside;
        let clicked_outside = !pointer_inside
            && ctx.input(|input| input.pointer.any_down() || input.pointer.any_released());

        let applied = (pointer_left || clicked_outside)
            && shared
                .weave
                .as_mut()
                .is_some_and(|weave| shared.ui.apply_text(weave));

        if applied || shared.weave_changed || shared.ui.settings_changed {
            self.built = false;
        }
    }
    fn ui(&mut self, shared: &mut EditorShared, ui: &mut Ui) {
        let Some(weave) = &mut shared.weave else {
            return;
        };

        self.view_rect = Some(ui.clip_rect());

        let default_color = ui.visuals().widgets.inactive.text_color();

        if self.default_color != default_color {
            self.default_color = default_color;
            self.built = false;
        }

        if !self.built {
            self.rebuild(weave, &mut shared.ui);
        }

        let mut font_id = TextStyle::Monospace.resolve(ui.style());
        font_id.size *= 1.1;

        let first_row_min_height = ui.fonts_mut(|fonts| fonts.row_height(&font_id));

        let contains_pointer = view_contains_pointer(ui);
        let autoscroll = AutoscrollData::new(ui);

        ScrollArea::vertical()
            .auto_shrink(false)
            .animated(false)
            .content_margin(ui.style().spacing.menu_spacing / 2.0)
            .show(ui, |ui| {
                let boundary_shape = ui.painter().add(Shape::Noop);

                let mut buffer = mem::take(&mut shared.ui.text.buffer);

                let output = TextEdit::multiline(&mut buffer)
                    .id(Id::new((shared.id.0, "textedit")))
                    .frame(Frame::new().inner_margin(Margin::symmetric(4, 2)))
                    .font(font_id.clone())
                    .lock_focus(true)
                    .min_size(ui.available_size())
                    .layouter(&mut |ui, buffer, wrap_width| {
                        let text = buffer.as_str();

                        self.update_preview(weave, text);

                        ui.painter().layout_job(LayoutJob {
                            sections: self.highlight(ui, &shared.ui, &font_id),
                            text: text.to_string(),
                            wrap: TextWrapping::wrap_at_width(wrap_width),
                            first_row_min_height,
                            ..Default::default()
                        })
                    })
                    .show(ui);

                shared.ui.text.buffer = buffer;
                shared.ui.text.dirty |= output.response.changed();

                ui.painter().set(
                    boundary_shape,
                    self.interact_snippets(weave, &mut shared.ui, ui, &output),
                );

                if let Some(target) = shared.ui.autoscroll_target()
                    && !contains_pointer
                    && let Some(range) = self.node_ranges.get(&target)
                {
                    let text = &shared.ui.text.buffer;
                    let rect = cursor_rect(&output, char_index(text, range.start))
                        .union(cursor_rect(&output, char_index(text, range.end)));

                    if autoscroll.allows(ui, rect.height()) {
                        ui.scroll_to_rect(rect, None);
                    }
                }

                self.sync_cursor(&mut shared.ui, ui, &output);

                if output.response.lost_focus() {
                    shared.ui.apply_text(weave);
                }
            });

        shared.ui.selection = self.selection.clone();
    }
}

fn char_index(text: &str, byte_index: usize) -> CCursor {
    CCursor::new(text.char_index_from_byte_index(ByteIndex(text.floor_char_boundary(byte_index))))
}

fn cursor_rect(output: &TextEditOutput, cursor: CCursor) -> Rect {
    output
        .galley
        .pos_from_cursor(cursor)
        .translate(output.galley_pos.to_vec2())
}

fn snippet_pieces<'a>(
    snippets: &'a [Snippet],
    top_left: Pos2,
    galley: &'a Galley,
) -> impl Iterator<Item = (usize, usize, &'a Snippet, Rect)> {
    let mut chars = galley
        .rows
        .iter()
        .enumerate()
        .flat_map(move |(index, row)| {
            let row_rect = row.rect().translate(top_left.to_vec2());

            row.glyphs
                .iter()
                .map(move |glyph| {
                    (
                        glyph.logical_rect().translate(row_rect.min.to_vec2()),
                        glyph.chr.len_utf8(),
                    )
                })
                .chain(
                    row.ends_with_newline
                        .then(|| (row_rect.with_min_x(row_rect.max.x), 1)),
                )
                .map(move |(rect, len)| (index, rect, len))
        })
        .peekable();

    let mut index = 0;
    let mut offset = 0;

    iter::from_fn(move || {
        while snippets.get(index)?.end <= offset {
            index += 1;
        }

        let snippet = &snippets[index];
        let row = chars.peek()?.0;

        let mut bounds = Rect::NOTHING;

        while let Some((_, rect, len)) =
            chars.next_if(|(char_row, ..)| *char_row == row && offset < snippet.end)
        {
            bounds = bounds.union(rect);
            offset += len;
        }

        Some((row, index, snippet, bounds))
    })
}
