//! `PointD`, a point on the board image. Only used for drawing.

/// `PointD`. `==` compares coordinates with `==` (so NaN != NaN), like C#.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct PointD {
    pub x: f64,
    pub y: f64,
}

impl PointD {
    pub fn new(x: f64, y: f64) -> Self {
        PointD { x, y }
    }

    pub fn translate(self, d_x: f64, d_y: f64) -> PointD {
        PointD::new(self.x + d_x, self.y + d_y)
    }
}
