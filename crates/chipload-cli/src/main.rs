// SPDX-License-Identifier: GPL-3.0-or-later
//! The `chipload` command line: import drawings into projects, plan and run them, check
//! G-code. Projects are `.chipload` files (docs/specs/project-file.md).

// A CLI's job is to print; the workspace-wide lint is meant for library crates.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod dxfs;
mod frames;
mod groups;
mod images;
mod instances;
mod lengths;
mod library;
mod materials;
mod models;
mod nesting;
mod parts;
mod reliefs;
mod setups;
mod shape_library;
mod signs;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_check::{CheckIssue, check_gcode, limits_for_tool, limits_from_job};
use chipload_core::chain::Contour;
use chipload_dxf::{
    ImportResult, UnitsFrom, contours_from_shapes, import_dxf, is_svg, shapes_from_file, units_name,
};
use chipload_i18n::{Language, Message};
use chipload_job::edit::Mirror;
use chipload_job::{
    Copies, Geometry, Instance, InstanceOrder, Job, Placement, Spindle, StockCorner, Units,
    WorkZero, ZZero, load_job, polyline_segment_count, save_job,
};
use chipload_pipeline::chip_load::{chip_load, chip_load_lines};
use chipload_pipeline::copies::{copies_text, instances_text, parts_lines, pitch_for_gap};
use chipload_pipeline::drawing_edit::{
    AlignTo, AlignWith, Combine, Corners, Cut, End, OffsetSide, Polar, align_shapes, array_grid,
    array_linear, array_polar, break_shape, combine_shapes, cut_shape_corners, distribute_shapes,
    extend_shape, offset_shapes, trim_shape,
};
use chipload_pipeline::estimate::{Lengths, estimate, estimate_lines};
use chipload_pipeline::explode::join_shapes;
use chipload_pipeline::library::{Library, settings_dir};
use chipload_pipeline::placement::{
    Move as PlaceMove, dxf_list, dxf_size, place_drawing, place_dxf, placement_text,
};
use chipload_pipeline::report::{
    format_contours, format_import_report, format_shapes, severity_name,
};
use chipload_pipeline::shapes::{
    add_shape, copy_shapes, delete_shape, drawn_middle, mirror_shapes, name_shape,
    set_construction, set_shape, transform_shapes,
};
use chipload_pipeline::sheet::{SheetInfo, setup_sheet, write_setup_sheet};
use chipload_pipeline::{
    PipelineError, RunResult, add_floor_pass, add_rest_pass, empty_project, foreign_text,
    load_drawing, new_project, run_job, set_op_rest, set_op_tool, update_drawing_with,
    write_atomically, write_gcode_files,
};
use chipload_post::{load_builtin_post, render_program};
use chipload_toolpath::arcs::Circle;
use chipload_toolpath::model::{Move, MoveKind, Program};
use chipload_toolpath::{ToolpathError, plan_job};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{Value, json};

mod mcp;
mod ops;
mod out;
mod placing;
mod reference;
mod running;
mod shapes;
mod styles;
mod texts;

use ops::*;
use placing::*;
use running::*;
use shapes::*;

use out::{EXIT_FAILED, Out};

mod commands;
mod inspect;
mod project_io;
mod values;

use commands::*;
use inspect::*;
use project_io::*;
use values::*;

#[derive(Parser)]
#[command(
    name = "chipload",
    version,
    about = "2.5D CAM: DXF in, checked G-code out"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Print one JSON object: the result, notes, files written, or the error with the
    /// project's problems (docs/specs/scripting.md §3).
    #[arg(long, global = true)]
    json: bool,
}

/// The command line's stack: Linux's default for a main thread. Windows gives its main
/// thread 1 MB, which the parser's command tree overflows in debug builds (CI, 2026-10-17:
/// every command exited with STATUS_STACK_OVERFLOW).
const STACK_BYTES: usize = 8 * 1024 * 1024;

fn main() -> ExitCode {
    let worker = std::thread::Builder::new()
        .name("chipload".into())
        .stack_size(STACK_BYTES)
        .spawn(command_line);
    match worker.map(std::thread::JoinHandle::join) {
        Ok(Ok(code)) => code,
        // A panic has already printed its message; a thread that can't start says why.
        Ok(Err(_)) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("error: can't start: {err}");
            ExitCode::FAILURE
        }
    }
}

