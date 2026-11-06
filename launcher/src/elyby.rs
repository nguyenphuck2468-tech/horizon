//! The Ely.by sign-in glyph, drawn for the launcher's Ely.by button and chip.
//!
//! Ely.by ships no SVG in this tree, so this is drawn from squares the same way
//! [`microsoft`](crate::microsoft) draws Microsoft's four squares: a block `E`
//! in Ely.by's green, 21 pixels square, square corners.
use super::*;

/// Ely.by's green, 21 pixels square.
const GREEN: u32 = 0x7ac143;

/// The `E`'s strokes in its 21-pixel box: left, top, width, height.
const STROKES: [(f32, f32, f32, f32); 4] = [
    (1., 1., 5., 19.),
    (1., 1., 19., 5.),
    (1., 8., 13., 5.),
    (1., 15., 19., 5.),
];

/// The glyph, 21 pixels square.
pub(super) fn symbol() -> Div {
    logo(21.)
}

/// The glyph at `size` pixels square.
pub(super) fn logo(size: f32) -> Div {
    let u = size / 21.;
    div()
        .relative()
        .flex_none()
        .size(px(size))
        .children(STROKES.map(|(x, y, w, h)| {
            div()
                .absolute()
                .left(px(x * u))
                .top(px(y * u))
                .w(px(w * u))
                .h(px(h * u))
                .bg(gpui::rgb(GREEN))
        }))
}
