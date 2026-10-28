// SPDX-License-Identifier: GPL-3.0-or-later
//! The stock around the drawing, and the cut's outline (docs/specs/stock-around-drawing.md).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_dxf::shapes_from_dxf;
use chipload_job::StockOrigin;
use chipload_pipeline::placement::{Move, place_drawing};
use chipload_pipeline::{new_project, run_job};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

#[test]
fn a_new_project_s_stock_is_centred_on_the_drawing_and_fits_it() {
    // simple_part.dxf spans X 50..250, Y 50..170.
    let job = new_project(shapes_from_dxf(&corpus("dxf/simple_part.dxf"), None).unwrap());
    assert_eq!(job.stock.origin, StockOrigin::Drawing);
    assert_eq!(job.stock.fit, Some(10.0));
    assert_eq!(job.stock_size(), (220.0, 140.0));
    assert_eq!(job.stock_bounds(), ((40.0, 40.0), (260.0, 180.0)));
    // It follows the drawing when the drawing moves.
    let (moved, _) = place_drawing(&job, Move::By(-50.0, -50.0)).unwrap();
    assert_eq!(moved.stock_bounds(), ((-10.0, -10.0), (210.0, 130.0)));
    // Aligning the drawing to a stock that follows it is refused.
    assert!(place_drawing(&job, Move::Center).is_err());
}

#[test]
fn a_fixed_size_stays_centred_and_the_run_reports_the_cut_outline() {
    let mut job = new_project(shapes_from_dxf(&corpus("dxf/simple_part.dxf"), None).unwrap());
    job.stock.fit = None;
    (job.stock.x, job.stock.y) = (400.0, 300.0);
    assert_eq!(job.stock_bounds(), ((-50.0, -40.0), (350.0, 260.0)));
    // With an operation, the estimate knows the box the tool cuts in.
    let with_ops = chipload_job::load_job(&corpus("jobs/v5/drawn_part.chipload")).unwrap();
    let result = run_job(&with_ops).unwrap();
    let ((x0, y0), (x1, y1)) = result.estimate.outline.unwrap();
    assert!(x1 - x0 > 130.0 && y1 - y0 > 95.0);
}
