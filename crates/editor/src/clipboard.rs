use crate::Editor;
use document::{Document, Node, NodeId, NodeKind, Point};
use std::{collections::BTreeMap, sync::Arc};

/// A detached subtree and its original image bytes. UI/OS clipboard state lives outside the engine.
#[derive(Clone)]
pub struct ObjectClipboard {
    pub nodes: Vec<Node>,
    pub assets: BTreeMap<String, Arc<Vec<u8>>>,
}

impl Editor {
    pub fn copy_selected(&self) -> Option<ObjectClipboard> {
        let mut nodes = Vec::new();
        for id in self.selected_ordered() {
            let mut node = self.document.node(id)?.clone();
            node.frame = self.absolute_frame(id)?;
            let mut parent = self.parent_of(id);
            while let Some(id) = parent {
                node.opacity *= self.document.node(id)?.opacity;
                parent = self.parent_of(id);
            }
            nodes.push(node);
        }
        if nodes.is_empty() {
            return None;
        }
        let mut assets = BTreeMap::new();
        fn gather(
            n: &Node,
            doc: &Document,
            assets: &mut BTreeMap<String, Arc<Vec<u8>>>,
        ) -> Option<()> {
            if let NodeKind::Image { asset } = &n.kind {
                assets.insert(asset.clone(), doc.assets.get(asset)?.clone());
            }
            for child in &n.children {
                gather(child, doc, assets)?;
            }
            Some(())
        }
        for node in &nodes {
            gather(node, &self.document, &mut assets)?;
        }
        Some(ObjectClipboard { nodes, assets })
    }

    /// Paste at page level, retaining the copied root's world position plus an explicit offset.
    /// Build and validate a candidate before touching history or the live document.
    pub fn paste_object(
        &mut self,
        clip: &ObjectClipboard,
        offset: Point,
    ) -> Result<NodeId, String> {
        let mut doc = self.document.clone();
        if clip.nodes.is_empty() {
            return Err("No layers to paste".into());
        }
        let mut nodes = clip.nodes.clone();
        fn reid(n: &mut Node, next: &mut u64) -> Result<(), String> {
            n.id = *next;
            *next = next.checked_add(1).ok_or("Object IDs exhausted")?;
            for child in &mut n.children {
                reid(child, next)?;
            }
            Ok(())
        }
        for node in &mut nodes {
            reid(node, &mut doc.next_id)?;
            node.frame.x += offset.x;
            node.frame.y += offset.y;
            node.locked = false;
            node.visible = true;
        }
        for (key, bytes) in &clip.assets {
            if bytes.len() > 64 * 1024 * 1024 {
                return Err("Image exceeds 64 MB".into());
            }
            if doc
                .assets
                .get(key)
                .is_some_and(|existing| existing != bytes)
            {
                return Err("Conflicting image identifier".into());
            }
            doc.assets.insert(key.clone(), bytes.clone());
        }
        if doc.assets.values().map(|b| b.len() as u64).sum::<u64>() > 256 * 1024 * 1024 {
            return Err("Images exceed the 256 MB document limit".into());
        }
        let ids: Vec<_> = nodes.iter().map(|node| node.id).collect();
        let id = *ids.last().unwrap();
        doc.pages
            .get_mut(self.page)
            .ok_or("Missing page")?
            .nodes
            .extend(nodes);
        doc.validate()?;
        self.checkpoint();
        self.document = doc;
        self.set_selection(ids);
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use document::Rect;

    #[test]
    fn nested_image_pastes_across_documents_with_original_bytes_and_history() {
        let mut source = Editor::default();
        let frame = source.insert(NodeKind::Frame, Rect::new(100., 80., 300., 200.));
        source
            .document
            .assets
            .insert("photo".into(), Arc::new(vec![1, 2, 3]));
        let image = source.insert(
            NodeKind::Image {
                asset: "photo".into(),
            },
            Rect::new(120., 110., 40., 20.),
        );
        source.selected_node_mut().unwrap().radius = 8.;
        assert!(source.reparent_selected(Some(frame)));
        // Copying a nested child must retain its absolute coordinates.
        let child_clip = source.copy_selected().unwrap();
        assert_eq!(child_clip.nodes[0].frame.x, 120.);
        source.select_only(Some(frame));
        let clip = source.copy_selected().unwrap();
        let mut target = Editor::default();
        target.insert(NodeKind::Shape, Rect::new(0., 0., 10., 10.));
        let before = target.document.clone();
        let pasted = target
            .paste_object(&clip, Point { x: 16., y: 16. })
            .unwrap();
        let node = target.document.node(pasted).unwrap();
        assert_eq!(node.frame.x, 116.);
        assert_eq!(node.children[0].radius, 8.);
        assert_ne!(node.children[0].id, image);
        assert!(Arc::ptr_eq(
            &target.document.assets["photo"],
            &clip.assets["photo"]
        ));
        let after = target.document.clone();
        assert!(target.undo());
        assert_eq!(target.document, before);
        assert!(target.redo());
        assert_eq!(target.document, after);
        target
            .paste_object(&clip, Point { x: 32., y: 32. })
            .unwrap();
        assert!(target.document.validate().is_ok());
    }

    #[test]
    fn invalid_clipboard_is_atomic_and_locked_copy_is_editable() {
        let mut e = Editor::default();
        e.insert(NodeKind::Shape, Rect::new(20., 30., 100., 80.));
        e.selected_node_mut().unwrap().locked = true;
        let mut clip = e.copy_selected().unwrap();
        let mut target = Editor::default();
        clip.nodes[0].frame.width = f32::NAN;
        assert!(target.paste_object(&clip, Point::default()).is_err());
        assert_eq!(target.document, Document::default());
        assert!(!target.can_undo());
        clip.nodes[0].frame.width = 100.;
        target.paste_object(&clip, Point::default()).unwrap();
        assert!(!target.selected_node().unwrap().locked);
    }
}
