use bevy::prelude::*;
use std::path::Path;

use crate::analysis::{self, SymmetryAxes};
use crate::camera::CameraState;
use crate::data::{self, ElementData};
use crate::io;
use crate::ui::{self, menu::MenuAction};
use crate::vis::{self, atoms::AtomMarker, UnitCellOutline, WyckoffHighlight, MirrorPlaneVis};

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct PoscarPath(pub Option<String>);

#[derive(Resource)]
pub struct VasprunPath(pub Option<String>);

#[derive(Resource)]
pub struct VolumetricPath(pub Option<String>);

#[derive(Resource)]
pub struct LoadedVolume(pub Option<data::VolumeGrid>);

#[derive(Resource)]
pub struct LoadedStructure(pub Option<data::Structure>);

#[derive(Resource)]
pub struct StructureExtent(pub f32);

#[derive(Resource)]
pub struct SymmetryTolerance(pub f64);

/// Number of atoms in the primitive cell (before supercell expansion).
/// Used to map supercell atom indices back to PDOS/force data.
#[derive(Resource)]
pub struct PrimitiveAtomCount(pub usize);

#[derive(Resource)]
pub struct UnitCellVisible(pub bool);

#[derive(Resource)]
pub struct WyckoffVisible(pub bool);

#[derive(Resource)]
pub struct MirrorsVisible(pub bool);

#[derive(Resource)]
pub struct PeriodicImages(pub bool);

#[derive(Resource)]
pub struct IsoState {
    pub isovalue: f32,
    pub opacity: f32,
    pub show: bool,
    pub changed: bool,
}

