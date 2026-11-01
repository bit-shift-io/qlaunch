//! Single-line labels that use real font measurements to fit an ellipsis.

use iced::advanced::text::{Paragraph as _, Renderer as _};
use iced::advanced::{
    Layout, Widget, layout, mouse, renderer, text,
    widget::{Tree, tree},
};
use iced::{Element, Fill, Length, Rectangle, Size};

use crate::{Message, theme::Typography};

type Paragraph = <iced::Renderer as text::Renderer>::Paragraph;

pub(crate) fn ellipsized<'a>(value: &str, typography: Typography) -> Element<'a, Message> {
    Element::new(Label {
        value: value.to_owned(),
        typography,
    })
}

struct Label {
    value: String,
    typography: Typography,
}

#[derive(Default)]
struct State {
    paragraph: text::paragraph::Plain<Paragraph>,
    key: Option<(String, Typography, f32)>,
}

impl Label {
    fn text<'a>(&self, content: &'a str) -> text::Text<&'a str> {
        text::Text {
            content,
            bounds: Size::new(f32::INFINITY, self.typography.line_height),
            size: self.typography.size.into(),
            line_height: self.typography.line_height(),
            font: self.typography.font,
            align_x: text::Alignment::Default,
            align_y: iced::alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
        }
    }
}

impl Widget<Message, iced::Theme, iced::Renderer> for Label {
    fn size(&self) -> Size<Length> {
        Size::new(Fill, Length::Fixed(self.typography.line_height))
    }
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Fill, self.typography.line_height, Size::ZERO);
        let state = tree.state.downcast_mut::<State>();
        let key = (self.value.clone(), self.typography, size.width);
        if state.key.as_ref() != Some(&key) {
            let display = fit_label(&self.value, |candidate| {
                Paragraph::with_text(self.text(candidate)).min_width() <= size.width
            });
            state.paragraph.update(self.text(&display));
            state.key = Some(key);
        }
        layout::Node::new(size)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if let Some(clip) = bounds.intersection(viewport) {
            renderer.fill_paragraph(
                tree.state.downcast_ref::<State>().paragraph.raw(),
                bounds.position(),
                style.text_color,
                clip,
            );
        }
    }

    fn operate(
        &mut self,
        _tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        operation.text(None, layout.bounds(), &self.value);
    }
}

fn fit_label(value: &str, mut fits: impl FnMut(&str) -> bool) -> String {
    if fits(value) {
        return value.to_owned();
    }
    if !fits("…") {
        return String::new();
    }
    let boundaries: Vec<usize> = value.char_indices().map(|(index, _)| index).collect();
    let mut low = 0;
    let mut high = boundaries.len();
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        let candidate = format!("{}…", &value[..boundaries[mid - 1]]);
        if fits(&candidate) {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    format!(
        "{}…",
        &value[..if low == 0 { 0 } else { boundaries[low - 1] }]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_labels_end_with_an_ellipsis_instead_of_overflowing() {
        assert_eq!(
            fit_label("Folder name", |s| s.chars().count() <= 6),
            "Folde…"
        );
        assert_eq!(fit_label("Apps", |s| s.chars().count() <= 6), "Apps");
    }

    #[test]
    fn ellipsis_fitting_handles_unicode_and_tiny_widths() {
        assert_eq!(
            fit_label("日本語のフォルダ", |s| s.chars().count() <= 4),
            "日本語…"
        );
        assert_eq!(fit_label("Folder", |s| s.is_empty()), "");
        assert_eq!(fit_label("", |_| true), "");
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn labels_use_measured_width_and_restore_the_full_name_after_resize() {
        use iced::advanced::renderer::Headless;
        let renderer = futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            16.0.into(),
            None,
        ))
        .expect("headless wgpu adapter");
        let mut label = Label {
            value: "A destination with a long name".into(),
            typography: crate::theme::Text::new().role(crate::theme::TextRole::BodyLarge),
        };
        let mut tree = Tree::new(&label as &dyn Widget<Message, iced::Theme, iced::Renderer>);
        label.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(80.0, 48.0)),
        );
        let paragraph = &tree.state.downcast_ref::<State>().paragraph;
        assert!(paragraph.content().ends_with('…'));
        assert!(paragraph.min_width() <= 80.0);
        label.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(800.0, 48.0)),
        );
        assert_eq!(
            tree.state.downcast_ref::<State>().paragraph.content(),
            "A destination with a long name"
        );
    }
}
