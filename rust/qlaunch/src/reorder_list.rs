//! Grip-only reordering: scroll and ordinary row controls keep their gestures.

use crate::{Message, theme};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, layout, mouse, renderer};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector, touch, window};
use std::time::{Duration, Instant};

pub(crate) struct Entry<'a> {
    pub id: String,
    pub content: Element<'a, Message>,
}

pub(crate) struct ReorderList<'a> {
    entries: Vec<Entry<'a>>,
}

impl<'a> ReorderList<'a> {
    pub(crate) fn new(entries: Vec<Entry<'a>>) -> Self {
        Self { entries }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Pointer {
    Mouse,
    Touch(touch::Finger),
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    index: usize,
    pointer: Pointer,
    position: Point,
    grab_offset: f32,
    last_frame: Option<Instant>,
}

#[derive(Default)]
struct State {
    keys: Vec<String>,
    drag: Option<Drag>,
}

fn stride() -> f32 {
    theme::metrics::GROUP_CARD_HEIGHT + theme::metrics::ACTION_GAP
}

// Scrollable translates the viewport/cursor, but not the event's position.
// Keep the active pointer in window coordinates and convert it afresh for each
// layout/draw/update, including when edge scrolling moves a stationary pointer.
fn scroll_offset(bounds: Rectangle, viewport: Rectangle) -> f32 {
    (viewport.y + theme::metrics::GROUP_LIST_PADDING_Y - bounds.y).max(0.0)
}

fn destination(drag: Drag, bounds: Rectangle, count: usize, offset: f32) -> usize {
    let top = drag.position.y + offset - drag.grab_offset - bounds.y;
    (((top + theme::metrics::GROUP_CARD_HEIGHT / 2.0) / stride())
        .floor()
        .max(0.0) as usize)
        .min(count.saturating_sub(1))
}

fn translation(
    drag: Option<Drag>,
    bounds: Rectangle,
    count: usize,
    index: usize,
    offset: f32,
) -> Vector {
    let Some(drag) = drag else {
        return Vector::ZERO;
    };
    let target = destination(drag, bounds, count, offset);
    let y = if index == drag.index {
        (drag.position.y + offset - drag.grab_offset - bounds.y)
            .clamp(0.0, count.saturating_sub(1) as f32 * stride())
            - index as f32 * stride()
    } else if drag.index < index && index <= target {
        -stride()
    } else if target <= index && index < drag.index {
        stride()
    } else {
        0.0
    };
    Vector::new(0.0, y)
}

impl Widget<Message, iced::Theme, iced::Renderer> for ReorderList<'_> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            keys: self.entries.iter().map(|entry| entry.id.clone()).collect(),
            drag: None,
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.entries
            .iter()
            .map(|entry| Tree::new(&entry.content))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let keys: Vec<_> = self.entries.iter().map(|entry| entry.id.clone()).collect();
        let state = tree.state.downcast_mut::<State>();
        if state.keys != keys {
            state.keys = keys;
            state.drag = None;
            tree.children = self.children();
        } else {
            tree.diff_children(
                &self
                    .entries
                    .iter()
                    .map(|entry| &entry.content)
                    .collect::<Vec<_>>(),
            );
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let height = self.entries.len() as f32 * stride()
            - if self.entries.is_empty() {
                0.0
            } else {
                theme::metrics::ACTION_GAP
            };
        let size = limits.resolve(Length::Fill, height, Size::ZERO);
        let children = self
            .entries
            .iter_mut()
            .zip(&mut tree.children)
            .enumerate()
            .map(|(index, (entry, tree))| {
                entry
                    .content
                    .as_widget_mut()
                    .layout(
                        tree,
                        renderer,
                        &layout::Limits::new(
                            Size::new(size.width, theme::metrics::GROUP_CARD_HEIGHT),
                            Size::new(size.width, theme::metrics::GROUP_CARD_HEIGHT),
                        ),
                    )
                    .move_to(Point::new(0.0, index as f32 * stride()))
            })
            .collect();
        layout::Node::with_children(size, children)
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
        let Some(clip) = layout.bounds().intersection(viewport) else {
            return;
        };
        let drag = tree.state.downcast_ref::<State>().drag;
        // Draw the lifted card last so it floats above the rows it passes.
        let indices = (0..self.entries.len())
            .filter(|index| drag.is_none_or(|d| d.index != *index))
            .chain(drag.map(|d| d.index));
        renderer.with_layer(clip, |renderer| {
            for index in indices {
                let shift = translation(
                    drag,
                    layout.bounds(),
                    self.entries.len(),
                    index,
                    scroll_offset(layout.bounds(), *viewport),
                );
                let child_viewport = Rectangle::new(viewport.position() - shift, viewport.size());
                renderer.with_translation(shift, |renderer| {
                    self.entries[index].content.as_widget().draw(
                        &tree.children[index],
                        renderer,
                        theme,
                        style,
                        layout.child(index),
                        if drag.is_some() {
                            mouse::Cursor::Unavailable
                        } else {
                            cursor
                        },
                        &child_viewport,
                    );
                });
            }
        });
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
        let state = tree.state.downcast_mut::<State>();
        let offset = scroll_offset(layout.bounds(), *viewport);
        if let Some(mut drag) = state.drag {
            let movement = match event {
                Event::Touch(touch::Event::FingerMoved { id, position })
                    if drag.pointer == Pointer::Touch(*id) =>
                {
                    Some(*position)
                }
                Event::Mouse(mouse::Event::CursorMoved { position })
                    if drag.pointer == Pointer::Mouse =>
                {
                    Some(*position)
                }
                _ => None,
            };
            if let Some(position) = movement {
                drag.position = position;
            }
            let released = match event {
                Event::Touch(touch::Event::FingerLifted { id, position })
                    if drag.pointer == Pointer::Touch(*id) =>
                {
                    drag.position = *position;
                    true
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                    if drag.pointer == Pointer::Mouse =>
                {
                    true
                }
                _ => false,
            };
            let cancelled = matches!(event, Event::Touch(touch::Event::FingerLost { id, .. }) if drag.pointer == Pointer::Touch(*id))
                || matches!(event, Event::Window(window::Event::Unfocused));
            if released || cancelled {
                state.drag = None;
                if released && !cancelled {
                    let target = destination(drag, layout.bounds(), self.entries.len(), offset);
                    if target != drag.index {
                        shell.publish(Message::GroupMove(
                            self.entries[drag.index].id.clone(),
                            target as i32,
                        ));
                    }
                }
            } else {
                // Keep moving through long lists while the grip is held at an edge.
                if let Event::Window(window::Event::RedrawRequested(now)) = event {
                    let elapsed = drag
                        .last_frame
                        .map_or(1.0 / 60.0, |last| {
                            now.saturating_duration_since(last).as_secs_f32()
                        })
                        .min(0.05);
                    drag.last_frame = Some(*now);
                    let content_y = drag.position.y + offset;
                    let direction = if content_y < viewport.y + 40.0 {
                        -1.0
                    } else if content_y > viewport.y + viewport.height - 40.0 {
                        1.0
                    } else {
                        0.0
                    };
                    let maximum = (layout.bounds().height
                        + theme::metrics::GROUP_LIST_PADDING_Y * 2.0
                        - viewport.height)
                        .max(0.0);
                    let offset = offset.clamp(0.0, maximum);
                    if direction != 0.0
                        && ((direction < 0.0 && offset > 0.0)
                            || (direction > 0.0 && offset < maximum))
                    {
                        shell.publish(Message::GroupDragScroll(
                            (offset + direction * 360.0 * elapsed).clamp(0.0, maximum),
                        ));
                        shell.request_redraw_at(*now + Duration::from_millis(16));
                    }
                }
                state.drag = Some(drag);
            }
            if movement.is_some() || released || cancelled {
                shell.request_redraw();
            }
            if matches!(event, Event::Touch(_) | Event::Mouse(_)) {
                shell.capture_event();
            }
            return;
        }
        let press = match event {
            Event::Touch(touch::Event::FingerPressed { id, position }) => {
                Some((Pointer::Touch(*id), *position))
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => cursor
                .position()
                .map(|p| (Pointer::Mouse, p - Vector::new(0.0, offset))),
            _ => None,
        };
        if let Some((pointer, position)) =
            press.filter(|(_, p)| viewport.contains(*p + Vector::new(0.0, offset)))
        {
            let content_position = position + Vector::new(0.0, offset);
            for (index, child) in layout.children().enumerate() {
                let handle = Rectangle {
                    width: theme::metrics::TOUCH_TARGET,
                    ..child.bounds()
                };
                if handle.contains(content_position) {
                    state.drag = Some(Drag {
                        index,
                        pointer,
                        position,
                        grab_offset: content_position.y - child.bounds().y,
                        last_frame: None,
                    });
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }
            }
        }
        for ((entry, tree), child) in self
            .entries
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            entry.content.as_widget_mut().update(
                tree, event, child, cursor, renderer, clipboard, shell, viewport,
            );
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
        if tree.state.downcast_ref::<State>().drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        for (index, child) in layout.children().enumerate() {
            let handle = Rectangle {
                width: theme::metrics::TOUCH_TARGET,
                ..child.bounds()
            };
            if cursor.is_over(handle) && cursor.is_over(*viewport) {
                return mouse::Interaction::Grab;
            }
            let interaction = self.entries[index].content.as_widget().mouse_interaction(
                &tree.children[index],
                child,
                cursor,
                viewport,
                renderer,
            );
            if interaction != mouse::Interaction::None {
                return interaction;
            }
        }
        mouse::Interaction::None
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        for ((entry, tree), child) in self
            .entries
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            entry
                .content
                .as_widget_mut()
                .operate(tree, child, renderer, operation);
        }
    }
}

impl<'a> From<ReorderList<'a>> for Element<'a, Message> {
    fn from(list: ReorderList<'a>) -> Self {
        Element::new(list)
    }
}
