use ab_glyph::{Font as _, FontRef, PxScale, ScaleFont as _, point};
use anyhow::Context as _;

use crate::rgba_image::{Rgba, RgbaImage};

pub const STACKED_AVATARS_SIDE: u32 = 256;
const SUPERSAMPLING: u32 = 4;
const GAP: f32 = 0.035;
const MOST_CIRCLES: usize = 4;
const AVATARS_BESIDE_COUNT: usize = 3;
const COUNT_BACKGROUND: Rgba = [78, 80, 88, 255];
const COUNT_TEXT: Rgba = [242, 243, 245, 255];
const COUNT_TEXT_EM: f32 = 0.42;
const COUNT_FONT: &[u8] = include_bytes!("../../assets/fonts/Lexend-SemiBold.ttf");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackFace {
    Avatar(usize),
    HiddenCount(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StackCircle {
    pub face: StackFace,
    pub center: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct StackLayout {
    pub diameter: f32,
    pub circles: Vec<StackCircle>,
}

impl StackLayout {
    pub fn of_organizer_count(organizer_count: usize) -> Self {
        let faces: Vec<StackFace> = if organizer_count <= MOST_CIRCLES {
            (0..organizer_count).map(StackFace::Avatar).collect()
        } else {
            let hidden = StackFace::HiddenCount(organizer_count - AVATARS_BESIDE_COUNT);
            (0..AVATARS_BESIDE_COUNT).map(StackFace::Avatar).chain([hidden]).collect()
        };
        let (diameter, centers) = circle_centers(faces.len());
        let circles = faces.into_iter().zip(centers).map(|(face, center)| StackCircle { face, center }).collect();
        Self { diameter, circles }
    }

    pub fn shown_avatars(&self) -> impl Iterator<Item = usize> + '_ {
        self.circles.iter().filter_map(|circle| match circle.face {
            StackFace::Avatar(index) => Some(index),
            StackFace::HiddenCount(_) => None,
        })
    }

    pub fn png_of(&self, avatars: &[RgbaImage]) -> anyhow::Result<Vec<u8>> {
        let side = STACKED_AVATARS_SIDE * SUPERSAMPLING;
        let diameter = self.diameter * side as f32;
        let mut canvas = RgbaImage::filled(side, side, [0; 4]);
        for circle in self.circles.iter().rev() {
            let drawn_count;
            let face = match circle.face {
                StackFace::Avatar(index) => avatars.get(index).context("an avatar of the stack is missing")?,
                StackFace::HiddenCount(count) => {
                    drawn_count = count_face(diameter, count);
                    &drawn_count
                }
            };
            let center = circle.center.map(|fraction| fraction * side as f32);
            paint_circle(&mut canvas, face, center, diameter / 2.0);
        }
        canvas.shrunk_by(SUPERSAMPLING).to_png()
    }
}

fn near_and_far(diameter: f32) -> (f32, f32) {
    (diameter / 2.0, 1.0 - diameter / 2.0)
}

fn circle_centers(count: usize) -> (f32, Vec<[f32; 2]>) {
    match count {
        0..=2 => {
            let (near, far) = near_and_far(0.66);
            (0.66, vec![[near, near], [far, far]])
        }
        3 => {
            let (near, far) = near_and_far(0.56);
            (0.56, vec![[0.5, near], [near, far], [far, far]])
        }
        _ => {
            let (near, far) = near_and_far(0.54);
            (0.54, vec![[near, near], [far, near], [near, far], [far, far]])
        }
    }
}

fn paint_circle(canvas: &mut RgbaImage, face: &RgbaImage, center: [f32; 2], radius: f32) {
    let outer = radius + GAP * canvas.width() as f32;
    let span = |middle: f32, limit: u32| ((middle - outer).floor().max(0.0) as u32)..((middle + outer).ceil() as u32).min(limit);
    for y in span(center[1], canvas.height()) {
        for x in span(center[0], canvas.width()) {
            let (right, down) = (x as f32 + 0.5 - center[0], y as f32 + 0.5 - center[1]);
            let distance = right.hypot(down);
            if distance <= radius {
                let color = face.sample_at_fraction((right + radius) / (2.0 * radius), (down + radius) / (2.0 * radius));
                canvas.set_pixel(x, y, color);
            } else if distance <= outer {
                canvas.set_pixel(x, y, [0; 4]);
            }
        }
    }
}

fn count_face(diameter: f32, hidden_count: usize) -> RgbaImage {
    let side = diameter.round() as u32;
    let mut face = RgbaImage::filled(side, side, COUNT_BACKGROUND);
    let font = FontRef::try_from_slice(COUNT_FONT).expect("the bundled font parses");
    let em = (COUNT_TEXT_EM * diameter).round();
    let scale = PxScale::from(em * font.height_unscaled() / font.units_per_em().unwrap_or(1000.0));
    let font = font.as_scaled(scale);
    let glyphs: Vec<_> = format!("+{hidden_count}").chars().map(|character| font.glyph_id(character)).collect();
    let width: f32 = glyphs.iter().map(|&glyph| font.h_advance(glyph)).sum();
    let baseline = side as f32 / 2.0 + (font.ascent() + font.descent()) / 2.0;
    let mut caret = (side as f32 - width) / 2.0;
    for glyph in glyphs {
        if let Some(outline) = font.outline_glyph(glyph.with_scale_and_position(scale, point(caret, baseline))) {
            let corner = outline.px_bounds().min;
            outline.draw(|x, y, coverage| {
                face.blend_pixel(corner.x as u32 + x, corner.y as u32 + y, COUNT_TEXT, coverage);
            });
        }
        caret += font.h_advance(glyph);
    }
    face
}
