use std::cmp;

use gpui::{
    AnyElement, App, BorderStyle, Bounds, Corners, Edges, HighlightStyle, Hsla, StyledText,
    TextLayout, TextStyle, point, prelude::*, quad, size,
};
use settings::Settings;
use theme::ThemeSettings;
use ui::prelude::*;

pub struct CompletionDiffElement {
    element: AnyElement,
    text_layout: TextLayout,
    cursor_offset: usize,
}

impl CompletionDiffElement {
    pub fn new(prediction_text: &str, cx: &App) -> Self {
        log::debug!("CopilotV2 UI: Creating CompletionDiffElement with text: '{}'", prediction_text);

        // For now, create a simple styled text element for mock completions
        // In the future, this would process actual edit diffs like Zeta does

        let mut diff_highlights = Vec::new();

        // Add green background for the entire completion text to show it's new
        if !prediction_text.is_empty() {
            diff_highlights.push((
                0..prediction_text.len(),
                HighlightStyle {
                    background_color: Some(cx.theme().status().created_background),
                    ..Default::default()
                },
            ));
            log::debug!("CopilotV2 UI: Added creation highlight for entire text");
        }

        // Apply theme and styling
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

        let element = StyledText::new(prediction_text.to_string()).with_default_highlights(&text_style, diff_highlights);
        let text_layout = element.layout().clone();

        log::debug!("CopilotV2 UI: CompletionDiffElement created successfully");

        CompletionDiffElement {
            element: element.into_any_element(),
            text_layout,
            cursor_offset: 0, // For mock implementations, start at beginning
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
        log::debug!("CopilotV2 UI: Painting CompletionDiffElement");

        // Paint active line background and cursor
        if let Some(position) = self.text_layout.position_for_index(self.cursor_offset) {
            let bounds = self.text_layout.bounds();
            let line_height = self.text_layout.line_height();
            let line_width = self
                .text_layout
                .line_layout_for_index(self.cursor_offset)
                .map_or(bounds.size.width, |layout| layout.width());

            // Paint active line background
            window.paint_quad(quad(
                Bounds::new(
                    point(bounds.origin.x, position.y),
                    size(cmp::max(bounds.size.width, line_width), line_height),
                ),
                Corners::default(),
                cx.theme().colors().editor_active_line_background,
                Edges::default(),
                Hsla::transparent_black(),
                BorderStyle::default(),
            ));

            log::debug!("CopilotV2 UI: Painted active line background at {:?}", position);

            // Paint the text with diff highlights
            self.element.paint(window, cx);

            // Paint cursor
            window.paint_quad(quad(
                Bounds::new(position, size(px(2.), line_height)),
                Corners::default(),
                cx.theme().players().local().cursor,
                Edges::default(),
                Hsla::transparent_black(),
                BorderStyle::default(),
            ));

            log::debug!("CopilotV2 UI: Painted cursor at {:?}", position);
        } else {
            // Just paint the text if we can't determine cursor position
            self.element.paint(window, cx);
            log::debug!("CopilotV2 UI: Painted text without cursor (position not found)");
        }
    }
}