# Spec: Chipload in the browser

Status: **step 1 approved and built 2026-10-13** ("Go ahead with recommendations"): the
libraries keep compiling for the browser, checked in CI. **Step 2, the browser app, comes
later**, after the desktop app has had user testing; its decisions (§4) are proposals, to
be reviewed when it starts.

## 1. Why

A browser version lets people try Chipload without installing anything. It matters most
once the repository is public and people arrive from the user guide. It is a second way to
run the same app, not a new feature (ADR 0017's lean scope). Everything runs in the
browser: no server, and drawings never leave the user's computer.

## 2. Step 1: keep the door open (built)

- **The check:** CI runs `cargo check --target wasm32-unknown-unknown` on the libraries:
  core, job, dxf, toolpath, post, check and pipeline. It's a compile check, not clippy:
  some lints fire only on that target, such as `drop_non_drop` on `File`, and the native
  clippy already covers the code.
- **Randomness sources:** two dependencies need one in the browser:
  - the dxf crate's `uuid`: the `js` feature;
  - boostvoronoi's map, through `rand` and `getrandom` 0.3: the `wasm_js` feature, plus
    the `getrandom_backend` cfg in `.cargo/config.toml`.

  Both are dependencies for `wasm32` only (MIT/Apache-2.0, through `wasm-bindgen` and
  `js-sys`). Native builds, their notices and the G-code are unchanged. Randomness never
  reaches output: boostvoronoi's map uses it only for its internal layout.
- **Run it locally:**
  ```
  cargo check --target wasm32-unknown-unknown -p chipload-core -p chipload-job \
    -p chipload-dxf -p chipload-image -p chipload-toolpath -p chipload-post -p chipload-check -p chipload-pipeline
  ```

**Known gaps**, which compile but fail when they run in a browser, for step 2:
- `SystemTime::now()` in `chipload-pipeline`'s recovery panics.
- `std::fs` returns errors.
- `catch_unwind` doesn't catch: a panic aborts.

## 3. Step 2: the browser app (later)

eframe's web runner, built with trunk or wasm-bindgen, as static files. What changes:

1. **Panics:** a panic can't be caught in the browser, so the `catch_unwind` guards (DXF
   parsing, offsets, pockets, generation, simulation) stop working.
2. **Threads:** generation and simulation run on threads today, and the browser has no
   plain threads.
3. **Files:**
   - opening a file is an upload;
   - saving is a download, or a write back to the same file where the File System Access
     API exists (Chrome, Edge);
   - a program with several tools is several downloads, or a zip.
4. **Storage:** the tool library, machines, settings, user posts and recovery notes move to
   browser storage. There's no `.bak` file.
5. **Fonts:** the GUI loads installed system fonts, which a page can't read.
6. **Clock:** `Instant::now()` panics in the browser.
7. **Speed:** expect it 1.5 to 2 times slower than native. That's fine for typical parts;
   large V-carves will feel it.

## 4. Decisions for step 2 (proposals)

1. **A Web Worker for generation and simulation**, with the job and the result passed as
   messages. **Recommended:** it keeps the page responsive, and a panic kills only the
   worker, which the page reports as an error and restarts, so the safety rule "a panic
   becomes an error" still holds. Alternative: run on the page itself; that's simpler, but
   it freezes the page while generating, and one panic loses unsaved work.
2. **Files: upload and download everywhere, write back where the browser allows.**
   **Recommended.** Several output files go as one zip. Alternative: download only.
3. **Storage: IndexedDB for the library, machines, posts and recovery; localStorage for
   settings.** **Recommended.** Export and import of the library already exist, and the
   page points to them, since browser storage can be cleared.
4. **Font: bundle one, with a license from the allowed list.** **Recommended:** choose it
   when step 2 starts; egui's own fonts were turned down on 2026-10-09 (OFL isn't on the
   list). Needs a license decision if it's OFL.
5. **Clock: the `web-time` crate** (MIT/Apache-2.0) in place of `std::time::Instant` in
   the GUI. **Recommended:** one swap, the same on native.
6. **Hosting: next to the user guide** (GitHub Pages, once public), as `…/app/`, built by
   the same release steps. **Recommended.**
7. **Left out of the browser version:** the command line, `.bak` files and output folders.
   **Recommended:** the desktop app stays the full version; the page says so and links to
   the downloads.
8. **Same output:** the browser version must give byte-identical G-code to the desktop for
   the golden corpus. A test runs the golden jobs under wasm (wasm-bindgen-test or
   wasmtime) and compares. **Recommended:** determinism is a project rule (CLAUDE.md), and
   floats in wasm are IEEE 754 like native, but this proves it.
