use easy_musiclib_shared::{Id, RelationEdge, RelationNode};
use std::collections::HashMap;

pub(crate) const GRAPH_VIEWBOX_WIDTH: f64 = 1200.0;
pub(crate) const GRAPH_VIEWBOX_HEIGHT: f64 = 700.0;

const GRAPH_CENTER_X: f64 = GRAPH_VIEWBOX_WIDTH / 2.0;
const GRAPH_CENTER_Y: f64 = GRAPH_VIEWBOX_HEIGHT / 2.0;
const GOLDEN_ANGLE: f64 = 2.399_963_229_728_653;
const SETTLE_FRAME_LIMIT: u32 = 48;
const SETTLE_STOP_SHIFT: f64 = 0.18;
const NODE_HIT_RADIUS: f64 = 26.0;
const NODE_LABEL_HIT_X: f64 = 10.0;
const NODE_LABEL_CHAR_WIDTH: f64 = 7.2;
const NODE_LABEL_PADDING: f64 = 24.0;
const NODE_LABEL_MIN_WIDTH: f64 = 52.0;
const NODE_LABEL_MAX_WIDTH: f64 = 260.0;
const NODE_LABEL_TRUNCATION: &str = "...";

#[derive(Clone, Copy)]
pub(crate) struct LayoutPosition {
    pub(crate) id: Id,
    pub(crate) x: f64,
    pub(crate) y: f64,
}

