use std::collections::{HashMap, hash_map::Entry};

use rust_sugiyama::{
    configure::{Config, CrossingMinimization, RankingType},
    from_vertices_and_edges,
};
use tapestry_weave::ulid::Ulid;

use crate::editor::shared::weave::WeaveWrapper;

#[derive(Debug)]
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
    pub fn load_weave(
        &mut self,
        weave: &WeaveWrapper,
        node_sizes: impl ExactSizeIterator<Item = (Ulid, (f64, f64))>,
    ) {
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
    pub fn layout_weave(&self, spacing: f64) -> ArrangedWeave {
        let layout = from_vertices_and_edges(
            &self.vertices,
            &self.edges,
            &Config {
                minimum_length: 1,
                vertex_spacing: spacing,
                dummy_vertices: false,
                dummy_size: 1.0,
                ranking_type: RankingType::Up,
                c_minimization: CrossingMinimization::Barycenter,
                transpose: false,
            },
        );

        let mut offset = 0.0;
        let mut width = 0.0;
        let mut height = 0.0;

        let mut positions = HashMap::with_capacity(self.vertices.len());

        for (subgraph, _, _) in layout {
            let mut subgraph_width: f64 = 0.0;
            let mut subgraph_height: f64 = 0.0;

            for (vertex_index, (x, y)) in subgraph {
                let identifier = self
                    .identifier_unmap
                    .get(&self.vertices[vertex_index].0)
                    .unwrap();
                positions.insert(*identifier, (x + offset, y));

                subgraph_width = subgraph_width.max(x);
                subgraph_height = subgraph_height.max(y);
            }

            offset += subgraph_width + spacing;
            width += subgraph_width + spacing;
            height = f64::max(height, subgraph_height);
        }

        ArrangedWeave {
            positions,
            width,
            height,
        }
    }
}

#[derive(Default, Debug)]
pub struct ArrangedWeave {
    pub positions: HashMap<Ulid, (f64, f64)>,
    pub width: f64,
    pub height: f64,
}