fn command_line() -> ExitCode {
    let cli = Cli::parse();
    if matches!(cli.command, Command::Mcp) {
        return mcp::serve_stdio();
    }
    let mut out = Out::new(cli.json);
    let code = execute(&mut out, cli.command);
    out.finish(code)
}

/// Run one command, its output into `out` (also for the MCP server's tools).
fn execute(out: &mut Out, command: Command) -> ExitCode {
    let units = |u: Option<UnitsArg>| u.map(UnitsArg::value);
    match command {
        Command::Inspect {
            file,
            units: u,
            px_per_inch,
        } => inspect(out, &file, units(u), px_per_inch),
        Command::New { output, start } => create(
            out,
            empty_project(),
            "New empty project.",
            &output,
            &start,
            false,
        ),
        Command::Import {
            dxf,
            units: u,
            px_per_inch,
            output,
            start,
            no_rules,
        } => import(
            out,
            &dxf,
            (units(u), px_per_inch),
            &output,
            &start,
            !no_rules,
        ),
        Command::Update {
            project,
            from,
            units: u,
            px_per_inch,
            dxf,
            output,
        } => update(
            out,
            &project,
            &from,
            (units(u), px_per_inch),
            dxf,
            output.as_deref(),
        ),
        Command::Plan {
            project,
            parts,
            setup,
            sets,
        } => plan(out, &project, &with_setup(sets, setup), &parts),
        Command::Get { project, path } => get(out, &project, &path),
        Command::Set {
            project,
            values,
            output,
        } => set(out, &project, &values, output.as_deref()),
        Command::CheckProject { project } => check_project(out, &project),
        Command::Run {
            project,
            output,
            tool_change,
            no_sheet,
            sheet_units,
            sheet_lang,
            parts,
            setup,
            piece,
            sets,
        } => {
            let sheet = (!no_sheet).then(|| {
                (
                    units(sheet_units).unwrap_or(Units::Mm),
                    sheet_lang.map_or(Language::En, LangArg::value),
                )
            });
            run(
                out,
                &project,
                &output,
                tool_change,
                sheet,
                &sets,
                (&parts, setup, piece.as_deref()),
            )
        }
        Command::Sheet {
            project,
            output,
            gcode,
            tool_change,
            units: sheet_units,
            lang,
            parts,
            setup,
            sets,
        } => sheet(
            out,
            &project,
            &output,
            gcode.as_deref(),
            tool_change,
            (
                units(sheet_units).unwrap_or(Units::Mm),
                lang.map_or(Language::En, LangArg::value),
            ),
            (&with_setup(sets, setup), &parts),
        ),
        Command::Simulate {
            project,
            output,
            cell,
            no_image,
            after,
            hash,
            setup,
            piece,
            face,
            sets,
        } => simulate(
            out,
            &project,
            output.as_deref(),
            SimulateArgs {
                cell,
                after,
                image: !no_image,
                hash,
                back_face: face.map(|f| f == "back"),
                piece,
            },
            &with_setup(sets, setup),
        ),
        Command::Render { program, post } => render(out, &program, &post),
        Command::Check { gcode, job, tool } => check(out, &gcode, &job, tool),
        Command::Op { action } => op(out, action),
        Command::Shape { action } => shape(out, action),
        Command::Image { action } => images::image(out, action),
        Command::Model { action } => models::model(out, action),
        Command::Text { action } => texts::text(out, action),
        Command::Frame { action } => frames::frame(out, action),
        Command::Sign { action } => signs::sign(out, action),
        Command::Relief { action } => reliefs::relief(out, action),
        Command::Group { action } => groups::group_command(out, action),
        Command::Dxf { action } => dxfs::dxf(out, action),
        Command::Tool { action } => library::tool(out, action),
        Command::Material { action } => materials::material(out, action),
        Command::Style { action } => styles::style(out, action),
        Command::Machine { action } => library::machine(out, action),
        Command::Post { action } => library::post(out, action),
        Command::Part { action } => parts::part(out, action),
        Command::Nest(args) => nesting::nest(out, args),
        Command::Setup { action } => setups::setup(out, action),
        Command::Zero {
            project,
            stock,
            drawing_origin,
            at,
            z,
            output,
        } => zero(
            out,
            &project,
            ZeroArgs {
                stock,
                drawing_origin,
                at,
                z,
            },
            output.as_deref(),
        ),
        Command::Copies {
            project,
            part,
            grid,
            pitch,
            gap,
            order,
            off,
            output,
        } => copies(
            out,
            &project,
            CopiesArgs {
                part,
                grid,
                pitch,
                gap,
                order,
                off,
            },
            output.as_deref(),
        ),
        Command::Place {
            project,
            by,
            to,
            at,
            align,
            margin,
            rotate,
            scale,
            width,
            height,
            reset,
            dxf,
            image,
            text,
            output,
        } => place(
            out,
            &project,
            PlaceArgs {
                by,
                to,
                at,
                align,
                margin,
                rotate,
                scale,
                width,
                height,
                reset,
                dxf,
                image,
                text,
            },
            output.as_deref(),
        ),
        Command::Reference { page } => {
            out.text("text", &reference_page(page));
            ExitCode::SUCCESS
        }
        Command::Schema { format } => {
            let format = match format {
                SchemaArg::Project => chipload_job::schema::Format::Project,
                SchemaArg::Tools => chipload_job::schema::Format::Tools,
                SchemaArg::Machines => chipload_job::schema::Format::Machines,
                SchemaArg::Materials => chipload_job::schema::Format::Materials,
            };
            out.document(chipload_job::schema::schema(format));
            ExitCode::SUCCESS
        }
        Command::Licenses => {
            let text = format!(
                "{}\n\n{}\n{}\n",
                chipload_pipeline::notices::short_notice(),
                chipload_pipeline::notices::LICENSE,
                chipload_pipeline::notices::THIRD_PARTY
            );
            out.text("text", &text);
            ExitCode::SUCCESS
        }
        Command::Mcp => out.fail("the MCP server is started on its own: chipload mcp"),
    }
}