#[derive(Clone, Copy)]
struct SimNode {
    id: Id,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

#[derive(Clone, Copy)]
struct SimEdge {
    source: usize,
    target: usize,
    distance: f64,
    strength: f64,
}

#[derive(Clone, Copy)]
struct NodeExtents {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl NodeExtents {
    fn scaled(self, scale: f64) -> Self {
        Self {
            left: self.left * scale,
            right: self.right * scale,
            top: self.top * scale,
            bottom: self.bottom * scale,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct LayoutBounds {
    pub(crate) min_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_x: f64,
    pub(crate) max_y: f64,
}

impl LayoutBounds {
    pub(crate) fn width(self) -> f64 {
        (self.max_x - self.min_x).max(1.0)
    }

    pub(crate) fn height(self) -> f64 {
        (self.max_y - self.min_y).max(1.0)
    }
}

fn initial_position(index: usize) -> (f64, f64) {
    let radius = 14.0 * (index as f64 + 1.0).sqrt();
    let angle = index as f64 * GOLDEN_ANGLE;
    (
        GRAPH_CENTER_X + radius * angle.cos(),
        GRAPH_CENTER_Y + radius * angle.sin(),
    )
}

pub(crate) fn relation_node_label_text(name: &str) -> String {
    let max_text_width = NODE_LABEL_MAX_WIDTH - NODE_LABEL_PADDING;
    if relation_node_text_width(name) <= max_text_width {
        return name.to_string();
    }

    let truncation_width = relation_node_text_width(NODE_LABEL_TRUNCATION);
    let budget = (max_text_width - truncation_width).max(0.0);
    let mut text = String::new();
    let mut width = 0.0;
    for ch in name.chars() {
        let ch_width = relation_node_char_width(ch);
        if width + ch_width > budget {
            break;
        }
        text.push(ch);
        width += ch_width;
    }
    text.push_str(NODE_LABEL_TRUNCATION);
    text
}

pub(crate) fn relation_node_label_width(name: &str) -> f64 {
    let label = relation_node_label_text(name);
    (relation_node_text_width(&label) + NODE_LABEL_PADDING)
        .clamp(NODE_LABEL_MIN_WIDTH, NODE_LABEL_MAX_WIDTH)
}

pub(crate) fn layout_position(positions: &[LayoutPosition], id: Id) -> (f64, f64) {
    positions
        .iter()
        .find(|position| position.id == id)
        .map(|position| (position.x, position.y))
        .unwrap_or((GRAPH_CENTER_X, GRAPH_CENTER_Y))
}

pub(crate) fn relation_graph_bounds_with_scale(
    nodes: &[RelationNode],
    positions: &[LayoutPosition],
    padding: f64,
    node_scale: f64,
) -> LayoutBounds {
    let node_scale = node_scale.max(0.001);
    let Some(first) = nodes.first() else {
        return LayoutBounds {
            min_x: GRAPH_CENTER_X - GRAPH_VIEWBOX_WIDTH / 2.0,
            min_y: GRAPH_CENTER_Y - GRAPH_VIEWBOX_HEIGHT / 2.0,
            max_x: GRAPH_CENTER_X + GRAPH_VIEWBOX_WIDTH / 2.0,
            max_y: GRAPH_CENTER_Y + GRAPH_VIEWBOX_HEIGHT / 2.0,
        };
    };

    let (first_x, first_y) = layout_position(positions, first.id);
    let first_extents = relation_node_extents(first).scaled(node_scale);
    let mut bounds = LayoutBounds {
        min_x: first_x - first_extents.left,
        min_y: first_y - first_extents.top,
        max_x: first_x + first_extents.right,
        max_y: first_y + first_extents.bottom,
    };

    for node in nodes.iter().skip(1) {
        let (x, y) = layout_position(positions, node.id);
        let extents = relation_node_extents(node).scaled(node_scale);
        bounds.min_x = bounds.min_x.min(x - extents.left);
        bounds.min_y = bounds.min_y.min(y - extents.top);
        bounds.max_x = bounds.max_x.max(x + extents.right);
        bounds.max_y = bounds.max_y.max(y + extents.bottom);
    }

    LayoutBounds {
        min_x: bounds.min_x - padding,
        min_y: bounds.min_y - padding,
        max_x: bounds.max_x + padding,
        max_y: bounds.max_y + padding,
    }
}

pub(crate) fn relation_graph_layout(
    nodes: &[RelationNode],
    edges: &[RelationEdge],
) -> Vec<LayoutPosition> {
    let count = nodes.len();
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![LayoutPosition {
            id: nodes[0].id,
            x: GRAPH_CENTER_X,
            y: GRAPH_CENTER_Y,
        }];
    }

    let positions = seeded_positions(nodes, &[]);
    let iterations = if count <= 80 {
        360
    } else if count <= 180 {
        270
    } else {
        210
    };

    run_force_layout(nodes, edges, positions, None, iterations, 1.0, 0.986, true)
}

pub(crate) fn relation_graph_layout_settled(
    nodes: &[RelationNode],
    edges: &[RelationEdge],
) -> Vec<LayoutPosition> {
    let mut positions = relation_graph_layout(nodes, edges);
    for frame in 0..SETTLE_FRAME_LIMIT {
        let updated = relation_graph_layout_relax(nodes, edges, &positions, frame);
        let shift = max_layout_shift(&positions, &updated);
        positions = updated;
        if shift <= SETTLE_STOP_SHIFT {
            break;
        }
    }
    positions
}

pub(crate) fn relation_graph_layout_tick(
    nodes: &[RelationNode],
    edges: &[RelationEdge],
    current_positions: &[LayoutPosition],
    fixed_position: LayoutPosition,
) -> Vec<LayoutPosition> {
    let count = nodes.len();
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        let (x, y) = if nodes[0].id == fixed_position.id {
            (fixed_position.x, fixed_position.y)
        } else {
            current_positions
                .first()
                .map(|position| (position.x, position.y))
                .unwrap_or((GRAPH_CENTER_X, GRAPH_CENTER_Y))
        };
        return vec![LayoutPosition {
            id: nodes[0].id,
            x,
            y,
        }];
    }

    let positions = seeded_positions(nodes, current_positions);
    let iterations = if count <= 80 {
        14
    } else if count <= 180 {
        8
    } else {
        5
    };

    run_force_layout(
        nodes,
        edges,
        positions,
        Some(fixed_position),
        iterations,
        0.42,
        0.86,
        false,
    )
}

pub(crate) fn relation_graph_layout_relax(
    nodes: &[RelationNode],
    edges: &[RelationEdge],
    current_positions: &[LayoutPosition],
    frame: u32,
) -> Vec<LayoutPosition> {
    let count = nodes.len();
    if count <= 1 {
        return current_positions.to_vec();
    }

    let positions = seeded_positions(nodes, current_positions);
    let iterations = if count <= 80 {
        12
    } else if count <= 180 {
        7
    } else {
        4
    };
    let alpha = (0.34 * 0.94_f64.powi(frame.min(48) as i32)).max(0.018);

    run_force_layout(nodes, edges, positions, None, iterations, alpha, 0.9, false)
}

pub(crate) fn max_layout_shift(before: &[LayoutPosition], after: &[LayoutPosition]) -> f64 {
    let after_by_id = after
        .iter()
        .map(|position| (position.id, position))
        .collect::<HashMap<_, _>>();

    before.iter().fold(0.0_f64, |max_shift, position| {
        let Some(updated) = after_by_id.get(&position.id) else {
            return max_shift;
        };
        let dx = updated.x - position.x;
        let dy = updated.y - position.y;
        max_shift.max((dx * dx + dy * dy).sqrt())
    })
}

fn seeded_positions(
    nodes: &[RelationNode],
    current_positions: &[LayoutPosition],
) -> Vec<LayoutPosition> {
    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let (x, y) = current_positions
                .iter()
                .find(|position| position.id == node.id)
                .map(|position| (position.x, position.y))
                .unwrap_or_else(|| initial_position(index));
            LayoutPosition { id: node.id, x, y }
        })
        .collect()
}

fn run_force_layout(
    nodes: &[RelationNode],
    edges: &[RelationEdge],
    positions: Vec<LayoutPosition>,
    fixed_position: Option<LayoutPosition>,
    iterations: usize,
    mut alpha: f64,
    alpha_decay: f64,
    fit: bool,
) -> Vec<LayoutPosition> {
    let count = positions.len();
    if count < 2 {
        return positions;
    }

    let index_by_id = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id, index))
        .collect::<HashMap<_, _>>();
    let mut degree = vec![0_usize; count];
    for edge in edges {
        let (Some(&source), Some(&target)) =
            (index_by_id.get(&edge.source), index_by_id.get(&edge.target))
        else {
            continue;
        };
        degree[source] += 1;
        degree[target] += 1;
    }

    let sim_edges = edges
        .iter()
        .filter_map(|edge| {
            let (&source, &target) = (
                index_by_id.get(&edge.source)?,
                index_by_id.get(&edge.target)?,
            );
            let weight = (edge.strength.max(1) as f64).ln_1p().min(3.2);
            let degree_scale = degree[source].min(degree[target]).max(1) as f64;
            Some(SimEdge {
                source,
                target,
                distance: (155.0 - 20.0 * weight).clamp(86.0, 155.0),
                strength: ((0.06 + 0.035 * weight) / degree_scale.sqrt()).clamp(0.018, 0.13),
            })
        })
        .collect::<Vec<_>>();

    let fixed = fixed_position.and_then(|position| {
        index_by_id
            .get(&position.id)
            .copied()
            .map(|index| (index, position))
    });
    let mut sim_nodes = positions
        .into_iter()
        .map(|position| SimNode {
            id: position.id,
            x: position.x,
            y: position.y,
            vx: 0.0,
            vy: 0.0,
        })
        .collect::<Vec<_>>();

    if let Some((index, position)) = fixed {
        sim_nodes[index].x = position.x;
        sim_nodes[index].y = position.y;
    }

    let charge_radius = if count <= 80 {
        470.0
    } else if count <= 180 {
        400.0
    } else {
        340.0
    };
    let charge_radius2 = charge_radius * charge_radius;
    let charge = if count <= 80 {
        330.0
    } else if count <= 180 {
        260.0
    } else {
        205.0
    };
    let center_strength = if fit { 0.018 } else { 0.026 };
    let velocity_decay = if fit { 0.58 } else { 0.52 };

    for _ in 0..iterations {
        apply_link_force(&mut sim_nodes, &sim_edges, alpha);
        apply_charge_force(&mut sim_nodes, charge, charge_radius2, alpha);
        apply_center_force(&mut sim_nodes, center_strength, alpha);
        integrate(&mut sim_nodes, fixed, velocity_decay);
        alpha *= alpha_decay;
    }

    let mut result = sim_nodes
        .into_iter()
        .map(|node| LayoutPosition {
            id: node.id,
            x: node.x,
            y: node.y,
        })
        .collect::<Vec<_>>();

    if fit {
        fit_layout(nodes, &mut result);
    }

    result
}

