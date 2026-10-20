use qlaunch_core::model::{IconFormat, IconImage};
use qlaunch_core::platform::android::{argb_pixels_to_rgba, icon_from_int_slots};

#[test]
fn png_wraps_bytes_with_no_size() {
    let img = IconImage::png(vec![1, 2, 3]);
    assert_eq!(img.format, IconFormat::Png);
    assert_eq!((img.width, img.height), (0, 0));
    assert_eq!(img.bytes, vec![1, 2, 3]);
}

#[test]
fn argb_int_pixels_become_rgba_bytes() {
    // Android packs 0xAARRGGBB into an int; 0xFF00FF00 is opaque green.
    let out = argb_pixels_to_rgba(&[0xFF00_FF00u32]);
    assert_eq!(out, vec![0x00, 0xFF, 0x00, 0xFF]);
}

#[test]
fn int_slots_parse_to_an_rgba_image() {
    let slots = [2, 2, 0xFF00_FF00u32 as i32, 0, 0, 0];
    let image = icon_from_int_slots(&slots).unwrap();
    assert_eq!(image.format, IconFormat::Rgba);
    assert_eq!((image.width, image.height), (2, 2));
    assert_eq!(image.bytes.len(), 16);
    assert_eq!(&image.bytes[0..4], &[0x00, 0xFF, 0x00, 0xFF]);
}

#[test]
fn int_slots_with_the_wrong_pixel_count_are_rejected() {
    // Two slots say 2x2, but only three pixels follow.
    assert_eq!(icon_from_int_slots(&[2, 2, 0, 0, 0]), None);
}

#[test]
fn int_slots_with_no_or_non_positive_size_are_rejected() {
    assert_eq!(icon_from_int_slots(&[]), None);
    assert_eq!(icon_from_int_slots(&[0, 2]), None);
    assert_eq!(icon_from_int_slots(&[2, -1, 0, 0, 0, 0]), None);
}
