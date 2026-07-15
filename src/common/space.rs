use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Space {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub h: f32,
    pub d: f32,
}

impl Space {
    pub fn new(x: f32, y: f32, z: f32, w: f32, h: f32, d: f32) -> Self {
        Self { x, y, z, w, h, d }
    }

    /// 2D constructor for backward compatibility
    pub fn new_2d(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self::new(x, y, 0.0, w, h, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_space_new() {
        let s = Space::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        assert_eq!(s.x, 1.0);
        assert_eq!(s.y, 2.0);
        assert_eq!(s.z, 3.0);
        assert_eq!(s.w, 4.0);
        assert_eq!(s.h, 5.0);
        assert_eq!(s.d, 6.0);
    }

    #[test]
    fn test_space_new_2d() {
        let s = Space::new_2d(1.0, 2.0, 4.0, 5.0);
        assert_eq!(s.x, 1.0);
        assert_eq!(s.y, 2.0);
        assert_eq!(s.z, 0.0);
        assert_eq!(s.w, 4.0);
        assert_eq!(s.h, 5.0);
        assert_eq!(s.d, 0.0);
    }
}