impl Default for IsoState {
    fn default() -> Self {
        Self { isovalue: 0.0, opacity: 0.5, show: true, changed: false }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute the camera pivot (lattice centroid) and scene extent from a lattice matrix.
pub fn compute_pivot_and_extent(lattice: &nalgebra::Matrix3<f64>) -> (Vec3, f32) {
    let a = Vec3::new(lattice[(0, 0)] as f32, lattice[(0, 1)] as f32, lattice[(0, 2)] as f32);
    let b = Vec3::new(lattice[(1, 0)] as f32, lattice[(1, 1)] as f32, lattice[(1, 2)] as f32);
    let c = Vec3::new(lattice[(2, 0)] as f32, lattice[(2, 1)] as f32, lattice[(2, 2)] as f32);
    let pivot = (a + b + c) / 2.0;
    let corners = [Vec3::ZERO, a, b, c, a + b, a + c, b + c, a + b + c];
    let max_dist = corners.iter()
        .map(|corner| (*corner - pivot).length())
        .fold(0.0f32, f32::max);
    (pivot, max_dist.max(2.0) * 1.2)
}

/// Despawn all atom entities and the unit cell outline.
pub fn despawn_scene_entities(
    commands: &mut Commands,
    atom_query: &Query<Entity, With<AtomMarker>>,
    outline_query: &Query<Entity, With<UnitCellOutline>>,
) {
    for entity in atom_query.iter() {
        commands.entity(entity).despawn();
    }
    vis::despawn_unit_cell(commands, outline_query);
}

/// Reposition the camera to the default diagonal view for a given pivot and extent.
pub fn reposition_camera(cam: &mut Transform, pivot: Vec3, extent: f32) {
    let offset = Vec3::new(1.0, 1.0, 1.0).normalize() * extent * 2.0;
    cam.translation = pivot + offset;
    cam.look_at(pivot, Vec3::Y);
}

/// Convert a vasprun Structure to our canonical Structure type.
pub fn convert_vasprun_structure(vs: &io::vasprun::types::Structure) -> data::Structure {
    use crate::data::structure::symbol_to_z;
    let lattice = nalgebra::Matrix3::from_rows(&[
        nalgebra::RowVector3::new(vs.lattice[0][0], vs.lattice[0][1], vs.lattice[0][2]),
        nalgebra::RowVector3::new(vs.lattice[1][0], vs.lattice[1][1], vs.lattice[1][2]),
        nalgebra::RowVector3::new(vs.lattice[2][0], vs.lattice[2][1], vs.lattice[2][2]),
    ]);
    let positions = vs.positions.iter()
        .map(|p| nalgebra::Vector3::new(p[0], p[1], p[2]))
        .collect();
    let species = vs.species.clone();
    let atomic_numbers = species.iter().map(|s| symbol_to_z(s)).collect();
    data::Structure {
        lattice,
        positions,
        atomic_numbers,
        species,
        comment: String::new(),
        is_cartesian: false,
    }
}

/// Shared logic to load a structure into the scene: compute pivot/extent,
/// detect symmetry, spawn atoms and unit cell.
pub fn load_structure_into_scene(
    structure: &data::Structure,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    elements: &data::ElementData,
    camera_state: &mut CameraState,
    sym_axes: &mut SymmetryAxes,
    extent: &mut StructureExtent,
    symprec: f64,
    periodic_images: bool,
    n_primitive: usize,
) {
    let (pivot, ext) = compute_pivot_and_extent(&structure.lattice);
    camera_state.pivot = pivot;
    extent.0 = ext;

    *sym_axes = analysis::symmetry::detect_symmetry(structure, symprec);

    vis::spawn_structure(commands, meshes, materials, elements, structure, periodic_images, n_primitive);
    vis::spawn_unit_cell(commands, meshes, materials, structure);
}

// ---------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------

pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    poscar_path: Res<PoscarPath>,
    vasprun_path: Res<VasprunPath>,
    elements: Res<ElementData>,
    mut camera_state: ResMut<CameraState>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut extent: ResMut<StructureExtent>,
    symprec: Res<SymmetryTolerance>,
    mut loaded: ResMut<LoadedStructure>,
    mut vasprun_data: ResMut<ui::VasprunData>,
    mut vasprun_ui: ResMut<ui::VasprunUiState>,
    volumetric_path: Res<VolumetricPath>,
    mut loaded_volume: ResMut<LoadedVolume>,
    mut iso_state: ResMut<IsoState>,
) {
    // Lighting
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
    });

    // Load POSCAR
    if let Some(ref path) = poscar_path.0 {
        match io::parse_poscar(Path::new(path)) {
            Ok(structure) => {
                println!("Loaded {} atoms from {}", structure.num_atoms(), path);
                commands.insert_resource(PrimitiveAtomCount(structure.num_atoms()));
                load_structure_into_scene(
                    &structure, &mut commands, &mut meshes, &mut materials,
                    &elements, &mut camera_state, &mut sym_axes, &mut extent,
                    symprec.0, true, 0,
                );
                loaded.0 = Some(structure);
            }
            Err(e) => eprintln!("Error loading POSCAR: {e}"),
        }
    } else {
        println!("No POSCAR file provided. Run with: cargo run -- <path/to/POSCAR>");
    }

    // Load vasprun.xml
    if let Some(ref path) = vasprun_path.0 {
        println!("Loading vasprun.xml from {path}...");
        let opts = io::vasprun::ParseOptions {
            parse_projected: true,
            ..Default::default()
        };
        match io::vasprun::parse_vasprun(Path::new(path), opts) {
            Ok(vr) => {
                println!("Loaded vasprun.xml: {} ionic steps, {} atoms",
                    vr.ionic_steps.len(), vr.atominfo.atoms.len());

                if let Some(ref dos) = vr.dos {
                    vasprun_ui.init_from_dos(dos);
                }

                let last_step_idx = vr.ionic_steps.len().saturating_sub(1);
                vasprun_ui.current_step = last_step_idx;

                if loaded.0.is_none() {
                    let vr_struct = if !vr.ionic_steps.is_empty() {
                        &vr.ionic_steps[last_step_idx].structure
                    } else {
                        &vr.final_structure
                    };
                    let structure = convert_vasprun_structure(vr_struct);
                    commands.insert_resource(PrimitiveAtomCount(structure.num_atoms()));
                    load_structure_into_scene(
                        &structure, &mut commands, &mut meshes, &mut materials,
                        &elements, &mut camera_state, &mut sym_axes, &mut extent,
                        symprec.0, true, 0,
                    );
                    loaded.0 = Some(structure);
                }

                vasprun_data.0 = Some(vr);
            }
            Err(e) => eprintln!("Error loading vasprun.xml: {e}"),
        }
    }

    // Load volumetric data
    if let Some(ref path) = volumetric_path.0 {
        println!("Loading volumetric data from {path}...");
        match io::parse_volumetric(Path::new(path)) {
            Ok((vol_structure, grid)) => {
                println!("Loaded {}x{}x{} grid ({} points)",
                    grid.dims[0], grid.dims[1], grid.dims[2], grid.data.len());
                println!("  min={:.6e}, max={:.6e}, std={:.6e}",
                    grid.min(), grid.max(), grid.std_dev());

                let suggested = grid.suggest_isovalue();
                println!("  suggested isovalue: {:.6e}", suggested);
                iso_state.isovalue = suggested as f32;
                iso_state.changed = true;

                if loaded.0.is_none() {
                    commands.insert_resource(PrimitiveAtomCount(vol_structure.num_atoms()));
                    load_structure_into_scene(
                        &vol_structure, &mut commands, &mut meshes, &mut materials,
                        &elements, &mut camera_state, &mut sym_axes, &mut extent,
                        symprec.0, true, 0,
                    );
                    loaded.0 = Some(vol_structure);
                }

                loaded_volume.0 = Some(grid);
            }
            Err(e) => eprintln!("Error loading volumetric data: {e}"),
        }
    }

    // Main camera — look perpendicular to highest-fold symmetry axis if available,
    // otherwise look down the longest lattice vector.
    let dist = extent.0 * 2.5;
    let (view_dir, cam_up) = if let Some(axis) = sym_axes.axes.first() {
        let arbitrary = if axis.direction.y.abs() > 0.99 { Vec3::X } else { Vec3::Y };
        let perp = axis.direction.cross(arbitrary).normalize();
        let up = if perp.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
        (perp, up)
    } else if let Some(ref structure) = loaded.0 {
        // No symmetry axes — look down the longest lattice vector
        let lat = &structure.lattice;
        let vecs = [
            Vec3::new(lat[(0,0)] as f32, lat[(0,1)] as f32, lat[(0,2)] as f32),
            Vec3::new(lat[(1,0)] as f32, lat[(1,1)] as f32, lat[(1,2)] as f32),
            Vec3::new(lat[(2,0)] as f32, lat[(2,1)] as f32, lat[(2,2)] as f32),
        ];
        let longest = vecs.into_iter()
            .max_by(|a, b| a.length().partial_cmp(&b.length()).unwrap())
            .unwrap();
        let dir = longest.normalize();
        let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
        let up = (up - dir * dir.dot(up)).normalize();
        (dir, up)
    } else {
        (Vec3::new(1.0, 1.0, 1.0).normalize(), Vec3::Y)
    };
    let cam_pos = camera_state.pivot + view_dir * dist;
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(cam_pos).looking_at(camera_state.pivot, cam_up),
    ));

    vis::setup_axes_gizmo(&mut commands, &mut meshes, &mut materials);
}