/// A generated page of the user guide (docs/specs/user-guide.md §4).
fn reference_page(page: ReferencePage) -> String {
    use chipload_job::schema::{Format, schema};
    match page {
        ReferencePage::Cli => {
            // Built, so every argument knows its value count.
            let mut root = <Cli as clap::CommandFactory>::command();
            root.build();
            reference::cli_page(&root)
        }
        ReferencePage::Project => reference::schema_page(
            "Project file",
            "A project (`.chipload`) is a JSON file. Chipload writes every field; when you write \
             one by hand or with a script, the fields marked *no* may be left out. Lengths are \
             in mm, angles in radians, feeds in mm/min. `chipload schema project` prints the \
             JSON Schema this page is made from.",
            &schema(Format::Project),
        ),
        ReferencePage::Tools => reference::schema_page(
            "Tool library file",
            "Your tool library (`tools.json` in the settings folder) and the files `tool export` \
             writes. Lengths in mm, angles in radians, feeds in mm/min.",
            &schema(Format::Tools),
        ),
        ReferencePage::Machines => reference::schema_page(
            "Machine profiles file",
            "Your machine profiles (`machines.json` in the settings folder) and the files \
             `machine export` writes. Lengths in mm, feeds in mm/min.",
            &schema(Format::Machines),
        ),
        ReferencePage::Materials => reference::schema_page(
            "Materials file",
            "Your materials (`materials.json` in the settings folder) and the files `material \
             export` writes: your own values per library tool. Lengths in mm, feeds in mm/min.",
            &schema(Format::Materials),
        ),
        ReferencePage::Posts => posts_page(),
    }
}

/// The post-processors' reference: the config's fields, then each built-in post's values.
fn posts_page() -> String {
    use std::fmt::Write as _;
    let config = schemars::SchemaGenerator::default()
        .into_root_schema_for::<chipload_post::PostConfig>()
        .to_value();
    let mut page = reference::schema_page(
        "Post-processors",
        "A post-processor is a small TOML file (`name.toml`) and a Jinja template \
         (`name.*.j2`); see [Post-processors](../posts.md) for writing your own. The TOML \
         file's fields:",
        &config,
    );
    page.push_str("## Built-in post-processors\n\n");
    page.push_str(
        "| Name | File extension | Decimals | Arcs | Spindle wait (s) | Tool changes | Note |\n\
         |---|---|---|---|---|---|---|\n",
    );
    for name in chipload_post::builtin_posts() {
        let Ok(post) = load_builtin_post(name) else {
            continue;
        };
        let changes: Vec<&str> = post.tool_changes.iter().map(|c| c.name()).collect();
        let _ = writeln!(
            page,
            "| `{name}` | `{}` | {} | {} | {} | {} | {} |",
            post.file_extension,
            post.decimals,
            if post.arcs { "G2/G3" } else { "lines" },
            post.spindle_dwell_s,
            changes.join(", "),
            post.note.as_deref().unwrap_or("")
        );
    }
    page
}
