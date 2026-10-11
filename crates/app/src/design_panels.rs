use crate::{
    panels::{LINE, MUTED, color, icon, label},
    workspace::{ColorDrag, ColorTarget, Field, Popup, Workspace},
};
use gpui::{prelude::*, *};
use text::palette::{Hsv, PALETTE};

impl Workspace {
    pub fn typography_choices(&self, cx: &Context<Self>) -> Div {
        let style = self.active_style();
        let choices = self.format_values();
        let mixed = choices.iter().any(|s| s.family != style.family);
        let mut weights = div().flex().gap(px(3.));
        let choice = renderer::FONT_CHOICES
            .iter()
            .find(|f| f.family == style.family)
            .unwrap_or(&renderer::FONT_CHOICES[0]);
        for (i, (weight, name)) in choice.weights.iter().enumerate() {
            let weight = *weight;
            weights = weights.child(
                div()
                    .id(("font-weight", i))
                    .flex_1()
                    .h(px(27.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .bg(if choices.iter().all(|s| s.weight == weight) {
                        rgb(0xe6efff)
                    } else {
                        rgb(0xf4f5f2)
                    })
                    .text_size(px(11.))
                    .child(*name)
                    .cursor_pointer()
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.format(|t| t.weight = weight);
                        cx.notify();
                    })),
            );
        }
        let mut presets = div().flex().gap(px(3.));
        for (i, preset) in crate::text_menu::TEXT_PRESETS.iter().take(4).enumerate() {
            presets = presets.child(
                div()
                    .id(("type-preset", i))
                    .flex_1()
                    .h(px(25.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_size(px(10.))
                    .text_color(rgb(MUTED))
                    .hover(|d| d.bg(rgb(0xf0f2ed)))
                    .child(preset.name)
                    .cursor_pointer()
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.type_preset(i);
                        cx.notify();
                    })),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .id("font-picker")
                    .h(px(34.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(LINE))
                    .bg(rgb(0xf8f9f6))
                    .cursor_pointer()
                    .child(icon("typeface"))
                    .child(div().flex_1().child(if mixed {
                        "Mixed typefaces".into()
                    } else {
                        style.family
                    }))
                    .child(icon("chevron-down").size(px(12.)))
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.toggle_popup(Popup::Fonts);
                        cx.notify();
                    })),
            )
            .child(weights)
            .child(presets)
    }
    pub fn color_row(&self, target: ColorTarget, cx: &Context<Self>) -> Stateful<Div> {
        let c = self.current_color(target);
        let target_name = if target == ColorTarget::Fill {
            "Fill"
        } else {
            "Stroke"
        };
        div()
            .id(if target == ColorTarget::Fill {
                "fill-picker"
            } else {
                "stroke-picker"
            })
            .h(px(34.))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(9.))
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(LINE))
            .bg(rgb(0xf8f9f6))
            .cursor_pointer()
            .child(
                div()
                    .size(px(20.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(rgb(0xd6dcd4))
                    .bg(color(c)),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(px(12.))
                    .child(format!("{:02X}{:02X}{:02X}", c.r, c.g, c.b)),
            )
            .child(label(format!("{}%", (c.a as f32 / 2.55).round())))
            .child(icon("chevron-down").size(px(12.)))
            .tooltip(move |_, cx| {
                cx.new(|_| crate::panels::Hint(format!("{target_name} color").into()))
                    .into()
            })
            .on_click(cx.listener(move |s, _, _, cx| {
                s.open_color(target);
                cx.notify();
            }))
    }
    pub fn effects_panel(&self, cx: &Context<Self>) -> Div {
        let Some(n) = self.editor.selected_node() else {
            return div();
        };
        let a = &n.appearance;
        let line =
            matches!(n.kind, document::NodeKind::Shape) && n.shape == document::ShapeKind::Line;
        self.section(
            if line { "Shadow" } else { "Stroke & shadow" },
            div()
                .flex()
                .flex_col()
                .gap(px(9.))
                .when(!line, |d| {
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(label("Stroke"))
                            .child(self.button(
                                "toggle-stroke",
                                if a.stroke_width > 0. { "On" } else { "Off" },
                                Some("stroke"),
                                a.stroke_width > 0.,
                                false,
                                cx,
                                |s, _, _| s.toggle_stroke(),
                            )),
                    )
                })
                .when(a.stroke_width > 0. && !line, |d| {
                    d.child(self.color_row(ColorTarget::Stroke, cx))
                        .child(self.field_view(
                            Field::StrokeWidth,
                            "pt",
                            format!("{:.1}", a.stroke_width),
                            cx,
                        ))
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(label("Drop shadow"))
                        .child(self.button(
                            "toggle-shadow",
                            if a.shadow { "On" } else { "Off" },
                            Some("shadow"),
                            a.shadow,
                            false,
                            cx,
                            |s, _, _| s.toggle_shadow(),
                        )),
                )
                .when(a.shadow, |d| {
                    d.child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(self.field_view(
                                Field::ShadowBlur,
                                "B",
                                format!("{:.0}", a.shadow_blur),
                                cx,
                            ))
                            .child(self.field_view(
                                Field::ShadowY,
                                "Y",
                                format!("{:.0}", a.shadow_y),
                                cx,
                            )),
                    )
                }),
        )
    }
    pub fn assets_panel(&self, cx: &Context<Self>) -> Div {
        let mut list = div().flex().flex_col().gap(px(8.)).p(px(12.));
        list = list.child(
            div()
                .mb(px(4.))
                .text_color(rgb(MUTED))
                .text_size(px(11.))
                .line_height(px(16.))
                .child("A few good starting points. Every layer is editable."),
        );
        for (i, component) in editor::Component::ALL.into_iter().enumerate() {
            let entity = cx.entity();
            let drag_entity = entity.clone();
            let epoch = self.drag_epoch;
            let preview = match component {
                editor::Component::Title => div()
                    .child(
                        div()
                            .text_size(px(7.))
                            .text_color(rgb(MUTED))
                            .child("A NEW PERSPECTIVE"),
                    )
                    .child(
                        div()
                            .font_family("Noto Serif")
                            .text_size(px(20.))
                            .child("A better beginning."),
                    ),
                editor::Component::Callout => div()
                    .p(px(9.))
                    .bg(rgb(0xe5effc))
                    .rounded(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::BOLD)
                            .child("Worth remembering"),
                    )
                    .child(div().mt(px(4.)).w(px(125.)).h(px(3.)).bg(rgb(0xbdcde0))),
                editor::Component::Quote => div()
                    .pl(px(8.))
                    .border_l_2()
                    .border_color(rgb(0x728265))
                    .font_family("Noto Serif")
                    .italic()
                    .text_size(px(17.))
                    .child("“Make it meaningful.”"),
                editor::Component::Stat => div()
                    .flex()
                    .items_end()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(31.))
                            .font_weight(FontWeight::BOLD)
                            .child("86%"),
                    )
                    .child(label("of something good").pb(px(6.))),
                editor::Component::Card => div()
                    .p(px(7.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(LINE))
                    .child(div().size(px(17.)).rounded(px(5.)).bg(rgb(0xe0ebfb)))
                    .child(
                        div()
                            .mt(px(6.))
                            .text_size(px(12.))
                            .child("Thoughtfully made"),
                    ),
                editor::Component::Divider => div()
                    .w_full()
                    .child(div().h(px(1.)).bg(rgb(0xc9d0c5)))
                    .child(
                        div()
                            .mt(px(10.))
                            .text_size(px(8.))
                            .text_color(rgb(MUTED))
                            .child("THE NEXT CHAPTER"),
                    ),
            };
            list = list.child(
                div()
                    .on_children_prepainted(move |bounds, _, cx| {
                        if let Some(b) = bounds.first() {
                            entity.update(cx, |s, _| {
                                s.selection_control_bounds.insert(
                                    [
                                        "asset-title",
                                        "asset-callout",
                                        "asset-quote",
                                        "asset-stat",
                                        "asset-card",
                                        "asset-divider",
                                    ][i],
                                    *b,
                                );
                            });
                        }
                    })
                    .id(("component", i))
                    .p(px(10.))
                    .rounded(px(7.))
                    .border_1()
                    .border_color(rgb(LINE))
                    .bg(rgb(0xfafbf8))
                    .cursor_pointer()
                    .hover(|d| d.border_color(rgb(0x9cbcea)).bg(rgb(0xf5f8ff)))
                    .child(
                        div()
                            .h(px(66.))
                            .flex()
                            .flex_col()
                            .justify_center()
                            .child(preview),
                    )
                    .child(
                        div()
                            .mt(px(8.))
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(component.name()),
                            )
                            .child(icon("plus").size(px(13.))),
                    )
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.insert_block(component);
                        cx.notify();
                    }))
                    .on_drag(
                        crate::drag_drop::InsertDrag {
                            kind: crate::drag_drop::InsertKind::Component(component),
                            epoch,
                        },
                        move |_, position, _, cx| {
                            drag_entity.update(cx, |s, _| {
                                s.end_edit();
                                s.gesture = crate::workspace::Gesture::None;
                            });
                            cx.new(|_| crate::drag_drop::DragLabel {
                                name: component.name().into(),
                                position,
                            })
                        },
                    ),
            );
        }
        list
    }
    fn popup_header(&self, title: &str, cx: &Context<Self>) -> Div {
        div()
            .flex()
            .items_center()
            .justify_between()
            .mb(px(12.))
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title.to_string()),
            )
            .child(self.button(
                "close-popover",
                "",
                Some("close"),
                false,
                false,
                cx,
                |s, _, _| s.close_popup(),
            ))
    }
    pub fn popup_view(&self, cx: &Context<Self>) -> AnyElement {
        let Some(popup) = self.popup else {
            return div().into_any_element();
        };
        let mut body = div()
            .w(px(304.))
            .p(px(14.))
            .bg(white())
            .rounded(px(10.))
            .border_1()
            .border_color(rgb(LINE))
            .shadow_lg();
        match popup {
            Popup::Shapes => {
                body = body
                    .child(self.popup_header(
                        if self.shape_change_target.is_some() {
                            "Shape type"
                        } else {
                            "Shapes"
                        },
                        cx,
                    ))
                    .child(self.shape_menu(cx));
            }
            Popup::Link => {
                body = body
                    .child(self.popup_header("Text link", cx))
                    .child(self.link_popup_view(cx));
            }
            Popup::Background => {
                body = body
                    .child(self.popup_header("Background removal", cx))
                    .child(self.background_settings_view(cx));
            }
            Popup::Context => return self.context_menu_view(cx),
            Popup::Modes => {
                body = body
                    .child(self.popup_header("Document mode", cx))
                    .child(self.mode_menu(cx));
            }
            Popup::Export => {
                body = body
                    .child(self.popup_header("Export & print", cx))
                    .child(self.output_menu(cx));
            }
            Popup::Images => return self.image_picker_view(cx),
            Popup::Fonts => {
                body = body
                    .child(self.popup_header("Typeface", cx))
                    .child(self.field_view(Field::FontSearch, "⌕", self.font_search.clone(), cx));
                let style = self.active_style();
                let mut list = div()
                    .id("font-list")
                    .mt(px(8.))
                    .max_h(px(340.))
                    .overflow_y_scroll();
                let search = self.font_search.to_lowercase();
                let mut count = 0;
                for (i, f) in renderer::FONT_CHOICES.iter().enumerate().filter(|(_, f)| {
                    format!("{} {}", f.label, f.category)
                        .to_lowercase()
                        .contains(&search)
                }) {
                    count += 1;
                    let family = f.family;
                    list = list.child(
                        div()
                            .id(("font-choice", i))
                            .p(px(10.))
                            .rounded(px(5.))
                            .cursor_pointer()
                            .bg(if family == style.family {
                                rgb(0xe6efff)
                            } else {
                                rgb(0xffffff)
                            })
                            .hover(|d| d.bg(rgb(0xf1f5fc)))
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div().font_family(family).text_size(px(22.)).child(f.label),
                                    )
                                    .when(family == style.family, |d| {
                                        d.child(icon("check").size(px(15.)))
                                    }),
                            )
                            .child(label(f.category).mt(px(4.)))
                            .on_click(cx.listener(move |s, _, _, cx| {
                                s.set_font(family);
                                cx.notify();
                            })),
                    );
                }
                if count == 0 {
                    list = list.child(
                        div()
                            .p(px(16.))
                            .text_color(rgb(MUTED))
                            .child("No typefaces match."),
                    );
                }
                body = body
                    .child(list)
                    .child(label("6 bundled typefaces · scroll to explore").mt(px(10.)));
            }
            Popup::Colors => {
                let Some(picker) = &self.picker else {
                    return div().into_any_element();
                };
                let target = picker.target;
                let c = picker.hsv.color();
                let entity = cx.entity();
                let entity2 = entity.clone();
                body = body
                    .child(self.popup_header(
                        if target == ColorTarget::Stroke {
                            "Stroke color"
                        } else {
                            "Color"
                        },
                        cx,
                    ))
                    .child(
                        div()
                            .id("saturation-picker")
                            .h(px(164.))
                            .w_full()
                            .rounded(px(6.))
                            .overflow_hidden()
                            .cursor(CursorStyle::Crosshair)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|s, e: &MouseDownEvent, _, cx| {
                                    s.start_color_drag(ColorDrag::Saturation, e.position);
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            )
                            .child(
                                canvas(
                                    move |_, _, _| (),
                                    move |bounds, (), w, cx| {
                                        entity.update(cx, |s, _| {
                                            s.paint_color_picker(bounds, false, w)
                                        });
                                    },
                                )
                                .size_full(),
                            ),
                    )
                    .child(
                        div()
                            .id("hue-picker")
                            .mt(px(12.))
                            .h(px(14.))
                            .w_full()
                            .rounded(px(7.))
                            .overflow_hidden()
                            .cursor(CursorStyle::PointingHand)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|s, e: &MouseDownEvent, _, cx| {
                                    s.start_color_drag(ColorDrag::Hue, e.position);
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            )
                            .child(
                                canvas(
                                    move |_, _, _| (),
                                    move |bounds, (), w, cx| {
                                        entity2.update(cx, |s, _| {
                                            s.paint_color_picker(bounds, true, w)
                                        });
                                    },
                                )
                                .size_full(),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(13.))
                            .flex()
                            .gap(px(8.))
                            .child(self.field_view(
                                Field::ColorHex,
                                "#",
                                format!("{:02X}{:02X}{:02X}", c.r, c.g, c.b),
                                cx,
                            ))
                            .child(div().w(px(87.)).child(self.field_view(
                                Field::ColorAlpha,
                                "%",
                                format!("{:.0}", c.a as f32 / 2.55),
                                cx,
                            ))),
                    )
                    .child(label("Palette").mt(px(14.)))
                    .child(self.swatch_grid(PALETTE.to_vec(), "palette", cx));
                let colors = self.document_colors();
                if !colors.is_empty() {
                    body = body
                        .child(label("In this document").mt(px(13.)))
                        .child(self.swatch_grid(colors, "document-swatches", cx));
                }
            }
            Popup::Pages => {
                let p = &self.editor.document.pages[self.editor.page];
                body = body.child(self.popup_header("Page setup", cx));
                for (i, (name, w, h)) in [
                    ("A4", 595.28, 841.89),
                    ("US Letter", 612., 792.),
                    ("Square", 720., 720.),
                    ("Landscape", 960., 540.),
                ]
                .into_iter()
                .enumerate()
                {
                    body = body.child(
                        div()
                            .id(("page-format", i))
                            .h(px(42.))
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .rounded(px(5.))
                            .bg(if (p.width - w).abs() < 1. && (p.height - h).abs() < 1. {
                                rgb(0xe6efff)
                            } else {
                                white().into()
                            })
                            .child(name)
                            .child(label(format!("{w:.0} × {h:.0} pt")))
                            .cursor_pointer()
                            .on_click(cx.listener(move |s, _, _, cx| {
                                s.set_page_size(w, h);
                                cx.notify();
                            })),
                    );
                }
                body = body.child(
                    self.button(
                        "rotate-page",
                        "Swap orientation",
                        Some("rotate"),
                        false,
                        false,
                        cx,
                        |s, _, _| {
                            let p = &s.editor.document.pages[s.editor.page];
                            s.set_page_size(p.height, p.width)
                        },
                    )
                    .mt(px(10.)),
                );
            }
            Popup::TextStyles | Popup::TextWeights | Popup::TextSizes => {
                let title = match popup {
                    Popup::TextStyles => "Text styles",
                    Popup::TextWeights => "Font weight",
                    _ => "Font size",
                };
                body = body
                    .child(self.popup_header(title, cx))
                    .child(self.text_options_view(popup, cx));
            }
        }
        let root = div().absolute().inset_0().child(
            div()
                .id("popup-dismiss")
                .occlude()
                .absolute()
                .inset_0()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|s, _, _, cx| {
                        s.close_popup();
                        cx.stop_propagation();
                        cx.notify();
                    }),
                ),
        );
        if popup == Popup::Shapes {
            let anchor = self
                .selection_control_bounds
                .get(if self.shape_change_target.is_some() {
                    "shape-type"
                } else {
                    "tool-shape"
                });
            let position = anchor
                .map(|b| point(b.left(), b.bottom() + px(6.)))
                .unwrap_or(point(px(236.), px(60.)));
            root.child(
                anchored()
                    .position(position)
                    .snap_to_window_with_margin(Edges::all(px(12.)))
                    .child(div().id("popup-body").occlude().child(body)),
            )
            .into_any_element()
        } else if let Some(anchor) = self.text_popup_anchor {
            let entity = cx.entity();
            let position = point(
                anchor.left() + (anchor.size.width - px(304.)) / 2.,
                anchor.bottom() + px(6.),
            );
            root.child(
                anchored()
                    .position(position)
                    .snap_to_window_with_margin(Edges {
                        top: self.viewport.top() + px(8.),
                        bottom: px(32.),
                        left: self.viewport.left() + px(8.),
                        right: px(280.),
                    })
                    .child(
                        div()
                            .on_children_prepainted(move |bounds, _, cx| {
                                if let Some(bounds) = bounds.first() {
                                    entity.update(cx, |s, _| s.text_popup_bounds = *bounds);
                                }
                            })
                            .id("text-popup-body")
                            .occlude()
                            .max_h(self.viewport.size.height - px(16.))
                            .overflow_y_scroll()
                            .child(body),
                    ),
            )
            .into_any_element()
        } else {
            root.child(
                div()
                    .id("popup-body")
                    .occlude()
                    .absolute()
                    .top(px(60.))
                    .right(px(16.))
                    .child(body),
            )
            .into_any_element()
        }
    }
    fn swatch_grid(
        &self,
        colors: Vec<document::Color>,
        id: &'static str,
        cx: &Context<Self>,
    ) -> Div {
        let active = self.picker.as_ref().map(|p| p.hsv.color());
        let mut grid = div().flex().flex_wrap().gap(px(7.)).mt(px(8.));
        for (i, c) in colors.into_iter().enumerate() {
            grid = grid.child(
                div()
                    .id((id, i))
                    .size(px(27.))
                    .rounded(px(5.))
                    .p(px(2.))
                    .border_1()
                    .border_color(if active == Some(c) {
                        rgb(0x2878ed)
                    } else {
                        rgb(LINE)
                    })
                    .cursor_pointer()
                    .child(div().size_full().rounded(px(3.)).bg(color(c)))
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.choose_color(c);
                        cx.notify();
                    })),
            );
        }
        grid
    }
    fn paint_color_picker(&mut self, bounds: Bounds<Pixels>, hue: bool, window: &mut Window) {
        let Some(p) = &self.picker else { return };
        let hsv = p.hsv;
        if hue {
            self.hue_bounds = bounds;
            for i in 0..6 {
                let left = bounds.left() + bounds.size.width * (i as f32 / 6.);
                let w = bounds.size.width / 6.;
                let a = Hsv {
                    hue: i as f32 * 60.,
                    saturation: 1.,
                    value: 1.,
                    alpha: 255,
                }
                .color();
                let b = Hsv {
                    hue: (i + 1) as f32 * 60.,
                    saturation: 1.,
                    value: 1.,
                    alpha: 255,
                }
                .color();
                window.paint_quad(fill(
                    Bounds::new(
                        point(left, bounds.top()),
                        size(w + px(0.5), bounds.size.height),
                    ),
                    linear_gradient(
                        90.,
                        linear_color_stop(color(a), 0.),
                        linear_color_stop(color(b), 1.),
                    ),
                ));
            }
            let x = bounds.left() + bounds.size.width * (hsv.hue / 360.);
            window.paint_quad(
                outline(
                    Bounds::new(
                        point(x - px(3.), bounds.top()),
                        size(px(6.), bounds.size.height),
                    ),
                    white(),
                    BorderStyle::Solid,
                )
                .border_widths(px(2.))
                .corner_radii(px(3.)),
            );
        } else {
            self.color_bounds = bounds;
            let hue = Hsv {
                hue: hsv.hue,
                saturation: 1.,
                value: 1.,
                alpha: 255,
            }
            .color();
            window.paint_quad(fill(
                bounds,
                linear_gradient(
                    90.,
                    linear_color_stop(white(), 0.),
                    linear_color_stop(color(hue), 1.),
                ),
            ));
            window.paint_quad(fill(
                bounds,
                linear_gradient(
                    180.,
                    linear_color_stop(transparent_black(), 0.),
                    linear_color_stop(black(), 1.),
                ),
            ));
            let center = point(
                bounds.left() + bounds.size.width * hsv.saturation,
                bounds.top() + bounds.size.height * (1. - hsv.value),
            );
            window.paint_quad(
                outline(
                    Bounds::new(center - point(px(6.), px(6.)), size(px(12.), px(12.))),
                    hsla(0., 0., 0., 0.45),
                    BorderStyle::Solid,
                )
                .corner_radii(px(6.))
                .border_widths(px(1.)),
            );
            window.paint_quad(
                outline(
                    Bounds::new(center - point(px(5.), px(5.)), size(px(10.), px(10.))),
                    white(),
                    BorderStyle::Solid,
                )
                .corner_radii(px(5.))
                .border_widths(px(2.)),
            );
        }
    }
}
