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
    Word,
}
impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Compose => "compose",
            Self::Png => "png",
            Self::Word => "docx",
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
pub(crate) fn filename(name: &str) -> String {
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
    let name = name.trim().trim_end_matches(['.', ' ']);
    if name.is_empty() {
        "Untitled".into()
    } else {
        // Windows treats device names as reserved even with an extension.
        let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
        let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ["COM", "LPT"].iter().any(|prefix| {
                stem.strip_prefix(prefix).is_some_and(|n| {
                    matches!(
                        n,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                })
            });
        if reserved {
            format!("_{name}")
        } else {
            name.into()
        }
    }
}

#[cfg(test)]
mod filename_tests {
    use super::{OutputFormat, destination, filename};

    #[test]
    fn document_titles_produce_portable_filenames() {
        for (title, expected) in [
            ("My report", "My report"),
            ("a/b\\c:d?e", "a_b_c_d_e"),
            ("  Report... ", "Report"),
            ("...", "Untitled"),
            ("con", "_con"),
            ("LPT1.notes", "_LPT1.notes"),
            ("COM¹", "_COM¹"),
            ("Résumé", "Résumé"),
        ] {
            assert_eq!(filename(title), expected);
        }
        assert_eq!(filename(&"é".repeat(200)).chars().count(), 120);
    }

    #[test]
    fn adding_extension_does_not_overwrite_an_unconfirmed_file() {
        let dir = tempfile::tempdir().unwrap();
        let full = dir.path().join("report.compose");
        std::fs::write(&full, b"existing document").unwrap();
        assert!(destination(dir.path().join("report"), OutputFormat::Compose).is_err());
        assert_eq!(std::fs::read(&full).unwrap(), b"existing document");
        assert_eq!(
            destination(full.clone(), OutputFormat::Compose).unwrap(),
            full
        );
        assert!(destination(dir.path().join("report.txt"), OutputFormat::Compose).is_err());
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
        OutputFormat::Word => {
            export::word::write_docx(path, doc)?;
            Ok("Word document exported · editable text and images".into())
        }
    }
}
impl Workspace {
    pub fn export_document(&mut self, format: OutputFormat, cx: &mut Context<Self>) {
        if self.busy || self.pending.is_some() {
            return;
        }
        if format == OutputFormat::Word && !self.word_mode() {
            self.set_status(
                "Word export is available only in Word-compatible mode",
                true,
            );
            cx.notify();
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
        if self.pending.is_some() {
            return;
        }
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
                "export-word",
                "Word document (.docx)",
                if self.word_mode() {
                    "Editable text and images · may reflow in Word"
                } else {
                    "Available only in Word-compatible mode"
                },
                "word",
                cx,
                |s, _, cx| s.export_document(OutputFormat::Word, cx),
            ))
            .when(!self.word_mode(), |d| d.child(label("Free placement, shapes, rounded images and effects would disappear in Word. Keep this design as Compose, PNG or print; create a Word-compatible document for .docx.").line_height(px(16.))))
            .when(self.word_mode(), |d| d.child(label("Word may substitute fonts and change line or page breaks. Your editable Compose original stays intact.").line_height(px(16.))))
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
        let disabled = self.busy
            || (id == "print-document" && !cfg!(target_os = "macos"))
            || (id == "export-word" && !self.word_mode());
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
