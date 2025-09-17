use crate::Completion;
use gpui::{
    AnyElement, App, BorderStyle, Bounds, Corners, Edges, HighlightStyle, Hsla, StyledText,
    TextLayout, TextStyle, point, prelude::*, quad, size,
};
use settings::Settings;
use theme::ThemeSettings;
use ui::prelude::*;

/// Renders a Copilot completion preview in menus and popovers.
pub struct CompletionDiffElement {
    element: AnyElement,
    text_layout: TextLayout,
    cursor_offset: usize,
}

impl CompletionDiffElement {
    pub fn new(completion: &Completion, cx: &App) -> Self {
        let display_text = completion
            .display_text
            .as_deref()
            .unwrap_or(&completion.text);

        let mut highlights = Vec::new();
        if !display_text.is_empty() {
            highlights.push((
                0..display_text.len(),
                HighlightStyle {
                    background_color: Some(cx.theme().status().created_background),
                    ..HighlightStyle::default()
                },
            ));
        }

        let settings = ThemeSettings::get_global(cx).clone();
        let text_style = TextStyle {
            color: cx.theme().colors().editor_foreground,
            font_size: settings.buffer_font_size(cx).into(),
            font_family: settings.buffer_font.family,
            font_features: settings.buffer_font.features,
            font_fallbacks: settings.buffer_font.fallbacks,
            line_height: relative(settings.buffer_line_height.value()),
            font_weight: settings.buffer_font.weight,
            font_style: settings.buffer_font.style,
            ..Default::default()
        };

        let element = StyledText::new(display_text.to_string())
            .with_default_highlights(&text_style, highlights);
        let text_layout = element.layout().clone();

        CompletionDiffElement {
            element: element.into_any_element(),
            text_layout,
            cursor_offset: display_text.len(),
        }
    }
}

impl IntoElement for CompletionDiffElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for CompletionDiffElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        (self.element.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(position) = self.text_layout.position_for_index(self.cursor_offset) {
            let bounds = self.text_layout.bounds();
            let line_height = self.text_layout.line_height();
            let line_width = self
                .text_layout
                .line_layout_for_index(self.cursor_offset)
                .map_or(bounds.size.width, |layout| layout.width());
            window.paint_quad(quad(
                Bounds::new(
                    point(bounds.origin.x, position.y),
                    size(bounds.size.width.max(line_width), line_height),
                ),
                Corners::default(),
                cx.theme().colors().editor_active_line_background,
                Edges::default(),
                Hsla::transparent_black(),
                BorderStyle::default(),
            ));
        }

        self.element.paint(window, cx);
    }
}