fn apply_link_force(nodes: &mut [SimNode], edges: &[SimEdge], alpha: f64) {
    for edge in edges {
        let dx = nodes[edge.target].x - nodes[edge.source].x;
        let dy = nodes[edge.target].y - nodes[edge.source].y;
        let distance = (dx * dx + dy * dy).sqrt().max(0.01);
        let force = (distance - edge.distance) / distance * edge.strength * alpha;
        let fx = dx * force;
        let fy = dy * force;

        nodes[edge.source].vx += fx;
        nodes[edge.source].vy += fy;
        nodes[edge.target].vx -= fx;
        nodes[edge.target].vy -= fy;
    }
}

fn apply_charge_force(nodes: &mut [SimNode], charge: f64, charge_radius2: f64, alpha: f64) {
    for a in 0..nodes.len() {
        for b in (a + 1)..nodes.len() {
            let mut dx = nodes[a].x - nodes[b].x;
            let mut dy = nodes[a].y - nodes[b].y;
            let mut distance2 = dx * dx + dy * dy;
            if distance2 < 0.01 {
                let (jx, jy) = pair_jitter(a, b);
                dx = jx;
                dy = jy;
                distance2 = dx * dx + dy * dy;
            }
            if distance2 > charge_radius2 {
                continue;
            }

            let force = (charge * alpha / distance2).min(0.075);
            let fx = dx * force;
            let fy = dy * force;
            nodes[a].vx += fx;
            nodes[a].vy += fy;
            nodes[b].vx -= fx;
            nodes[b].vy -= fy;
        }
    }
}

