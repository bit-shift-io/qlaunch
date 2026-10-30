//! Flutter's scroll-edge opacity, applied to cell primitives rather than painting
//! over the wallpaper. The scrollable supplies its translated content viewport
//! at draw time, so wheel, touch and programmatic scrolling all use fresh bounds.
use iced::advanced::Renderer as _;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer, widget::Tree};
use iced::{Element, Event, Length, Rectangle, Size};
use qlaunch_core::settings::ScrollFadeStyle;

use crate::Message;

const EXTENT: f32 = 32.0;

fn tile_alpha(top: f32, height: f32, viewport_top: f32, viewport_height: f32) -> f32 {
    if height <= 0.0 {
        return 0.0;
    }
    let visible = (top + height).min(viewport_top + viewport_height) - top.max(viewport_top);
    ((visible / height - 0.5) * 2.0).clamp(0.0, 1.0)
}

struct Edges {
    top: f32,
    bottom: f32,
    extent: f32,
    before: f32,
    after: f32,
}

impl Edges {
    fn new(offset: f32, height: f32, content_height: f32) -> Self {
        Self {
            top: offset,
            bottom: offset + height,
            extent: EXTENT.min(height / 2.0),
            before: (offset / EXTENT).clamp(0.0, 1.0),
            after: ((content_height - offset - height) / EXTENT).clamp(0.0, 1.0),
        }
    }

    fn alpha(&self, y: f32) -> f32 {
        if self.extent <= 0.0 {
            return 1.0;
        }
        let top = 1.0 - self.before * (1.0 - (y - self.top) / self.extent).clamp(0.0, 1.0);
        let bottom = 1.0 - self.after * (1.0 - (self.bottom - y) / self.extent).clamp(0.0, 1.0);
        top.min(bottom)
    }
}

/// `make` must return identical geometry and widget structure for every alpha.
/// Only primitive colours/opacities change; the real child retains input state
/// and text layout, including while the pager is moving horizontally.
pub(crate) fn cell<'a>(
    mode: ScrollFadeStyle,
    row: usize,
    rows: usize,
    row_height: f32,
    make: impl Fn(f32) -> Element<'a, Message> + 'a,
) -> Element<'a, Message> {
    let content = make(1.0);
    Element::new(Fade {
        content,
        make: Box::new(make),
        mode,
        row,
        rows,
        row_height,
    })
}

struct Fade<'a> {
    content: Element<'a, Message>,
    make: Box<dyn Fn(f32) -> Element<'a, Message> + 'a>,
    mode: ScrollFadeStyle,
    row: usize,
    rows: usize,
    row_height: f32,
}

