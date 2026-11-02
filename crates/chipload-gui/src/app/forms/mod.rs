// SPDX-License-Identifier: GPL-3.0-or-later
//! The property forms, by what they are for: an operation (`operation`, `passes`), the
//! project (`project`), the machine and tools (`machine`), the drawing sources (`sources`),
//! and the pieces they share (`widgets`).

mod machine;
mod operation;
mod passes;
mod project;
mod sources;
mod widgets;

pub(super) use machine::*;
pub(super) use operation::*;
pub(super) use passes::*;
pub(super) use project::*;
pub(super) use sources::*;
pub(super) use widgets::*;

use super::problems::{self, flag};
use super::user_library::{LibraryRequest, LibraryRows, library_rows};
use super::*;
use crate::editor::DEFAULT_STEP_DOWN;
use crate::i18n::{Lang, Language};
use chipload_job::library::{Cutting, MachineProfile};
use chipload_job::{ClearingPass, ImageTrace, MaterialRef, Placement, Units};
use chipload_job::{Copies, CopyOrder, Corners, DEFAULT_CORNER_ANGLE_RAD, MAX_COPIES_PER_SIDE};
use chipload_job::{
    DEFAULT_RAMP_ANGLE_RAD, Entry, Lead, MAX_RAMP_ANGLE_RAD, Machine, Tool, rest_refusal,
};
use chipload_job::{
    Finish, HoleMethod, HolesOp, ProfileOp, Rest, STRAIGHT_FLANK_RAD, Skin, ToolChange,
};
use chipload_pipeline::library::Library;
use chipload_pipeline::placement::Move as PlaceMove;

/// What the job form asks of the app beyond editing values.
pub(super) enum JobRequest {
    /// Choose the project's material by uid, or none (docs/specs/materials.md §2).
    UseMaterial(Option<String>),
    /// docs/specs/tool-library-and-machines.md §8.
    Library(LibraryRequest),
    /// Open operation i (a rest pass's pocket).
    OpenOperation(usize),
    /// Show the work zero's properties (docs/specs/work-zero-marker.md §4).
    OpenWorkZero,
    /// Make setup `id` the active one; remove it (docs/specs/setups.md §7).
    ActivateSetup(u64),
    RemoveSetup(u64),
    /// Remove pocket i's rest pass k (docs/specs/rest-passes.md §6).
    RemoveRestPass(usize, usize),
    /// Remove V-carve i's floor pass.
    RemoveFloorPass(usize),
    /// Set part `id`'s pitch so this many mm are left between its copies' cuts
    /// (docs/specs/copies-grid.md decision 2).
    CopiesGap(u64, f64),
    /// Make part `id` the active part (docs/specs/parts.md §8).
    Activate(u64),
    /// The open tool was removed: show the tools instead.
    ToolRemoved,
    /// Write a setup sheet with exported G-code, or not (an app setting).
    /// DXF `id`: update it from a file, or remove it (docs/specs/properties-for-tools-and-
    /// dxfs.md §3).
    UpdateDxf(u64),
    RemoveDxf(u64),
    /// Explode DXF `id` into drawn shapes (docs/specs/explode-and-join.md §5).
    ExplodeDxf(u64),
    /// Picture `id`: its window, a new size, place and layer, or removed (§3b).
    TraceSettings(u64),
    ImagePlace(u64, ImagePlace),
    RemoveImage(u64),
    /// Text `id` as its properties left it, or removed (docs/specs/text.md §5).
    TextChange(u64, Box<chipload_job::Text>),
    RemoveText(u64),
    /// Move DXF `id` (docs/specs/drawing-placement.md, drawing-sources.md §2).
    Place(u64, PlaceMove),
    /// Make this the project's post-processor; one of the user's is copied in (again:
    /// updated) (docs/specs/user-posts.md).
    Post(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_offers_only_larger_endmills() {
        // docs/specs/rest-machining.md §6.
        let tool = |id: i64, kind: ToolType, diameter: f64| Tool {
            id,
            name: None,
            library_uid: None,
            kind,
            diameter,
            flutes: None,
            center_cutting: None,
            rpm: None,
            angle: None,
            tip_diameter: None,
        };
        let tools = [
            tool(1, ToolType::Endmill, 12.0),
            tool(2, ToolType::Endmill, 3.0),
            tool(3, ToolType::Drill, 8.0),
            tool(4, ToolType::Vbit, 12.7),
            tool(5, ToolType::Endmill, 6.0),
        ];
        let ids = |d: f64| {
            rest_choices(&tools, Some(&tool(9, ToolType::Endmill, d)))
                .map(|t| t.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(3.0), [1, 5]);
        assert_eq!(ids(6.0), [1]);
        assert!(ids(12.0).is_empty());
        assert!(rest_choices(&tools, None).next().is_none());
    }

    #[test]
    fn summaries_are_short() {
        assert_eq!(len_in(Units::Mm, 12.0), "12");
        assert_eq!(len_in(Units::Mm, 0.25), "0.25");
        assert_eq!(len_in(Units::Mm, 12.3456), "12.346");
        assert_eq!(len_in(Units::Inch, 6.35), "0.25");
        let ramp = Entry::Ramp {
            ramp_angle: 3f64.to_radians(),
        };
        let en = Language::En;
        assert_eq!(entry_summary(en, &ramp, None, Units::Mm), "ramp 3.0°");
        let helix = Entry::Helix {
            radius: 1.5,
            ramp_angle: 3f64.to_radians(),
        };
        let lead = Lead { radius: 2.0 };
        assert_eq!(
            entry_summary(en, &helix, Some(&lead), Units::Mm),
            "helix r 1.5 mm, 3.0°, lead 2 mm"
        );
        assert_eq!(
            entry_summary(en, &helix, Some(&lead), Units::Inch),
            "helix r 0.0591 in, 3.0°, lead 0.0787 in"
        );
    }
}
