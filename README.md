# Alfred

A Rust-based visualization and analysis platform for atomistic simulation data, focused on VASP workflows.

## Features

- **Structure visualization** — POSCAR/CONTCAR loading, element-colored atom spheres with correct radii, periodic boundary images, unit cell outline, supercell generation
- **Symmetry analysis** — spacegroup detection via spglib, rotation axes, mirror planes, Wyckoff positions with colored highlights and legend
- **Camera controls** — orbit (left drag), pan (right drag), zoom (scroll), axis-aligned views (X/Y/Z keys), symmetry axis cycling (N), axis-locked rotation (Shift+N)
- **vasprun.xml support** — ionic step trajectory scrubber, force vectors, magnetic moments (including PDOS-derived for VASP 6.x), density of states plot with spin-polarized mirrored display
- **LDOS on structure** — color atoms by their DOS contribution at a selected energy, with orbital filtering (s/p/d/f)
- **Volumetric data** — CHGCAR/LOCPOT/wavefunction parsing, marching cubes isosurface extraction with auto isovalue detection, dual +/- surfaces for wavefunctions
- **Atom selection** — click to select, info panel with coordinates, Wyckoff site, and species
- **File dialogs** — native OS file picker for opening structures, vasprun.xml, and volumetric data
- **Screenshots** — F12 to save PNG

## Prerequisites

### Rust toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### System dependencies (Debian/Ubuntu)

```bash
sudo apt install -y cmake libclang-dev libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev pkg-config
```

- `cmake` + `libclang-dev` — required by spglib (symmetry library, built from source via cargo)
- `libasound2-dev` + `libudev-dev` — required by Bevy (rendering engine)
- `libwayland-dev` + `libxkbcommon-dev` — Wayland display support

### Fedora/RHEL

```bash
sudo dnf install -y cmake clang-devel alsa-lib-devel systemd-devel wayland-devel libxkbcommon-devel
```

### macOS

```bash
brew install cmake llvm
```

Bevy dependencies are handled automatically on macOS.

## Building

```bash
# Development build (fast iteration)
cargo build

# Release build (optimized, stripped, ~50% smaller)
cargo build --release

# Run tests
cargo test

# Run a single test
cargo test test_parse_nacl
```

## Usage

```bash
# Open a POSCAR file
cargo run -- POSCAR

# Open a vasprun.xml (with DOS, forces, trajectory)
cargo run -- vasprun.xml

# Open a volumetric data file (CHGCAR, wavefunction, etc.)
cargo run -- density.vasp

# Combine: structure + vasprun + volumetric
cargo run -- POSCAR vasprun.xml CHGCAR

# Release mode (faster rendering for large systems)
cargo run --release -- POSCAR
```

Files can also be opened at runtime via **File > Open...** dialogs.

## Keyboard shortcuts

| Key | Action |
|-----|--------|
| **Left drag** | Orbit camera |
| **Right drag** | Pan camera |
| **Scroll** | Zoom |
| **X / Y / Z** | Snap to axis-aligned view |
| **Space** | Reset to default view |
| **N** | Cycle symmetry axes |
| **Shift+N** | Lock/unlock rotation to symmetry axis |
| **U** | Toggle unit cell outline |
| **P** | Toggle periodic boundary images |
| **W** | Toggle Wyckoff position highlights |
| **M** | Toggle all mirror planes |
| **Esc** | Deselect atom |
| **F12** | Save screenshot |

## Architecture

```
src/
├── main.rs          # Bevy app, camera, UI wiring
├── data/            # Canonical data models (Structure, VolumeGrid, ElementData)
├── io/              # File parsers (POSCAR, vasprun.xml, volumetric)
├── analysis/        # Symmetry, marching cubes, magnetic moments
├── vis/             # Rendering (atoms, arrows, isosurface, unit cell, etc.)
└── ui/              # egui panels (menu, vasprun, atom info)
```

Layers depend downward only: IO → Data → Analysis → Visualization → Application.

## License

MIT
