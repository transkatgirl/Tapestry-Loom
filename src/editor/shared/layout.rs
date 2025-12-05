use std::collections::{HashMap, hash_map::Entry};

use rust_sugiyama::{configure::Config, from_vertices_and_edges};
use tapestry_weave::{ulid::Ulid, v0::TapestryWeave};

pub struct WeaveLayout {
    identifier_map: HashMap<Ulid, u32>,
    identifier_unmap: HashMap<u32, Ulid>,
    vertices: Vec<(u32, (f64, f64))>,
    edges: Vec<(u32, u32)>,
    id_counter: u32,
}

impl WeaveLayout {
    pub fn with_capacity(node_capacity: usize, edge_capacity: usize) -> Self {
        Self {
            identifier_map: HashMap::with_capacity(node_capacity),
            identifier_unmap: HashMap::with_capacity(node_capacity),
            vertices: Vec::with_capacity(node_capacity),
            edges: Vec::with_capacity(edge_capacity),
            id_counter: 0,
        }
    }
    pub fn load_weave(&mut self, weave: TapestryWeave, node_sizes: HashMap<Ulid, (f64, f64)>) {
        self.identifier_map.clear();
        self.identifier_unmap.clear();
        self.vertices.clear();
        self.edges.clear();
        self.id_counter = 0;

        assert!(node_sizes.len() < (u32::MAX as usize));

        for (node, size) in node_sizes {
            let node_identifier = self.get_node_identifier(node);

            self.vertices.push((node_identifier, size));

            if let Some(weave_node) = weave.get_node(&node)
                && let Some(parent_node) = weave_node.from.map(Ulid)
            {
                let parent_node_identifier = self.get_node_identifier(parent_node);
                self.edges.push((parent_node_identifier, node_identifier));
            }
        }

        assert_eq!(self.identifier_map.len(), self.vertices.len());
    }
    fn get_node_identifier(&mut self, node: Ulid) -> u32 {
        match self.identifier_map.entry(node) {
            Entry::Occupied(occupied) => *occupied.get(),
            Entry::Vacant(vacant) => {
                let identifier = self.id_counter;
                self.id_counter += 1;

                vacant.insert(identifier);
                self.identifier_unmap.insert(identifier, node);

                identifier
            }
        }
    }
    pub fn layout_weave(&self, config: &Config) -> ArrangedWeave {
        let layout = from_vertices_and_edges(&self.vertices, &self.edges, config);

        let mut offset = 0.0;
        let mut width = 0.0;
        let mut height = 0.0;

        let mut positions = Vec::with_capacity(self.vertices.len());

        for (subgraph, subgraph_width, subgraph_height) in layout {
            for (vertex_index, (x, y)) in subgraph {
                let identifier = self
                    .identifier_unmap
                    .get(&self.vertices[vertex_index].0)
                    .unwrap();
                positions.push((*identifier, (x + offset, y)));
            }

            offset += subgraph_width;
            width += subgraph_width;
            height = f64::max(height, subgraph_height);
        }

        ArrangedWeave {
            positions,
            width,
            height,
        }
    }
}

pub struct ArrangedWeave {
    pub positions: Vec<(Ulid, (f64, f64))>,
    pub width: f64,
    pub height: f64,
}