// ---------------------------------------------------------------------------
// File open handlers
// ---------------------------------------------------------------------------

pub fn handle_open_structure(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventReader<MenuAction>,
    mut loaded: ResMut<LoadedStructure>,
    elements: Res<ElementData>,
    mut camera_state: ResMut<CameraState>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut extent: ResMut<StructureExtent>,
    symprec: Res<SymmetryTolerance>,
    periodic: Res<PeriodicImages>,
    mut prim_count: ResMut<PrimitiveAtomCount>,
    atom_query: Query<Entity, With<AtomMarker>>,
    outline_query: Query<Entity, With<UnitCellOutline>>,
    mut cam_query: Query<&mut Transform, (With<Camera3d>, Without<vis::axes_gizmo::GizmoCamera>)>,
) {
    let mut path_to_load = None;
    for event in events.read() {
        if let MenuAction::OpenStructure(path) = event {
            path_to_load = Some(path.clone());
        }
    }

    let Some(path) = path_to_load else { return };

    match io::parse_poscar(Path::new(&path)) {
        Ok(structure) => {
            println!("Loaded {} atoms from {}", structure.num_atoms(), path.display());
            prim_count.0 = structure.num_atoms();

            despawn_scene_entities(&mut commands, &atom_query, &outline_query);

            load_structure_into_scene(
                &structure, &mut commands, &mut meshes, &mut materials,
                &elements, &mut camera_state, &mut sym_axes, &mut extent,
                symprec.0, periodic.0, 0,
            );

            reposition_camera(&mut cam_query.single_mut(), camera_state.pivot, extent.0);
            loaded.0 = Some(structure);
        }
        Err(e) => eprintln!("Error loading structure: {e}"),
    }
}

