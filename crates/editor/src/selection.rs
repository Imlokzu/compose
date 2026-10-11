use crate::Editor;
use document::{Appearance, Color, ImageSizing, Node, NodeId, NodeKind, Point, Rect};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug)]
pub enum Align {
    Left,
    CenterX,
    Right,
    Top,
    CenterY,
    Bottom,
}
#[derive(Clone, Copy, Debug)]
pub enum Distribute {
    Horizontal,
    Vertical,
}

pub fn union(frames: impl IntoIterator<Item = Rect>) -> Option<Rect> {
    let mut frames = frames.into_iter();
    let first = frames.next()?;
    let (mut x, mut y, mut right, mut bottom) = (
        first.x,
        first.y,
        first.x + first.width,
        first.y + first.height,
    );
    for r in frames {
        x = x.min(r.x);
        y = y.min(r.y);
        right = right.max(r.x + r.width);
        bottom = bottom.max(r.y + r.height);
    }
    Some(Rect::new(x, y, right - x, bottom - y))
}
fn intersects(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}
fn contains(a: Rect, b: Rect) -> bool {
    a.x <= b.x && a.y <= b.y && a.x + a.width >= b.x + b.width && a.y + a.height >= b.y + b.height
}
fn normalize(nodes: &mut Vec<Node>) {
    for n in nodes.iter_mut() {
        normalize(&mut n.children);
        if n.is_group
            && let Some(bounds) = union(n.children.iter().map(|c| c.frame))
        {
            n.frame.x += bounds.x;
            n.frame.y += bounds.y;
            n.frame.width = bounds.width;
            n.frame.height = bounds.height;
            for child in &mut n.children {
                child.frame.x -= bounds.x;
                child.frame.y -= bounds.y;
            }
        }
    }
    nodes.retain(|n| !n.is_group || !n.children.is_empty());
}

