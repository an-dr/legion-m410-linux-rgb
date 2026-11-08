/// An axis-aligned rectangle in logical canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    /// Whether the point is inside; the right and bottom edges are outside.
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x as f32
            && x < (self.x + self.w as i32) as f32
            && y >= self.y as f32
            && y < (self.y + self.h as i32) as f32
    }

    pub fn center(self) -> (i32, i32) {
        (self.x + self.w as i32 / 2, self.y + self.h as i32 / 2)
    }

    pub fn right(self) -> i32 {
        self.x + self.w as i32
    }

    pub fn bottom(self) -> i32 {
        self.y + self.h as i32
    }

    /// Where `x` falls across the rectangle, from 0 at the left edge to 1 at the right, clamped.
    pub fn fraction_x(self, x: f32) -> f32 {
        ((x - self.x as f32) / self.w as f32).clamp(0.0, 1.0)
    }
}
