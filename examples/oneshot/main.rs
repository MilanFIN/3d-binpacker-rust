use rustport::common::bin::Bin;
use rustport::common::bin_box::BinBox;
use rustport::common::point3f::Point3f as SolverPoint;
use rustport::solver::rectangles::best_fit_ems::BestFitEMS;
use rustport::solver::solver_interface::Solver;
use rustport::solver::common::solver_properties::SolverProperties;

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

    println!("Initializing BestFitEMS...");
    
    let mut solver = BestFitEMS::default();
    let props = SolverProperties::new(
        bin.clone(),
        false,
        "x".to_owned(),
        vec![0, 1, 2],
        0.0
    );
    solver.init(&props);

    println!("Running one-shot packing...");
    let solved = solver.solve(&boxes);
    
    println!("Done.");
    let mut total_boxes_packed = 0;
    for (i, bin_content) in solved.bins.iter().enumerate() {
        println!("Bin {} contains {} boxes", i, bin_content.len());
        total_boxes_packed += bin_content.len();
    }
    println!("Total boxes packed: {} / {}", total_boxes_packed, boxes.len());
    
    Ok(())
}
