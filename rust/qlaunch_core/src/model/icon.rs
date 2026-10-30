use serde::{Deserialize, Serialize};

/// How an [`IconImage`]'s bytes are encoded.
///
/// Android hands back raw pixels (the bitmap it already drew), so nothing has to
/// be encoded or decoded; the Linux host passes the icon file's PNG bytes
/// through and rasterises SVG icons to PNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IconFormat {
    Png,
    Rgba,
}

/// An icon's bytes plus what they mean.
///
/// `width` and `height` describe the pixels of an `Rgba` image and are `0` for
/// `Png`, whose dimensions are in the PNG itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IconImage {
    pub format: IconFormat,
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

impl IconImage {
    /// An icon that is already a PNG file, with no separate dimensions.
    pub fn png(bytes: Vec<u8>) -> IconImage {
        IconImage {
            format: IconFormat::Png,
            width: 0,
            height: 0,
            bytes,
        }
    }
}
