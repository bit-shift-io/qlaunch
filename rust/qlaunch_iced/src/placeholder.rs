//! Tiny generated placeholders, cached once rather than regenerated per frame.
use iced::widget::image;
use std::sync::OnceLock;

pub fn handle(folder: bool) -> image::Handle {
    static ICONS: OnceLock<[image::Handle; 2]> = OnceLock::new();
    ICONS.get_or_init(|| [make(false), make(true)])[usize::from(folder)].clone()
}

fn make(folder: bool) -> image::Handle {
    const SIDE: usize = 128;
    let mut pixels = vec![0; SIDE * SIDE * 4];
    for y in 0..SIDE {
        for x in 0..SIDE {
            let mut alpha = 0u32;
            // Supersample edges so the placeholder stays smooth at phone density.
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = (x as f32 + (sx as f32 + 0.5) / 4.0) / SIDE as f32;
                    let py = (y as f32 + (sy as f32 + 0.5) / 4.0) / SIDE as f32;
                    let dx = (px - 0.5).abs() - 0.3;
                    let dy = (py - 0.5).abs() - 0.3;
                    if dx.max(0.0).hypot(dy.max(0.0)) <= 0.2 {
                        let glyph = if folder {
                            ((0.25..0.75).contains(&px) && (0.37..0.69).contains(&py))
                                || ((0.25..0.48).contains(&px) && (0.31..0.38).contains(&py))
                        } else {
                            let cell = |v: f32| {
                                (0.28..0.39).contains(&v)
                                    || (0.445..0.555).contains(&v)
                                    || (0.61..0.72).contains(&v)
                            };
                            cell(px) && cell(py)
                        };
                        alpha += if glyph { 197 } else { 61 };
                    }
                }
            }
            let offset = (y * SIDE + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&[255, 255, 255, (alpha / 16) as u8]);
        }
    }
    image::Handle::from_rgba(SIDE as u32, SIDE as u32, pixels)
}