fn pair_jitter(a: usize, b: usize) -> (f64, f64) {
    let seed = ((a as u64 + 1) * 0x9e37_79b9) ^ ((b as u64 + 1) * 0x85eb_ca6b);
    let angle = (seed % 6283) as f64 / 1000.0;
    (angle.cos() * 0.1, angle.sin() * 0.1)
}

fn apply_center_force(nodes: &mut [SimNode], strength: f64, alpha: f64) {
    for node in nodes {
        node.vx += (GRAPH_CENTER_X - node.x) * strength * alpha;
        node.vy += (GRAPH_CENTER_Y - node.y) * strength * alpha;
    }
}

fn integrate(nodes: &mut [SimNode], fixed: Option<(usize, LayoutPosition)>, velocity_decay: f64) {
    for (index, node) in nodes.iter_mut().enumerate() {
        if let Some((fixed_index, position)) = fixed {
            if index == fixed_index {
                node.x = position.x;
                node.y = position.y;
                node.vx = 0.0;
                node.vy = 0.0;
                continue;
            }
        }

        node.vx *= velocity_decay;
        node.vy *= velocity_decay;
        node.x += node.vx;
        node.y += node.vy;
    }
}

fn fit_layout(nodes: &[RelationNode], positions: &mut [LayoutPosition]) {
    let Some(first) = positions.first().copied() else {
        return;
    };
    let first_extents = nodes
        .first()
        .map(relation_node_extents)
        .unwrap_or_else(|| relation_node_extents_for_label_width(0.0));
    let (mut min_x, mut max_x) = (first.x - first_extents.left, first.x + first_extents.right);
    let (mut min_y, mut max_y) = (first.y - first_extents.top, first.y + first_extents.bottom);
    for (node, position) in nodes.iter().zip(positions.iter()) {
        let extents = relation_node_extents(node);
        min_x = min_x.min(position.x - extents.left);
        max_x = max_x.max(position.x + extents.right);
        min_y = min_y.min(position.y - extents.top);
        max_y = max_y.max(position.y + extents.bottom);
    }

    let source_width = (max_x - min_x).max(1.0);
    let source_height = (max_y - min_y).max(1.0);
    let target_width = 980.0_f64;
    let target_height = 540.0_f64;
    let scale = (target_width / source_width)
        .min(target_height / source_height)
        .min(1.0);
    let offset_x = GRAPH_CENTER_X - ((min_x + max_x) * scale / 2.0);
    let offset_y = GRAPH_CENTER_Y - ((min_y + max_y) * scale / 2.0);

    for position in positions {
        position.x = position.x * scale + offset_x;
        position.y = position.y * scale + offset_y;
    }
}

fn relation_node_extents(node: &RelationNode) -> NodeExtents {
    relation_node_extents_for_label_width(relation_node_label_width(&node.name))
}

fn relation_node_text_width(text: &str) -> f64 {
    text.chars().map(relation_node_char_width).sum()
}

fn relation_node_char_width(ch: char) -> f64 {
    if ch.is_ascii() {
        NODE_LABEL_CHAR_WIDTH
    } else {
        12.0
    }
}

fn relation_node_extents_for_label_width(label_width: f64) -> NodeExtents {
    NodeExtents {
        left: NODE_HIT_RADIUS,
        right: NODE_LABEL_HIT_X + label_width,
        top: NODE_HIT_RADIUS,
        bottom: NODE_HIT_RADIUS,
    }
}
