use crate::common::point3f::Point3f;

pub trait Item: Clone + Send + Sync {
    fn id(&self) -> i32;
    fn weight(&self) -> f32;
    fn volume(&self) -> f64;
    fn longest_side(&self) -> f64;
    fn position(&self) -> Point3f;
    fn set_position(&mut self, p: Point3f);
}
