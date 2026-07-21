use crate::common::bin::Bin;
use crate::common::point3f::Point3f;
use crate::common::sphere_spec::Sphere;
use crate::common::pack_result::PackResult;
use crate::solver::common::solver_properties::SolverProperties;
use crate::solver::solver_interface::Solver;

pub struct FirstFitSpheres {
    bin_template: Option<Bin>,
    weight_limit: f32,
    _growing_bin: bool,
}

impl Default for FirstFitSpheres {
    fn default() -> Self {
        Self {
            bin_template: None,
            weight_limit: 0.0,
            _growing_bin: false,
        }
    }
}

impl Solver<Sphere, Bin> for FirstFitSpheres {
    fn init(&mut self, properties: &SolverProperties<Bin>) {
        self.bin_template = Some(properties.bin.clone());
        self.weight_limit = properties.weight;
        self._growing_bin = properties.growing_bin;
    }

    fn solve(&mut self, spheres: &[Sphere]) -> PackResult<Sphere> {
        let mut result_bins: Vec<Vec<Sphere>> = Vec::new();
        let bin_template = self.bin_template.clone().unwrap();

        for sphere in spheres {
            let mut placed = false;
            
            for bin_spheres in result_bins.iter_mut() {
                let current_weight: f32 = bin_spheres.iter().map(|s| s.weight).sum();
                if self.weight_limit > 0.0 && current_weight + sphere.weight > self.weight_limit {
                    continue;
                }

                if let Some(pos) = Self::find_position(sphere, bin_spheres, &bin_template) {
                    let mut new_sphere = sphere.clone();
                    new_sphere.position = pos;
                    bin_spheres.push(new_sphere);
                    placed = true;
                    break;
                }
            }

            if !placed {
                let mut new_bin_spheres = Vec::new();
                if let Some(pos) = Self::find_position(sphere, &new_bin_spheres, &bin_template) {
                    let mut new_sphere = sphere.clone();
                    new_sphere.position = pos;
                    new_bin_spheres.push(new_sphere);
                } else {
                    eprintln!("Sphere too big for bin: {:?}", sphere);
                }
                result_bins.push(new_bin_spheres);
            }
        }

        PackResult::new(Vec::new(), 0.0, result_bins)
    }
}

impl FirstFitSpheres {
    fn find_position(sphere: &Sphere, placed_spheres: &[Sphere], bin: &Bin) -> Option<Point3f> {
        let r = sphere.radius;
        let step = r.max(1.0);

        let max_x = bin.w - r;
        let max_y = bin.h - r;
        let max_z = bin.d - r;

        if max_x < r || max_y < r || max_z < r {
            return None; // Cannot fit even one
        }

        let mut z = r;
        while z <= max_z {
            let mut y = r;
            while y <= max_y {
                let mut x = r;
                while x <= max_x {
                    let pos = Point3f::new(x, y, z);
                    let mut test_sphere = sphere.clone();
                    test_sphere.position = pos;

                    if !test_sphere.collides_with_walls(bin.w, bin.h, bin.d) {
                        let mut collides = false;
                        for placed in placed_spheres {
                            if test_sphere.collides_with_sphere(placed) {
                                collides = true;
                                break;
                            }
                        }
                        if !collides {
                            return Some(pos);
                        }
                    }
                    x += step;
                }
                y += step;
            }
            z += step;
        }

        None
    }
}
