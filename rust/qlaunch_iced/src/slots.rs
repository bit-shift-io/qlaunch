//! Three slotted children in one clipped layer.
//!
//! The launcher needs two three-slot rows: the wrapped pages the pager drags
//! and the title strip. Both were drawn by translating `float` overlays, which
//! take input before the rest of the tree and cannot gate it. This widget draws
//! the same geometry with one clip layer and decides which slot an event
//! reaches, so a slot that is mid-motion receives nothing.

use crate::Message;
use iced::advanced::layout;
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Renderer, Shell, mouse, renderer};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector, window};

/// One slot's content and the semantic key it belongs to.
pub(crate) struct Slot<'a> {
    pub key: String,
    pub content: Element<'a, Message>,
}

/// Which slots may receive input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Interaction {
    /// None of them: the row is moving.
    None,
    /// Only the middle slot: the pages, when the pager is settled.
    Centre,
    /// Every slot: the title strip, when the pager is settled.
    All,
}

/// How the three slots are arranged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arrangement {
    /// Full-width pages side by side, dragged by the pager offset.
    Pages,
    /// Three equal thirds of the width, shifted together.
    Strip,
}

/// The slot's own rectangle, before the arrangement's translation.
pub(crate) fn child_bounds(
    arrangement: Arrangement,
    index: usize,
    width: f32,
    height: f32,
) -> Rectangle {
    match arrangement {
        Arrangement::Pages => Rectangle::new(Point::ORIGIN, Size::new(width, height)),
        Arrangement::Strip => {
            let third = width / 3.0;
            Rectangle::new(
                Point::new(index as f32 * third, 0.0),
                Size::new(third, height),
            )
        }
    }
}

/// Where the slot is drawn, relative to its own rectangle.
pub(crate) fn translation(
    arrangement: Arrangement,
    index: usize,
    offset: f32,
    width: f32,
) -> Vector {
    match arrangement {
        Arrangement::Pages => Vector::new((index as f32 - 1.0 - offset) * width, 0.0),
        Arrangement::Strip => Vector::new(-offset * width / 3.0, 0.0),
    }
}

/// Whether an event is the window's redraw request, which every arrangement
/// forwards to its centre slot so images and animations keep up.
fn is_redraw(event: &Event) -> bool {
    matches!(event, Event::Window(window::Event::RedrawRequested(_)))
}

/// Which slot an event reaches.
fn reaches(interaction: Interaction, index: usize, redraw: bool) -> bool {
    if redraw {
        return index == 1;
    }

    match interaction {
        Interaction::None => false,
        Interaction::Centre => index == 1,
        Interaction::All => index < 3,
    }
}

/// Where one slot sits and what its content is given for an event: the
/// translation its draw applies, plus its own cursor and clip.
///
/// `draw` needs this for every slot, because the settled page's neighbours are
/// visible while the pager is moving even though they take no input.
fn slot_geometry(
    arrangement: Arrangement,
    offset: f32,
    width: f32,
    index: usize,
    cursor: mouse::Cursor,
    viewport: &Rectangle,
) -> (mouse::Cursor, Rectangle, Vector) {
    let shift = translation(arrangement, index, offset, width);
    let (cursor, clip) = child_input(cursor, viewport, shift);

    (cursor, clip, shift)
}

/// [`slot_geometry`] for the slots an event is allowed to reach; `None` means
/// the slot receives nothing at all. `update` and `mouse_interaction` take
/// their arguments from this, so the gating rule, the inverse translation and
/// the clip cannot drift apart.
#[allow(clippy::too_many_arguments)]
fn slot_input(
    interaction: Interaction,
    arrangement: Arrangement,
    offset: f32,
    width: f32,
    index: usize,
    redraw: bool,
    cursor: mouse::Cursor,
    viewport: &Rectangle,
) -> Option<(mouse::Cursor, Rectangle, Vector)> {
    if !reaches(interaction, index, redraw) {
        return None;
    }

    Some(slot_geometry(
        arrangement,
        offset,
        width,
        index,
        cursor,
        viewport,
    ))
}

