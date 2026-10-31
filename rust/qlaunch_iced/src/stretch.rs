//! Velocity-proportional elastic stretch shared by both axes.
//!
//! The pinned renderer can only scale uniformly, so stretch is faked the same
//! way as the edge bounce: translate the drawn subtree along the gesture axis.
//! A scroller or the pager feeds a speed into the region's [`Motion`]; this
//! module turns it into a bounded lag and draws the wrapped subtree there.

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

use crate::Message;
use crate::elastic_scroll::Motion;

/// A drag slower than this (logical pixels per second) produces no stretch.
const SPEED_FLOOR: f32 = 120.0;
/// How quickly the lag approaches its cap; larger is more eager.
const EAGERNESS: f32 = 8.0;

/// Which way a stretch moves its subtree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
    Vertical,
    Horizontal,
}

/// Bounded, signed lag for a scroll/offset speed, in logical pixels per second.
///
/// Below [`SPEED_FLOOR`] there is no stretch, so a slow drag or a settled list
/// is perfectly still; the magnitude approaches `limit` asymptotically.
pub(crate) fn lag(speed: f32, limit: f32) -> f32 {
    if !speed.is_finite() || !limit.is_finite() || limit <= 0.0 || speed.abs() < SPEED_FLOOR {
        return 0.0;
    }
    speed.signum() * limit * (1.0 - (-speed.abs() / (limit * EAGERNESS)).exp())
}

/// The translation a stretch applies, on its own axis only.
pub(crate) fn vector(axis: Axis, total: f32) -> Vector {
    match axis {
        Axis::Vertical => Vector::new(0.0, total),
        Axis::Horizontal => Vector::new(total, 0.0),
    }
}

/// Wrap a subtree so it is drawn at the region's stretch displacement.
pub(crate) fn wrap<'a>(
    child: Element<'a, Message>,
    motion: &Motion,
    axis: Axis,
) -> Element<'a, Message> {
    Element::new(Stretch {
        child,
        motion: motion.clone(),
        axis,
    })
}

struct Stretch<'a> {
    child: Element<'a, Message>,
    motion: Motion,
    axis: Axis,
}

impl Widget<Message, iced::Theme, iced::Renderer> for Stretch<'_> {
    fn size(&self) -> Size<Length> {
        self.child.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.child.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.child)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.child));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.child
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.child
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        self.child.as_widget_mut().update(
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
        let total = self.motion.values().total;
        if total == 0.0 {
            self.child.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
            return;
        }
        if let Some(clip) = layout.bounds().intersection(viewport) {
            renderer.with_layer(clip, |renderer| {
                renderer.with_translation(vector(self.axis, total), |renderer| {
                    self.child.as_widget().draw(
                        &tree.children[0],
                        renderer,
                        theme,
                        style,
                        layout,
                        cursor,
                        viewport,
                    );
                });
            });
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.child.as_widget().mouse_interaction(
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
        self.child.as_widget_mut().overlay(
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
    fn lag_is_signed_bounded_and_monotonic() {
        let limit = 40.0;
        assert_eq!(lag(0.0, limit), 0.0);
        assert_eq!(lag(50.0, limit), 0.0, "below the speed floor");
        let slow = lag(600.0, limit);
        let fast = lag(2400.0, limit);
        assert!(slow > 0.0 && slow < fast, "{slow} {fast}");
        assert!(fast < limit);
        assert_eq!(lag(-600.0, limit), -slow);
        assert_eq!(lag(f32::NAN, limit), 0.0);
        assert_eq!(lag(1000.0, 0.0), 0.0);
    }

    #[test]
    fn vector_moves_only_along_its_axis() {
        assert_eq!(vector(Axis::Vertical, 5.0), Vector::new(0.0, 5.0));
        assert_eq!(vector(Axis::Horizontal, -3.0), Vector::new(-3.0, 0.0));
    }

    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn wrap_is_layout_transparent_and_forwards_operations() {
        use iced::advanced::renderer::Headless;
        use iced::advanced::widget::Operation;
        use iced::advanced::widget::operation::Scrollable;
        use iced::advanced::{layout, widget::Tree};
        use iced::widget::{column, container, scrollable};
        use iced::{Fill, Font, Size};

        struct Probe(usize);
        impl Operation for Probe {
            fn traverse(
                &mut self,
                operate: &mut dyn FnMut(&mut dyn Operation<()>),
            ) {
                operate(self);
            }
            fn scrollable(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: Rectangle,
                _content_bounds: Rectangle,
                _translation: Vector,
                _state: &mut dyn Scrollable,
            ) {
                self.0 += 1;
            }
        }

        let renderer = futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            16.0.into(),
            None,
        ))
        .expect("headless wgpu adapter");
        let mut rows = column![];
        for index in 0..30 {
            rows = rows.push(container(iced::widget::text(format!("{index}"))).height(48.0));
        }
        let limits = layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0));
        let mut bare: Element<'static, Message> = scrollable(rows)
            .id("stretch-scroll")
            .width(Fill)
            .height(Fill)
            .into();
        let mut bare_tree = Tree::new(&bare);
        let bare_node = bare
            .as_widget_mut()
            .layout(&mut bare_tree, &renderer, &limits);

        let motion = Motion::new();
        motion.set(12.0, crate::elastic_scroll::Edge::Top);
        let mut rows = column![];
        for index in 0..30 {
            rows = rows.push(container(iced::widget::text(format!("{index}"))).height(48.0));
        }
        let mut wrapped = wrap(
            scrollable(rows).id("stretch-scroll").width(Fill).height(Fill).into(),
            &motion,
            Axis::Horizontal,
        );
        let mut wrapped_tree = Tree::new(&wrapped);
        let wrapped_node = wrapped
            .as_widget_mut()
            .layout(&mut wrapped_tree, &renderer, &limits);
        assert_eq!(wrapped_node.size(), bare_node.size());

        let mut probe = Probe(0);
        wrapped
            .as_widget_mut()
            .operate(&mut wrapped_tree, Layout::new(&wrapped_node), &renderer, &mut probe);
        assert_eq!(probe.0, 1, "the scrollable below the wrapper is reachable");
    }
}
