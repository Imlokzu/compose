mod actions_smoke;
mod canvas;
mod clipboard;
mod context_menu;
mod creation;
mod creation_smoke;
mod cutout;
mod design_actions;
mod design_panels;
mod document_mode;
mod document_mode_smoke;
mod drag_drop;
mod image_picker;
mod image_picker_smoke;
mod image_picker_ui;
mod input;
mod links;
mod output;
mod output_smoke;
mod panels;
mod polish_smoke;
mod preset_smoke;
mod printing;
mod properties;
mod refinement_smoke;
mod selection;
mod selection_smoke;
mod shapes;
mod shapes_smoke;
mod shared_service_smoke;
mod smoke;
mod text_menu;
mod text_menu_smoke;
mod workflow_smoke;
mod workspace;
mod zoom;
mod zoom_smoke;
use gpui::*;
use workspace::Workspace;
actions!(
    compose,
    [
        NewDocument,
        NewWordDocument,
        OpenDocument,
        SaveDocument,
        SaveAs,
        ExportMenu,
        ExportCompose,
        ExportPng,
        ExportWord,
        PrintDocument,
        GroupSelection,
        UngroupSelection,
        UndoEdit,
        RedoEdit,
        CopyText,
        CutText,
        PasteText,
        SelectText,
        QuitCompose
    ]
);
fn main() {
    if std::env::args().any(|a| a == "--smoke-test") {
        if let Err(e) = smoke::run() {
            eprintln!("Smoke test failed: {e:#}");
            std::process::exit(1)
        }
        return;
    }
    Application::new()
        .with_assets(panels::Assets)
        .run(|cx: &mut App| {
            cx.set_menus(vec![
                Menu {
                    name: "Compose".into(),
                    items: vec![MenuItem::action("Quit Compose", QuitCompose)],
                },
                Menu {
                    name: "File".into(),
                    items: vec![
                        MenuItem::action("New document…", NewDocument),
                        MenuItem::action("New Word-compatible document…", NewWordDocument),
                        MenuItem::action("Open…", OpenDocument),
                        MenuItem::separator(),
                        MenuItem::action("Save", SaveDocument),
                        MenuItem::action("Save As…", SaveAs),
                        MenuItem::separator(),
                        MenuItem::action("Export Compose file…", ExportCompose),
                        MenuItem::action("Export PNG…", ExportPng),
                        MenuItem::action("Export Word (.docx)…", ExportWord),
                        MenuItem::action("Print…", PrintDocument),
                    ],
                },
                Menu {
                    name: "Edit".into(),
                    items: vec![
                        MenuItem::action("Undo", UndoEdit),
                        MenuItem::action("Redo", RedoEdit),
                        MenuItem::separator(),
                        MenuItem::action("Cut", CutText),
                        MenuItem::action("Copy", CopyText),
                        MenuItem::action("Paste", PasteText),
                        MenuItem::action("Select All", SelectText),
                        MenuItem::separator(),
                        MenuItem::action("Group selection", GroupSelection),
                        MenuItem::action("Ungroup selection", UngroupSelection),
                    ],
                },
            ]);
            cx.bind_keys([
                KeyBinding::new("cmd-n", NewDocument, None),
                KeyBinding::new("cmd-o", OpenDocument, None),
                KeyBinding::new("cmd-s", SaveDocument, None),
                KeyBinding::new("cmd-shift-s", SaveAs, None),
                KeyBinding::new("cmd-shift-e", ExportMenu, None),
                KeyBinding::new("cmd-p", PrintDocument, None),
                KeyBinding::new("cmd-g", GroupSelection, None),
                KeyBinding::new("cmd-shift-g", UngroupSelection, None),
                KeyBinding::new("cmd-z", UndoEdit, None),
                KeyBinding::new("cmd-shift-z", RedoEdit, None),
                KeyBinding::new("cmd-c", CopyText, None),
                KeyBinding::new("cmd-x", CutText, None),
                KeyBinding::new("cmd-v", PasteText, None),
                KeyBinding::new("cmd-a", SelectText, None),
                KeyBinding::new("cmd-q", QuitCompose, None),
            ]);
            let dimensions = if std::env::args().any(|a| a == "--compact") {
                size(px(1000.), px(680.))
            } else {
                size(px(1360.), px(900.))
            };
            let bounds = Bounds::centered(None, dimensions, cx);
            let window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        window_min_size: Some(size(px(1000.), px(680.))),
                        titlebar: Some(TitlebarOptions {
                            title: Some("Compose".into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    |window, cx| cx.new(|cx| Workspace::new(window, cx)),
                )
                .expect("Could not open Compose window");
            window
                .update(cx, |app, window, cx| {
                    let entity = cx.entity().downgrade();
                    window.on_window_should_close(cx, move |_, cx| {
                        entity
                            .update(cx, |app, cx| {
                                app.commit_field();
                                if app.dirty() {
                                    app.pending = Some(workspace::Pending::Close);
                                    cx.notify();
                                    false
                                } else {
                                    true
                                }
                            })
                            .unwrap_or(true)
                    });
                    app.focus.focus(window);
                })
                .ok();
            if std::env::args().any(|a| a == "--image-picker-test" || a == "--image-picker-hold") {
                image_picker_smoke::start(window, cx);
            }
            if std::env::args().any(|a| a.starts_with("--ui-smoke")) {
                smoke::native(window, cx);
            }
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit()
                }
            })
            .detach();
        });
    if smoke::failed() || image_picker_smoke::failed() {
        std::process::exit(1)
    }
}