/// The cursor and clip a slot's content is given, moved into the slot's own
/// coordinates by the inverse of its translation.
fn child_input(
    cursor: mouse::Cursor,
    viewport: &Rectangle,
    translation: Vector,
) -> (mouse::Cursor, Rectangle) {
    let cursor = match cursor {
        mouse::Cursor::Available(position) => mouse::Cursor::Available(position - translation),
        mouse::Cursor::Levitating(position) => mouse::Cursor::Levitating(position - translation),
        mouse::Cursor::Unavailable => mouse::Cursor::Unavailable,
    };

    (
        cursor,
        Rectangle::new(viewport.position() - translation, viewport.size()),
    )
}

/// The three semantic keys of the current slots, kept in the widget state so a
/// slot that changes group gets a fresh tree instead of a stale scroll offset.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotKeys([String; 3]);

pub(crate) struct Slots<'a> {
    slots: [Slot<'a>; 3],
    arrangement: Arrangement,
    offset: f32,
    width: f32,
    height: Length,
    interaction: Interaction,
}

impl<'a> Slots<'a> {
    /// Three full-size pages at `offset`, each `width` wide.
    pub(crate) fn pages(
        slots: [Slot<'a>; 3],
        offset: f32,
        width: f32,
        interaction: Interaction,
    ) -> Self {
        Slots {
            slots,
            arrangement: Arrangement::Pages,
            offset,
            width,
            height: Length::Fill,
            interaction,
        }
    }

    /// Three third-width slots in a row `height` tall.
    pub(crate) fn strip(
        slots: [Slot<'a>; 3],
        offset: f32,
        width: f32,
        height: f32,
        interaction: Interaction,
    ) -> Self {
        Slots {
            slots,
            arrangement: Arrangement::Strip,
            offset,
            width,
            height: Length::Fixed(height),
            interaction,
        }
    }

    fn keys(&self) -> SlotKeys {
        SlotKeys(std::array::from_fn(|index| self.slots[index].key.clone()))
    }
}

impl Widget<Message, iced::Theme, iced::Renderer> for Slots<'_> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, self.height)
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SlotKeys>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(self.keys())
    }

    fn children(&self) -> Vec<Tree> {
        self.slots
            .iter()
            .map(|slot| Tree::new(slot.content.as_widget()))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.len() != self.slots.len() {
            tree.children = self.children();
            tree.state = tree::State::new(self.keys());
            return;
        }

        // A slot whose group changed gets a fresh tree. Reconciling it instead
        // would carry the previous group's scroll offset into the new one.
        let previous = match &tree.state {
            tree::State::Some(_) => Some(tree.state.downcast_ref::<SlotKeys>().0.clone()),
            tree::State::None => None,
        };
        for (index, slot) in self.slots.iter().enumerate() {
            let unchanged = previous
                .as_ref()
                .is_some_and(|keys| keys[index] == slot.key);
            if unchanged {
                // `Tree::diff` checks the tag first, so content of a different
                // type is rebuilt instead of being reconciled against stale
                // state.
                tree.children[index].diff(slot.content.as_widget());
            } else {
                tree.children[index] = Tree::new(slot.content.as_widget());
            }
        }
        tree.state = tree::State::new(self.keys());
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, self.height, Size::ZERO);
        let mut children = Vec::with_capacity(self.slots.len());

        for (index, (slot, child_tree)) in self
            .slots
            .iter_mut()
            .zip(tree.children.iter_mut())
            .enumerate()
        {
            let bounds = child_bounds(self.arrangement, index, self.width, size.height);
            let child_limits = layout::Limits::new(Size::ZERO, bounds.size());
            let node = slot
                .content
                .as_widget_mut()
                .layout(child_tree, renderer, &child_limits);
            children.push(node.move_to(bounds.position()));
        }

        layout::Node::with_children(size, children)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(clip) = layout.bounds().intersection(viewport) else {
            return;
        };

