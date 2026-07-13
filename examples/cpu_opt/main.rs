use rustport::common::bin::Bin;
use rustport::common::box_spec::BinBox;
use rustport::common::point3f::Point3f as SolverPoint;
use rustport::optimizer::base::CpuOptimizer;
use rustport::solver::best_fit_ems::BestFitEMS;
use rustport::solver::solver_interface::Solver;
use rustport::solver::solver_properties::SolverProperties;

use rand::Rng;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating 100 random boxes...");
    let boxes = generate_random_boxes(100);

    let bin = Bin::new(0, 30.0, 30.0, 30.0);

    println!("Initializing CpuOptimizer with BestFitEMS...");
    
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

    println!("Running 2 generations...");
    let mut solved = Vec::new();
    for i in 1..=2 {
        println!("Generation {}...", i);
        solved = optimizer.execute_next_generation();
        let score = optimizer.rate(&solved);
        println!("Score: {}", score);
    }
    
    println!("Done.");
    let mut total_boxes_packed = 0;
    for (i, bin_content) in solved.iter().enumerate() {
        println!("Bin {} contains {} boxes", i, bin_content.len());
        total_boxes_packed += bin_content.len();
    }
    println!("Total boxes packed: {}", total_boxes_packed);
    
    Ok(())
}
