//! Underline decoration shared by form fields and launcher search.
//! The child retains all input state, focus operations and overlays.

use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer, widget::Tree,
};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

use crate::{Message, theme};

type InputState =
    iced::widget::text_input::State<<iced::Renderer as iced::advanced::text::Renderer>::Paragraph>;

pub(crate) fn underline<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    Element::new(Underline {
        content,
        input: false,
    })
}

/// The direct child must be a text input, so its actual focus state drives
/// the line without a second application-level focus model.
pub(crate) fn input<'a>(content: iced::widget::TextInput<'a, Message>) -> Element<'a, Message> {
    Element::new(Underline {
        content: content.into(),
        input: true,
    })
}

struct Underline<'a> {
    content: Element<'a, Message>,
    input: bool,
}

impl Underline<'_> {
    fn focused(&self, tree: &Tree) -> bool {
        self.input
            && tree.children[0]
                .state
                .downcast_ref::<InputState>()
                .is_focused()
    }
}

impl Widget<Message, iced::Theme, iced::Renderer> for Underline<'_> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        let (color, height) = theme::input_underline(self.focused(tree));
        let bounds = layout.bounds();
        if let Some(clip) = bounds.intersection(viewport) {
            renderer.with_layer(clip, |renderer| {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            y: bounds.y + bounds.height - height,
                            height,
                            ..bounds
                        },
                        ..Default::default()
                    },
                    color,
                )
            });
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'a, Message, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn input_focus_operation_reaches_the_decorated_child() {
        use iced::advanced::{renderer::Headless, widget::operation};
        let renderer = futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            16.0.into(),
            None,
        ))
        .expect("headless wgpu adapter");
        let mut field = Underline {
            content: iced::widget::text_input("Name", "Example")
                .id("focus-test")
                .on_input(|_| Message::Back)
                .into(),
            input: true,
        };
        let mut tree = Tree::new(&field as &dyn Widget<Message, iced::Theme, iced::Renderer>);
        let node = field.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(300.0, 56.0)),
        );
        assert!(!field.focused(&tree));
        field.operate(
            &mut tree,
            Layout::new(&node),
            &renderer,
            &mut operation::focusable::focus::<()>(iced::widget::Id::new("focus-test")),
        );
        assert!(field.focused(&tree));
        field.operate(
            &mut tree,
            Layout::new(&node),
            &renderer,
            &mut operation::focusable::unfocus::<()>(),
        );
        assert!(!field.focused(&tree));
    }
}
