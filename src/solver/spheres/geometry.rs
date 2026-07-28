use crate::common::point3f::Point3f;

/// Epsilon for floating point comparisons.
pub const EPS: f32 = 1e-4;

/// Subtract two points.
pub fn sub(a: &Point3f, b: &Point3f) -> Point3f {
    Point3f::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

/// Add two points.
pub fn add(a: &Point3f, b: &Point3f) -> Point3f {
    Point3f::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

/// Scale a point.
pub fn scale(a: &Point3f, s: f32) -> Point3f {
    Point3f::new(a.x * s, a.y * s, a.z * s)
}

/// Dot product.
pub fn dot(a: &Point3f, b: &Point3f) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// Cross product.
pub fn cross(a: &Point3f, b: &Point3f) -> Point3f {
    Point3f::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

/// Squared length.
pub fn len_sq(a: &Point3f) -> f32 {
    dot(a, a)
}

/// Length.
pub fn len(a: &Point3f) -> f32 {
    len_sq(a).sqrt()
}

/// Normalize, returns None if zero-length.
pub fn normalize(a: &Point3f) -> Option<Point3f> {
    let l = len(a);
    if l < EPS {
        None
    } else {
        Some(scale(a, 1.0 / l))
    }
}

/// Axis enum for wall planes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// Solve position for three plane constraints (corner case).
/// Each plane fixes one coordinate to `value`, sphere center is at `value + r` (min wall)
/// or `value - r` (max wall). We represent min walls as value=0 with sign=+1
/// and max walls as value=W with sign=-1, but for simplicity we store the
/// offset direction in the constraint itself.
pub fn solve_three_planes(
    axes: [(Axis, f32); 3],
    r: f32,
    bin_w: f32,
    bin_h: f32,
    bin_d: f32,
) -> Option<Point3f> {
    let mut pos = Point3f::new(0.0, 0.0, 0.0);
    for (axis, value) in axes {
        let coord = if value < EPS {
            // min wall: center at r
            r
        } else {
            // max wall: center at value - r
            value - r
        };
        if coord < r - EPS {
            return None;
        }
        match axis {
            Axis::X => pos.x = coord,
            Axis::Y => pos.y = coord,
            Axis::Z => pos.z = coord,
        }
    }
    // Verify fits in bin
    if pos.x + r > bin_w + EPS || pos.y + r > bin_h + EPS || pos.z + r > bin_d + EPS {
        return None;
    }
    if pos.x - r < -EPS || pos.y - r < -EPS || pos.z - r < -EPS {
        return None;
    }
    Some(pos)
}

/// Solve position for one sphere + two planes.
/// The two planes fix two coordinates; the third is solved from tangency.
pub fn solve_sphere_two_planes(
    sphere_center: &Point3f,
    sphere_radius: f32,
    plane1: (Axis, f32),
    plane2: (Axis, f32),
    r: f32,
    bin_w: f32,
    bin_h: f32,
    bin_d: f32,
) -> Option<Point3f> {
    let mut pos = Point3f::new(0.0, 0.0, 0.0);
    let mut free_axis = Axis::X;

    // Set the two fixed coordinates from planes
    for &(axis, value) in &[plane1, plane2] {
        let coord = if value < EPS { r } else { value - r };
        match axis {
            Axis::X => pos.x = coord,
            Axis::Y => pos.y = coord,
            Axis::Z => pos.z = coord,
        }
    }

    // Find the free axis
    let used_axes = [plane1.0, plane2.0];
    if !used_axes.contains(&Axis::X) {
        free_axis = Axis::X;
    } else if !used_axes.contains(&Axis::Y) {
        free_axis = Axis::Y;
    } else {
        free_axis = Axis::Z;
    }

    // Tangency: |pos - sphere_center|^2 = (r + sphere_radius)^2
    let dist_needed_sq = (r + sphere_radius) * (r + sphere_radius);

    // Sum of squared differences for the fixed axes
    let dx = pos.x - sphere_center.x;
    let dy = pos.y - sphere_center.y;
    let dz = pos.z - sphere_center.z;

    let (fixed_sq, center_free) = match free_axis {
        Axis::X => (dy * dy + dz * dz, sphere_center.x),
        Axis::Y => (dx * dx + dz * dz, sphere_center.y),
        Axis::Z => (dx * dx + dy * dy, sphere_center.z),
    };

    let remaining = dist_needed_sq - fixed_sq;
    if remaining < 0.0 {
        return None; // No solution — too far apart on fixed axes
    }

    let offset = remaining.sqrt();

    // Two candidate values: center_free ± offset
    // Pick the one that is valid and closest to the origin (lowest score).
    let v1 = center_free - offset;
    let v2 = center_free + offset;

    let max_val = match free_axis {
        Axis::X => bin_w,
        Axis::Y => bin_h,
        Axis::Z => bin_d,
    };

    let valid1 = v1 >= r - EPS && v1 + r <= max_val + EPS;
    let valid2 = v2 >= r - EPS && v2 + r <= max_val + EPS;

    let chosen = match (valid1, valid2) {
        (true, true) => {
            // Prefer the smaller (closer to origin)
            if v1 < v2 { v1 } else { v2 }
        }
        (true, false) => v1,
        (false, true) => v2,
        (false, false) => return None,
    };

    match free_axis {
        Axis::X => pos.x = chosen,
        Axis::Y => pos.y = chosen,
        Axis::Z => pos.z = chosen,
    }

    // Final bounds check
    if pos.x - r < -EPS || pos.y - r < -EPS || pos.z - r < -EPS {
        return None;
    }
    if pos.x + r > bin_w + EPS || pos.y + r > bin_h + EPS || pos.z + r > bin_d + EPS {
        return None;
    }

    Some(pos)
}

/// Solve position for two spheres + one plane.
/// Two spheres define a circle of possible centers; the plane picks a point on it.
pub fn solve_two_spheres_one_plane(
    c1: &Point3f,
    r1: f32,
    c2: &Point3f,
    r2: f32,
    plane: (Axis, f32),
    r: f32,
    bin_w: f32,
    bin_h: f32,
    bin_d: f32,
) -> Option<Point3f> {
    // The new sphere center must satisfy:
    //   |P - C1|^2 = (r + r1)^2
    //   |P - C2|^2 = (r + r2)^2
    //   P.axis = plane_coord (r or value-r)
    //
    // Subtracting the first two gives a linear equation (a plane).
    // Combined with the axis-plane, we get a line.
    // Substituting back gives a quadratic in one variable.

    let d1 = r + r1;
    let d2 = r + r2;

    let plane_coord = if plane.1 < EPS { r } else { plane.1 - r };

    // P = (px, py, pz), one coordinate is fixed by the plane
    // |P-C1|^2 = d1^2  ... (1)
    // |P-C2|^2 = d2^2  ... (2)
    // (1)-(2): 2*(C2-C1).P = d1^2 - d2^2 - |C1|^2 + |C2|^2   ... (3)

    let diff = sub(c2, c1);
    let rhs = d1 * d1 - d2 * d2 - len_sq(c1) + len_sq(c2);
    // (3): 2 * dot(diff, P) = rhs
    // => 2*(diff.x*px + diff.y*py + diff.z*pz) = rhs

    // One coordinate is fixed. We need to solve for the other two.
    // Let's work in terms of the two free axes.
    let (fixed_idx, fixed_val) = match plane.0 {
        Axis::X => (0, plane_coord),
        Axis::Y => (1, plane_coord),
        Axis::Z => (2, plane_coord),
    };

    let diff_arr = [diff.x, diff.y, diff.z];
    let c1_arr = [c1.x, c1.y, c1.z];

    // From (3): 2*(diff[free1]*p_free1 + diff[free2]*p_free2) = rhs - 2*diff[fixed]*fixed_val
    let free_indices: Vec<usize> = (0..3).filter(|&i| i != fixed_idx).collect();
    let fi0 = free_indices[0];
    let fi1 = free_indices[1];

    let linear_rhs = rhs - 2.0 * diff_arr[fixed_idx] * fixed_val;
    // 2*diff[fi0]*u + 2*diff[fi1]*v = linear_rhs   ... (L)

    // From (1): (u - c1[fi0])^2 + (v - c1[fi1])^2 + (fixed_val - c1[fixed])^2 = d1^2
    let fixed_term = (fixed_val - c1_arr[fixed_idx]) * (fixed_val - c1_arr[fixed_idx]);
    let circle_rhs = d1 * d1 - fixed_term;
    // (u - c1[fi0])^2 + (v - c1[fi1])^2 = circle_rhs   ... (C)

    if circle_rhs < 0.0 {
        return None;
    }

    // Solve (L) for one variable in terms of the other, substitute into (C).
    let a0 = 2.0 * diff_arr[fi0];
    let a1 = 2.0 * diff_arr[fi1];

    let bin_dims = [bin_w, bin_h, bin_d];

    if a0.abs() > a1.abs() {
        // u = (linear_rhs - a1*v) / a0
        // substitute into (C):
        // ((linear_rhs - a1*v)/a0 - c1[fi0])^2 + (v - c1[fi1])^2 = circle_rhs
        let solutions = solve_substituted(a0, a1, linear_rhs, c1_arr[fi0], c1_arr[fi1], circle_rhs);
        pick_best_solution(solutions, fixed_idx, fixed_val, fi0, fi1, a0, a1, linear_rhs, r, &bin_dims, true)
    } else if a1.abs() > EPS {
        // v = (linear_rhs - a0*u) / a1
        let solutions = solve_substituted(a1, a0, linear_rhs, c1_arr[fi1], c1_arr[fi0], circle_rhs);
        pick_best_solution(solutions, fixed_idx, fixed_val, fi0, fi1, a0, a1, linear_rhs, r, &bin_dims, false)
    } else {
        // Both coefficients are ~0 — spheres are concentric on the free plane. No unique solution.
        None
    }
}

fn solve_substituted(
    a_primary: f32,
    a_secondary: f32,
    linear_rhs: f32,
    c_primary: f32,
    c_secondary: f32,
    circle_rhs: f32,
) -> Option<(f32, f32)> {
    // primary = (linear_rhs - a_secondary * secondary) / a_primary
    // (primary - c_primary)^2 + (secondary - c_secondary)^2 = circle_rhs
    //
    // Let t = secondary
    // primary(t) = (linear_rhs - a_secondary * t) / a_primary
    // Let p(t) = primary(t) - c_primary = (linear_rhs - a_secondary*t)/a_primary - c_primary
    //          = (linear_rhs - a_secondary*t - c_primary*a_primary) / a_primary
    // Let A = -a_secondary / a_primary, B = (linear_rhs - c_primary * a_primary) / a_primary
    // p(t) = A*t + B
    // (A*t + B)^2 + (t - c_secondary)^2 = circle_rhs
    // (A^2+1)*t^2 + (2*A*B - 2*c_secondary)*t + (B^2 + c_secondary^2 - circle_rhs) = 0

    let big_a = -a_secondary / a_primary;
    let big_b = (linear_rhs - c_primary * a_primary) / a_primary;

    let qa = big_a * big_a + 1.0;
    let qb = 2.0 * big_a * big_b - 2.0 * c_secondary;
    let qc = big_b * big_b + c_secondary * c_secondary - circle_rhs;

    let disc = qb * qb - 4.0 * qa * qc;
    if disc < 0.0 {
        return None;
    }

    let sqrt_disc = disc.sqrt();
    let t1 = (-qb - sqrt_disc) / (2.0 * qa);
    let t2 = (-qb + sqrt_disc) / (2.0 * qa);

    Some((t1, t2))
}

fn pick_best_solution(
    solutions: Option<(f32, f32)>,
    fixed_idx: usize,
    fixed_val: f32,
    fi0: usize,
    fi1: usize,
    a0: f32,
    a1: f32,
    linear_rhs: f32,
    r: f32,
    bin_dims: &[f32; 3],
    primary_is_fi0: bool,
) -> Option<Point3f> {
    let (t1, t2) = solutions?;

    let mut best: Option<Point3f> = None;
    let mut best_score = f32::MAX;

    for &t in &[t1, t2] {
        let (u, v) = if primary_is_fi0 {
            let u = (linear_rhs - a1 * t) / a0;
            (u, t)
        } else {
            let v = (linear_rhs - a0 * t) / a1;
            (t, v)
        };

        let mut coords = [0.0f32; 3];
        coords[fixed_idx] = fixed_val;
        coords[fi0] = u;
        coords[fi1] = v;

        // Bounds check
        let mut valid = true;
        for i in 0..3 {
            if coords[i] - r < -EPS || coords[i] + r > bin_dims[i] + EPS {
                valid = false;
                break;
            }
        }
        if !valid {
            continue;
        }

        let score = coords[0] + coords[1] + coords[2];
        if score < best_score {
            best_score = score;
            best = Some(Point3f::new(coords[0], coords[1], coords[2]));
        }
    }

    best
}

/// Solve position for three spheres (Apollonius problem in 3D).
/// Returns up to two solutions; picks the valid one with lower score.
pub fn solve_three_spheres(
    c1: &Point3f,
    r1: f32,
    c2: &Point3f,
    r2: f32,
    c3: &Point3f,
    r3: f32,
    r: f32,
    bin_w: f32,
    bin_h: f32,
    bin_d: f32,
) -> Option<Point3f> {
    let d1 = r + r1;
    let d2 = r + r2;
    let d3 = r + r3;

    // We solve:
    //   |P - C1|^2 = d1^2  ... (1)
    //   |P - C2|^2 = d2^2  ... (2)
    //   |P - C3|^2 = d3^2  ... (3)
    //
    // (1)-(2) and (1)-(3) give two linear equations (two planes).
    // Their intersection is a line. Substituting into (1) gives a quadratic.

    // (1)-(2): 2*(C2-C1).P = d1^2 - d2^2 - |C1|^2 + |C2|^2
    let rhs12 = d1 * d1 - d2 * d2 - len_sq(c1) + len_sq(c2);
    // (1)-(3): 2*(C3-C1).P = d1^2 - d3^2 - |C1|^2 + |C3|^2
    let rhs13 = d1 * d1 - d3 * d3 - len_sq(c1) + len_sq(c3);

    let n1 = scale(&sub(c2, c1), 2.0); // normal of plane 1
    let n2 = scale(&sub(c3, c1), 2.0); // normal of plane 2

    // Line = intersection of the two planes n1.P = rhs12, n2.P = rhs13
    // Direction = n1 x n2
    let dir = cross(&n1, &n2);
    if len_sq(&dir) < EPS * EPS {
        return None; // Planes are parallel — degenerate
    }

    // Find a point on the line by solving the 2-plane system + setting one coord to find a particular solution.
    // Use the coordinate where dir has the largest component as the free parameter.
    let dir_abs = [dir.x.abs(), dir.y.abs(), dir.z.abs()];
    let free_idx = if dir_abs[0] >= dir_abs[1] && dir_abs[0] >= dir_abs[2] {
        0
    } else if dir_abs[1] >= dir_abs[2] {
        1
    } else {
        2
    };

    // Set P[free_idx] = 0, solve the 2x2 system for the other two.
    let n1_arr = [n1.x, n1.y, n1.z];
    let n2_arr = [n2.x, n2.y, n2.z];

    let other: Vec<usize> = (0..3).filter(|&i| i != free_idx).collect();
    let oi0 = other[0];
    let oi1 = other[1];

    // n1[oi0]*p0 + n1[oi1]*p1 = rhs12
    // n2[oi0]*p0 + n2[oi1]*p1 = rhs13
    let det = n1_arr[oi0] * n2_arr[oi1] - n1_arr[oi1] * n2_arr[oi0];
    if det.abs() < EPS {
        return None;
    }

    let p0 = (rhs12 * n2_arr[oi1] - rhs13 * n1_arr[oi1]) / det;
    let p1 = (n1_arr[oi0] * rhs13 - n2_arr[oi0] * rhs12) / det;

    let mut base = [0.0f32; 3];
    base[oi0] = p0;
    base[oi1] = p1;
    // base[free_idx] = 0.0 already

    let base_pt = Point3f::new(base[0], base[1], base[2]);
    let dir_norm = normalize(&dir)?;

    // Substitute P = base_pt + t * dir_norm into (1):
    // |base_pt + t*dir_norm - C1|^2 = d1^2
    let diff = sub(&base_pt, c1);
    let a_coeff = 1.0; // |dir_norm|^2 = 1
    let b_coeff = 2.0 * dot(&diff, &dir_norm);
    let c_coeff = len_sq(&diff) - d1 * d1;

    let disc = b_coeff * b_coeff - 4.0 * a_coeff * c_coeff;
    if disc < 0.0 {
        return None;
    }

    let sqrt_disc = disc.sqrt();
    let t1 = (-b_coeff - sqrt_disc) / (2.0 * a_coeff);
    let t2 = (-b_coeff + sqrt_disc) / (2.0 * a_coeff);

    let mut best: Option<Point3f> = None;
    let mut best_score = f32::MAX;

    for &t in &[t1, t2] {
        let p = add(&base_pt, &scale(&dir_norm, t));

        // Bounds check
        if p.x - r < -EPS || p.y - r < -EPS || p.z - r < -EPS {
            continue;
        }
        if p.x + r > bin_w + EPS || p.y + r > bin_h + EPS || p.z + r > bin_d + EPS {
            continue;
        }

        let score = p.x + p.y + p.z;
        if score < best_score {
            best_score = score;
            best = Some(p);
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_three_planes_corner() {
        let pos = solve_three_planes(
            [(Axis::X, 0.0), (Axis::Y, 0.0), (Axis::Z, 0.0)],
            5.0,
            100.0, 100.0, 100.0,
        );
        assert!(pos.is_some());
        let p = pos.unwrap();
        assert!((p.x - 5.0).abs() < EPS);
        assert!((p.y - 5.0).abs() < EPS);
        assert!((p.z - 5.0).abs() < EPS);
    }

    #[test]
    fn test_three_planes_max_corner() {
        let pos = solve_three_planes(
            [(Axis::X, 100.0), (Axis::Y, 100.0), (Axis::Z, 100.0)],
            5.0,
            100.0, 100.0, 100.0,
        );
        assert!(pos.is_some());
        let p = pos.unwrap();
        assert!((p.x - 95.0).abs() < EPS);
        assert!((p.y - 95.0).abs() < EPS);
        assert!((p.z - 95.0).abs() < EPS);
    }

    #[test]
    fn test_sphere_two_planes() {
        // Sphere at (5,5,5) r=5. New sphere r=5 on floor (Y=0) and left wall (X=0).
        // X=5, Y=5, solve for Z: |P - C|^2 = 100
        // (5-5)^2 + (5-5)^2 + (z-5)^2 = 100 => z = 15 or z = -5
        // z=-5 is invalid, so z=15
        let pos = solve_sphere_two_planes(
            &Point3f::new(5.0, 5.0, 5.0),
            5.0,
            (Axis::X, 0.0),
            (Axis::Y, 0.0),
            5.0,
            100.0, 100.0, 100.0,
        );
        assert!(pos.is_some());
        let p = pos.unwrap();
        assert!((p.x - 5.0).abs() < EPS);
        assert!((p.y - 5.0).abs() < EPS);
        assert!((p.z - 15.0).abs() < EPS);
    }

    #[test]
    fn test_three_spheres_equal() {
        // Three spheres of radius 5 on the floor at known positions.
        // Place a fourth sphere of radius 5 tangent to all three.
        let c1 = Point3f::new(5.0, 5.0, 5.0);
        let c2 = Point3f::new(15.0, 5.0, 5.0);
        let c3 = Point3f::new(10.0, 5.0, 13.66);

        let pos = solve_three_spheres(
            &c1, 5.0, &c2, 5.0, &c3, 5.0,
            5.0,
            100.0, 100.0, 100.0,
        );
        assert!(pos.is_some());
        let p = pos.unwrap();
        // Verify tangency
        let d1 = len(&sub(&p, &c1));
        let d2 = len(&sub(&p, &c2));
        let d3 = len(&sub(&p, &c3));
        assert!((d1 - 10.0).abs() < 0.1);
        assert!((d2 - 10.0).abs() < 0.1);
        assert!((d3 - 10.0).abs() < 0.1);
    }
}
