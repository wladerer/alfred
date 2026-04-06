# Rust Materials Science Visualization & Analysis Platform

## Project Specification

---

# 1. Project Overview

## Goal

Develop a modern Rust-based visualization and analysis platform for atomistic simulation data with an initial focus on **VASP workflows**, designed to support:

* Structure visualization
* Volumetric data visualization
* Symmetry analysis
* Adsorption studies
* Workflow comparison
* Research productivity tools

The system should prioritize:

* Performance
* Memory safety
* Modular design
* Extensibility
* Scientific correctness
* Interactive visualization

---

# 2. Design Philosophy

## Core principles

### 1 — Separation of concerns

Strict separation between:

IO layer
Data layer
Analysis layer
Visualization layer
Application layer

No layer should depend on visualization except the UI layer.

---

### 2 — Internal canonical representations

All file formats must convert into standardized internal data models:

Structure
VolumeGrid

No analysis code should depend on file formats directly.

---

### 3 — Zero duplication of scientific work

Existing scientific libraries must be used when possible:

Do NOT reimplement:

Symmetry detection
File parsing
Spacegroup analysis

DO implement:

Visualization
Workflow tools
Analysis pipelines

---

### 4 — Data-oriented design

Prefer:

Simple data structures
Immutable scientific data
Explicit transformations

Avoid:

Deep inheritance trees
Complex object hierarchies

---

### 5 — Scientific reproducibility

All transformations must be:

Deterministic
Traceable
Explicit

---

# 3. Technology Stack

## Core language

Rust (stable)

MSRV target:

1.75+

---

## Dependencies

### Math

nalgebra
ndarray

Optional:

glam (Bevy compatibility)

---

### Scientific IO

vasp-poscar
vaspchg_rs
chemfiles (optional CIF/XYZ support)

I recommend looking at Ionizing on github. their repo rsgrad is wonderful and perhaps I could inheret part of it (or all of it?)

---

### Symmetry

spglib (Rust bindings)

---

### Algorithms

kiddo (KD-tree)
rayon (parallelism)

---

### Visualization

bevy
bevy_egui
bevy_mod_picking

---

### Optional scientific integration

pyo3 (Python bridge)

---

# 4. System Architecture

System layers:

Application Layer
Visualization Layer
Analysis Layer
Data Layer
IO Layer

Architecture diagram:

Application
Visualization
Analysis
Data
IO

Dependencies only flow downward.

---

# 5. Core Data Models

## 5.1 Structure

Canonical atomistic structure representation.

```rust
pub struct Structure {

    pub lattice: Mat3,

    pub positions: Vec<Vec3>,

    pub atomic_numbers: Vec<u8>,

    pub magnetic_moments: Option<Vec<Vec3>>,


}
```

Requirements:

Must support:

Cartesian coordinates
Fractional coordinates
Supercell generation
Transformations

Future extensions:

Charges
Velocities
Forces
Tags

---

## 5.2 VolumeGrid

Represents volumetric scalar fields.

Examples:

Charge density
Electrostatic potential
Spin density
Wavefunction amplitude

Definition:

```rust
pub struct VolumeGrid {

    pub lattice: Mat3,

    pub dims: [usize;3],

    pub data: Array3<f64>

}
```

Requirements:

Support:

Interpolation
Slicing
Isosurface extraction
Grid arithmetic

---

## 5.3 NeighborList

Used for bonding and coordination.

```rust
pub struct NeighborList {

    pub neighbors: Vec<Vec<usize>>

}
```

Generated using KD-tree queries.

---

## 5.4 SymmetryDataset

Wrapper around spglib output.

Contains:

Spacegroup number
International symbol
Symmetry operations
Wyckoff positions

---

# 6. IO Layer

## Responsibilities

Convert file formats into canonical models.

No analysis logic allowed.

---

## CIF reader (optional)

Source:

chemfiles

Output:

Structure

---

## Future IO targets

WAVECAR
DOSCAR
PROCAR
LOCPOT
ELFCAR

---

# 7. Analysis Layer

Contains scientific algorithms.

No rendering allowed.

---

## 7.1 Symmetry analysis

Using:

spglib

Provides:

Spacegroup detection
Primitive cells
Standard cells

---

## 7.2 Neighbor detection

Using:

KD-tree search.

Features:

Distance cutoff bonds
Coordination numbers
Local environments

---

## 7.3 Structure transformations

Implement:

Supercells
Rotations
Translations
Centering

---

## 7.4 Surface analysis (future)

Possible modules:

Slab detection
Vacuum detection
Surface atom identification
Adsorption site classification

---

## 7.5 Volumetric analysis

Implement:

Isosurfaces (marching cubes)
Planar slicing
Volume arithmetic

Future:

Bader interface
Charge integration

---

# 8. Visualization Layer

## Responsibilities

Render scientific data.

No file parsing allowed.

---

## Atom rendering

Representation:

Sphere meshes.

Features:

Element coloring
Radius scaling
Selection highlighting

---

## Bond rendering

Representation:

Cylinder meshes.

Features:

Distance cutoff bonds
Bond coloring

---

## Magnetic moments

Representation:

Arrow meshes.

Features:

Vector direction
Magnitude scaling

---

## Unit cell

Representation:

Line mesh.

Features:

Toggle visibility

---

## Volumetric rendering

Representation:

Triangle meshes.

Features:

Isosurfaces
Opacity control
Colormaps

---

# 9. Application Layer

Built with Bevy ECS.

Responsibilities:

User interaction
UI panels
Scene control
Workflow integration

---

## Scene entities

Examples:

Atom entity
Bond entity
Volume mesh entity
Unit cell entity

---

## Components

Examples:

Atom component
Position component
Element component
MagneticMoment component

---

## Systems

Examples:

Camera control
Selection updates
Structure loading
Mesh updates

---

# 10. UI Requirements

Use:

bevy_egui

Panels:

Structure tree
File loader
Visualization controls
Analysis tools

Future:

Workflow browser
Calculation comparison
Database view

---

# 11. Performance Requirements

Target:

Interactive performance for:

100k atoms
200³ grids

Use:

Rayon parallelism
GPU instancing
Lazy computation

---

# 12. Extensibility Goals

System should support future:

Plugins
Analysis modules
New file formats

Design approach:

Trait-based extension points.

Example:

```rust
trait AnalysisModule {

    fn run(&self, structure: &Structure);

}
```

---

# 13. MVP Definition

Minimum viable product must support:

POSCAR loading
Atom rendering
Camera controls
Basic bonds
Unit cell display

Stretch:

CHGCAR isosurface.

---

# 14. Development Phases

## Phase 1 — Core data

Structure model
POSCAR reader
Basic transformations

---

## Phase 2 — Viewer

Bevy setup
Atom rendering
Camera system

---

## Phase 3 — Analysis

Neighbor detection
Bond rendering
Symmetry integration

---

## Phase 4 — Volumetric

CHGCAR reader
Isosurface extraction

---

## Phase 5 — Research tools

Structure comparison
Adsorption tools
Workflow integration

---

# 15. Future Directions

Possible expansion:

VASP workflow browser
Calculation database
Automated analysis pipelines
Reaction coordinate visualization
Band structure visualization
k-path viewers

---

# 16. Non-Goals (Important)

Project does NOT aim to replace:

pymatgen
ASE
phonopy

Focus remains:

Visualization
Analysis integration
Research productivity

---

# 17. Long-Term Vision

Create a modern Rust-native scientific visualization environment combining:

OVITO visualization concepts
VESTA simplicity
pymatgen workflow integration

Target outcome:

A fast, extensible research platform tailored to atomistic simulation workflows.

---

# End Specification

