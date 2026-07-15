use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use crate::common::point3f::Point3f;
use crate::common::space::Space;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinBox {
    pub id: i32,
    pub position: Point3f,
    pub size: Point3f,
    pub weight: f32,
}

impl BinBox {
    pub fn new(id: i32, position: Point3f, size: Point3f, weight: f32) -> Self {
        Self {
            id,
            position,
            size,
            weight,
        }
    }

    pub fn new_without_weight(id: i32, position: Point3f, size: Point3f) -> Self {
        Self::new(id, position, size, 0.0)
    }

    pub fn volume(&self) -> f64 {
        (self.size.x * self.size.y * self.size.z) as f64
    }

    pub fn longest_side(&self) -> f64 {
        let max_xy = if self.size.x > self.size.y { self.size.x } else { self.size.y };
        let max_xyz = if max_xy > self.size.z { max_xy } else { self.size.z };
        max_xyz as f64
    }

    pub fn collides_with_box(&self, other: &BinBox) -> bool {
        self.position.x < other.position.x + other.size.x &&
        self.position.y < other.position.y + other.size.y &&
        self.position.z < other.position.z + other.size.z &&
        self.position.x + self.size.x > other.position.x &&
        self.position.y + self.size.y > other.position.y &&
        self.position.z + self.size.z > other.position.z
    }

    pub fn collides_with_space(&self, space: &Space) -> bool {
        self.position.x < space.x + space.w &&
        self.position.y < space.y + space.h &&
        self.position.z < space.z + space.d &&
        self.position.x + self.size.x > space.x &&
        self.position.y + self.size.y > space.y &&
        self.position.z + self.size.z > space.z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_volume() {
        let b = BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(2.0, 3.0, 4.0));
        assert_eq!(b.volume(), 24.0);
    }

    #[test]
    fn test_longest_side() {
        let b = BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(2.0, 5.0, 4.0));
        assert_eq!(b.longest_side(), 5.0);
    }

    #[test]
    fn test_collides_with_box() {
        let b1 = BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(2.0, 2.0, 2.0));
        let b2 = BinBox::new_without_weight(2, Point3f::new(1.0, 1.0, 1.0), Point3f::new(2.0, 2.0, 2.0));
        let b3 = BinBox::new_without_weight(3, Point3f::new(2.0, 2.0, 2.0), Point3f::new(2.0, 2.0, 2.0));
        
        assert!(b1.collides_with_box(&b2));
        assert!(!b1.collides_with_box(&b3)); // they just touch
    }

    #[test]
    fn test_collides_with_space() {
        let b1 = BinBox::new_without_weight(1, Point3f::new(1.0, 1.0, 1.0), Point3f::new(2.0, 2.0, 2.0));
        let s1 = Space::new(2.0, 2.0, 2.0, 2.0, 2.0, 2.0);
        let s2 = Space::new(3.0, 3.0, 3.0, 2.0, 2.0, 2.0);
        
        assert!(b1.collides_with_space(&s1));
        assert!(!b1.collides_with_space(&s2)); // they just touch
    }
}
