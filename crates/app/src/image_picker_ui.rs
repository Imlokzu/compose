use crate::{
    image_picker::ImageTab,
    panels::{LINE, MUTED, icon, label},
    workspace::{Field, Workspace},
};
use gpui::{prelude::*, *};
impl Workspace {
    pub fn image_picker_view(&self, cx: &Context<Self>) -> AnyElement {
        let picker = &self.image_picker;
        let height = (f32::from(self.viewport.size.height) + 28.).clamp(500., 620.);
        let mut body = div()
            .w(px(740.))
            .h(px(height))
            .flex()
            .flex_col()
            .rounded(px(12.))
            .border_1()
            .border_color(rgb(LINE))
            .bg(white())
            .shadow_lg()
            .overflow_hidden();
        body = body
            .child(
                div()
                    .px(px(20.))
                    .pt(px(16.))
                    .pb(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .child(icon("image").size(px(20.)))
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(if picker.replace_target.is_some() {
                                "Replace image"
                            } else {
                                "Add an image"
                            }),
                    )
                    .child(div().flex_1())
                    .child(self.button(
                        "close-images",
                        "",
                        Some("close"),
                        false,
                        false,
                        cx,
                        |s, _, _| s.close_popup(),
                    )),
            )
            .child(
                div()
                    .px(px(20.))
                    .pb(px(12.))
                    .flex()
                    .gap(px(6.))
                    .child(self.button(
                        "image-computer",
                        "My computer",
                        Some("open"),
                        picker.tab == ImageTab::Computer,
                        picker.inserting,
                        cx,
                        |s, _, cx| s.set_image_tab(ImageTab::Computer, cx),
                    ))
                    .child(self.button(
                        "image-online",
                        "Search online",
                        Some("search"),
                        picker.tab == ImageTab::Search,
                        picker.inserting,
                        cx,
                        |s, _, cx| s.set_image_tab(ImageTab::Search, cx),
                    ))
                    .child(div().flex_1())
                    .when(picker.tab == ImageTab::Search, |d| {
                        d.child(self.button(
                            "image-web",
                            "Web · Bing",
                            None,
                            picker.provider == image_search::Provider::Web,
                            picker.inserting,
                            cx,
                            |s, _, cx| s.set_image_provider(image_search::Provider::Web, cx),
                        ))
                        .child(self.button(
                            "image-commons",
                            "Commons",
                            None,
                            picker.provider == image_search::Provider::Commons,
                            picker.inserting,
                            cx,
                            |s, _, cx| s.set_image_provider(image_search::Provider::Commons, cx),
                        ))
                    }),
            )
            .child(div().h(px(1.)).bg(rgb(LINE)));
        if picker.tab == ImageTab::Computer {
            body = body
                .child(
                    div()
                        .flex_1()
                        .p(px(28.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap(px(16.))
                        .child(
                            div()
                                .size(px(64.))
                                .rounded(px(16.))
                                .bg(rgb(0xeaf0f8))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon("image").size(px(30.)).text_color(rgb(0x2878ed))),
                        )
                        .child(
                            div()
                                .text_size(px(21.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Bring your own image"),
                        )
                        .child(label(
                            "Photos, illustrations, screenshots — make them part of your page.",
                        ))
                        .child(
                            self.button(
                                "browse-images",
                                if picker.inserting {
                                    if picker.replace_target.is_some() {
                                        "Replacing image…"
                                    } else {
                                        "Adding image…"
                                    }
                                } else if self.busy {
                                    "Opening…"
                                } else {
                                    "Choose an image…"
                                },
                                Some("plus"),
                                true,
                                self.busy || picker.inserting,
                                cx,
                                |s, _, cx| s.choose_file(true, cx),
                            )
                            .px(px(18.))
                            .h(px(36.)),
                        )
                        .child(label("PNG, JPEG, WebP, GIF or BMP · up to 64 MB / 32 MP"))
                        .when_some(picker.error.clone(), |d, error| {
                            d.child(div().max_w(px(570.)).text_color(rgb(0xa44335)).child(error))
                        }),
                )
                .child(
                    div()
                        .border_t_1()
                        .border_color(rgb(LINE))
                        .p(px(16.))
                        .child(label(
                            "Your image is embedded in the document and stays available offline.",
                        )),
                );
        } else {
            let query = picker.query.clone();
            body = body.child(
                div()
                    .px(px(20.))
                    .pt(px(14.))
                    .pb(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(9.))
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(self.field_view(Field::ImageSearch, "⌕", query, cx))
                            .child(self.button(
                                "search-images",
                                if picker.loading {
                                    "Searching…"
                                } else {
                                    "Search"
                                },
                                None,
                                true,
                                picker.loading || picker.inserting,
                                cx,
                                |s, _, cx| s.search_images(false, cx),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(5.))
                            .child(label("Try").pt(px(8.)))
                            .children(["Nature", "Architecture", "Space", "Textures"].map(
                                |query| {
                                    div()
                                        .id(SharedString::from(format!("image-query-{query}")))
                                        .h(px(28.))
                                        .px(px(8.))
                                        .flex()
                                        .items_center()
                                        .rounded(px(5.))
                                        .bg(rgb(0xf4f5f2))
                                        .text_color(rgb(MUTED))
                                        .cursor_pointer()
                                        .child(query)
                                        .on_click(cx.listener(move |s, _, _, cx| {
                                            if s.image_picker.inserting {
                                                return;
                                            }
                                            s.commit_field();
                                            s.image_picker.query = query.into();
                                            s.search_images(false, cx);
                                        }))
                                },
                            )),
                    ),
            );
            let mut results = div()
                .id("image-results")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(px(20.))
                .pb(px(14.));
            if let Some(error) = &picker.error {
                results = results.child(
                    div()
                        .p(px(12.))
                        .mb(px(12.))
                        .rounded(px(7.))
                        .bg(rgb(0xfff1ec))
                        .child(
                            div()
                                .text_color(rgb(0xa44335))
                                .line_height(px(17.))
                                .child(error.clone()),
                        )
                        .child(
                            self.button(
                                "retry-images",
                                "Try again",
                                Some("redo"),
                                false,
                                picker.loading || picker.inserting,
                                cx,
                                |s, _, cx| {
                                    if s.image_picker.selected.is_some()
                                        && s.image_picker
                                            .error
                                            .as_ref()
                                            .is_some_and(|e| e.starts_with("Could not insert"))
                                    {
                                        s.insert_search_image(cx)
                                    } else {
                                        s.search_images(false, cx)
                                    }
                                },
                            )
                            .mt(px(7.)),
                        )
                        .child(self.button(
                            "image-open-web",
                            "Open search in browser",
                            Some("open"),
                            false,
                            false,
                            cx,
                            |s, _, cx| {
                                if let Ok(url) = image_search::browser_search_url(
                                    s.image_picker.provider,
                                    &s.image_picker.query,
                                ) {
                                    cx.open_url(&url);
                                }
                            },
                        )),
                );
            }
            if picker.results.is_empty() {
                let (title, detail) = if picker.loading {
                    (
                        "Finding images…",
                        "Your page stays responsive while images load.",
                    )
                } else if picker.searched.is_empty() {
                    (
                        "Find the right image",
                        if picker.provider == image_search::Provider::Web {
                            "Search images from websites across the web with Bing Images."
                        } else {
                            "Search the Wikimedia Commons collection."
                        },
                    )
                } else {
                    (
                        "No supported images found",
                        "Try a different phrase, or load more results.",
                    )
                };
                results = results.child(
                    div()
                        .py(px(35.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(11.))
                        .child(icon("search").size(px(25.)).text_color(rgb(MUTED)))
                        .child(
                            div()
                                .text_size(px(17.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(label(detail)),
                );
            } else {
                let mut grid = div().flex().flex_wrap().gap(px(10.));
                for result in &picker.results {
                    let id = result.item.id;
                    let selected = picker.selected == Some(id);
                    let title = result.item.title.clone();
                    let tile = div()
                        .id(("image-result", id))
                        .w(px(226.))
                        .h(px(168.))
                        .rounded(px(7.))
                        .overflow_hidden()
                        .border_2()
                        .border_color(if selected { rgb(0x2878ed) } else { rgb(LINE) })
                        .bg(rgb(0xf5f6f2))
                        .cursor_pointer()
                        .hover(|d| d.border_color(rgb(0x87aff0)))
                        .child(
                            div()
                                .h(px(126.))
                                .w_full()
                                .overflow_hidden()
                                .flex()
                                .items_center()
                                .justify_center()
                                .when_some(result.thumbnail.clone(), |d, thumbnail| {
                                    d.child(img(thumbnail).size_full().object_fit(ObjectFit::Cover))
                                })
                                .when(result.thumbnail.is_none(), |d| {
                                    d.child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .gap(px(7.))
                                            .child(icon("image").text_color(rgb(MUTED)))
                                            .child(label(if result.thumbnail_failed {
                                                "Preview unavailable"
                                            } else {
                                                "Loading preview…"
                                            })),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .px(px(8.))
                                .py(px(5.))
                                .bg(white())
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_ellipsis()
                                        .overflow_hidden()
                                        .child(title),
                                )
                                .child(
                                    label(if result.item.license.is_empty() {
                                        result.item.source_site()
                                    } else {
                                        result.item.license.clone()
                                    })
                                    .text_size(px(10.)),
                                ),
                        )
                        .on_click(cx.listener(move |s, _, _, cx| {
                            if !s.image_picker.inserting {
                                s.commit_field();
                                s.image_picker.selected = Some(id);
                                s.image_picker.error = None;
                                cx.notify();
                            }
                        }));
                    grid = grid.child(tile);
                }
                results = results.child(grid);
            }
            if picker.next.is_some() {
                results =
                    results.child(div().mt(px(14.)).flex().justify_center().child(self.button(
                        "more-images",
                        if picker.loading {
                            "Loading…"
                        } else {
                            "More images"
                        },
                        None,
                        false,
                        picker.loading || picker.inserting,
                        cx,
                        |s, _, cx| s.search_images(true, cx),
                    )));
            }
            body = body.child(results);
            let selected = picker.selected_result();
            let mut footer = div()
                .p(px(16.))
                .border_t_1()
                .border_color(rgb(LINE))
                .bg(rgb(0xfafbf8))
                .flex()
                .items_center()
                .gap(px(12.));
            if let Some(result) = selected {
                let item = &result.item;
                let source = item.source_url.clone();
                let license = item.license_url.clone();
                let mut details = div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(item.title.clone()),
                    )
                    .child(
                        label(if item.width == 0 || item.height == 0 {
                            format!("Original image · {}", item.provider.name())
                        } else {
                            format!(
                                "{} × {} px · {}",
                                item.width,
                                item.height,
                                item.provider.name()
                            )
                        })
                        .overflow_hidden()
                        .text_ellipsis(),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .id("image-source-link")
                                    .text_color(rgb(0x2878ed))
                                    .text_size(px(11.))
                                    .cursor_pointer()
                                    .child("View source")
                                    .on_click(move |_, _, cx| {
                                        if image_search::safe_external_url(&source) {
                                            cx.open_url(&source)
                                        }
                                    }),
                            )
                            .child(
                                div()
                                    .id("image-license-link")
                                    .text_color(rgb(MUTED))
                                    .text_size(px(11.))
                                    .when(image_search::safe_external_url(&license), |d| {
                                        d.cursor_pointer()
                                    })
                                    .child(if item.license.is_empty() {
                                        "Usage rights: check source".into()
                                    } else {
                                        item.license.clone()
                                    })
                                    .on_click(move |_, _, cx| {
                                        if image_search::safe_external_url(&license) {
                                            cx.open_url(&license)
                                        }
                                    }),
                            ),
                    );
                if !item.credit.is_empty() {
                    details = details.child(
                        label(item.credit.clone())
                            .text_size(px(10.))
                            .text_ellipsis()
                            .overflow_hidden(),
                    );
                }
                footer = footer.child(details);
            } else {
                footer = footer.child(
                    div()
                        .flex_1()
                        .child(label("Select an image to see its details and insert it.")),
                );
            }
            body = body.child(
                footer
                    .child(
                        self.button(
                            "insert-cutout",
                            "Without background",
                            Some("clear-style"),
                            false,
                            selected.is_none() || picker.inserting || picker.loading,
                            cx,
                            |s, _, cx| s.insert_search_image_with_cutout(true, cx),
                        )
                        .h(px(35.)),
                    )
                    .when(
                        picker.error.as_ref().is_some_and(|e| {
                            e.starts_with("Could not insert") || e.starts_with("Could not use")
                        }),
                        |d| {
                            d.child(self.button(
                                "image-use-preview",
                                "Insert preview",
                                None,
                                false,
                                selected.is_none_or(|r| r.thumbnail_bytes.is_none())
                                    || picker.inserting,
                                cx,
                                |s, _, cx| s.insert_search_preview(cx),
                            ))
                        },
                    )
                    .child(
                        self.button(
                            "insert-online-image",
                            if picker.inserting {
                                "Downloading…"
                            } else if picker.replace_target.is_some() {
                                "Replace image"
                            } else {
                                "Insert image"
                            },
                            Some("plus"),
                            true,
                            selected.is_none() || picker.inserting || picker.loading,
                            cx,
                            |s, _, cx| s.insert_search_image(cx),
                        )
                        .h(px(35.)),
                    ),
            );
        }
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id("image-picker-backdrop")
                    .occlude()
                    .absolute()
                    .inset_0()
                    .bg(rgba(0x17211733))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, _, _, cx| {
                            s.close_popup();
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    ),
            )
            .child(div().id("image-picker-dialog").occlude().child(body))
            .into_any_element()
    }
    pub fn image_source_panel(&self, cx: &Context<Self>) -> Div {
        let Some(source) = self.image_source_details() else {
            return div();
        };
        let url = source.page_url.clone();
        self.section(
            "Image source",
            div()
                .flex()
                .flex_col()
                .gap(px(5.))
                .child(label(source.provider.clone()))
                .when(source.credit.starts_with("Search preview"), |d| {
                    d.child(label("Search preview · lower resolution"))
                })
                .child(div().text_size(px(11.)).child(source.author.clone()))
                .child(label(if source.license.is_empty() {
                    "License details on source page".to_string()
                } else {
                    source.license.clone()
                }))
                .child(self.button(
                    "saved-image-source",
                    "View source",
                    None,
                    false,
                    !image_search::safe_external_url(&url),
                    cx,
                    move |_, _, cx| {
                        if image_search::safe_external_url(&url) {
                            cx.open_url(&url)
                        }
                    },
                )),
        )
    }
}
