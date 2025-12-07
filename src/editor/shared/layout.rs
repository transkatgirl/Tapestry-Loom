use std::collections::{HashMap, hash_map::Entry};

use eframe::egui::{Pos2, Rect, pos2};
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

        let mut x_offset = spacing;
        let y_offset = spacing;
        let mut width = 0.0;
        let mut height = 0.0;

        let mut positions = HashMap::with_capacity(self.vertices.len());
        let mut rects = HashMap::with_capacity(self.vertices.len());

        for (subgraph, _, _) in layout {
            let mut subgraph_width: f64 = 0.0;
            let mut subgraph_height: f64 = 0.0;

            for (vertex_index, (x, y)) in subgraph {
                let (id, (width, height)) = self.vertices[vertex_index];

                let x_pos = x + x_offset;
                let y_pos = y + y_offset;

                let identifier = self.identifier_unmap.get(&id).unwrap();
                positions.insert(*identifier, (x_pos, y_pos));
                rects.insert(
                    *identifier,
                    Rect {
                        min: Pos2 {
                            x: (x_pos - (width / 2.0)) as f32,
                            y: (y_pos - (height / 2.0)) as f32,
                        },
                        max: Pos2 {
                            x: (x_pos + (width / 2.0)) as f32,
                            y: (y_pos + (height / 2.0)) as f32,
                        },
                    },
                );

                subgraph_width = subgraph_width.max(x + (width / 2.0));
                subgraph_height = subgraph_height.max(y + (height / 2.0));
            }

            x_offset += subgraph_width + spacing;
            width += subgraph_width + spacing;
            height = f64::max(height, subgraph_height);
        }

        ArrangedWeave {
            positions,
            rects,
            width: width + spacing,
            height: height + (spacing * 2.0),
        }
    }
}

#[derive(Default, Debug)]
pub struct ArrangedWeave {
    pub positions: HashMap<Ulid, (f64, f64)>,
    pub rects: HashMap<Ulid, Rect>,
    pub width: f64,
    pub height: f64,
}

// Copied from egui-snarl
fn wire_bezier_5(frame_size: f32, from: Pos2, to: Pos2) -> [Pos2; 6] {
    let from_norm_x = frame_size;
    let from_2 = pos2(from.x + from_norm_x, from.y);
    let to_norm_x = -from_norm_x;
    let to_2 = pos2(to.x + to_norm_x, to.y);

    let between = (from_2 - to_2).length();

    if from_2.x <= to_2.x && between >= frame_size * 2.0 {
        let middle_1 = from_2 + (to_2 - from_2).normalized() * frame_size;
        let middle_2 = to_2 + (from_2 - to_2).normalized() * frame_size;

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.x <= to_2.x {
        let t = (between - (to_2.y - from_2.y).abs())
            / frame_size.mul_add(2.0, -(to_2.y - from_2.y).abs());

        let mut middle_1 = from_2 + (to_2 - from_2).normalized() * frame_size;
        let mut middle_2 = to_2 + (from_2 - to_2).normalized() * frame_size;

        if from_2.y >= to_2.y + frame_size {
            let u = (from_2.y - to_2.y - frame_size) / frame_size;

            let t0_middle_1 = pos2(
                (1.0 - u).mul_add(frame_size, from_2.x),
                frame_size.mul_add(-u, from_2.y),
            );
            let t0_middle_2 = pos2(to_2.x, to_2.y + frame_size);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if from_2.y >= to_2.y {
            let u = (from_2.y - to_2.y) / frame_size;

            let t0_middle_1 = pos2(
                u.mul_add(frame_size, from_2.x),
                frame_size.mul_add(1.0 - u, from_2.y),
            );
            let t0_middle_2 = pos2(to_2.x, to_2.y + frame_size);

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if to_2.y >= from_2.y + frame_size {
            let u = (to_2.y - from_2.y - frame_size) / frame_size;

            let t0_middle_1 = pos2(from_2.x, from_2.y + frame_size);
            let t0_middle_2 = pos2(
                (1.0 - u).mul_add(-frame_size, to_2.x),
                frame_size.mul_add(-u, to_2.y),
            );

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else if to_2.y >= from_2.y {
            let u = (to_2.y - from_2.y) / frame_size;

            let t0_middle_1 = pos2(from_2.x, from_2.y + frame_size);
            let t0_middle_2 = pos2(
                u.mul_add(-frame_size, to_2.x),
                frame_size.mul_add(1.0 - u, to_2.y),
            );

            middle_1 = t0_middle_1.lerp(middle_1, t);
            middle_2 = t0_middle_2.lerp(middle_2, t);
        } else {
            unreachable!();
        }

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= frame_size.mul_add(2.0, to_2.y) {
        let middle_1 = pos2(from_2.x, from_2.y - frame_size);
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= to_2.y + frame_size {
        let t = (from_2.y - to_2.y - frame_size) / frame_size;

        let middle_1 = pos2(
            (1.0 - t).mul_add(frame_size, from_2.x),
            frame_size.mul_add(-t, from_2.y),
        );
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if from_2.y >= to_2.y {
        let t = (from_2.y - to_2.y) / frame_size;

        let middle_1 = pos2(
            t.mul_add(frame_size, from_2.x),
            frame_size.mul_add(1.0 - t, from_2.y),
        );
        let middle_2 = pos2(to_2.x, to_2.y + frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= frame_size.mul_add(2.0, from_2.y) {
        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(to_2.x, to_2.y - frame_size);

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= from_2.y + frame_size {
        let t = (to_2.y - from_2.y - frame_size) / frame_size;

        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(
            (1.0 - t).mul_add(-frame_size, to_2.x),
            frame_size.mul_add(-t, to_2.y),
        );

        [from, from_2, middle_1, middle_2, to_2, to]
    } else if to_2.y >= from_2.y {
        let t = (to_2.y - from_2.y) / frame_size;

        let middle_1 = pos2(from_2.x, from_2.y + frame_size);
        let middle_2 = pos2(
            t.mul_add(-frame_size, to_2.x),
            frame_size.mul_add(1.0 - t, to_2.y),
        );

        [from, from_2, middle_1, middle_2, to_2, to]
    } else {
        unreachable!();
    }
}

// Copied from egui-snarl
pub fn wire_bezier_3(frame_size: f32, from: Pos2, to: Pos2) -> [Pos2; 4] {
    let [a, b, _, _, c, d] = wire_bezier_5(frame_size, from, to);
    [a, b, c, d]
}