        // One clipped layer holds all three slots; each is drawn at its own
        // translated position, so no slot can paint outside the pages area.
        renderer.with_layer(clip, |renderer| {
            for (index, ((slot, child_tree), child_layout)) in self
                .slots
                .iter()
                .zip(tree.children.iter())
                .zip(layout.children())
                .enumerate()
            {
                // Deliberately ungated: every slot is painted, including the
                // neighbours a drag reveals.
                let (child_cursor, child_viewport, shift) = slot_geometry(
                    self.arrangement,
                    self.offset,
                    self.width,
                    index,
                    cursor,
                    viewport,
                );

                renderer.with_translation(shift, |renderer| {
                    slot.content.as_widget().draw(
                        child_tree,
                        renderer,
                        theme,
                        style,
                        child_layout,
                        child_cursor,
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
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let redraw = is_redraw(event);
        let (arrangement, offset, width, interaction) =
            (self.arrangement, self.offset, self.width, self.interaction);

        for (index, ((slot, child_tree), child_layout)) in self
            .slots
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
            .enumerate()
        {
            // A slot the interaction does not allow receives nothing but a
            // redraw request, so a moving page cannot launch or scroll.
            let Some((child_cursor, child_viewport, _)) = slot_input(
                interaction,
                arrangement,
                offset,
                width,
                index,
                redraw,
                cursor,
                viewport,
            ) else {
                continue;
            };

            slot.content.as_widget_mut().update(
                child_tree,
                event,
                child_layout,
                child_cursor,
                renderer,
                clipboard,
                shell,
                &child_viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let mut interaction = mouse::Interaction::None;
        let (arrangement, offset, width, allowed) =
            (self.arrangement, self.offset, self.width, self.interaction);

        for (index, (slot, child_tree)) in self.slots.iter().zip(tree.children.iter()).enumerate() {
            let Some((child_cursor, child_viewport, _)) = slot_input(
                allowed,
                arrangement,
                offset,
                width,
                index,
                false,
                cursor,
                viewport,
            ) else {
                continue;
            };

            interaction = interaction.max(slot.content.as_widget().mouse_interaction(
                child_tree,
                layout.child(index),
                child_cursor,
                &child_viewport,
                renderer,
            ));
        }

        interaction
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        let allowed = self.interaction;

        for (index, (slot, child_tree)) in self
            .slots
            .iter_mut()
            .zip(tree.children.iter_mut())
            .enumerate()
        {
            // The centre page carries `scroll_to("apps")` and the scroll
            // resets, so operations reach exactly the slots input reaches.
            if !reaches(allowed, index, false) {
                continue;
            }

            slot.content.as_widget_mut().operate(
                child_tree,
                layout.child(index),
                renderer,
                operation,
            );
        }
    }
}

impl<'a> From<Slots<'a>> for Element<'a, Message> {
    fn from(slots: Slots<'a>) -> Self {
        Self::new(slots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::layout;
    use iced::advanced::widget::{Tree, Widget, tree};
    use iced::time::Instant;
    use iced::{Event, Length, Point, Size, mouse, window};
    use std::cell::RefCell;
    use std::marker::PhantomData;
    use std::rc::Rc;

    /// What the runtime did to a recorded child.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Touched {
        Tag(String),
        State(String),
        Diff(String),
    }

    /// A child that records whether it was created or reconciled.
    #[derive(Debug, Clone)]
    struct Recorder {
        key: String,
        log: Rc<RefCell<Vec<Touched>>>,
    }

    type Log = Rc<RefCell<Vec<Touched>>>;

    fn slot(key: &str, log: &Log) -> Slot<'static> {
        Slot {
            key: key.to_string(),
            content: Element::new(Recorder {
                key: key.to_string(),
                log: log.clone(),
            }),
        }
    }

    impl Widget<Message, iced::Theme, iced::Renderer> for Recorder {
        fn size(&self) -> Size<Length> {
            Size::new(Length::Fill, Length::Fill)
        }

        fn tag(&self) -> tree::Tag {
            self.log.borrow_mut().push(Touched::Tag(self.key.clone()));
            tree::Tag::stateless()
        }

        fn state(&self) -> tree::State {
            self.log.borrow_mut().push(Touched::State(self.key.clone()));
            tree::State::None
        }

        fn children(&self) -> Vec<Tree> {
            Vec::new()
        }

        fn diff(&self, _tree: &mut Tree) {
            self.log.borrow_mut().push(Touched::Diff(self.key.clone()));
        }

        fn layout(
            &mut self,
            _tree: &mut Tree,
            _renderer: &iced::Renderer,
            limits: &layout::Limits,
        ) -> layout::Node {
            layout::atomic(limits, Length::Fill, Length::Fill)
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut iced::Renderer,
            _theme: &iced::Theme,
            _style: &renderer::Style,
            _layout: iced::advanced::Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }
    }

    /// Two content types with distinct tags, for the case where a slot's key
    /// stays the same but the widget behind it changes.
    #[derive(Debug)]
    struct First;

    #[derive(Debug)]
    struct Second;

    #[derive(Debug, Clone)]
    struct Typed<M> {
        marker: PhantomData<M>,
        log: Log,
    }

    impl<M: 'static> Widget<Message, iced::Theme, iced::Renderer> for Typed<M> {
        fn size(&self) -> Size<Length> {
            Size::new(Length::Fill, Length::Fill)
        }

        fn tag(&self) -> tree::Tag {
            tree::Tag::of::<PhantomData<M>>()
        }

        fn state(&self) -> tree::State {
            tree::State::new(PhantomData::<M>)
        }

        fn children(&self) -> Vec<Tree> {
            Vec::new()
        }

        fn diff(&self, _tree: &mut Tree) {
            self.log.borrow_mut().push(Touched::Diff("typed".into()));
        }

        fn layout(
            &mut self,
            _tree: &mut Tree,
            _renderer: &iced::Renderer,
            limits: &layout::Limits,
        ) -> layout::Node {
            layout::atomic(limits, Length::Fill, Length::Fill)
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut iced::Renderer,
            _theme: &iced::Theme,
            _style: &renderer::Style,
            _layout: iced::advanced::Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }
    }

    fn typed<M: 'static>(key: &str, log: &Log) -> Slot<'static> {
        Slot {
            key: key.to_string(),
            content: Element::new(Typed::<M> {
                marker: PhantomData,
                log: log.clone(),
            }),
        }
    }

    #[test]
    fn a_new_content_type_under_the_same_key_rebuilds_the_tree() {
        let log: Log = Rc::new(RefCell::new(Vec::new()));
        let first = Slots::pages(
            [typed::<First>("a", &log), slot("b", &log), slot("c", &log)],
            0.0,
            WIDTH,
            Interaction::Centre,
        );
        let mut tree = Tree::new(Element::from(first));
        let replaced = tree.children[0].tag;

        let second = Slots::pages(
            [typed::<Second>("a", &log), slot("b", &log), slot("c", &log)],
            0.0,
            WIDTH,
            Interaction::Centre,
        );
        second.diff(&mut tree);

        assert_ne!(
            tree.children[0].tag, replaced,
            "the unchanged key kept a tree tagged for the old content type",
        );
    }

    #[test]
    fn a_slot_is_given_its_own_cursor_clip_and_translation() {
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(1200.0, 900.0));
        let cursor = mouse::Cursor::Available(Point::new(700.0, 100.0));

        // Gated: no input at all, not even a clip to use.
        assert!(
            slot_input(
                Interaction::None,
                Arrangement::Pages,
                0.0,
                WIDTH,
                1,
                false,
                cursor,
                &viewport,
            )
            .is_none()
        );

        // Settled pages on the centre slot, one page to the left.
        let (slot_cursor, clip, shift) = slot_input(
            Interaction::Centre,
            Arrangement::Pages,
            1.0,
            WIDTH,
            1,
            false,
            cursor,
            &viewport,
        )
        .expect("the centre slot is live");
        assert_eq!(shift, Vector::new(-WIDTH, 0.0));
        assert_eq!(clip.position(), Point::new(WIDTH, 0.0));
        assert_eq!(slot_cursor.position(), Some(Point::new(1300.0, 100.0)));

        // A neighbour of a settled pages row is not live.
        assert!(
            slot_input(
                Interaction::Centre,
                Arrangement::Pages,
                1.0,
                WIDTH,
                2,
                false,
                cursor,
                &viewport,
            )
            .is_none()
        );

        // A redraw still reaches the centre while the row is gated.
        assert!(
            slot_input(
                Interaction::None,
                Arrangement::Strip,
                0.0,
                WIDTH,
                1,
                true,
                cursor,
                &viewport,
            )
            .is_some()
        );
        assert!(
            slot_input(
                Interaction::None,
                Arrangement::Strip,
                0.0,
                WIDTH,
                0,
                true,
                cursor,
                &viewport,
            )
            .is_none()
        );

        // Drawing is ungated: a gated neighbour is still painted where it
        // belongs, which is what makes a drag show the incoming page.
        let (_, neighbour_clip, neighbour_shift) =
            slot_geometry(Arrangement::Pages, 0.5, WIDTH, 2, cursor, &viewport);
        assert_eq!(neighbour_shift, Vector::new(0.5 * WIDTH, 0.0));
        assert_eq!(neighbour_clip.position(), Point::new(-0.5 * WIDTH, 0.0));
    }

    /// Whether a row lets slot `index` take an ordinary event, through the same
    /// function `update` uses.
    fn allows(slots: &Slots<'_>, index: usize, redraw: bool) -> bool {
        slot_input(
            slots.interaction,
            slots.arrangement,
            slots.offset,
            slots.width,
            index,
            redraw,
            mouse::Cursor::Unavailable,
            &Rectangle::new(Point::ORIGIN, Size::new(WIDTH, HEIGHT)),
        )
        .is_some()
    }

    fn pages(log: &Log, keys: [&str; 3], interaction: Interaction) -> Slots<'static> {
        Slots::pages(
            [slot(keys[0], log), slot(keys[1], log), slot(keys[2], log)],
            0.0,
            600.0,
            interaction,
        )
    }

    const WIDTH: f32 = 600.0;
    const HEIGHT: f32 = 48.0;

    #[test]
    fn page_translations_follow_the_pager_offset() {
        for index in 0..3 {
            assert_eq!(
                child_bounds(Arrangement::Pages, index, WIDTH, HEIGHT),
                Rectangle::new(Point::ORIGIN, Size::new(WIDTH, HEIGHT)),
            );
        }
        assert_eq!(
            translation(Arrangement::Pages, 0, 0.0, WIDTH),
            Vector::new(-WIDTH, 0.0),
        );
        assert_eq!(
            translation(Arrangement::Pages, 1, 0.0, WIDTH),
            Vector::new(0.0, 0.0),
        );
        assert_eq!(
            translation(Arrangement::Pages, 2, 0.0, WIDTH),
            Vector::new(WIDTH, 0.0),
        );
        assert_eq!(
            translation(Arrangement::Pages, 1, 0.5, WIDTH),
            Vector::new(-0.5 * WIDTH, 0.0),
        );
    }

    #[test]
    fn strip_children_are_thirds_and_translate_together() {
        let third = WIDTH / 3.0;
        assert_eq!(
            child_bounds(Arrangement::Strip, 0, WIDTH, HEIGHT),
            Rectangle::new(Point::ORIGIN, Size::new(third, HEIGHT)),
        );
        assert_eq!(
            child_bounds(Arrangement::Strip, 1, WIDTH, HEIGHT),
            Rectangle::new(Point::new(third, 0.0), Size::new(third, HEIGHT)),
        );
        assert_eq!(
            child_bounds(Arrangement::Strip, 2, WIDTH, HEIGHT),
            Rectangle::new(Point::new(2.0 * third, 0.0), Size::new(third, HEIGHT)),
        );
        for index in 0..3 {
            assert_eq!(
                translation(Arrangement::Strip, index, 0.5, WIDTH),
                Vector::new(-0.5 * third, 0.0),
            );
            assert_eq!(
                translation(Arrangement::Strip, index, 1.0, WIDTH),
                Vector::new(-third, 0.0),
            );
        }
    }

    #[test]
    fn strip_translation_never_changes_child_spacing() {
        let left = |index: usize, offset: f32| {
            child_bounds(Arrangement::Strip, index, WIDTH, HEIGHT).x
                + translation(Arrangement::Strip, index, offset, WIDTH).x
        };
        for offset in [0.0, 0.25, 0.5, 1.0, 2.9] {
            let first = left(0, offset);
            let second = left(1, offset);
            let third = left(2, offset);
            assert_eq!(second - first, WIDTH / 3.0, "offset {offset}");
            assert_eq!(third - second, WIDTH / 3.0, "offset {offset}");
        }
    }

    #[test]
    fn changed_slot_key_marks_only_that_tree_for_recreation() {
        let log: Log = Rc::new(RefCell::new(Vec::new()));
        let first = pages(&log, ["a", "b", "c"], Interaction::Centre);
        let mut tree = Tree::new(Element::from(first));
        assert_eq!(tree.children.len(), 3);
        log.borrow_mut().clear();

        let second = pages(&log, ["a", "x", "c"], Interaction::Centre);
        second.diff(&mut tree);

        let seen = log.borrow().clone();
        assert!(seen.contains(&Touched::Tag("x".into())), "{seen:?}");
        assert!(seen.contains(&Touched::State("x".into())), "{seen:?}");
        // A recreated tree asks for its tag, state and children; a reconciled
        // one only has `diff` called, after the tag query that `Tree::diff`
        // makes. So state is the signal that a tree was rebuilt.
        assert!(
            !seen
                .iter()
                .any(|entry| matches!(entry, Touched::State(key) if key != "x")),
            "only the changed slot is recreated: {seen:?}"
        );
        assert!(seen.contains(&Touched::Diff("a".into())), "{seen:?}");
        assert!(seen.contains(&Touched::Diff("c".into())), "{seen:?}");
        assert!(!seen.contains(&Touched::Diff("x".into())), "{seen:?}");
        assert_eq!(
            tree.state.downcast_ref::<SlotKeys>(),
            &SlotKeys(["a".into(), "x".into(), "c".into()]),
        );
    }

    #[test]
    fn window_events_reach_the_centre_child_when_input_is_gated() {
        let log: Log = Rc::new(RefCell::new(Vec::new()));
        let gated = pages(&log, ["a", "b", "c"], Interaction::None);
        assert!(is_redraw(&Event::Window(window::Event::RedrawRequested(
            Instant::now()
        ))));
        assert!(!is_redraw(&Event::Window(window::Event::Focused)));

        for index in 0..3 {
            assert_eq!(
                allows(&gated, index, true),
                index == 1,
                "redraw reaches only the centre slot, index {index}",
            );
        }
    }

    #[test]
    fn touch_events_do_not_reach_a_gated_child() {
        let log: Log = Rc::new(RefCell::new(Vec::new()));
        let gated = pages(&log, ["a", "b", "c"], Interaction::None);
        for index in 0..3 {
            assert!(!allows(&gated, index, false), "index {index} is gated");
        }
    }

    #[test]
    fn a_gated_child_is_given_its_own_coordinates() {
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(1200.0, 900.0));
        let shift = Vector::new(-600.0, 0.0);
        let (cursor, clip) = child_input(
            mouse::Cursor::Available(Point::new(500.0, 100.0)),
            &viewport,
            shift,
        );
        assert_eq!(cursor.position(), Some(Point::new(1100.0, 100.0)));
        assert_eq!(
            clip,
            Rectangle::new(Point::new(600.0, 0.0), Size::new(1200.0, 900.0)),
        );
        let (missing, _) = child_input(mouse::Cursor::Unavailable, &viewport, shift);
        assert_eq!(missing.position(), None);
    }

    #[test]
    fn touch_events_reach_the_centre_only_for_pages() {
        let log: Log = Rc::new(RefCell::new(Vec::new()));
        let settled = pages(&log, ["a", "b", "c"], Interaction::Centre);
        assert!(allows(&settled, 1, false));
        assert!(!allows(&settled, 0, false));
        assert!(!allows(&settled, 2, false));
    }

    #[test]
    fn touch_events_reach_all_three_settled_strip_slots() {
        let log: Log = Rc::new(RefCell::new(Vec::new()));
        let settled = Slots::strip(
            [slot("a", &log), slot("b", &log), slot("c", &log)],
            0.0,
            WIDTH,
            HEIGHT,
            Interaction::All,
        );
        for index in 0..3 {
            assert!(allows(&settled, index, false), "index {index} is live");
        }

        let moving = Slots::strip(
            [slot("a", &log), slot("b", &log), slot("c", &log)],
            0.0,
            WIDTH,
            HEIGHT,
            Interaction::None,
        );
        for index in 0..3 {
            assert!(!allows(&moving, index, false), "index {index} is gated");
        }
        assert!(allows(&moving, 1, true));
    }
}
