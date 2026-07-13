use threecrate_core::{Point3f, TriangleMesh, Vector3};
use threecrate_visualization::InteractiveViewer;

use rustport::common::bin::Bin;
use rustport::common::box_spec::BinBox;
use rustport::common::point3f::Point3f as SolverPoint;
use rustport::optimizer::base::CpuOptimizer;
use rustport::solver::best_fit_ems::BestFitEMS;
use rustport::solver::solver_interface::Solver;
use rustport::solver::solver_properties::SolverProperties;

use rand::Rng;

fn make_boxes_mesh(boxes: &[BinBox]) -> TriangleMesh {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut faces = Vec::new();

    let c_x = [255u8, 100u8, 100u8];
    let c_y = [100u8, 255u8, 100u8];
    let c_z = [100u8, 100u8, 255u8];
    let face_colors = [c_x, c_x, c_y, c_y, c_z, c_z];

    let face_normals = [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 0.0, -1.0),
    ];

    let local = [
        [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5], [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5], [-0.5, -0.5, -0.5],
        [-0.5, 0.5, -0.5], [-0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5], [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5],
        [0.5, -0.5, -0.5], [-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5],
    ];

    let local_faces = vec![
        [0, 1, 2], [0, 2, 3],
        [4, 5, 6], [4, 6, 7],
        [8, 9, 10], [8, 10, 11],
        [12, 13, 14], [12, 14, 15],
        [16, 17, 18], [16, 18, 19],
        [20, 21, 22], [20, 22, 23],
    ];

    for b in boxes {
        let v_offset = vertices.len() as u32;

        for i in 0..24 {
            let lx = (local[i][0] + 0.5) * b.size.x + b.position.x;
            let ly = (local[i][1] + 0.5) * b.size.y + b.position.y;
            let lz = (local[i][2] + 0.5) * b.size.z + b.position.z;
            vertices.push(Point3f::new(lx, ly, lz));
            let face_id = i / 4;
            normals.push(face_normals[face_id]);
            colors.push(face_colors[face_id]);
        }

        for f in &local_faces {
            faces.push([
                (v_offset + f[0] as u32) as usize,
                (v_offset + f[1] as u32) as usize,
                (v_offset + f[2] as u32) as usize,
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

fn generate_random_boxes(count: usize) -> Vec<BinBox> {
    let mut rng = rand::thread_rng();
    let mut boxes = Vec::with_capacity(count);
    for i in 0..count {
        let w = rng.gen_range(2.0..10.0);
        let h = rng.gen_range(2.0..10.0);
        let d = rng.gen_range(2.0..10.0);
        boxes.push(BinBox::new_without_weight(
            i as i32,
            SolverPoint::new(0.0, 0.0, 0.0),
            SolverPoint::new(w, h, d),
        ));
    }
    boxes
}

fn main() -> anyhow::Result<()> {
    println!("Generating 100 random boxes...");
    let boxes = generate_random_boxes(100);

    let bin = Bin::new(0, 30.0, 30.0, 30.0);

    println!("Initializing CpuOptimizer with FirstFit3D...");
    
    let props_bin = bin.clone();
    let solver_factory = move || {
        let mut solver = BestFitEMS::default();
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
        solver_factory,
        boxes,
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
    
    println!("Formatting placed boxes for display...");
    let mut display_boxes = Vec::new();
    let mut bin_offset_x = -50.0;
    
    // Shift the bins along X axis so they don't overlap completely
    for bin_content in solved {
        for mut b in bin_content {
            b.position.x += bin_offset_x;
            display_boxes.push(b);
        }
        
        bin_offset_x += 40.0;
    }

    println!("Launching 3D Viewer...");
    let mesh = make_boxes_mesh(&display_boxes);
    let mut viewer = InteractiveViewer::new()?;
    viewer.set_mesh(&mesh);
    viewer.run()?;
    
    Ok(())
}
