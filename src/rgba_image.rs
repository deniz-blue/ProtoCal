use std::io::Cursor;

use anyhow::Context as _;

pub type Rgba = [u8; 4];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    width: u32,
    height: u32,
    pixels: Vec<Rgba>,
}

impl RgbaImage {
    pub fn filled(width: u32, height: u32, color: Rgba) -> Self {
        Self { width, height, pixels: vec![color; (width * height) as usize] }
    }

    pub fn of_png(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info()?;
        let mut buffer = vec![0; reader.output_buffer_size().context("PNG is too large")?];
        let frame = reader.next_frame(&mut buffer)?;
        let pixels = buffer[..frame.buffer_size()]
            .chunks_exact(frame.color_type.samples())
            .map(|samples| match *samples {
                [gray] => [gray, gray, gray, 255],
                [gray, alpha] => [gray, gray, gray, alpha],
                [red, green, blue] => [red, green, blue, 255],
                [red, green, blue, alpha] => [red, green, blue, alpha],
                _ => [0; 4],
            })
            .collect();
        Ok(Self { width: frame.width, height: frame.height, pixels })
    }

    pub fn to_png(&self) -> anyhow::Result<Vec<u8>> {
        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(self.pixels.as_flattened())?;
        Ok(png)
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn pixel(&self, x: u32, y: u32) -> Rgba {
        self.pixels[(y * self.width + x) as usize]
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: Rgba) {
        self.pixels[(y * self.width + x) as usize] = color;
    }

    pub fn blend_pixel(&mut self, x: u32, y: u32, color: Rgba, coverage: f32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let under = self.pixel(x, y);
        let mixed = std::array::from_fn(|channel| {
            let (under, over) = (f32::from(under[channel]), f32::from(color[channel]));
            (under + (over - under) * coverage.clamp(0.0, 1.0)).round() as u8
        });
        self.set_pixel(x, y, mixed);
    }

    pub fn sample_at_fraction(&self, fraction_x: f32, fraction_y: f32) -> Rgba {
        let x = (fraction_x * self.width as f32 - 0.5).clamp(0.0, (self.width - 1) as f32);
        let y = (fraction_y * self.height as f32 - 0.5).clamp(0.0, (self.height - 1) as f32);
        let (left, top) = (x.floor() as u32, y.floor() as u32);
        let (right, bottom) = ((left + 1).min(self.width - 1), (top + 1).min(self.height - 1));
        let (across, down) = (x.fract(), y.fract());
        std::array::from_fn(|channel| {
            let at = |x, y| f32::from(self.pixel(x, y)[channel]);
            let upper = at(left, top) + (at(right, top) - at(left, top)) * across;
            let lower = at(left, bottom) + (at(right, bottom) - at(left, bottom)) * across;
            (upper + (lower - upper) * down).round() as u8
        })
    }

    pub fn shrunk_by(&self, factor: u32) -> Self {
        let (width, height) = (self.width / factor, self.height / factor);
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| self.premultiplied_mean(x * factor, y * factor, factor))
            .collect();
        Self { width, height, pixels }
    }

    fn premultiplied_mean(&self, left: u32, top: u32, side: u32) -> Rgba {
        let block = (top..top + side).flat_map(|y| (left..left + side).map(move |x| self.pixel(x, y)));
        let [red, green, blue, alpha] = block.fold([0.0_f32; 4], |sum, [r, g, b, a]| {
            let weight = f32::from(a) / 255.0;
            [sum[0] + f32::from(r) * weight, sum[1] + f32::from(g) * weight, sum[2] + f32::from(b) * weight, sum[3] + f32::from(a)]
        });
        if alpha == 0.0 {
            return [0; 4];
        }
        let coverage = alpha / 255.0;
        let count = (side * side) as f32;
        [red / coverage, green / coverage, blue / coverage, alpha / count].map(|channel| channel.round().clamp(0.0, 255.0) as u8)
    }
}