pub fn handle_open_vasprun(
    mut events: EventReader<MenuAction>,
    mut vasprun_data: ResMut<ui::VasprunData>,
    mut vasprun_ui: ResMut<ui::VasprunUiState>,
    mut loaded: ResMut<LoadedStructure>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    elements: Res<ElementData>,
    mut camera_state: ResMut<CameraState>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut extent: ResMut<StructureExtent>,
    symprec: Res<SymmetryTolerance>,
    periodic: Res<PeriodicImages>,
    mut prim_count: ResMut<PrimitiveAtomCount>,
    atom_query: Query<Entity, With<AtomMarker>>,
    outline_query: Query<Entity, With<UnitCellOutline>>,
) {
    let mut path_to_load = None;
    for event in events.read() {
        if let MenuAction::OpenVasprun(path) = event {
            path_to_load = Some(path.clone());
        }
    }

    let Some(path) = path_to_load else { return };

    println!("Loading vasprun.xml from {}...", path.display());
    let opts = io::vasprun::ParseOptions {
        parse_projected: true,
        ..Default::default()
    };
    match io::vasprun::parse_vasprun(&path, opts) {
        Ok(vr) => {
            println!("Loaded vasprun.xml: {} ionic steps, {} atoms",
                vr.ionic_steps.len(), vr.atominfo.atoms.len());

            if let Some(ref dos) = vr.dos {
                vasprun_ui.init_from_dos(dos);
            }
            let last_step_idx = vr.ionic_steps.len().saturating_sub(1);
            vasprun_ui.current_step = last_step_idx;

            let vr_struct = if !vr.ionic_steps.is_empty() {
                &vr.ionic_steps[last_step_idx].structure
            } else {
                &vr.final_structure
            };
            let structure = convert_vasprun_structure(vr_struct);
            prim_count.0 = structure.num_atoms();

            despawn_scene_entities(&mut commands, &atom_query, &outline_query);

            load_structure_into_scene(
                &structure, &mut commands, &mut meshes, &mut materials,
                &elements, &mut camera_state, &mut sym_axes, &mut extent,
                symprec.0, periodic.0, 0,
            );

            // Camera repositioned via commands (no cam_query to stay under 16 params)
            let pivot = camera_state.pivot;
            let ext = extent.0;
            let offset = Vec3::new(1.0, 1.0, 1.0).normalize() * ext * 2.0;
            commands.spawn(()).insert(RepositionCamera(pivot + offset, pivot));

            loaded.0 = Some(structure);
            vasprun_data.0 = Some(vr);
        }
        Err(e) => eprintln!("Error loading vasprun.xml: {e}"),
    }
}

/// One-shot marker to reposition camera next frame (avoids extra Query param).
#[derive(Component)]
pub(crate) struct RepositionCamera(Vec3, Vec3);

/// System that applies deferred camera repositioning.
pub fn apply_camera_reposition(
    mut commands: Commands,
    reposition_query: Query<(Entity, &RepositionCamera)>,
    mut cam_query: Query<&mut Transform, (With<Camera3d>, Without<vis::axes_gizmo::GizmoCamera>)>,
) {
    for (entity, reposition) in reposition_query.iter() {
        let mut cam = cam_query.single_mut();
        cam.translation = reposition.0;
        cam.look_at(reposition.1, Vec3::Y);
        commands.entity(entity).despawn();
    }
}

