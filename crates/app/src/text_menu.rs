use crate::{
    panels::{INK, LINE, color, icon, label},
    workspace::{ColorTarget, Field, Popup, Workspace},
};
use gpui::{prelude::*, *};
use text::TextStyle;

pub const TEXT_TOOLBAR_WIDTH: f32 = 420.;
pub const TEXT_TOOLBAR_HEIGHT: f32 = 84.;

pub struct TextPreset {
    pub name: &'static str,
    pub description: &'static str,
    pub size: f32,
    weight: f32,
    line_height: f32,
    spacing: f32,
    family: Option<&'static str>,
    italic: bool,
}
impl TextPreset {
    pub fn apply(&self, s: &mut TextStyle) {
        s.size = self.size;
        s.weight = self.weight;
        s.line_height = self.line_height;
        s.letter_spacing = self.spacing;
        s.italic = self.italic;
        if let Some(family) = self.family {
            s.family = family.into();
        }
    }
    fn matches(&self, s: &TextStyle) -> bool {
        let mut expected = s.clone();
        self.apply(&mut expected);
        renderer::normalize_style(&mut expected);
        expected == *s
    }
}
pub const TEXT_PRESETS: &[TextPreset] = &[
    TextPreset {
        name: "Title",
        description: "A confident opening",
        size: 44.,
        weight: 700.,
        line_height: 1.15,
        spacing: -0.5,
        family: None,
        italic: false,
    },
    TextPreset {
        name: "Heading",
        description: "Start a new section",
        size: 28.,
        weight: 700.,
        line_height: 1.25,
        spacing: -0.2,
        family: None,
        italic: false,
    },
    TextPreset {
        name: "Body",
        description: "Comfortable reading",
        size: 16.,
        weight: 400.,
        line_height: 1.5,
        spacing: 0.,
        family: None,
        italic: false,
    },
    TextPreset {
        name: "Caption",
        description: "Details and small notes",
        size: 11.,
        weight: 400.,
        line_height: 1.4,
        spacing: 0.5,
        family: None,
        italic: false,
    },
    TextPreset {
        name: "Subheading",
        description: "A little more emphasis",
        size: 20.,
        weight: 700.,
        line_height: 1.3,
        spacing: 0.,
        family: None,
        italic: false,
    },
    TextPreset {
        name: "Quote",
        description: "Editorial serif, italic",
        size: 22.,
        weight: 400.,
        line_height: 1.45,
        spacing: 0.,
        family: Some("Noto Serif"),
        italic: true,
    },
    TextPreset {
        name: "Code",
        description: "Clear, evenly spaced letters",
        size: 14.,
        weight: 400.,
        line_height: 1.5,
        spacing: 0.,
        family: Some("Noto Sans Mono"),
        italic: false,
    },
];

