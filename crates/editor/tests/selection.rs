use document::{NodeKind, Point, Rect};
use editor::{Align, Distribute, Editor};
use std::sync::Arc;
fn shape(e: &mut Editor, x: f32, y: f32, w: f32, h: f32) -> u64 {
    e.insert(NodeKind::Shape, Rect::new(x, y, w, h))
}

#[test]
fn ancestor_selection_moves_once_and_is_restored_by_undo() {
    let mut e = Editor::default();
    let frame = e.insert(NodeKind::Frame, Rect::new(100., 80., 200., 180.));
    let child = shape(&mut e, 140., 120., 40., 30.);
    assert!(e.reparent_selected(Some(frame)));
    e.set_selection(vec![frame, child, frame]);
    assert_eq!(e.selection_ids(), [frame]);
    let before = e.absolute_frame(child).unwrap();
    assert!(e.translate_selection(Point { x: 20., y: 30. }));
    let after = e.absolute_frame(child).unwrap();
    assert_eq!((after.x - before.x, after.y - before.y), (20., 30.));
    assert!(e.undo());
    assert_eq!(e.absolute_frame(child).unwrap(), before);
    assert_eq!(e.selection_ids(), [frame]);
    e.toggle_selected(child);
    assert_eq!(e.selection_ids(), [child]);
}
#[test]
fn marquee_skips_hidden_locked_and_uncontained_parent_frames() {
    let mut e = Editor::default();
    let parent = e.insert(NodeKind::Frame, Rect::new(0., 0., 500., 600.));
    let a = shape(&mut e, 40., 40., 50., 50.);
    assert!(e.reparent_selected(Some(parent)));
    let b = shape(&mut e, 120., 40., 50., 50.);
    assert!(e.reparent_selected(Some(parent)));
    let hidden = shape(&mut e, 40., 40., 50., 50.);
    e.document.node_mut(hidden).unwrap().visible = false;
    let locked = shape(&mut e, 40., 40., 50., 50.);
    e.document.node_mut(locked).unwrap().locked = true;
    assert_eq!(e.marquee_ids(Rect::new(30., 30., 150., 70.)), [a, b]);
    assert_eq!(e.marquee_ids(Rect::new(-1., -1., 502., 602.)), [parent]);
    e.document.node_mut(parent).unwrap().locked = true;
    assert!(e.marquee_ids(Rect::new(30., 30., 150., 70.)).is_empty());
}
#[test]
fn grouping_nested_siblings_preserves_world_positions_ids_opacity_and_roundtrip() {
    let mut e = Editor::default();
    let parent = e.insert(NodeKind::Frame, Rect::new(90., 70., 360., 400.));
    let a = shape(&mut e, 120., 110., 100., 60.);
    assert!(e.reparent_selected(Some(parent)));
    let b = shape(&mut e, 270., 150., 80., 100.);
    assert!(e.reparent_selected(Some(parent)));
    let frames = [e.absolute_frame(a).unwrap(), e.absolute_frame(b).unwrap()];
    e.set_selection(vec![a, b]);
    let before = e.document.clone();
    let group = e.group_selected().unwrap();
    assert!(e.document.node(group).unwrap().is_group);
    assert_eq!(e.absolute_frame(a).unwrap(), frames[0]);
    assert_eq!(e.absolute_frame(b).unwrap(), frames[1]);
    e.document.node_mut(group).unwrap().opacity = 0.5;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("group.compose");
    file_format::save(&path, &e.document).unwrap();
    assert_eq!(file_format::load(&path).unwrap(), e.document);
    assert!(e.ungroup_selected());
    assert_eq!(e.absolute_frame(a).unwrap(), frames[0]);
    assert_eq!(e.absolute_frame(b).unwrap(), frames[1]);
    assert_eq!(e.document.node(a).unwrap().opacity, 0.5);
    assert_eq!(e.selection_count(), 2);
    assert!(e.document.validate().is_ok());
    e.undo();
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.selection_ids(), [a, b]);
}
#[test]
fn group_bounds_follow_a_moved_child_without_drifting_other_children() {
    let mut e = Editor::default();
    let a = shape(&mut e, 40., 50., 80., 70.);
    let b = shape(&mut e, 150., 60., 90., 70.);
    e.set_selection(vec![a, b]);
    let group = e.group_selected().unwrap();
    let stable = e.absolute_frame(b).unwrap();
    e.select_only(Some(a));
    let original = e.absolute_frame(a).unwrap();
    for amount in [10., 30., 70.] {
        assert!(e.apply_frames(&[(
            a,
            Rect::new(
                original.x - amount,
                original.y - amount,
                original.width,
                original.height
            )
        )]));
        assert_eq!(e.absolute_frame(b).unwrap(), stable);
        assert_eq!(e.absolute_frame(a).unwrap().x, original.x - amount);
    }
    let bounds = e.absolute_frame(group).unwrap();
    assert_eq!((bounds.x, bounds.y), (-30., -20.));
    assert!(e.document.validate().is_ok());
    e.delete_selected();
    e.select_only(Some(b));
    e.delete_selected();
    assert!(e.document.node(group).is_none());
}
#[test]
fn multiple_clipboard_duplicate_and_delete_are_single_transactions() {
    let mut e = Editor::default();
    let a = shape(&mut e, 40., 80., 100., 60.);
    e.document
        .assets
        .insert("photo".into(), Arc::new(vec![1, 2, 3, 4]));
    let b = e.insert(
        NodeKind::Image {
            asset: "photo".into(),
        },
        Rect::new(190., 80., 100., 60.),
    );
    e.set_selection(vec![b, a]);
    let clip = e.copy_selected().unwrap();
    assert_eq!(clip.nodes.iter().map(|n| n.id).collect::<Vec<_>>(), [a, b]);
    let before = e.document.clone();
    e.duplicate_selected();
    assert_eq!(e.selection_count(), 2);
    assert_eq!(e.document.pages[0].nodes.len(), 4);
    assert!(e.document.validate().is_ok());
    e.undo();
    assert_eq!(e.document, before);
    let mut target = Editor::default();
    target
        .paste_object(&clip, Point { x: 16., y: 16. })
        .unwrap();
    assert_eq!(target.selection_count(), 2);
    assert!(Arc::ptr_eq(
        &target.document.assets["photo"],
        &clip.assets["photo"]
    ));
    target.delete_selected();
    assert!(target.document.pages[0].nodes.is_empty());
    target.undo();
    assert_eq!(target.selection_count(), 2);
    target.undo();
    assert!(target.document.pages[0].nodes.is_empty());
}
#[test]
fn align_and_distribute_preserve_outer_anchors_and_restore_selection() {
    let mut e = Editor::default();
    let a = shape(&mut e, 40., 100., 40., 30.);
    let b = shape(&mut e, 130., 160., 60., 50.);
    let c = shape(&mut e, 320., 210., 80., 70.);
    e.set_selection(vec![a, b, c]);
    let before = e.document.clone();
    assert!(e.distribute_selected(Distribute::Horizontal));
    assert_eq!(e.absolute_frame(a).unwrap().x, 40.);
    assert_eq!(e.absolute_frame(c).unwrap().x, 320.);
    let middle = e.absolute_frame(b).unwrap();
    assert_eq!(middle.x, 170.);
    assert!(e.align_selected(Align::Top));
    assert!(e.selection_frames().iter().all(|(_, r)| r.y == 100.));
    e.undo();
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.selection_ids(), [a, b, c]);
}
#[test]
fn inherited_locks_block_batch_mutations_but_allow_copy_and_explicit_unlock() {
    let mut e = Editor::default();
    let parent = e.insert(NodeKind::Frame, Rect::new(0., 0., 300., 300.));
    let a = shape(&mut e, 40., 60., 50., 50.);
    assert!(e.reparent_selected(Some(parent)));
    let b = shape(&mut e, 100., 60., 50., 50.);
    assert!(e.reparent_selected(Some(parent)));
    e.document.node_mut(parent).unwrap().locked = true;
    e.set_selection(vec![a, b]);
    let before = e.document.clone();
    assert!(e.selection_locked());
    assert!(e.copy_selected().is_some());
    assert!(e.group_selected().is_none());
    assert!(!e.align_selected(Align::Top));
    assert!(!e.translate_selection(Point { x: 2., y: 2. }));
    e.delete_selected();
    e.duplicate_selected();
    assert_eq!(e.document, before);
    e.set_selection_locked(false);
    assert!(!e.selection_locked());
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn noncontiguous_grouping_keeps_selected_order_at_the_frontmost_selected_layer() {
    let mut e = Editor::default();
    let a = shape(&mut e, 0., 0., 40., 40.);
    let b = shape(&mut e, 50., 0., 40., 40.);
    let c = shape(&mut e, 100., 0., 40., 40.);
    let d = shape(&mut e, 150., 0., 40., 40.);
    e.set_selection(vec![c, a]);
    let group = e.group_selected().unwrap();
    assert_eq!(
        e.document.pages[0]
            .nodes
            .iter()
            .map(|n| n.id)
            .collect::<Vec<_>>(),
        [b, group, d]
    );
    assert_eq!(
        e.document
            .node(group)
            .unwrap()
            .children
            .iter()
            .map(|n| n.id)
            .collect::<Vec<_>>(),
        [a, c]
    );
    assert_eq!(e.hit_test(Point { x: 110., y: 10. }), Some(group));
    assert_eq!(e.hit_test_deep(Point { x: 110., y: 10. }), Some(c));
}
#[test]
fn legacy_nodes_default_to_frames_and_invalid_transforms_are_atomic() {
    let mut e = Editor::default();
    let a = shape(&mut e, 10., 20., 50., 60.);
    let b = shape(&mut e, 100., 20., 50., 60.);
    e.set_selection(vec![a, b]);
    let mut value = serde_json::to_value(&e.document).unwrap();
    for node in value["pages"][0]["nodes"].as_array_mut().unwrap() {
        node.as_object_mut().unwrap().remove("is_group");
    }
    let old: document::Document = serde_json::from_value(value).unwrap();
    assert!(old.pages[0].nodes.iter().all(|n| !n.is_group));
    let before = e.document.clone();
    assert!(!e.translate_selection(Point {
        x: f32::INFINITY,
        y: 0.
    }));
    assert!(!e.apply_frames(&[
        (a, Rect::new(30., 20., 50., 60.)),
        (b, Rect::new(f32::NAN, 20., 50., 60.))
    ]));
    assert_eq!(e.document, before);
}
