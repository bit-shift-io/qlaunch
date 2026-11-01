//! Frame-phase probe. `metrics` covers `update` and `view`; this wraps the whole
//! launcher tree so `layout` and `draw` (iced's own phases, which the app never
//! calls directly) are measured the same way. It is a pass-through widget.

use crate::Message;
use crate::metrics::Samples;
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Layout, Shell, layout, mouse, renderer};
use iced::{Element, Event, Length, Rectangle, Size};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub(crate) struct PhaseProbe<'a> {
    pub samples: Arc<Mutex<Samples>>,
    pub content: Element<'a, Message>,
}

impl Widget<Message, iced::Theme, iced::Renderer> for PhaseProbe<'_> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
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
        let started = Instant::now();
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        self.record(
            &self.samples,
            |samples, ms| samples.record_layout(ms),
            started,
        );
        node
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
        let started = Instant::now();
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        self.record(
            &self.samples,
            |samples, ms| samples.record_draw(ms),
            started,
        );
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
}

impl PhaseProbe<'_> {
    fn record(
        &self,
        samples: &Arc<Mutex<Samples>>,
        record: impl FnOnce(&mut Samples, f32),
        started: Instant,
    ) {
        // `Samples` ignores the call unless a motion window is open.
        record(
            &mut samples.lock().unwrap(),
            started.elapsed().as_secs_f32() * 1000.0,
        );
    }
}

impl<'a> From<PhaseProbe<'a>> for Element<'a, Message> {
    fn from(probe: PhaseProbe<'a>) -> Self {
        Element::new(probe)
    }
}
