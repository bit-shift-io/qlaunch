//! The app icons drawn inside a folder tile.
//!
//! A folder tile shows the first few apps it contains instead of a generic
//! folder glyph: one centred icon, two side by side, three in a triangle, or
//! four in a square. An empty folder (or one whose icons have not loaded yet)
//! has no handles and falls back to the generic glyph in `view::cell`.

use iced::advanced::image::{self, Renderer as _};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Layout, layout, mouse, renderer};
use iced::{Element, Length, Rectangle, Size};

use crate::Message;

/// Side of one sub-icon inside the tile, as a fraction of the tile side.
const ICON: f32 = 0.44;
/// Gap between sub-icons, as a fraction of the tile side.
const GAP: f32 = 0.08;

/// A custom widget so up to four decoded icons can be placed on the tile in one
/// clipped layer, without four nested layout containers.
pub(crate) fn preview<'a>(
    handles: Vec<image::Handle>,
    size: f32,
    alpha: f32,
) -> Element<'a, Message> {
    Element::new(FolderPreview {
        handles,
        size,
        alpha,
    })
}

/// Where each of `count` sub-icons sits inside a `size`-wide square tile. Pure,
/// so the 2/2, 3-triangle and 4-square arrangements are testable without a
/// renderer. The list is empty for an empty folder.
pub(crate) fn preview_rects(size: f32, count: usize) -> Vec<Rectangle> {
    let rect =
        |x: f32, y: f32, side: f32| Rectangle::new(iced::Point::new(x, y), Size::new(side, side));
    match count {
        0 => Vec::new(),
        // One member is drawn as a normal-sized icon, centred.
        1 => {
            let side = size * 0.62;
            vec![rect((size - side) / 2.0, (size - side) / 2.0, side)]
        }
        _ => {
            let side = size * ICON;
            let gap = size * GAP;
            let block = 2.0 * side + gap;
            let start = (size - block) / 2.0;
            match count {
                2 => vec![
                    rect(start, (size - side) / 2.0, side),
                    rect(start + side + gap, (size - side) / 2.0, side),
                ],
                3 => {
                    let top_x = (size - side) / 2.0;
                    let bottom_y = start + side + gap;
                    vec![
                        rect(top_x, start, side),
                        rect(start, bottom_y, side),
                        rect(start + side + gap, bottom_y, side),
                    ]
                }
                _ => vec![
                    rect(start, start, side),
                    rect(start + side + gap, start, side),
                    rect(start, start + side + gap, side),
                    rect(start + side + gap, start + side + gap, side),
                ],
            }
        }
    }
}

struct FolderPreview {
    handles: Vec<image::Handle>,
    size: f32,
    alpha: f32,
}

impl Widget<Message, iced::Theme, iced::Renderer> for FolderPreview {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let side = self.size;
        layout::Node::new(limits.resolve(
            Length::Fixed(side),
            Length::Fixed(side),
            Size::new(side, side),
        ))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        for (handle, cell) in self
            .handles
            .iter()
            .zip(preview_rects(bounds.width, self.handles.len()))
        {
            renderer.draw_image(
                image::Image {
                    handle: handle.clone(),
                    border_radius: Default::default(),
                    filter_method: image::FilterMethod::default(),
                    rotation: iced::Radians(0.0),
                    opacity: self.alpha,
                    snap: true,
                },
                Rectangle::new(
                    iced::Point::new(bounds.x + cell.x, bounds.y + cell.y),
                    cell.size(),
                ),
                clip,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::preview_rects;

    fn assert_inside(size: f32, rects: &[iced::Rectangle]) {
        for rect in rects {
            assert!(rect.x >= -1e-4, "{rect:?} escapes the tile");
            assert!(rect.y >= -1e-4, "{rect:?} escapes the tile");
            assert!(
                rect.x + rect.width <= size + 1e-4,
                "{rect:?} escapes the tile"
            );
            assert!(
                rect.y + rect.height <= size + 1e-4,
                "{rect:?} escapes the tile"
            );
        }
    }

    #[test]
    fn an_empty_folder_has_no_preview_icons() {
        assert!(preview_rects(100.0, 0).is_empty());
    }

    #[test]
    fn one_icon_is_centred() {
        let rects = preview_rects(100.0, 1);
        assert_eq!(rects.len(), 1);
        assert!((rects[0].center_x() - 50.0).abs() < 1e-4);
        assert!((rects[0].center_y() - 50.0).abs() < 1e-4);
        assert_inside(100.0, &rects);
    }

    #[test]
    fn two_icons_sit_side_by_side() {
        let rects = preview_rects(100.0, 2);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].y, rects[1].y);
        assert!(rects[0].x < rects[1].x);
        assert!(rects[0].x + rects[0].width <= rects[1].x);
        assert_inside(100.0, &rects);
    }

    #[test]
    fn three_icons_form_a_triangle() {
        let rects = preview_rects(100.0, 3);
        assert_eq!(rects.len(), 3);
        // The first is the top point, centred above the other two.
        assert!((rects[0].center_x() - 50.0).abs() < 1e-4);
        assert!(rects[0].y < rects[1].y);
        assert_eq!(rects[1].y, rects[2].y);
        assert_inside(100.0, &rects);
    }

    #[test]
    fn four_icons_form_a_square() {
        let rects = preview_rects(100.0, 4);
        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0].y, rects[1].y);
        assert_eq!(rects[2].y, rects[3].y);
        assert_eq!(rects[0].x, rects[2].x);
        assert_eq!(rects[1].x, rects[3].x);
        assert!(rects[0].y + rects[0].height <= rects[2].y);
        assert_inside(100.0, &rects);
    }
}
