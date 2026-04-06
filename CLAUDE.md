# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Alfred is a Rust-based visualization and analysis platform for atomistic simulation data, focused on VASP workflows. It combines structure/volumetric data visualization, symmetry analysis, and research productivity tools. Think: OVITO visualization + VESTA simplicity + pymatgen workflow integration, built in Rust.

## Build & Development

This is a Rust project targeting stable Rust 1.75+. Standard cargo commands apply:

- `cargo build` — build the project
- `cargo test` — run all tests
- `cargo test <test_name>` — run a single test
- `cargo clippy` — lint
- `cargo fmt` — format code
- `cargo run` — run the application

## Architecture

The system has five strict layers with dependencies flowing **downward only**:

1. **IO Layer** — File format parsing (POSCAR, CHGCAR, CIF, etc.) into canonical models. No analysis logic.
2. **Data Layer** — Canonical representations: `Structure` (lattice + positions + atomic numbers), `VolumeGrid` (3D scalar fields), `NeighborList`, `SymmetryDataset`.
3. **Analysis Layer** — Scientific algorithms (symmetry via spglib, neighbor detection via KD-tree, transformations, isosurfaces). No rendering.
4. **Visualization Layer** — Rendering scientific data (atoms as spheres, bonds as cylinders, isosurfaces as triangle meshes). No file parsing.
5. **Application Layer** — Bevy ECS: user interaction, UI panels (bevy_egui), scene control.

## Key Design Constraints

- **No layer may depend on a higher layer.** Analysis must never import visualization. IO must never import analysis.
- **All file formats convert to canonical internal models** (`Structure`, `VolumeGrid`). No analysis code depends on file formats directly.
- **Do not reimplement** symmetry detection, file parsing, or spacegroup analysis — use existing libraries (spglib, vasp-poscar, vaspchg_rs).
- **Do implement** visualization, workflow tools, and analysis pipelines.
- **Data-oriented design** — prefer simple structs, immutable scientific data, explicit transformations. Avoid deep inheritance/complex hierarchies.
- **All scientific transformations must be deterministic, traceable, and explicit.**

## Key Dependencies

| Purpose | Crate |
|---|---|
| Math | nalgebra, ndarray, glam (Bevy compat) |
| VASP IO | vasp-poscar, vaspchg_rs |
| Symmetry | spglib (Rust bindings) |
| Spatial queries | kiddo (KD-tree) |
| Parallelism | rayon |
| Rendering | bevy, bevy_egui, bevy_mod_picking |
| Python bridge | pyo3 (optional) |

## Non-Goals

This project does **not** aim to replace pymatgen, ASE, or phonopy. Focus is visualization, analysis integration, and research productivity.
