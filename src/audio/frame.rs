use std::{
    iter::Sum,
    ops::{Add, AddAssign, Mul},
};

#[derive(Copy, Clone, Debug, Default)]
pub struct Frame {
    pub left: f32,
    pub right: f32,
}

impl Add for Frame {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            left: self.left + rhs.left,
            right: self.right + rhs.right,
        }
    }
}

impl Sum for Frame {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Frame::default(), |acc, v| acc + v)
    }
}

impl Mul<f32> for Frame {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            left: self.left * rhs,
            right: self.right * rhs,
        }
    }
}

impl AddAssign for Frame {
    fn add_assign(&mut self, rhs: Self) {
        self.left += rhs.left;
        self.right += rhs.right;
    }
}
