use crate::Editor;
use document::NodeId;
impl Editor {
    /// Reorder/reparent with unchanged world geometry and one history transaction.
    pub fn drop_layer(&mut self, id: NodeId, target: NodeId, before: bool) -> bool {
        if id == target || self.is_locked(id) || self.is_locked(target) {
            return false;
        }
        let Some(absolute) = self.absolute_frame(id) else {
            return false;
        };
        if self.absolute_frame(target).is_none() {
            return false;
        }
        let previous = self.parent_of(id);
        let parent = self.parent_of(target);
        let mut ancestor = Some(target);
        while let Some(current) = ancestor {
            if current == id {
                return false;
            }
            ancestor = self.parent_of(current);
        }
        if self.document.mode == document::DocumentMode::Word
            && (previous.is_some() || parent.is_some())
        {
            return false;
        }
        let offset = parent
            .and_then(|p| self.absolute_frame(p))
            .unwrap_or_default();
        if previous == parent {
            let siblings = self.siblings_mut(parent);
            let a = siblings.iter().position(|n| n.id == id).unwrap();
            let b = siblings.iter().position(|n| n.id == target).unwrap();
            if (before && a + 1 == b) || (!before && a == b + 1) {
                return false;
            }
        }
        self.checkpoint();
        let siblings = self.siblings_mut(previous);
        let i = siblings.iter().position(|n| n.id == id).unwrap();
        let mut node = siblings.remove(i);
        node.frame.x = absolute.x - offset.x;
        node.frame.y = absolute.y - offset.y;
        let siblings = self.siblings_mut(parent);
        let i = siblings.iter().position(|n| n.id == target).unwrap() + usize::from(!before);
        siblings.insert(i, node);
        self.normalize_groups();
        self.select_only(Some(id));
        true
    }
}
#[cfg(test)]
mod tests {
    use crate::Editor;
    use document::{NodeKind, Rect};
    #[test]
    fn reorder_reparent_cycle_lock_and_undo() {
        let mut e = Editor::default();
        let a = e.insert(NodeKind::Shape, Rect::new(10., 20., 30., 40.));
        let b = e.insert(NodeKind::Shape, Rect::new(60., 20., 30., 40.));
        let frame = e.insert(NodeKind::Frame, Rect::new(100., 100., 120., 120.));
        e.select_only(Some(b));
        e.reparent_selected(Some(frame));
        let original = e.document.clone();
        let world = e.absolute_frame(a).unwrap();
        assert!(e.drop_layer(a, b, true));
        assert_eq!(e.parent_of(a), Some(frame));
        assert_eq!(e.absolute_frame(a), Some(world));
        assert!(!e.drop_layer(frame, b, false));
        e.undo();
        assert_eq!(e.document, original);
        e.toggle_lock(a);
        assert!(!e.drop_layer(a, b, true));
    }
    #[test]
    fn word_order_and_noops() {
        let mut e = Editor::default();
        e.document.mode = document::DocumentMode::Word;
        let a = e.insert(
            NodeKind::Text(text::RichText::new("first")),
            Rect::new(72., 72., 200., 40.),
        );
        let b = e.insert(
            NodeKind::Text(text::RichText::new("second")),
            Rect::new(72., 124., 200., 40.),
        );
        assert!(!e.drop_layer(a, b, true));
        assert!(e.drop_layer(a, b, false));
        assert_eq!(e.document.pages[0].nodes[1].id, a);
        e.undo();
        assert_eq!(e.document.pages[0].nodes[0].id, a);
    }
}
