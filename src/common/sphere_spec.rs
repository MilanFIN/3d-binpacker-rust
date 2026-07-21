use serde::{Deserialize, Serialize};
use crate::common::item::Item;
use crate::common::point3f::Point3f;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sphere {
    pub id: i32,
    pub position: Point3f, // center position
    pub radius: f32,
    pub weight: f32,
}

impl Sphere {
    pub fn new(id: i32, position: Point3f, radius: f32, weight: f32) -> Self {
        Self {
            id,
            position,
            radius,
            weight,
        }
    }

    pub fn new_without_weight(id: i32, position: Point3f, radius: f32) -> Self {
        Self::new(id, position, radius, 0.0)
    }

    pub fn collides_with_sphere(&self, other: &Sphere) -> bool {
        let dx = self.position.x - other.position.x;
        let dy = self.position.y - other.position.y;
        let dz = self.position.z - other.position.z;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        let r_sum = self.radius + other.radius;
        dist_sq < r_sum * r_sum
    }

    pub fn collides_with_walls(&self, w: f32, h: f32, d: f32) -> bool {
        self.position.x - self.radius < 0.0 ||
        self.position.y - self.radius < 0.0 ||
        self.position.z - self.radius < 0.0 ||
        self.position.x + self.radius > w ||
        self.position.y + self.radius > h ||
        self.position.z + self.radius > d
    }
}

impl Item for Sphere {
    fn id(&self) -> i32 {
        self.id
    }

    fn weight(&self) -> f32 {
        self.weight
    }

    fn volume(&self) -> f64 {
        (4.0 / 3.0 * std::f64::consts::PI * (self.radius as f64).powi(3)) as f64
    }

    fn longest_side(&self) -> f64 {
        (self.radius * 2.0) as f64
    }

    fn position(&self) -> Point3f {
        self.position
    }

    fn set_position(&mut self, p: Point3f) {
        self.position = p;
    }
}