impl Widget<Message, iced::Theme, iced::Renderer> for Fade<'_> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
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
        let bounds = layout.bounds();
        let Some(visible) = bounds.intersection(viewport) else {
            return;
        };
        let draw = |renderer: &mut iced::Renderer, alpha: f32, clip: Rectangle| {
            if alpha <= 0.0 {
                return;
            }
            let faded;
            let content = if alpha == 1.0 {
                &self.content
            } else {
                faded = (self.make)(alpha);
                &faded
            };
            renderer.with_layer(clip, |renderer| {
                content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor,
                    viewport,
                );
            });
        };
        match self.mode {
            ScrollFadeStyle::None => draw(renderer, 1.0, visible),
            ScrollFadeStyle::Tile => draw(
                renderer,
                tile_alpha(bounds.y, bounds.height, viewport.y, viewport.height),
                visible,
            ),
            ScrollFadeStyle::Gradient => {
                let origin = bounds.y - self.row as f32 * self.row_height;
                let edges = Edges::new(
                    viewport.y - origin,
                    viewport.height,
                    self.rows as f32 * self.row_height,
                );
                // Iced has no alpha-mask layer. Approximate the shader with
                // 1 dp bands only inside its two 32 dp ramps; the opaque middle
                // is a single draw. No background quad obscures the wallpaper.
                let mut y = visible.y;
                while y < visible.y + visible.height {
                    let relative = y - origin;
                    let end = if relative < edges.top + edges.extent && edges.before > 0.0 {
                        (y + 1.0).min(origin + edges.top + edges.extent)
                    } else if relative < edges.bottom - edges.extent {
                        origin + edges.bottom - edges.extent
                    } else if edges.after > 0.0 {
                        y + 1.0
                    } else {
                        visible.y + visible.height
                    }
                    .min(visible.y + visible.height);
                    let clip = Rectangle {
                        y,
                        height: end - y,
                        ..visible
                    };
                    draw(renderer, edges.alpha((y + end) / 2.0 - origin), clip);
                    y = end;
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_fades_uniformly_and_disappears_at_half_visible() {
        for (top, expected) in [
            (0.0, 1.0),
            (-25.0, 0.5),
            (-50.0, 0.0),
            (225.0, 0.5),
            (250.0, 0.0),
        ] {
            assert_eq!(tile_alpha(top, 100.0, 0.0, 300.0), expected);
        }
    }

    #[test]
    fn gradient_eases_out_at_real_list_boundaries() {
        // A viewport at offset 16 in a 600 dp list: top strength is half,
        // bottom strength full. The ramp is 32 dp wide.
        let fade = Edges::new(16.0, 300.0, 600.0);
        for (y, expected) in [
            (16.0, 0.5),
            (32.0, 0.75),
            (48.0, 1.0),
            (284.0, 1.0),
            (300.0, 0.5),
            (316.0, 0.0),
        ] {
            assert_eq!(fade.alpha(y), expected);
        }
        assert_eq!(Edges::new(0.0, 300.0, 600.0).alpha(0.0), 1.0);
        assert_eq!(Edges::new(300.0, 300.0, 600.0).alpha(600.0), 1.0);
    }

    #[test]
    fn fitting_content_has_no_gradient_and_short_viewports_do_not_overlap_ramps() {
        let fitting = Edges::new(0.0, 300.0, 200.0);
        assert_eq!(fitting.alpha(0.0), 1.0);
        assert_eq!(fitting.alpha(300.0), 1.0);
        let short = Edges::new(100.0, 40.0, 600.0);
        assert_eq!(short.alpha(100.0), 0.0);
        assert_eq!(short.alpha(110.0), 0.5);
        assert_eq!(short.alpha(120.0), 1.0);
        assert_eq!(short.alpha(130.0), 0.5);
    }

    /// Exercise the real draw path on a transparent surface, not a black
    /// overlay that would hide (rather than reveal) the Android wallpaper.
    #[test]
    #[ignore = "requires a headless wgpu adapter; run explicitly with --ignored"]
    fn rendered_fades_preserve_surface_transparency() {
        use iced::advanced::renderer::{Headless, Style};
        use iced::widget::{column, container, scrollable};
        use iced::{Color, Font, Point};

        let mut renderer = futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            16.0.into(),
            None,
        ))
        .expect("headless wgpu adapter");
        for (mode, first_alpha) in [
            (ScrollFadeStyle::None, 255),
            (ScrollFadeStyle::Tile, 127),
            (ScrollFadeStyle::Gradient, 59),
        ] {
            // First row is 25% outside the viewport; bottom has more content.
            let make = |alpha| {
                container(iced::widget::space())
                    .width(100)
                    .height(100)
                    .style(move |_| iced::widget::container::Style {
                        background: Some(Color::from_rgba(1.0, 0.0, 0.0, alpha).into()),
                        ..Default::default()
                    })
                    .into()
            };
            let mut content: Element<'_, Message> = scrollable(column![
                cell(mode, 0, 4, 100.0, make),
                cell(mode, 1, 4, 100.0, make),
                cell(mode, 2, 4, 100.0, make),
                cell(mode, 3, 4, 100.0, make),
            ])
            .id("fade-test")
            .width(100)
            .height(250)
            .into();
            let mut tree = Tree::new(&content);
            let node = content.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, Size::new(100.0, 250.0)),
            );
            // Set the actual scrollable state so its viewport is translated.
            let mut operation = iced::advanced::widget::operation::scrollable::scroll_to::<()>(
                iced::advanced::widget::Id::new("fade-test"),
                iced::widget::scrollable::AbsoluteOffset {
                    x: Some(0.0),
                    y: Some(25.0),
                },
            );
            content.as_widget_mut().operate(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &mut operation,
            );
            renderer.reset(Rectangle::with_size(Size::new(100.0, 250.0)));
            content.as_widget().draw(
                &tree,
                &mut renderer,
                &iced::Theme::Dark,
                &Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(Size::new(100.0, 250.0)),
            );
            let pixels =
                Headless::screenshot(&mut renderer, Size::new(100, 250), 1.0, Color::TRANSPARENT);
            let alpha_at =
                |point: Point| pixels[((point.y as usize * 100 + point.x as usize) * 4) + 3] as i32;
            assert!(
                (alpha_at(Point::new(50.0, 0.0)) - first_alpha).abs() <= 2,
                "{mode:?}"
            );
            assert_eq!(
                alpha_at(Point::new(50.0, 120.0)),
                255,
                "opaque middle: {mode:?}"
            );
        }
    }
}
