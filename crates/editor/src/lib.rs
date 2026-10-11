mod clipboard;
mod components;
mod layer_drop;
mod selection;
pub use clipboard::ObjectClipboard;
pub use components::Component;
use document::{Document, Node, NodeId, NodeKind, Point, Rect};
pub use selection::{Align, Distribute};
#[derive(Clone)]
struct Snapshot {
    document: Document,
    selected: Option<NodeId>,
    selection: Vec<NodeId>,
    page: usize,
}
pub struct Editor {
    pub document: Document,
    pub page: usize,
    selected: Option<NodeId>,
    selection: Vec<NodeId>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    batch: bool,
}
impl Default for Editor {
    fn default() -> Self {
        Self::new(Document::default())
    }
}
impl Editor {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            page: 0,
            selected: None,
            selection: vec![],
            undo: vec![],
            redo: vec![],
            batch: false,
        }
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            document: self.document.clone(),
            selected: self.selected,
            selection: self.selection.clone(),
            page: self.page,
        }
    }
    fn restore(&mut self, s: Snapshot) {
        self.document = s.document;
        self.selected = s.selected;
        self.selection = s.selection;
        self.page = s.page;
    }
    pub fn checkpoint(&mut self) {
        if self.batch {
            return;
        }
        let s = self.snapshot();
        if self.undo.last().is_none_or(|last| {
            last.document != s.document || last.selection != s.selection || last.page != s.page
        }) {
            self.undo.push(s);
            if self.undo.len() > 150 {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
    }
    pub fn undo(&mut self) -> bool {
        if let Some(s) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(s);
            true
        } else {
            false
        }
    }
    pub fn begin_batch(&mut self) {
        self.checkpoint();
        self.batch = true;
    }
    pub fn end_batch(&mut self) {
        self.batch = false;
    }
    pub fn redo(&mut self) -> bool {
        if let Some(s) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(s);
            true
        } else {
            false
        }
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn selected_node(&self) -> Option<&Node> {
        if self.selection.len() != 1 {
            return None;
        }
        self.document.node(self.selected?)
    }
    pub fn selected_node_mut(&mut self) -> Option<&mut Node> {
        if self.selection.len() != 1 {
            return None;
        }
        self.document.node_mut(self.selected?)
    }
    pub fn insert(&mut self, kind: NodeKind, frame: Rect) -> NodeId {
        self.checkpoint();
        let id = self.document.insert(self.page, kind, frame);
        self.select_only(Some(id));
        id
    }
    pub fn delete_selected(&mut self) {
        if self.selection.is_empty() || self.selection_locked() {
            return;
        }
        self.checkpoint();
        for id in self.selection.clone() {
            self.document.remove(id);
        }
        self.normalize_groups();
        self.select_only(None);
    }
    pub fn duplicate_selected(&mut self) {
        self.duplicate_selection();
    }
    pub fn parent_of(&self, id: NodeId) -> Option<NodeId> {
        fn find(nodes: &[Node], id: NodeId, parent: Option<NodeId>) -> Option<Option<NodeId>> {
            for n in nodes {
                if n.id == id {
                    return Some(parent);
                }
                if let Some(p) = find(&n.children, id, Some(n.id)) {
                    return Some(p);
                }
            }
            None
        }
        find(&self.document.pages[self.page].nodes, id, None).flatten()
    }
    pub fn absolute_frame(&self, id: NodeId) -> Option<Rect> {
        fn find(nodes: &[Node], id: NodeId, offset: Point) -> Option<Rect> {
            for n in nodes {
                let r = Rect::new(
                    offset.x + n.frame.x,
                    offset.y + n.frame.y,
                    n.frame.width,
                    n.frame.height,
                );
                if n.id == id {
                    return Some(r);
                }
                if let Some(r) = find(&n.children, id, Point { x: r.x, y: r.y }) {
                    return Some(r);
                }
            }
            None
        }
        find(&self.document.pages[self.page].nodes, id, Point::default())
    }
    fn siblings_mut(&mut self, parent: Option<NodeId>) -> &mut Vec<Node> {
        if let Some(id) = parent {
            &mut self
                .document
                .node_mut(id)
                .expect("Existing parent")
                .children
        } else {
            &mut self.document.pages[self.page].nodes
        }
    }
    pub fn reorder_selected(&mut self, forward: bool) {
        self.reorder_selection(forward);
    }
    /// Reparent without changing world-space geometry. Cycles are rejected.
    pub fn reparent_selected(&mut self, parent: Option<NodeId>) -> bool {
        if self.document.mode == document::DocumentMode::Word {
            return false;
        }
        if self.selection.len() != 1 || self.selection_locked() {
            return false;
        }
        let Some(id) = self.selected else {
            return false;
        };
        let previous = self.parent_of(id);
        if previous == parent {
            return false;
        }
        if let Some(target) = parent {
            if self.absolute_frame(target).is_none()
                || self.is_locked(target)
                || self
                    .document
                    .node(target)
                    .is_none_or(|n| !matches!(n.kind, NodeKind::Frame))
            {
                return false;
            }
            let mut ancestor = Some(target);
            while let Some(a) = ancestor {
                if a == id {
                    return false;
                }
                ancestor = self.parent_of(a);
            }
        }
        let Some(absolute) = self.absolute_frame(id) else {
            return false;
        };
        let offset = parent
            .and_then(|p| self.absolute_frame(p))
            .unwrap_or_default();
        self.checkpoint();
        let nodes = self.siblings_mut(previous);
        let i = nodes.iter().position(|n| n.id == id).unwrap();
        let mut node = nodes.remove(i);
        node.frame.x = absolute.x - offset.x;
        node.frame.y = absolute.y - offset.y;
        self.siblings_mut(parent).push(node);
        self.normalize_groups();
        true
    }
    pub fn hit_test(&self, p: Point) -> Option<NodeId> {
        self.hit(p, false, false)
    }
    pub fn hit_test_deep(&self, p: Point) -> Option<NodeId> {
        self.hit(p, true, false)
    }
    /// Context menus can inspect/unlock visible locked objects. Mutation guards still apply.
    pub fn hit_test_context(&self, p: Point, deep: bool) -> Option<NodeId> {
        self.hit(p, deep, true)
    }
    fn hit(&self, p: Point, deep: bool, include_locked: bool) -> Option<NodeId> {
        fn hit(nodes: &[Node], p: Point, deep: bool, include_locked: bool) -> Option<NodeId> {
            for n in nodes.iter().rev() {
                if !n.visible || (n.locked && !include_locked) {
                    continue;
                }
                let local = Point {
                    x: p.x - n.frame.x,
                    y: p.y - n.frame.y,
                };
                if n.is_group && !deep && n.frame.contains(p) {
                    return Some(n.id);
                }
                if let Some(id) = hit(&n.children, local, deep, include_locked) {
                    return Some(id);
                }
                let inside = if matches!(n.kind, NodeKind::Shape) {
                    n.shape.contains(
                        n.frame,
                        n.radius,
                        n.line_flipped,
                        n.appearance.stroke_width,
                        p,
                    )
                } else {
                    n.frame.contains(p)
                };
                if inside {
                    return Some(n.id);
                }
            }
            None
        }
        hit(
            &self.document.pages[self.page].nodes,
            p,
            deep,
            include_locked,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_hits_locked_visible_objects_without_enabling_mutations() {
        let mut editor = Editor::default();
        let id = editor.insert(NodeKind::Shape, Rect::new(10., 10., 80., 80.));
        editor.toggle_lock(id);
        let p = Point { x: 40., y: 40. };
        assert_eq!(editor.hit_test(p), None);
        assert_eq!(editor.hit_test_context(p, false), Some(id));
        editor.delete_selected();
        assert!(editor.document.node(id).is_some());
        editor.toggle_visibility(id);
        assert_eq!(editor.hit_test_context(p, true), None);
    }
    #[test]
    fn edits_and_layer_order_undo() {
        let mut e = Editor::default();
        let a = e.insert(NodeKind::Shape, Rect::new(0., 0., 100., 100.));
        let b = e.insert(NodeKind::Shape, Rect::new(0., 0., 100., 100.));
        assert_eq!(e.hit_test(Point { x: 10., y: 10. }), Some(b));
        e.reorder_selected(false);
        assert_eq!(e.hit_test(Point { x: 10., y: 10. }), Some(a));
        e.undo();
        assert_eq!(e.hit_test(Point { x: 10., y: 10. }), Some(b));
        e.checkpoint();
        e.selected_node_mut().unwrap().frame.x = 200.;
        e.undo();
        assert_eq!(e.selected_node().unwrap().frame.x, 0.);
        e.redo();
        assert_eq!(e.selected_node().unwrap().frame.x, 200.);
    }
    #[test]
    fn duplicate_ids_and_deletion() {
        let mut e = Editor::default();
        e.insert(NodeKind::Shape, Rect::new(1., 2., 30., 40.));
        e.duplicate_selected();
        assert!(e.document.validate().is_ok());
        assert_eq!(e.document.pages[0].nodes.len(), 2);
        e.delete_selected();
        e.undo();
        assert_eq!(e.document.pages[0].nodes.len(), 2);
    }
}

#[cfg(test)]
mod hierarchy_tests {
    use super::*;
    #[test]
    fn nesting_preserves_position_and_undo() {
        let mut e = Editor::default();
        let frame = e.insert(NodeKind::Frame, Rect::new(100., 80., 300., 200.));
        let child = e.insert(NodeKind::Shape, Rect::new(140., 120., 40., 40.));
        assert!(e.reparent_selected(Some(frame)));
        assert_eq!(e.selected_node().unwrap().frame.x, 40.);
        assert_eq!(e.absolute_frame(child).unwrap().x, 140.);
        assert_eq!(e.hit_test(Point { x: 150., y: 130. }), Some(child));
        e.duplicate_selected();
        assert_eq!(e.document.node(frame).unwrap().children.len(), 2);
        e.undo();
        e.undo();
        assert_eq!(e.parent_of(child), None);
    }
    #[test]
    fn parent_cycles_rejected() {
        let mut e = Editor::default();
        let a = e.insert(NodeKind::Frame, Rect::new(0., 0., 300., 200.));
        let b = e.insert(NodeKind::Frame, Rect::new(20., 20., 100., 100.));
        assert!(e.reparent_selected(Some(a)));
        e.select_only(Some(a));
        assert!(!e.reparent_selected(Some(b)));
        assert!(e.document.validate().is_ok());
    }
}