impl Editor {
    pub fn selected(&self) -> Option<NodeId> {
        self.selected
    }
    pub fn selection_ids(&self) -> &[NodeId] {
        &self.selection
    }
    pub fn selection_count(&self) -> usize {
        self.selection.len()
    }
    pub fn is_selected(&self, id: NodeId) -> bool {
        self.selection.contains(&id)
    }
    pub fn select_only(&mut self, id: Option<NodeId>) {
        self.set_selection(id.into_iter().collect());
    }
    pub fn set_selection(&mut self, ids: Vec<NodeId>) {
        let all: HashSet<_> = ids
            .iter()
            .copied()
            .filter(|id| self.absolute_frame(*id).is_some())
            .collect();
        let mut seen = HashSet::new();
        self.selection = ids
            .into_iter()
            .filter(|id| {
                if !all.contains(id) || !seen.insert(*id) {
                    return false;
                }
                let mut ancestor = self.parent_of(*id);
                while let Some(parent) = ancestor {
                    if all.contains(&parent) {
                        return false;
                    }
                    ancestor = self.parent_of(parent);
                }
                true
            })
            .collect();
        self.selected = self.selection.last().copied();
    }
    pub fn toggle_selected(&mut self, id: NodeId) {
        let mut ids = self.selection.clone();
        if ids.contains(&id) {
            ids.retain(|i| *i != id);
        } else {
            // Drilling into a selected ancestor replaces that ancestor, never moves both.
            ids.retain(|selected| {
                let mut parent = self.parent_of(id);
                while let Some(p) = parent {
                    if p == *selected {
                        return false;
                    }
                    parent = self.parent_of(p);
                }
                true
            });
            ids.push(id);
        }
        self.set_selection(ids);
    }
    pub fn selection_locked(&self) -> bool {
        self.selection.iter().any(|id| self.is_locked(*id))
    }
    pub fn selection_bounds(&self) -> Option<Rect> {
        union(
            self.selection
                .iter()
                .filter_map(|id| self.absolute_frame(*id)),
        )
    }
    pub fn selection_frames(&self) -> Vec<(NodeId, Rect)> {
        self.selection
            .iter()
            .filter_map(|id| Some((*id, self.absolute_frame(*id)?)))
            .collect()
    }
    pub fn select_all(&mut self) {
        let ids = self.document.pages[self.page]
            .nodes
            .iter()
            .filter(|n| n.visible && !n.locked)
            .map(|n| n.id)
            .collect();
        self.set_selection(ids);
    }
    pub fn marquee_ids(&self, rect: Rect) -> Vec<NodeId> {
        fn gather(nodes: &[Node], x: f32, y: f32, rect: Rect, out: &mut Vec<NodeId>) {
            for n in nodes {
                if !n.visible || n.locked {
                    continue;
                }
                let r = Rect::new(x + n.frame.x, y + n.frame.y, n.frame.width, n.frame.height);
                if contains(rect, r) || (n.children.is_empty() && intersects(rect, r)) {
                    out.push(n.id);
                } else {
                    gather(&n.children, r.x, r.y, rect, out);
                }
            }
        }
        let mut ids = Vec::new();
        gather(
            &self.document.pages[self.page].nodes,
            0.,
            0.,
            rect,
            &mut ids,
        );
        ids
    }
    pub fn normalize_groups(&mut self) {
        normalize(&mut self.document.pages[self.page].nodes);
        self.set_selection(self.selection.clone());
    }
    /// Apply world-space frames inside an already checkpointed gesture.
    /// Ancestor normalization prevents a selected subtree moving twice.
    pub fn apply_frames(&mut self, frames: &[(NodeId, Rect)]) -> bool {
        let Some(candidate) = self.transformed_document(frames) else {
            return false;
        };
        self.document = candidate;
        self.set_selection(self.selection.clone());
        true
    }
    fn commit_frames(&mut self, frames: &[(NodeId, Rect)]) -> bool {
        let Some(candidate) = self.transformed_document(frames) else {
            return false;
        };
        if candidate == self.document {
            return false;
        }
        self.checkpoint();
        self.document = candidate;
        self.set_selection(self.selection.clone());
        true
    }
    fn transformed_document(&self, frames: &[(NodeId, Rect)]) -> Option<document::Document> {
        if frames.is_empty()
            || frames.iter().any(|(id, r)| {
                !self.is_selected(*id)
                    || self.is_locked(*id)
                    || !r.valid()
                    || self.absolute_frame(*id).is_none()
            })
        {
            return None;
        }
        let locals: Vec<_> = frames
            .iter()
            .map(|(id, r)| {
                let parent = self
                    .parent_of(*id)
                    .and_then(|p| self.absolute_frame(p))
                    .unwrap_or_default();
                (
                    *id,
                    Rect::new(r.x - parent.x, r.y - parent.y, r.width, r.height),
                )
            })
            .collect();
        if locals.iter().any(|(_, r)| !r.valid()) {
            return None;
        }
        let mut candidate = self.document.clone();
        for (id, r) in locals {
            candidate.node_mut(id)?.frame = r;
        }
        normalize(&mut candidate.pages[self.page].nodes);
        candidate.validate().ok()?;
        Some(candidate)
    }
    pub fn translate_selection(&mut self, delta: Point) -> bool {
        if self.selection.is_empty()
            || self.selection_locked()
            || !delta.x.is_finite()
            || !delta.y.is_finite()
            || (delta.x == 0. && delta.y == 0.)
        {
            return false;
        }
        let frames: Vec<_> = self
            .selection_frames()
            .into_iter()
            .map(|(id, r)| {
                (
                    id,
                    Rect::new(r.x + delta.x, r.y + delta.y, r.width, r.height),
                )
            })
            .collect();
        if frames.iter().any(|(_, r)| !r.valid()) {
            return false;
        }
        self.commit_frames(&frames)
    }
    pub fn snap_bounds(&self) -> Vec<Rect> {
        fn gather(e: &Editor, nodes: &[Node], x: f32, y: f32, out: &mut Vec<Rect>) {
            for n in nodes {
                if !n.visible || e.is_selected(n.id) {
                    continue;
                }
                let r = Rect::new(x + n.frame.x, y + n.frame.y, n.frame.width, n.frame.height);
                let contains_selected = e.selection_ids().iter().any(|id| {
                    let mut parent = e.parent_of(*id);
                    while let Some(p) = parent {
                        if p == n.id {
                            return true;
                        }
                        parent = e.parent_of(p);
                    }
                    false
                });
                if !contains_selected {
                    out.push(r);
                }
                gather(e, &n.children, r.x, r.y, out);
            }
        }
        let mut out = Vec::new();
        gather(
            self,
            &self.document.pages[self.page].nodes,
            0.,
            0.,
            &mut out,
        );
        out
    }
    pub fn set_selection_locked(&mut self, locked: bool) {
        let mut ids = HashSet::new();
        for selected in &self.selection {
            let mut current = Some(*selected);
            while let Some(id) = current {
                if self.document.node(id).is_some_and(|n| n.locked != locked) {
                    ids.insert(id);
                }
                if locked {
                    break;
                }
                current = self.parent_of(id);
            }
        }
        if ids.is_empty() {
            return;
        }
        self.checkpoint();
        for id in ids {
            self.document.node_mut(id).unwrap().locked = locked;
        }
    }
    pub(crate) fn selected_ordered(&self) -> Vec<NodeId> {
        fn gather(nodes: &[Node], selected: &[NodeId], ids: &mut Vec<NodeId>) {
            for n in nodes {
                if selected.contains(&n.id) {
                    ids.push(n.id);
                } else {
                    gather(&n.children, selected, ids);
                }
            }
        }
        let mut ids = Vec::new();
        gather(
            &self.document.pages[self.page].nodes,
            &self.selection,
            &mut ids,
        );
        ids
    }
    fn candidate_siblings(
        doc: &mut document::Document,
        page: usize,
        parent: Option<NodeId>,
    ) -> &mut Vec<Node> {
        if let Some(parent) = parent {
            &mut doc.node_mut(parent).unwrap().children
        } else {
            &mut doc.pages[page].nodes
        }
    }
    pub(crate) fn duplicate_selection(&mut self) {
        if self.selection.is_empty() || self.selection_locked() {
            return;
        }
        let mut candidate = self.document.clone();
        let mut ids = Vec::new();
        fn reid(n: &mut Node, next: &mut u64) -> Option<()> {
            n.id = *next;
            *next = next.checked_add(1)?;
            for c in &mut n.children {
                reid(c, next)?;
            }
            Some(())
        }
        for id in self.selected_ordered() {
            let mut node = self.document.node(id).unwrap().clone();
            if reid(&mut node, &mut candidate.next_id).is_none() {
                return;
            }
            node.frame.x += 16.;
            node.frame.y += 16.;
            node.name.push_str(" copy");
            ids.push(node.id);
            Self::candidate_siblings(&mut candidate, self.page, self.parent_of(id)).push(node);
        }
        normalize(&mut candidate.pages[self.page].nodes);
        if candidate.validate().is_err() {
            return;
        }
        self.checkpoint();
        self.document = candidate;
        self.set_selection(ids);
    }
    pub(crate) fn reorder_selection(&mut self, forward: bool) {
        if self.selection.is_empty() || self.selection_locked() {
            return;
        }
        let mut candidate = self.document.clone();
        let parents: HashSet<_> = self
            .selection
            .iter()
            .map(|id| self.parent_of(*id))
            .collect();
        for parent in parents {
            let nodes = Self::candidate_siblings(&mut candidate, self.page, parent);
            if forward {
                for i in (0..nodes.len().saturating_sub(1)).rev() {
                    if self.is_selected(nodes[i].id) && !self.is_selected(nodes[i + 1].id) {
                        nodes.swap(i, i + 1);
                    }
                }
            } else {
                for i in 1..nodes.len() {
                    if self.is_selected(nodes[i].id) && !self.is_selected(nodes[i - 1].id) {
                        nodes.swap(i, i - 1);
                    }
                }
            }
        }
        if candidate != self.document {
            self.checkpoint();
            self.document = candidate;
        }
    }
    pub fn can_group(&self) -> bool {
        self.document.mode == document::DocumentMode::Compose
            && self.selection.len() > 1
            && !self.selection_locked()
            && self
                .selection
                .iter()
                .all(|id| self.parent_of(*id) == self.parent_of(self.selection[0]))
    }
    pub fn group_selected(&mut self) -> Option<NodeId> {
        if !self.can_group() {
            return None;
        }
        let parent = self.parent_of(self.selection[0]);
        let mut candidate = self.document.clone();
        let id = candidate.next_id;
        candidate.next_id = candidate.next_id.checked_add(1)?;
        let nodes = Self::candidate_siblings(&mut candidate, self.page, parent);
        let bound = union(
            nodes
                .iter()
                .filter(|n| self.is_selected(n.id))
                .map(|n| n.frame),
        )?;
        let front = nodes.iter().rposition(|n| self.is_selected(n.id))?;
        let insertion = nodes[..front]
            .iter()
            .filter(|n| !self.is_selected(n.id))
            .count();
        let mut children = Vec::new();
        nodes.retain(|n| {
            if self.is_selected(n.id) {
                let mut n = n.clone();
                n.frame.x -= bound.x;
                n.frame.y -= bound.y;
                children.push(n);
                false
            } else {
                true
            }
        });
        nodes.insert(
            insertion,
            Node {
                id,
                name: "Group".into(),
                frame: bound,
                opacity: 1.,
                fill: Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                },
                radius: 0.,
                shape: document::ShapeKind::Rectangle,
                line_flipped: false,
                appearance: Appearance::default(),
                image_sizing: ImageSizing::Scale,
                image_source: None,
                visible: true,
                locked: false,
                kind: NodeKind::Frame,
                is_group: true,
                children,
            },
        );
        if candidate.validate().is_err() {
            return None;
        }
        self.checkpoint();
        self.document = candidate;
        self.select_only(Some(id));
        Some(id)
    }
    pub fn can_ungroup(&self) -> bool {
        !self.selection.is_empty()
            && !self.selection_locked()
            && self.selection.iter().all(|id| {
                self.document.node(*id).is_some_and(|n| {
                    n.is_group
                        && n.fill.a == 0
                        && n.appearance.stroke_width == 0.
                        && !n.appearance.shadow
                })
            })
    }
    pub fn ungroup_selected(&mut self) -> bool {
        if !self.can_ungroup() {
            return false;
        }
        let mut candidate = self.document.clone();
        let mut selected = Vec::new();
        for id in self.selected_ordered() {
            let nodes = Self::candidate_siblings(&mut candidate, self.page, self.parent_of(id));
            let index = nodes.iter().position(|n| n.id == id).unwrap();
            let group = nodes.remove(index);
            let mut children = group.children;
            for child in &mut children {
                child.frame.x += group.frame.x;
                child.frame.y += group.frame.y;
                child.opacity *= group.opacity;
                child.visible &= group.visible;
                selected.push(child.id);
            }
            nodes.splice(index..index, children);
        }
        normalize(&mut candidate.pages[self.page].nodes);
        if candidate.validate().is_err() {
            return false;
        }
        self.checkpoint();
        self.document = candidate;
        self.set_selection(selected);
        true
    }
    pub fn align_selected(&mut self, align: Align) -> bool {
        if self.selection.len() < 2 || self.selection_locked() {
            return false;
        }
        let bound = self.selection_bounds().unwrap();
        let frames: Vec<_> = self
            .selection_frames()
            .into_iter()
            .map(|(id, mut r)| {
                match align {
                    Align::Left => r.x = bound.x,
                    Align::CenterX => r.x = bound.x + (bound.width - r.width) / 2.,
                    Align::Right => r.x = bound.x + bound.width - r.width,
                    Align::Top => r.y = bound.y,
                    Align::CenterY => r.y = bound.y + (bound.height - r.height) / 2.,
                    Align::Bottom => r.y = bound.y + bound.height - r.height,
                }
                (id, r)
            })
            .collect();
        if frames == self.selection_frames() {
            return false;
        }
        self.commit_frames(&frames)
    }
    pub fn distribute_selected(&mut self, direction: Distribute) -> bool {
        if self.selection.len() < 3 || self.selection_locked() {
            return false;
        }
        let horizontal = matches!(direction, Distribute::Horizontal);
        let mut frames = self.selection_frames();
        frames.sort_by(|a, b| {
            if horizontal {
                a.1.x.total_cmp(&b.1.x)
            } else {
                a.1.y.total_cmp(&b.1.y)
            }
        });
        let start = if horizontal {
            frames[0].1.x
        } else {
            frames[0].1.y
        };
        let last = frames.last().unwrap().1;
        let end = if horizontal {
            last.x + last.width
        } else {
            last.y + last.height
        };
        let sum: f32 = frames
            .iter()
            .map(|(_, r)| if horizontal { r.width } else { r.height })
            .sum();
        let gap = (end - start - sum) / (frames.len() - 1) as f32;
        let mut position = start;
        let mut changed = false;
        for (_, r) in &mut frames {
            if horizontal {
                changed |= (r.x - position).abs() > 0.001;
                r.x = position;
                position += r.width + gap;
            } else {
                changed |= (r.y - position).abs() > 0.001;
                r.y = position;
                position += r.height + gap;
            }
        }
        if !changed {
            return false;
        }
        self.commit_frames(&frames)
    }
}
