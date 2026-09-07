use rustport::common::bin::Bin;
use rustport::common::bin_box::BinBox;
use rustport::common::point3f::Point3f as SolverPoint;
use rustport::postprocessor::postprocessor_interface::Postprocessor;
use rustport::postprocessor::rectangles::box_last_bin_cleanup_processor::BoxLastBinCleanupProcessor;
use rustport::solver::common::solver_properties::SolverProperties;
use rustport::solver::rectangles::best_fit_ems::BestFitEMS;
use rustport::solver::solver_interface::Solver;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bin = Bin::new(0, 10.0, 10.0, 10.0);

    // Four (5x5x5) boxes in a (10x10x10) bin.
    // Standard EMS places Box 0 at (0,0,0), Box 1 at (5,0,0), Box 2 at (0,0,5),
    // and Box 3 stacked on top at (0,5,0).
    let boxes = vec![
        BinBox::new_without_weight(0, SolverPoint::new(0.0, 0.0, 0.0), SolverPoint::new(5.0, 5.0, 5.0)),
        BinBox::new_without_weight(1, SolverPoint::new(0.0, 0.0, 0.0), SolverPoint::new(5.0, 5.0, 5.0)),
        BinBox::new_without_weight(2, SolverPoint::new(0.0, 0.0, 0.0), SolverPoint::new(5.0, 5.0, 5.0)),
        BinBox::new_without_weight(3, SolverPoint::new(0.0, 0.0, 0.0), SolverPoint::new(5.0, 5.0, 5.0)),
    ];

    println!("Initializing BestFitEMS solver...");
    let mut solver = BestFitEMS::default();
    let props = SolverProperties::new(
        bin.clone(),
        false,
        "x".to_owned(),
        vec![0, 1, 2],
        0.0,
    );
    solver.init(&props);

    println!("Running Best-Fit EMS algorithm directly...");
    let mut pack_result = solver.solve(&boxes);

    // NON-POSTPROCESSED RESULT: The raw/original packing result from solver.solve() exists here in pack_result.bins
    println!("Non-postprocessed solution ({} bin(s)):", pack_result.bins.len());
    for (bin_idx, bin_boxes) in pack_result.bins.iter().enumerate() {
        println!("  Bin {}:", bin_idx);
        for b in bin_boxes {
            println!(
                "    Box id={} pos=({:.1}, {:.1}, {:.1}) size=({:.1}, {:.1}, {:.1})",
                b.id, b.position.x, b.position.y, b.position.z, b.size.x, b.size.y, b.size.z
            );
        }
    }

    println!("\nApplying BoxLastBinCleanupProcessor postprocessor...");
    let postprocessor = BoxLastBinCleanupProcessor::new();
    postprocessor.process(&mut pack_result.bins, &bin);

    // CLEANED UP RESULT: The postprocessed solution with the cleaned up last bin exists here in pack_result.bins
    println!("Postprocessed solution ({} bin(s)):", pack_result.bins.len());
    for (bin_idx, bin_boxes) in pack_result.bins.iter().enumerate() {
        println!("  Bin {}:", bin_idx);
        for b in bin_boxes {
            println!(
                "    Box id={} pos=({:.1}, {:.1}, {:.1}) size=({:.1}, {:.1}, {:.1})",
                b.id, b.position.x, b.position.y, b.position.z, b.size.x, b.size.y, b.size.z
            );
        }
    }

    Ok(())
}