impl Workspace {
    pub fn text_toolbar_bounds(&self) -> Option<Bounds<Pixels>> {
        let (x, y) = self.selection_toolbar_position(TEXT_TOOLBAR_WIDTH, TEXT_TOOLBAR_HEIGHT)?;
        Some(Bounds::new(
            self.viewport.origin + point(px(x), px(y)),
            size(px(TEXT_TOOLBAR_WIDTH), px(TEXT_TOOLBAR_HEIGHT)),
        ))
    }
    pub fn open_text_popup(&mut self, popup: Popup) {
        let anchor = self.text_toolbar_bounds();
        let was_open = self.popup == Some(popup) && self.text_popup_anchor.is_some();
        self.close_popup();
        if !was_open && self.editing && !self.range().is_empty() && !self.selection_locked() {
            self.popup = Some(popup);
            self.text_popup_anchor = anchor;
            if popup == Popup::Fonts {
                self.font_search.clear();
                self.begin_field(Field::FontSearch, String::new());
            }
        }
    }
    fn active_preset(&self) -> Option<usize> {
        let styles = self.format_values();
        TEXT_PRESETS
            .iter()
            .position(|p| styles.iter().all(|s| p.matches(s)))
    }
    fn text_menu_button(
        &self,
        id: &'static str,
        title: String,
        popup: Popup,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let active = self.popup == Some(popup) && self.text_popup_anchor.is_some();
        self.button(id, "", None, active, false, cx, move |s, _, _| {
            s.open_text_popup(popup)
        })
        .justify_between()
        .min_w_0()
        .gap(px(5.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .child(title),
        )
        .child(icon("chevron-down").size(px(11.)))
    }
    pub fn text_selection_toolbar(&self, cx: &Context<Self>) -> Option<Stateful<Div>> {
        if self.preview || !self.editing || self.range().is_empty() || self.selection_locked() {
            return None;
        }
        let bounds = self.text_toolbar_bounds()?;
        let styles = self.format_values();
        let first = &styles[0];
        let family = if styles.iter().any(|s| s.family != first.family) {
            "Mixed fonts".into()
        } else {
            first.family.clone()
        };
        let face = renderer::FONT_CHOICES
            .iter()
            .find(|f| f.family == first.family)
            .unwrap_or(&renderer::FONT_CHOICES[0]);
        let weight = if styles.iter().any(|s| s.weight != first.weight) {
            "Mixed".into()
        } else {
            face.weights
                .iter()
                .find(|w| w.0 == first.weight)
                .map(|w| w.1.to_string())
                .unwrap_or_else(|| format!("{:.0}", first.weight))
        };
        let preset = self
            .active_preset()
            .map(|i| TEXT_PRESETS[i].name)
            .unwrap_or("Styles");
        let mixed_color = styles.iter().any(|s| s.color != first.color);
        Some(
            div()
                .id("text-toolbar")
                .occlude()
                .absolute()
                .left(bounds.left() - self.viewport.left())
                .top(bounds.top() - self.viewport.top())
                .w(px(TEXT_TOOLBAR_WIDTH))
                .h(px(TEXT_TOOLBAR_HEIGHT))
                .p(px(8.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .rounded(px(9.))
                .border_1()
                .border_color(rgb(LINE))
                .bg(white())
                .shadow_md()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            self.text_menu_button("float-font", family, Popup::Fonts, cx)
                                .flex_1(),
                        )
                        .child(
                            self.text_menu_button("float-weight", weight, Popup::TextWeights, cx)
                                .w(px(96.)),
                        )
                        .child(
                            div()
                                .w(px(108.))
                                .flex()
                                .items_center()
                                .gap(px(2.))
                                .child(self.field_view_at(
                                    Field::Size,
                                    "pt",
                                    self.style_value(Field::Size),
                                    "floating",
                                    cx,
                                ))
                                .child(
                                    self.button(
                                        "float-sizes",
                                        "",
                                        Some("chevron-down"),
                                        self.popup == Some(Popup::TextSizes),
                                        false,
                                        cx,
                                        |s, _, _| s.open_text_popup(Popup::TextSizes),
                                    )
                                    .w(px(24.))
                                    .min_w(px(24.))
                                    .px(px(3.)),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            self.text_menu_button(
                                "float-styles",
                                preset.into(),
                                Popup::TextStyles,
                                cx,
                            )
                            .flex_1(),
                        )
                        .child(div().w(px(1.)).h(px(18.)).bg(rgb(LINE)))
                        .child(self.text_controls(true, cx))
                        .child(
                            self.capture_control(
                                "float-link",
                                self.button(
                                    "float-link",
                                    "",
                                    Some("link"),
                                    self.format_values().iter().any(|s| s.link.is_some()),
                                    false,
                                    cx,
                                    |s, _, _| s.open_link_editor(),
                                )
                                .w(px(30.)),
                                cx,
                            ),
                        )
                        .child(
                            self.button(
                                "float-color",
                                "A",
                                None,
                                self.popup == Some(Popup::Colors),
                                false,
                                cx,
                                |s, _, _| {
                                    let anchor = s.text_toolbar_bounds();
                                    s.open_color(ColorTarget::Fill);
                                    s.text_popup_anchor = anchor;
                                },
                            )
                            .w(px(34.))
                            .flex_shrink_0()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(INK))
                            .border_b_2()
                            .border_color(if mixed_color {
                                rgb(0xa9b7cb).into()
                            } else {
                                color(first.color)
                            }),
                        )
                        .child(
                            self.button(
                                "float-reset",
                                "",
                                Some("clear-style"),
                                false,
                                false,
                                cx,
                                |s, _, _| {
                                    s.format(|v| *v = TextStyle::default());
                                    s.set_status("Formatting cleared from selected text", false);
                                },
                            )
                            .w(px(34.))
                            .flex_shrink_0(),
                        ),
                ),
        )
    }
    pub fn text_options_view(&self, popup: Popup, cx: &Context<Self>) -> Div {
        match popup {
            Popup::TextStyles => {
                let active = self.active_preset();
                let current_family = self.active_style().family;
                let mut options = div()
                    .id("text-style-list")
                    .max_h(px(440.))
                    .overflow_y_scroll();
                for (i, preset) in TEXT_PRESETS.iter().enumerate() {
                    options = options.child(
                        div()
                            .id(("text-style-choice", i))
                            .px(px(10.))
                            .py(px(9.))
                            .rounded(px(5.))
                            .bg(if active == Some(i) {
                                rgb(0xe6efff)
                            } else {
                                white().into()
                            })
                            .hover(|d| d.bg(rgb(0xf1f5fc)))
                            .cursor_pointer()
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .font_family(
                                                preset
                                                    .family
                                                    .map(str::to_string)
                                                    .unwrap_or_else(|| current_family.clone()),
                                            )
                                            .font_weight(FontWeight(preset.weight))
                                            .when(preset.italic, |d| d.italic())
                                            .text_size(px(16.))
                                            .child(preset.name),
                                    )
                                    .child(label(format!("{:.0} pt", preset.size))),
                            )
                            .child(label(preset.description).mt(px(3.)))
                            .on_click(cx.listener(move |s, _, _, cx| {
                                s.type_preset(i);
                                s.close_popup();
                                cx.notify();
                            })),
                    );
                }
                div().child(options)
            }
            Popup::TextWeights => {
                let styles = self.format_values();
                let face = renderer::FONT_CHOICES
                    .iter()
                    .find(|f| f.family == styles[0].family)
                    .unwrap_or(&renderer::FONT_CHOICES[0]);
                let mut options = div().flex().flex_col().gap(px(3.));
                for (i, (weight, name)) in face.weights.iter().enumerate() {
                    let weight = *weight;
                    options = options.child(
                        div()
                            .id(("text-weight-choice", i))
                            .h(px(36.))
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .rounded(px(5.))
                            .font_family(face.family)
                            .font_weight(FontWeight(weight))
                            .bg(if styles.iter().all(|s| s.weight == weight) {
                                rgb(0xe6efff)
                            } else {
                                white().into()
                            })
                            .hover(|d| d.bg(rgb(0xf1f5fc)))
                            .cursor_pointer()
                            .child(*name)
                            .child(label(format!("{weight:.0}")))
                            .on_click(cx.listener(move |s, _, _, cx| {
                                s.format(|v| v.weight = weight);
                                s.close_popup();
                                cx.notify();
                            })),
                    );
                }
                options.child(label("Weights available for this font").mt(px(8.)))
            }
            Popup::TextSizes => {
                let styles = self.format_values();
                let mut options = div().flex().flex_wrap().gap(px(6.));
                for (i, value) in [
                    10., 11., 12., 14., 16., 18., 20., 24., 28., 32., 40., 48., 64., 72., 96.,
                ]
                .into_iter()
                .enumerate()
                {
                    options = options.child(
                        div()
                            .id(("text-size-choice", i))
                            .w(px(86.))
                            .h(px(34.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.))
                            .bg(if styles.iter().all(|s| s.size == value) {
                                rgb(0xe6efff)
                            } else {
                                rgb(0xf4f5f2)
                            })
                            .hover(|d| d.bg(rgb(0xeaf0fa)))
                            .cursor_pointer()
                            .child(format!("{value:.0}"))
                            .on_click(cx.listener(move |s, _, _, cx| {
                                s.format(|v| v.size = value);
                                s.close_popup();
                                cx.notify();
                            })),
                    );
                }
                options
            }
            _ => div(),
        }
    }
}
