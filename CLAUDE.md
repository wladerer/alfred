# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Alfred is a Rust-based visualization and analysis platform for atomistic simulation data, focused on VASP workflows. It combines structure/volumetric data visualization, symmetry analysis, and research productivity tools. Think: OVITO visualization + VESTA simplicity + pymatgen workflow integration, built in Rust.

## Build & Development

Rust stable 1.75+. System deps: `cmake`, `libclang-dev`, `libasound2-dev`, `libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`.

- `cargo build` — dev build
- `cargo build --release` — optimized release (stripped, LTO)
- `cargo test` — run all tests (14 currently)
- `cargo test <test_name>` — run a single test
- `cargo run -- POSCAR` — run with a structure file
- `cargo run -- vasprun.xml` — run with vasprun data
- `cargo run -- density.vasp` — run with volumetric data

## Architecture

Five layers, dependencies flow **downward only**:

1. **IO** (`src/io/`) — POSCAR parser, vendored vasprun.xml parser (from vasprunrs), volumetric data parser. No analysis logic.
2. **Data** (`src/data/`) — `Structure` (lattice + positions + atomic numbers), `VolumeGrid` (3D scalar fields), `ElementData` (colors/radii from `resources/atoms.json`).
3. **Analysis** (`src/analysis/`) — Symmetry (spglib), marching cubes isosurface, magnetic moment computation from PDOS. No rendering.
4. **Visualization** (`src/vis/`) — Atom spheres, arrows (forces/moments), isosurface meshes, unit cell, Wyckoff highlights, mirror planes, axes gizmo, selection highlight, LDOS coloring. No file parsing.
5. **UI/Application** (`src/ui/`, `src/main.rs`) — Bevy ECS app, egui panels (menu bar, vasprun panel, isosurface panel, atom info, Wyckoff legend), camera control, keyboard shortcuts.

## Key Design Constraints

- No layer may depend on a higher layer.
- All file formats convert to canonical models (`Structure`, `VolumeGrid`). Analysis code never depends on file formats directly.
- The vasprun.xml parser is vendored from `~/github/vasprunrs` (MIT license) in `src/io/vasprun/`. Module paths use `super::` to reference the vendored types/errors.
- Element colors and radii are loaded at compile time from `resources/atoms.json` via `include_str!`.
- Bevy features are explicitly trimmed in Cargo.toml to reduce binary size.

## Key Dependencies

| Purpose | Crate |
|---|---|
| Rendering engine | bevy (0.15, trimmed features) |
| UI panels | bevy_egui (0.33) |
| Math | nalgebra, ndarray |
| Symmetry | spglib (builds C library via cmake) |
| XML parsing | quick-xml (vendored vasprun parser) |
| File dialogs | rfd |
| Compression | flate2 (gzipped vasprun.xml) |

## Non-Goals

Does **not** aim to replace pymatgen, ASE, or phonopy. Focus is visualization, analysis integration, and research productivity.
