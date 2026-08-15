use threecrate_core::{Point3f, TriangleMesh, Vector3};
use threecrate_visualization::InteractiveViewer;

use rustport::common::bin::Bin;
use rustport::common::sphere_spec::Sphere;
use rustport::common::point3f::Point3f as SolverPoint;
use rustport::optimizer::base::CpuOptimizer;
use rustport::solver::spheres::advancing_front::AdvancingFrontSpheres;
use rustport::solver::solver_interface::Solver;
use rustport::solver::common::solver_properties::SolverProperties;

use rand::Rng;
use std::f32::consts::PI;

fn make_spheres_mesh(spheres: &[Sphere]) -> TriangleMesh {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut faces = Vec::new();

    // Generate a simple UV sphere template
    let lat_segments = 12;
    let lon_segments = 24;
    
    let mut template_verts = Vec::new();
    let mut template_normals = Vec::new();
    
    for i in 0..=lat_segments {
        let v = i as f32 / lat_segments as f32;
        let phi = v * PI;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();
        
        for j in 0..=lon_segments {
            let u = j as f32 / lon_segments as f32;
            let theta = u * PI * 2.0;
            
            let x = sin_phi * theta.cos();
            let y = cos_phi;
            let z = sin_phi * theta.sin();
            
            template_verts.push(Point3f::new(x, y, z));
            template_normals.push(Vector3::new(x, y, z));
        }
    }
    
    let mut template_faces = Vec::new();
    for i in 0..lat_segments {
        for j in 0..lon_segments {
            let first = i * (lon_segments + 1) + j;
            let second = first + lon_segments + 1;
            
            template_faces.push([first, second, first + 1]);
            template_faces.push([second, second + 1, first + 1]);
        }
    }

    let color = [100u8, 200u8, 255u8];

    for s in spheres {
        let v_offset = vertices.len() as u32;

        for (i, v) in template_verts.iter().enumerate() {
            let vx = v.x * s.radius + s.position.x;
            let vy = v.y * s.radius + s.position.y;
            let vz = v.z * s.radius + s.position.z;
            vertices.push(Point3f::new(vx, vy, vz));
            normals.push(template_normals[i]);
            colors.push(color);
        }

        for f in &template_faces {
            faces.push([
                (v_offset + f[0]) as usize,
                (v_offset + f[1]) as usize,
                (v_offset + f[2]) as usize,
            ]);
        }
    }

    TriangleMesh {
        vertices,
        faces,
        normals: Some(normals),
        colors: Some(colors),
    }
}

fn generate_random_spheres(count: usize) -> Vec<Sphere> {
    let mut rng = rand::thread_rng();
    let mut spheres = Vec::with_capacity(count);
    for i in 0..count {
        let r = rng.gen_range(1.5..5.0);
        spheres.push(Sphere::new_without_weight(
            i as i32,
            SolverPoint::new(0.0, 0.0, 0.0),
            r,
        ));
    }
    spheres
}

fn main() -> anyhow::Result<()> {
    println!("Generating 150 random spheres...");
    let spheres = generate_random_spheres(150);

    let bin = Bin::new(0, 30.0, 30.0, 30.0);

    println!("Initializing CpuOptimizer with AdvancingFrontSpheres...");
    
    let props_bin = bin.clone();
    let solver_factory = move || {
        let mut solver = AdvancingFrontSpheres::default();
        let fresh_props = SolverProperties::new(
            props_bin.clone(),
            false,
            "x".to_owned(),
            vec![0, 1, 2],
            0.0
        );
        solver.init(&fresh_props);
        solver
    };
    
    let mut optimizer = CpuOptimizer::new(
        Box::new(solver_factory),
        spheres,
        bin.clone(),
        // growing_bin
        false,
        // grow_axis
        "x".to_owned(),
        // rotation_axes
        vec![0, 1, 2],
        // population_size
        30,
        // elite_count
        3,
        // threads
        0,
    );

    println!("Running 1 generation...");
    let solved = optimizer.execute_next_generation();
    
    let score = optimizer.rate(&solved);
    println!("Done. Best score rating: {}", score);
    
    println!("Formatting placed spheres for display...");
    let mut display_spheres = Vec::new();
    let mut bin_offset_x = -50.0;
    
    // Shift the bins along X axis so they don't overlap completely
    for bin_content in solved {
        for mut s in bin_content {
            s.position.x += bin_offset_x;
            display_spheres.push(s);
        }
        
        bin_offset_x += 40.0;
    }

    println!("Launching 3D Viewer...");
    let mesh = make_spheres_mesh(&display_spheres);
    let mut viewer = InteractiveViewer::new()?;
    viewer.set_mesh(&mesh);
    viewer.run()?;
    
    Ok(())
}
