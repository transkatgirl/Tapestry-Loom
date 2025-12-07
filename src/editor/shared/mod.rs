use std::{borrow::Cow, cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use chrono::{DateTime, offset};
use eframe::egui::{
    Color32, Rgba, RichText, TextFormat, TextStyle, Ui,
    text::{LayoutJob, LayoutSection},
};
use egui_notify::Toasts;
use flagset::FlagSet;
use log::{debug, warn};
use reqwest::Client;
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, NodeContent},
};
use tokio::runtime::Runtime;

use crate::{
    editor::shared::weave::WeaveWrapper,
    settings::{
        Settings, UITheme,
        inference::{InferenceHandle, InferenceParameters},
        shortcuts::Shortcuts,
    },
};

pub(super) mod layout;
pub(super) mod weave;

pub struct SharedState {
    pub identifier: Ulid,
    pub runtime: Arc<Runtime>,
    pub client: Rc<RefCell<Option<Client>>>,
    pub inference: InferenceParameters,
    cursor_node: NodeIndex,
    last_cursor_node: NodeIndex,
    hovered_node: NodeIndex,
    last_hovered_node: NodeIndex,
    last_changed_node: Option<Ulid>,
    pub has_cursor_node_changed: bool,
    pub has_hover_node_changed: bool,
    pub has_weave_changed: bool,
    pub has_weave_layout_changed: bool,
    requests: HashMap<Ulid, InferenceHandle>,
    responses: Vec<Result<DependentNode<NodeContent>, anyhow::Error>>,
    last_theme: UITheme,
    pub has_theme_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeIndex {
    WithinNode(Ulid, usize),
    Node(Ulid),
    None,
}

impl NodeIndex {
    pub fn into_node(self) -> Option<Ulid> {
        match self {
            Self::WithinNode(node, _) => Some(node),
            Self::Node(node) => Some(node),
            Self::None => None,
        }
    }
    pub fn has_node(&self) -> bool {
        match self {
            Self::WithinNode(_, _) => true,
            Self::Node(_) => true,
            Self::None => false,
        }
    }
}

impl SharedState {
    pub fn new(
        identifier: Ulid,
        runtime: Arc<Runtime>,
        client: Rc<RefCell<Option<Client>>>,
        settings: &Settings,
    ) -> Self {
        Self {
            identifier,
            runtime,
            client,
            inference: settings.inference.default_parameters.clone(),
            cursor_node: NodeIndex::None,
            last_cursor_node: NodeIndex::None,
            hovered_node: NodeIndex::None,
            last_hovered_node: NodeIndex::None,
            last_changed_node: None,
            has_cursor_node_changed: false,
            has_hover_node_changed: false,
            has_weave_changed: false,
            has_weave_layout_changed: false,
            requests: HashMap::with_capacity(128),
            responses: Vec::with_capacity(128),
            last_theme: settings.interface.ui_theme,
            has_theme_changed: false,
        }
    }
    /*pub fn reset(&mut self) {
        self.cursor_node = NodeIndex::None;
        self.last_cursor_node = NodeIndex::None;
        self.hovered_node = NodeIndex::None;
        self.last_hovered_node = NodeIndex::None;
        self.has_cursor_node_changed = false;
        self.has_hover_node_changed = false;
        self.has_weave_changed = false;
        self.has_weave_layout_changed = false;
        self.requests.clear();
        self.responses.clear();
    }*/
    pub fn update(
        &mut self,
        weave: &mut WeaveWrapper,
        settings: &Settings,
        toasts: &mut Toasts,
        shortcuts: FlagSet<Shortcuts>,
    ) {
        self.has_cursor_node_changed = false;
        self.has_hover_node_changed = false;
        self.last_changed_node = None;
        if self.last_hovered_node != self.hovered_node && self.hovered_node != NodeIndex::None {
            self.last_changed_node = self.hovered_node.into_node();
            self.last_hovered_node = self.hovered_node;
            self.has_hover_node_changed = true;
        }
        self.hovered_node = NodeIndex::None;
        if let Some(cursor_node) = self.cursor_node.into_node()
            && !weave.contains(&cursor_node)
        {
            self.cursor_node = NodeIndex::None;
        }
        if !self.cursor_node.has_node()
            && let Some(active) = weave.get_active_thread_first()
        {
            self.cursor_node = NodeIndex::Node(active);
        }
        if self.last_cursor_node != self.cursor_node {
            self.last_changed_node = self.cursor_node.into_node();
            self.last_cursor_node = self.cursor_node;
            self.has_cursor_node_changed = true;
        }
        if self.last_theme != settings.interface.ui_theme {
            self.has_theme_changed = true;
            self.last_theme = settings.interface.ui_theme;
        } else {
            self.has_theme_changed = false;
        }

        InferenceParameters::get_responses(
            &self.runtime,
            self.client.borrow().as_ref(),
            &mut self.requests,
            &mut self.responses,
        );

        for response in self.responses.drain(..) {
            match response {
                Ok(node) => {
                    let identifier = node.id;
                    let parent = node.from;

                    if weave.add_node(node) {
                        if self.last_changed_node.is_none() {
                            self.last_changed_node = Some(Ulid(identifier));
                        }

                        if let Some(parent) = parent {
                            weave.sort_node_children_u128(&parent);
                        } else {
                            weave.sort_roots();
                        }
                    } else {
                        debug!("Failed to add node to weave");
                    }
                }
                Err(error) => {
                    toasts.error(format!("Inference failed: {error}"));
                    warn!("Inference failed: {error:#?}");
                }
            }
        }

        if shortcuts.contains(Shortcuts::GenerateAtCursor) {
            match self.last_cursor_node {
                NodeIndex::WithinNode(node, index) => {
                    if index == 0 {
                        let parent = weave.get_node(&node).and_then(|node| node.from.map(Ulid));
                        self.generate_children(weave, parent, settings);
                    } else {
                        weave.split_node(&node, index);
                        self.generate_children(weave, Some(node), settings);
                    }
                }
                NodeIndex::Node(node) => {
                    self.generate_children(weave, Some(node), settings);
                }
                NodeIndex::None => {}
            }
        }

        if shortcuts.contains(Shortcuts::ToggleNodeBookmarked)
            && let Some(node) = self
                .last_cursor_node
                .into_node()
                .and_then(|id| weave.get_node(&id))
        {
            let identifier = node.id;
            weave.set_node_bookmarked_status_u128(&identifier, !node.bookmarked);
        }

        if shortcuts.contains(Shortcuts::AddChild)
            && let Some(node) = self
                .last_cursor_node
                .into_node()
                .and_then(|id| weave.get_node(&id).cloned())
        {
            let identifier = Ulid::new().0;
            if weave.add_node(DependentNode {
                id: identifier,
                from: Some(node.id),
                to: IndexSet::default(),
                active: node.active,
                bookmarked: false,
                contents: NodeContent {
                    content: InnerNodeContent::Snippet(vec![]),
                    metadata: IndexMap::new(),
                    model: None,
                },
            }) && node.active
            {
                self.cursor_node = NodeIndex::Node(Ulid(identifier));
            }
        }

        if shortcuts.contains(Shortcuts::AddSibling)
            && let Some(node) = self
                .last_cursor_node
                .into_node()
                .and_then(|id| weave.get_node(&id).cloned())
        {
            let identifier = Ulid::new().0;
            if weave.add_node(DependentNode {
                id: identifier,
                from: node.from,
                to: IndexSet::default(),
                active: node.active,
                bookmarked: false,
                contents: NodeContent {
                    content: InnerNodeContent::Snippet(vec![]),
                    metadata: IndexMap::new(),
                    model: None,
                },
            }) && node.active
            {
                self.cursor_node = NodeIndex::Node(Ulid(identifier));
            }
        }

        if shortcuts.contains(Shortcuts::DeleteCurrent)
            && let Some(node) = self.last_cursor_node.into_node()
        {
            let parent = weave.get_node(&node).and_then(|node| node.from).map(Ulid);

            if weave.remove_node(&node)
                && let Some(parent) = parent
            {
                self.cursor_node = NodeIndex::Node(parent);
            }
        }

        if shortcuts.contains(Shortcuts::DeleteChildren)
            && let Some(node) = self
                .last_cursor_node
                .into_node()
                .and_then(|id| weave.get_node(&id))
        {
            let children: Vec<Ulid> = node.to.iter().copied().map(Ulid).collect();

            for child in children {
                weave.remove_node(&child);
            }
        }

        if shortcuts.contains(Shortcuts::DeleteSiblings)
            && let Some(node) = self
                .last_cursor_node
                .into_node()
                .and_then(|id| weave.get_node(&id))
        {
            let siblings: Vec<Ulid> =
                if let Some(parent) = node.from.and_then(|id| weave.get_node_u128(&id)) {
                    parent
                        .to
                        .iter()
                        .copied()
                        .filter(|id| *id != node.id)
                        .map(Ulid)
                        .collect()
                } else {
                    weave
                        .get_roots_u128()
                        .filter(|id| *id != node.id)
                        .map(Ulid)
                        .collect()
                };

            for sibling in siblings {
                weave.remove_node(&sibling);
            }
        }

        if shortcuts.contains(Shortcuts::DeleteSiblingsAndCurrent)
            && let Some(node) = self
                .last_cursor_node
                .into_node()
                .and_then(|id| weave.get_node(&id))
        {
            if let Some(parent) = node.from {
                self.cursor_node = NodeIndex::Node(Ulid(parent));
            }

            let siblings_and_current: Vec<Ulid> =
                if let Some(parent) = node.from.and_then(|id| weave.get_node_u128(&id)) {
                    parent.to.iter().copied().map(Ulid).collect()
                } else {
                    weave.get_roots().collect()
                };

            for item in siblings_and_current {
                weave.remove_node(&item);
            }
        }

        if shortcuts.contains(Shortcuts::MergeWithParent)
            && let Some(node) = self.last_cursor_node.into_node()
        {
            let parent = weave.get_node(&node).and_then(|node| node.from).map(Ulid);

            if weave.merge_with_parent(&node)
                && let Some(parent) = parent
            {
                self.cursor_node = NodeIndex::Node(parent);
            }
        }

        if shortcuts.contains(Shortcuts::SplitAtCursor)
            && let NodeIndex::WithinNode(node, index) = self.last_cursor_node
        {
            weave.split_node(&node, index);
        }

        if shortcuts.contains(Shortcuts::MoveToParent)
            && let Some(node) = self
                .last_cursor_node
                .into_node()
                .and_then(|id| weave.get_node(&id))
            && let Some(parent) = node.from.map(Ulid)
        {
            self.cursor_node = NodeIndex::Node(parent);
            weave.set_node_active_status(&parent, true);
        }

        if shortcuts.contains(Shortcuts::MoveToChild)
            && let Some(node) = self
                .last_cursor_node
                .into_node()
                .and_then(|id| weave.get_node(&id))
            && let Some(child) = node.to.first().copied().map(Ulid)
        {
            self.cursor_node = NodeIndex::Node(child);
            weave.set_node_active_status(&child, true);
        }

        if shortcuts.contains(Shortcuts::MoveToPreviousSibling)
            && let Some(node) = self
                .last_cursor_node
                .into_node()
                .and_then(|id| weave.get_node(&id))
            && let parent_children = node
                .from
                .and_then(|id| weave.get_node(&Ulid(id)))
                .map(|parent| &parent.to)
                .unwrap_or(weave.get_roots_u128_direct())
            && let Some(current_index) = parent_children.get_index_of(&node.id)
            && let Some(previous_sibling) = parent_children
                .get_index(current_index.saturating_sub(1))
                .copied()
                .map(Ulid)
        {
            self.cursor_node = NodeIndex::Node(previous_sibling);
            weave.set_node_active_status(&previous_sibling, true);
        }

        if shortcuts.contains(Shortcuts::MoveToNextSibling)
            && let Some(node) = self
                .last_cursor_node
                .into_node()
                .and_then(|id| weave.get_node(&id))
            && let parent_children = node
                .from
                .and_then(|id| weave.get_node(&Ulid(id)))
                .map(|parent| &parent.to)
                .unwrap_or(weave.get_roots_u128_direct())
            && let Some(current_index) = parent_children.get_index_of(&node.id)
            && let Some(next_sibling) = parent_children
                .get_index(current_index + 1)
                .copied()
                .map(Ulid)
        {
            self.cursor_node = NodeIndex::Node(next_sibling);
            weave.set_node_active_status(&next_sibling, true);
        }

        self.has_weave_layout_changed = weave.has_layout_changed();
        self.has_weave_changed = weave.has_changed();
    }
    pub fn get_cursor_node(&self) -> NodeIndex {
        self.last_cursor_node
    }
    pub fn get_hovered_node(&self) -> NodeIndex {
        self.last_hovered_node
    }
    pub fn get_changed_node(&self) -> Option<Ulid> {
        self.last_changed_node
    }
    pub fn set_cursor_node(&mut self, value: NodeIndex) {
        self.cursor_node = value;
    }
    pub fn set_hovered_node(&mut self, value: NodeIndex) {
        self.hovered_node = value;
    }
    pub fn generate_children(
        &mut self,
        weave: &mut WeaveWrapper,
        parent: Option<Ulid>,
        settings: &Settings,
    ) {
        if self.inference.models.is_empty() {
            self.responses
                .push(Err(anyhow::Error::msg("No models loaded")));
            return;
        }

        let content: Vec<u8> = if let Some(parent) = parent {
            let thread: Vec<u128> = weave.get_thread_from_u128(&parent.0).rev().collect();

            thread
                .into_iter()
                .filter_map(|id| weave.get_node_u128(&id))
                .flat_map(|node| node.contents.content.as_bytes().into_owned().into_iter())
                .collect()
        } else {
            vec![]
        };

        if let Some(client) = self.client.borrow().as_ref() {
            self.inference.create_request(
                &settings.inference,
                &self.runtime,
                client,
                parent,
                content,
                &mut self.requests,
            );
        } else {
            self.responses
                .push(Err(anyhow::Error::msg("Client is not initialized")));
        }
    }
    pub fn get_request_count(&self) -> usize {
        self.requests.len()
    }
}

pub fn render_node_metadata_tooltip(ui: &mut Ui, node: &DependentNode<NodeContent>) {
    ui.set_max_width(ui.spacing().tooltip_width);

    if let Some(model) = &node.contents.model {
        if let Some(color) = model
            .metadata
            .get("color")
            .and_then(|h| Color32::from_hex(h).ok())
        {
            ui.colored_label(color, &model.label);
        } else {
            ui.label(&model.label);
        }
    }

    for (key, value) in &node.contents.metadata {
        ui.label(format!("{key}: {value}"));
    }

    let datetime: DateTime<offset::Local> = DateTime::from(Ulid(node.id).datetime());

    ui.label(format!("{}", datetime.format("%x %r")));

    #[cfg(debug_assertions)]
    ui.label(Ulid(node.id).to_string());
}

pub fn render_token_tooltip(ui: &mut Ui, token: &[u8], token_metadata: &IndexMap<String, String>) {
    if token_metadata
        .get("original_length")
        .and_then(|value| value.parse::<usize>().ok())
        .map(|original_length| original_length == token.len())
        .unwrap_or(true)
    {
        if let Ok(string) = str::from_utf8(token) {
            ui.label(RichText::new(format!("{string:#?}")).monospace());
        } else {
            ui.label(RichText::new(format!("{token:?}")).monospace());
        }
    }

    render_token_metadata_tooltip(ui, token.len(), token_metadata);
}

pub fn render_token_metadata_tooltip(
    ui: &mut Ui,
    token_len: usize,
    token_metadata: &IndexMap<String, String>,
) {
    for (key, value) in token_metadata {
        if key == "probability"
            && let Ok(probability) = value.parse::<f32>()
        {
            ui.label(format!("probability: {:.2}%", probability * 100.0));
        } else if key == "original_length"
            && let Ok(original_length) = value.parse::<usize>()
        {
            if original_length != token_len {
                ui.colored_label(
                    ui.style().visuals.warn_fg_color,
                    "modified_boundaries: true",
                );
            }
        } else {
            ui.label(format!("{key}: {value}"));
        }
    }
}

pub fn get_token_color(
    node_color: Option<Color32>,
    token_metadata: &IndexMap<String, String>,
    settings: &Settings,
) -> Option<Color32> {
    if let Some(color) = node_color {
        if settings.interface.show_token_probabilities
            && let Some(probability) = token_metadata
                .get("probability")
                .and_then(|p| p.parse::<f32>().ok())
        {
            let probability = probability.clamp(0.0, 1.0);
            let rgba = Rgba::from(color).to_opaque();
            let opacity = (1.0 - (f32::ln(1.0 / probability)) / 10.0)
                .clamp(settings.interface.minimum_token_opacity / 100.0, 1.0);

            Some(Color32::from(Rgba::from_rgba_unmultiplied(
                rgba.r(),
                rgba.g(),
                rgba.b(),
                opacity,
            )))
        } else {
            Some(color)
        }
    } else if settings.interface.show_token_probabilities
        && let Some(probability) = token_metadata
            .get("probability")
            .and_then(|p| p.parse::<f32>().ok())
    {
        // TODO: Perform color blending in perceptual color space
        let probability = probability.clamp(0.0, 1.0);
        let (r, g, b) = if probability > 0.5 {
            ((1.0 - ((probability - 0.5) * 2.0)) * 255.0, 255.0, 0.0)
        } else {
            (255.0, (probability * 2.0) * 255.0, 0.0)
        };

        Some(Color32::from(Rgba::from_rgb(r, g, b)))
    } else {
        None
    }
}

pub fn get_node_color(node: &DependentNode<NodeContent>, settings: &Settings) -> Option<Color32> {
    if settings.interface.show_model_colors {
        node.contents.model.as_ref().and_then(|model| {
            model
                .metadata
                .get("color")
                .and_then(|h| Color32::from_hex(h).ok())
        })
    } else {
        None
    }
}

pub fn render_node_text(
    ui: &Ui,
    node: &DependentNode<NodeContent>,
    settings: &Settings,
    override_color: Option<Color32>,
    break_on_newline: bool,
) -> LayoutJob {
    let color = if let Some(override_color) = override_color {
        override_color
    } else {
        get_node_color(node, settings).unwrap_or(ui.visuals().widgets.inactive.text_color())
    };
    let font_id = TextStyle::Monospace.resolve(ui.style());

    match &node.contents.content {
        InnerNodeContent::Tokens(tokens) => {
            let mut text = String::with_capacity(tokens.iter().map(|(t, _)| t.len()).sum());
            let mut offset = 0;

            let mut sections = Vec::with_capacity(tokens.len());

            for (token, token_metadata) in tokens {
                let color = get_token_color(Some(color), token_metadata, settings)
                    .unwrap_or(ui.visuals().widgets.inactive.text_color());
                let token_text = from_utf8_lossy(token);

                sections.push(LayoutSection {
                    leading_space: 0.0,
                    byte_range: offset..(offset + token_text.len()),
                    format: TextFormat {
                        font_id: font_id.clone(),
                        color,
                        valign: ui.text_valign(),
                        ..Default::default()
                    },
                });
                offset += token_text.len();
                text.push_str(&token_text);
            }

            LayoutJob {
                text,
                sections,
                break_on_newline,
                ..Default::default()
            }
        }
        InnerNodeContent::Snippet(snippet) => {
            let text = from_utf8_lossy(snippet).to_string();
            let text_length = text.len();

            LayoutJob {
                text,
                sections: vec![LayoutSection {
                    leading_space: 0.0,
                    byte_range: 0..text_length,
                    format: TextFormat {
                        font_id,
                        color,
                        valign: ui.text_valign(),
                        ..Default::default()
                    },
                }],
                break_on_newline,
                ..Default::default()
            }
        }
    }
}

// Modified version of String::from_utf8_lossy()
fn from_utf8_lossy(v: &[u8]) -> Cow<'_, str> {
    let mut iter = v.utf8_chunks();

    let first_valid = if let Some(chunk) = iter.next() {
        let valid = chunk.valid();
        if chunk.invalid().is_empty() {
            return Cow::Borrowed(valid);
        }
        valid
    } else {
        return Cow::Borrowed("");
    };

    const REPLACEMENT: &str = "\u{1A}";

    let mut res = String::with_capacity(v.len());
    res.push_str(first_valid);
    res.push_str(REPLACEMENT);

    for chunk in iter {
        res.push_str(chunk.valid());
        if !chunk.invalid().is_empty() {
            res.push_str(REPLACEMENT);
        }
    }

    Cow::Owned(res)
}