pub fn handle_open_volumetric(
    mut events: EventReader<MenuAction>,
    mut loaded_volume: ResMut<LoadedVolume>,
    mut iso_state: ResMut<IsoState>,
    mut loaded: ResMut<LoadedStructure>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    elements: Res<ElementData>,
    mut camera_state: ResMut<CameraState>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut extent: ResMut<StructureExtent>,
    symprec: Res<SymmetryTolerance>,
    atom_query: Query<Entity, With<AtomMarker>>,
    outline_query: Query<Entity, With<UnitCellOutline>>,
    iso_query: Query<Entity, With<vis::isosurface::IsosurfaceEntity>>,
    mut cam_query: Query<&mut Transform, (With<Camera3d>, Without<vis::axes_gizmo::GizmoCamera>)>,
) {
    let mut path_to_load = None;
    for event in events.read() {
        if let MenuAction::OpenVolumetric(path) = event {
            path_to_load = Some(path.clone());
        }
    }

    let Some(path) = path_to_load else { return };

    println!("Loading volumetric data from {}...", path.display());
    match io::parse_volumetric(Path::new(&path)) {
        Ok((vol_structure, grid)) => {
            println!("Loaded {}x{}x{} grid", grid.dims[0], grid.dims[1], grid.dims[2]);

            let suggested = grid.suggest_isovalue();
            iso_state.isovalue = suggested as f32;
            iso_state.show = true;
            iso_state.changed = true;

            vis::isosurface::despawn_isosurface(&mut commands, &iso_query);

            // Load structure if none loaded (periodic defaults to true for fresh load)
            if loaded.0.is_none() {
                commands.insert_resource(PrimitiveAtomCount(vol_structure.num_atoms()));
                despawn_scene_entities(&mut commands, &atom_query, &outline_query);

                load_structure_into_scene(
                    &vol_structure, &mut commands, &mut meshes, &mut materials,
                    &elements, &mut camera_state, &mut sym_axes, &mut extent,
                    symprec.0, true, 0,
                );

                reposition_camera(&mut cam_query.single_mut(), camera_state.pivot, extent.0);
                loaded.0 = Some(vol_structure);
            }

            loaded_volume.0 = Some(grid);
        }
        Err(e) => eprintln!("Error loading volumetric data: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Scene manipulation systems
// ---------------------------------------------------------------------------

pub fn handle_create_supercell(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventReader<MenuAction>,
    mut loaded: ResMut<LoadedStructure>,
    elements: Res<ElementData>,
    mut camera_state: ResMut<CameraState>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut extent: ResMut<StructureExtent>,
    symprec: Res<SymmetryTolerance>,
    periodic: Res<PeriodicImages>,
    prim_count: Res<PrimitiveAtomCount>,
    atom_query: Query<Entity, With<AtomMarker>>,
    outline_query: Query<Entity, With<UnitCellOutline>>,
    decor_query: Query<Entity, Or<(With<WyckoffHighlight>, With<MirrorPlaneVis>)>>,
    mut cam_query: Query<&mut Transform, (With<Camera3d>, Without<vis::axes_gizmo::GizmoCamera>)>,
) {
    let mut supercell_dims = None;
    for event in events.read() {
        if let MenuAction::CreateSupercell(na, nb, nc) = event {
            supercell_dims = Some((*na, *nb, *nc));
        }
    }

    let Some((na, nb, nc)) = supercell_dims else { return };
    let Some(ref structure) = loaded.0 else { return };

    let new_structure = structure.supercell(na, nb, nc);
    println!("Created {}x{}x{} supercell: {} atoms", na, nb, nc, new_structure.num_atoms());

    despawn_scene_entities(&mut commands, &atom_query, &outline_query);
    for entity in decor_query.iter() {
        commands.entity(entity).despawn();
    }

    load_structure_into_scene(
        &new_structure, &mut commands, &mut meshes, &mut materials,
        &elements, &mut camera_state, &mut sym_axes, &mut extent,
        symprec.0, periodic.0, prim_count.0,
    );

    reposition_camera(&mut cam_query.single_mut(), camera_state.pivot, extent.0);
    loaded.0 = Some(new_structure);
}

/// React to ionic step changes: update atom positions, forces, and magnetic moments.
pub fn update_trajectory_step(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vasprun: Res<ui::VasprunData>,
    mut ui_state: ResMut<ui::VasprunUiState>,
    elements: Res<ElementData>,
    periodic: Res<PeriodicImages>,
    mut loaded: ResMut<LoadedStructure>,
    mut camera_state: ResMut<CameraState>,
    mut extent: ResMut<StructureExtent>,
    prim_count: Res<PrimitiveAtomCount>,
    atom_query: Query<Entity, With<AtomMarker>>,
    outline_query: Query<Entity, With<UnitCellOutline>>,
    force_query: Query<Entity, With<vis::arrows::ForceArrow>>,
    mag_query: Query<Entity, With<vis::arrows::MagMomentArrow>>,
) {
    if !ui_state.step_changed {
        return;
    }
    ui_state.step_changed = false;

    let Some(ref vr) = vasprun.0 else { return };
    let step_idx = ui_state.current_step.min(vr.ionic_steps.len().saturating_sub(1));
    let step = &vr.ionic_steps[step_idx];

    let structure = convert_vasprun_structure(&step.structure);

    despawn_scene_entities(&mut commands, &atom_query, &outline_query);

    // Recompute pivot/extent without full symmetry detection (too expensive per step)
    let (pivot, ext) = compute_pivot_and_extent(&structure.lattice);
    camera_state.pivot = pivot;
    extent.0 = ext;

    vis::spawn_structure(&mut commands, &mut meshes, &mut materials, &elements, &structure, periodic.0, prim_count.0);
    vis::spawn_unit_cell(&mut commands, &mut meshes, &mut materials, &structure);

    // Despawn old arrows
    vis::arrows::despawn_by_marker(&mut commands, &force_query);
    vis::arrows::despawn_by_marker(&mut commands, &mag_query);

    if ui_state.show_forces {
        let cart_positions = structure.to_cartesian();
        let positions: Vec<Vec3> = cart_positions.iter()
            .map(|p| Vec3::new(p.x as f32, p.y as f32, p.z as f32))
            .collect();
        vis::arrows::spawn_arrows(
            &mut commands, &mut meshes, &mut materials,
            &positions, &step.forces, ui_state.force_scale,
            vis::arrows::ForceArrow, vis::arrows::force_color,
        );
    }

    if ui_state.show_mag_moments {
        let vectors: Option<Vec<[f64; 3]>> = if let Some(ref mag) = step.magnetization {
            Some(mag.iter().map(|m| {
                if m.len() >= 3 {
                    [m[0], m[1], m[2]]
                } else if !m.is_empty() {
                    [0.0, 0.0, m[0]]
                } else {
                    [0.0, 0.0, 0.0]
                }
            }).collect())
        } else if let Some(ref dos) = vr.dos {
            analysis::magnetic::moments_from_pdos(dos)
        } else {
            None
        };

        if let Some(vectors) = vectors {
            let cart_positions = structure.to_cartesian();
            let positions: Vec<Vec3> = cart_positions.iter()
                .map(|p| Vec3::new(p.x as f32, p.y as f32, p.z as f32))
                .collect();
            vis::arrows::spawn_arrows(
                &mut commands, &mut meshes, &mut materials,
                &positions, &vectors, ui_state.mag_scale,
                vis::arrows::MagMomentArrow, vis::arrows::mag_color,
            );
        }
    }

    loaded.0 = Some(structure);
}

/// React to LDOS energy/orbital changes: recolor atoms by their DOS weight.
pub fn update_ldos_coloring(
    vasprun: Res<ui::VasprunData>,
    mut ui_state: ResMut<ui::VasprunUiState>,
    atom_query: Query<(&AtomMarker, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    elements: Res<ElementData>,
) {
    if !ui_state.ldos_changed {
        return;
    }
    ui_state.ldos_changed = false;

    let Some(ref vr) = vasprun.0 else { return };

    if !ui_state.show_ldos {
        vis::ldos_coloring::reset_atom_colors(&atom_query, &mut materials, &elements);
        return;
    }

    let Some(ref dos) = vr.dos else { return };
    let Some(ref pdos) = dos.partial else { return };

    let weights = vis::ldos_coloring::compute_atom_weights(
        pdos,
        &dos.total.energies,
        ui_state.ldos_energy as f64,
        &ui_state.selected_orbitals,
        0,
    );

    vis::ldos_coloring::apply_ldos_coloring(&weights, &atom_query, &mut materials);
}

// ---------------------------------------------------------------------------
// Toggle systems
// ---------------------------------------------------------------------------

/// U key or menu: toggle unit cell outline.
pub fn toggle_unit_cell(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut events: EventReader<MenuAction>,
    mut visible: ResMut<UnitCellVisible>,
    outline_query: Query<Entity, With<UnitCellOutline>>,
    loaded: Res<LoadedStructure>,
) {
    let toggle = keys.just_pressed(KeyCode::KeyU)
        || events.read().any(|e| matches!(e, MenuAction::ToggleUnitCell));

    if !toggle { return; }

    if visible.0 {
        vis::despawn_unit_cell(&mut commands, &outline_query);
        visible.0 = false;
        println!("Unit cell hidden");
    } else if let Some(ref structure) = loaded.0 {
        vis::spawn_unit_cell(&mut commands, &mut meshes, &mut materials, structure);
        visible.0 = true;
        println!("Unit cell shown");
    }
}

/// Re-detect symmetry when tolerance changes via menu.
pub fn handle_rerun_symmetry(
    mut events: EventReader<MenuAction>,
    loaded: Res<LoadedStructure>,
    symprec: Res<SymmetryTolerance>,
    mut sym_axes: ResMut<SymmetryAxes>,
) {
    if !events.read().any(|e| matches!(e, MenuAction::RerunSymmetry)) {
        return;
    }
    if let Some(ref structure) = loaded.0 {
        *sym_axes = analysis::symmetry::detect_symmetry(structure, symprec.0);
    }
}

/// W key or menu: toggle Wyckoff position highlights.
pub fn toggle_wyckoff(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut events: EventReader<MenuAction>,
    mut visible: ResMut<WyckoffVisible>,
    highlight_query: Query<Entity, With<WyckoffHighlight>>,
    loaded: Res<LoadedStructure>,
    sym_axes: Res<SymmetryAxes>,
) {
    let toggle = keys.just_pressed(KeyCode::KeyW)
        || events.read().any(|e| matches!(e, MenuAction::ToggleWyckoff));

    if !toggle { return; }

    if visible.0 {
        vis::despawn_wyckoff_highlights(&mut commands, &highlight_query);
        visible.0 = false;
        println!("Wyckoff highlights hidden");
    } else if let Some(ref structure) = loaded.0 {
        vis::spawn_wyckoff_highlights(
            &mut commands, &mut meshes, &mut materials,
            structure, &sym_axes.wyckoff_sites,
        );
        visible.0 = true;
        println!("Wyckoff highlights shown ({} sites)", sym_axes.wyckoff_sites.len());
    }
}

/// M key: toggle all mirror planes on/off.
pub fn toggle_mirror_planes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<MirrorsVisible>,
    mut mirror_vis: ResMut<ui::menu::MirrorVisibility>,
    mirror_query: Query<Entity, With<MirrorPlaneVis>>,
    sym_axes: Res<SymmetryAxes>,
    camera_state: Res<CameraState>,
    extent: Res<StructureExtent>,
) {
    if !keys.just_pressed(KeyCode::KeyM) { return; }

    if visible.0 {
        vis::despawn_mirror_planes(&mut commands, &mirror_query);
        for v in mirror_vis.0.iter_mut() { *v = false; }
        visible.0 = false;
        println!("Mirror planes hidden");
    } else {
        mirror_vis.0.resize(sym_axes.mirrors.len(), false);
        for v in mirror_vis.0.iter_mut() { *v = true; }
        vis::spawn_mirror_planes(
            &mut commands, &mut meshes, &mut materials,
            camera_state.pivot, &sym_axes.mirrors, extent.0,
            &mirror_vis.0,
        );
        visible.0 = true;
        println!("Mirror planes shown ({} planes)", sym_axes.mirrors.len());
    }
}

/// Handle individual mirror toggle from menu checkboxes.
pub fn handle_individual_mirror_toggle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventReader<MenuAction>,
    mirror_query: Query<(Entity, &MirrorPlaneVis)>,
    sym_axes: Res<SymmetryAxes>,
    camera_state: Res<CameraState>,
    extent: Res<StructureExtent>,
    mut visible: ResMut<MirrorsVisible>,
) {
    for event in events.read() {
        if let MenuAction::ToggleMirror(index, show) = event {
            if *show {
                if let Some(mirror) = sym_axes.mirrors.get(*index) {
                    vis::mirror_plane::spawn_single_mirror(
                        &mut commands, &mut meshes, &mut materials,
                        camera_state.pivot, mirror, *index, extent.0,
                    );
                    visible.0 = true;
                }
            } else {
                vis::mirror_plane::despawn_mirror_by_index(&mut commands, &mirror_query, *index);
            }
        }
    }
}

/// P key or menu: toggle periodic boundary images.
pub fn toggle_periodic_images(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut events: EventReader<MenuAction>,
    mut periodic: ResMut<PeriodicImages>,
    elements: Res<ElementData>,
    loaded: Res<LoadedStructure>,
    prim_count: Res<PrimitiveAtomCount>,
    atom_query: Query<Entity, With<AtomMarker>>,
) {
    let toggle = keys.just_pressed(KeyCode::KeyP)
        || events.read().any(|e| matches!(e, MenuAction::TogglePeriodicImages));

    if !toggle { return; }

    periodic.0 = !periodic.0;
    println!("Periodic images: {}", if periodic.0 { "on" } else { "off" });

    for entity in atom_query.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(ref structure) = loaded.0 {
        vis::spawn_structure(&mut commands, &mut meshes, &mut materials, &elements, structure, periodic.0, prim_count.0);
    }
}

// ---------------------------------------------------------------------------
// Isosurface
// ---------------------------------------------------------------------------

/// Rebuild isosurface mesh when isovalue or visibility changes.
pub fn update_isosurface(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut iso_state: ResMut<IsoState>,
    loaded_volume: Res<LoadedVolume>,
    iso_query: Query<Entity, With<vis::isosurface::IsosurfaceEntity>>,
) {
    if !iso_state.changed { return; }
    iso_state.changed = false;

    vis::isosurface::despawn_isosurface(&mut commands, &iso_query);

    if !iso_state.show { return; }

    let Some(ref grid) = loaded_volume.0 else { return };

    println!("Computing isosurface at level {:.4e}...", iso_state.isovalue);
    vis::isosurface::spawn_isosurface(
        &mut commands, &mut meshes, &mut materials,
        grid, iso_state.isovalue as f64, iso_state.opacity,
    );
}

/// F12 or menu: save screenshot.
pub fn screenshot_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut events: EventReader<MenuAction>,
    mut commands: Commands,
) {
    let take = keys.just_pressed(KeyCode::F12)
        || events.read().any(|e| matches!(e, MenuAction::TakeScreenshot));

    if !take { return; }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let filename = format!("alfred_screenshot_{timestamp}.png");
    let path = std::path::PathBuf::from(&filename);

    use bevy::render::view::screenshot::{Screenshot, save_to_disk};

    commands.spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    println!("Screenshot saved to {filename}");
}
