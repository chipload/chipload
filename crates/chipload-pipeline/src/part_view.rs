// SPDX-License-Identifier: GPL-3.0-or-later
//! What the window plans while a part is the one shown (docs/specs/parts-and-setups.md
//! §8a): the part alone, once, where it is drawn, on the board it is cut from; never
//! written as G-code (an export plans its setup).

use chipload_job::{DEFAULT_PIECE_MARGIN, Instance, Job};

/// Part `id` planned alone, once, where it is drawn: every other part off, its board
/// surfaces and dowel holes off (they are the board's work), its home board's stock with
/// the part's own thickness and material when it asks for them, its through cuts following.
///
/// Drawn in place on that board (the simple case), the board is the setup's as it is.
/// Placed elsewhere, or only moved, turned or copied there, the board is fitted round the
/// part's shapes with the stock's margin (`placed`, never saved), and the machine's travel
/// isn't checked: the part as drawn may lie anywhere in the design space.
///
/// None when there is no such part, or it is on no board and there is none to borrow.
pub fn part_view(job: &Job, id: u64) -> Option<Job> {
    job.part(id)?;
    let home = job.home_setup(id).or_else(|| job.active_setup_id())?;
    let mut view = job.for_setup(home);
    let index = view.setups.iter().position(|s| s.id == home)?;
    let drawn_here = view.setups[index]
        .instances
        .iter()
        .any(|i| i.part == id && i.enabled && i.is_drawn());
    for part in &mut view.parts {
        part.enabled = part.id == id;
    }
    for op in &mut view.operations {
        if op.part() == id && (op.is_board_surface() || op.is_dowels()) {
            op.set_enabled(false);
        }
    }
    let wanted = view.part(id).and_then(|p| p.stock.clone());
    let box_ = view.part_box(id);
    let setup = &mut view.setups[index];
    let others: Vec<Instance> = setup
        .instances
        .iter()
        .filter(|i| i.part != id)
        .cloned()
        .collect();
    setup.instances = others;
    setup.instances.push(Instance::drawn(id, 1));
    if let Some(stock) = wanted {
        if let Some(thickness) = stock.thickness {
            setup.stock.thickness = thickness;
        }
        if stock.material.is_some() {
            setup.stock.material = stock.material;
        }
    }
    setup.pieces = None;
    if !drawn_here {
        setup.back_of = None;
        if let Some(((x0, y0), (x1, y1))) = box_ {
            let margin = setup.stock.fit.unwrap_or(DEFAULT_PIECE_MARGIN);
            setup.placed = Some(((x0 - margin, y0 - margin), (x1 + margin, y1 + margin)));
        }
        view.machine.travel = None;
    }
    view.active_part = Some(id);
    view.follow_thickness();
    Some(view)
}
