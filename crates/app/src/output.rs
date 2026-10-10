use crate::{
    panels::{icon, label},
    workspace::Workspace,
};
use gpui::{prelude::*, *};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Compose,
    Png,
}
impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Compose => "compose",
            Self::Png => "png",
        }
    }
}
pub fn destination(mut path: PathBuf, format: OutputFormat) -> anyhow::Result<PathBuf> {
    if path.extension().is_none() {
        path.set_extension(format.extension());
        anyhow::ensure!(
            !path.exists(),
            "The destination already exists. Select its full filename to confirm replacement"
        );
    }
    anyhow::ensure!(
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case(format.extension())),
        "Choose a filename ending in .{}",
        format.extension()
    );
    Ok(path)
}
fn filename(name: &str) -> String {
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .take(120)
        .collect();
    if name.trim().is_empty() {
        "Untitled".into()
    } else {
        name
    }
}
pub fn write(
    path: &Path,
    format: OutputFormat,
    doc: &document::Document,
    page: usize,
    dpi: u32,
) -> anyhow::Result<String> {
    match format {
        OutputFormat::Compose => {
            file_format::save(path, doc)?;
            Ok("Compose file exported".into())
        }
        OutputFormat::Png => {
            let (w, h) = export::write_png(path, doc, page, dpi)?;
            Ok(format!("PNG exported · {w} × {h} px"))
        }
    }
}
impl Workspace {
    pub fn export_document(&mut self, format: OutputFormat, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.commit_field();
        self.typing = None;
        self.measure();
        self.close_popup();
        let doc = self.editor.document.clone();
        let page = self.editor.page;
        let dpi = self.export_dpi;
        let default_dir = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .map(|p| p.join("Documents"))
            .filter(|p| p.is_dir())
            .unwrap_or_else(|| PathBuf::from("."));
        let dir = self
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .unwrap_or(&default_dir);
        let suffix = if format == OutputFormat::Png && doc.pages.len() > 1 {
            format!(" - Page {}", page + 1)
        } else {
            String::new()
        };
        let name = format!("{}{suffix}.{}", filename(&doc.name), format.extension());
        let prompt = cx.prompt_for_new_path(dir, Some(&name));
        self.busy = true;
        cx.spawn(async move |this, cx| {
            let response = prompt.await;
            let path = match response {
                Ok(Ok(Some(path))) => destination(path, format),
                Ok(Ok(None)) => {
                    this.update(cx, |s, cx| {
                        s.busy = false;
                        cx.notify();
                    })
                    .ok();
                    return;
                }
                _ => Err(anyhow::anyhow!("Could not open the export dialog")),
            };
            let result = match path {
                Ok(path) => {
                    this.update(cx, |s, cx| {
                        s.set_status("Exporting…", false);
                        cx.notify();
                    })
                    .ok();
                    cx.background_executor()
                        .spawn(async move { write(&path, format, &doc, page, dpi) })
                        .await
                }
                Err(error) => Err(error),
            };
            this.update(cx, |s, cx| {
                s.busy = false;
                match result {
                    Ok(message) => s.set_status(message, false),
                    Err(error) => s.set_status(format!("Export failed: {error}"), true),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    pub fn print_document(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.commit_field();
        self.typing = None;
        self.measure();
        self.close_popup();
        let doc = self.editor.document.clone();
        self.busy = true;
        self.set_status("Preparing pages for printing…", false);
        cx.spawn(async move |this, cx| {
            let prepared = cx
                .background_executor()
                .spawn(async move { crate::printing::prepare(&doc) })
                .await;
            let result = match prepared {
                Ok(job) => {
                    this.update(cx, |s, cx| {
                        s.set_status("Print dialog open", false);
                        cx.notify();
                    })
                    .ok();
                    cx.background_executor()
                        .spawn(async move { job.run() })
                        .await
                }
                Err(error) => Err(error),
            };
            this.update(cx, |s, cx| {
                s.busy = false;
                match result {
                    Ok(true) => s.set_status("Print operation completed", false),
                    Ok(false) => s.set_status("Printing cancelled", false),
                    Err(error) => s.set_status(format!("Print failed: {error}"), true),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    pub fn output_menu(&self, cx: &Context<Self>) -> Div {
        let page = &self.editor.document.pages[self.editor.page];
        let resolution = export::dimensions(page.width, page.height, self.export_dpi);
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(self.output_row(
                "export-compose",
                "Compose file",
                "Editable document · all pages and images",
                "save",
                cx,
                |s, _, cx| s.export_document(OutputFormat::Compose, cx),
            ))
            .child(div().h(px(1.)).bg(rgb(crate::panels::LINE)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(icon("image"))
                    .child("PNG image"),
            )
            .child(label(match resolution {
                Ok((w, h)) => format!("{} · {w} × {h} px", page.name),
                Err(ref e) => e.to_string(),
            }))
            .child(div().flex().gap(px(6.)).children(
                [(72, "1×"), (144, "2×"), (300, "300 DPI")].map(|(dpi, title)| {
                    let entity = cx.entity();
                    div()
                        .on_children_prepainted(move |bounds, _, cx| {
                            if let Some(bounds) = bounds.first() {
                                entity.update(cx, |s, _| {
                                    s.output_resolution_bounds.insert(dpi, *bounds);
                                });
                            }
                        })
                        .child(self.button(
                            match dpi {
                                72 => "export-1x",
                                144 => "export-2x",
                                _ => "export-300dpi",
                            },
                            title,
                            None,
                            self.export_dpi == dpi,
                            self.busy,
                            cx,
                            move |s, _, _| s.export_dpi = dpi,
                        ))
                }),
            ))
            .child(self.button(
                "export-png",
                "Export PNG…",
                Some("export"),
                true,
                self.busy || resolution.is_err(),
                cx,
                |s, _, cx| s.export_document(OutputFormat::Png, cx),
            ))
            .child(div().h(px(1.)).bg(rgb(crate::panels::LINE)))
            .child(self.output_row(
                "print-document",
                "Print…",
                if cfg!(target_os = "macos") {
                    "All pages · choose printer or Save as PDF"
                } else {
                    "Native printing is available on macOS"
                },
                "print",
                cx,
                |s, _, cx| s.print_document(cx),
            ))
    }
    #[allow(clippy::too_many_arguments)]
    fn output_row(
        &self,
        id: &'static str,
        title: &'static str,
        detail: &'static str,
        image: &'static str,
        cx: &Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> Div {
        let disabled = self.busy || (id == "print-document" && !cfg!(target_os = "macos"));
        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(
                self.button(id, title, Some(image), false, disabled, cx, action)
                    .justify_start()
                    .px(px(0.)),
            )
            .child(label(detail))
    }
}
