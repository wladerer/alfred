mod analysis;
mod data;
mod io;
mod ui;
mod vis;

use analysis::SymmetryAxes;
use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiContexts, egui};
use std::path::Path;
use ui::menu::MenuAction;
use vis::{AxisIndicator, UnitCellOutline, WyckoffHighlight, MirrorPlaneVis};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let poscar_path = args.get(1).cloned();
    let vasprun_path = args.iter().find(|a| a.ends_with(".xml") || a.ends_with(".xml.gz")).cloned();
    let volumetric_path = args.iter().find(|a| {
        let lower = a.to_lowercase();
        lower.contains("chg") || lower.contains("locpot") || lower.contains("elfcar")
            || lower.ends_with(".vasp") || lower.contains("parchg")
    }).cloned();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Alfred".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .add_event::<MenuAction>()
        .insert_resource(PoscarPath(poscar_path))
        .insert_resource(VasprunPath(vasprun_path))
        .insert_resource(VolumetricPath(volumetric_path))
        .insert_resource(data::ElementData::load())
        .insert_resource(LoadedVolume(None))
        .insert_resource(IsoState::default())
        .insert_resource(ui::VasprunData::default())
        .insert_resource(ui::VasprunUiState::default())
        .insert_resource(CameraState::default())
        .insert_resource(SymmetryAxes::default())
        .insert_resource(StructureExtent(5.0))
        .insert_resource(SymmetryTolerance(1e-5))
        .insert_resource(LoadedStructure(None))
        .insert_resource(UnitCellVisible(true))
        .insert_resource(WyckoffVisible(false))
        .insert_resource(MirrorsVisible(false))
        .insert_resource(PeriodicImages(true))
        .insert_resource(vis::selection::SelectedAtom::default())
        .insert_resource(ui::menu::SupercellInput::default())
        .insert_resource(ui::menu::SymprecInput::default())
        .insert_resource(ui::menu::MirrorVisibility::default())
        .add_systems(Startup, setup_scene)
        // UI layout order matters: top panel first, then side panels, then windows
        .add_systems(Update, ui::menu_bar_system
            .before(ui::vasprun_panel_system)
            .before(isosurface_panel)
        )
        .add_systems(Update, ui::vasprun_panel_system)
        .add_systems(Update, isosurface_panel)
        .add_systems(Update, ui::menu::wyckoff_legend_system)
        .add_systems(Update, ui::menu::handle_menu_actions)
        .add_systems(Update, orbit_camera)
        .add_systems(Update, axis_view_shortcuts)
        .add_systems(Update, cycle_symmetry_axis)
        .add_systems(Update, toggle_unit_cell)
        .add_systems(Update, toggle_periodic_images)
        .add_systems(Update, toggle_wyckoff)
        .add_systems(Update, toggle_mirror_planes)
        .add_systems(Update, handle_individual_mirror_toggle)
        .add_systems(Update, handle_rerun_symmetry)
        .add_systems(Update, handle_create_supercell)
        .add_systems(Update, update_trajectory_step)
        .add_systems(Update, update_ldos_coloring)
        .add_systems(Update, update_isosurface)
        .add_systems(Update, handle_open_structure)
        .add_systems(Update, handle_open_vasprun)
        .add_systems(Update, handle_open_volumetric)
        .add_systems(Update, atom_pick_system)
        .add_systems(Update, ui::atom_info::atom_info_panel)
        .add_systems(Update, screenshot_system)
        .add_systems(Update, vis::sync_gizmo_camera)
        .add_systems(Update, update_gizmo_viewport)
        .run();
}

#[derive(Resource)]
struct PoscarPath(Option<String>);

#[derive(Resource)]
struct VasprunPath(Option<String>);

#[derive(Resource)]
struct VolumetricPath(Option<String>);

#[derive(Resource)]
struct LoadedVolume(Option<data::VolumeGrid>);

#[derive(Resource)]
struct IsoState {
    isovalue: f32,
    opacity: f32,
    show: bool,
    changed: bool,
}

impl Default for IsoState {
    fn default() -> Self {
        Self { isovalue: 0.0, opacity: 0.5, show: true, changed: false }
    }
}

#[derive(Resource)]
struct StructureExtent(f32);

#[derive(Resource)]
pub struct SymmetryTolerance(pub f64);

#[derive(Resource)]
struct LoadedStructure(Option<data::Structure>);

#[derive(Resource)]
struct UnitCellVisible(bool);

#[derive(Resource)]
struct WyckoffVisible(bool);

#[derive(Resource)]
struct MirrorsVisible(bool);

#[derive(Resource)]
struct PeriodicImages(bool);

#[derive(Resource)]
struct CameraState {
    pivot: Vec3,
    locked_axis: Option<Vec3>,
    /// Angular velocity quaternion for inertia (rotation per second).
    angular_velocity: Quat,
    /// Whether the user is currently dragging.
    dragging: bool,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            pivot: Vec3::ZERO,
            locked_axis: None,
            angular_velocity: Quat::IDENTITY,
            dragging: false,
        }
    }
}

/// Marker for atom entities so we can despawn/respawn the scene.
#[derive(Component)]
struct SceneAtom;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    poscar_path: Res<PoscarPath>,
    vasprun_path: Res<VasprunPath>,
    elements: Res<data::ElementData>,
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

    // Load structure if path provided
    if let Some(ref path) = poscar_path.0 {
        match io::parse_poscar(Path::new(path)) {
            Ok(structure) => {
                println!("Loaded {} atoms from {}", structure.num_atoms(), path);
                load_structure_into_scene(
                    &structure, &mut commands, &mut meshes, &mut materials,
                    &elements, &mut camera_state, &mut sym_axes, &mut extent,
                    symprec.0, true,
                );
                loaded.0 = Some(structure);
            }
            Err(e) => eprintln!("Error loading POSCAR: {e}"),
        }
    } else {
        println!("No POSCAR file provided. Run with: cargo run -- <path/to/POSCAR>");
    }

    // Load vasprun.xml if provided
    if let Some(ref path) = vasprun_path.0 {
        println!("Loading vasprun.xml from {path}...");
        let opts = io::vasprun::ParseOptions {
            parse_projected: true,
            ..Default::default()
        };
        match io::vasprun::parse_vasprun(Path::new(path), opts) {
            Ok(vr) => {
                println!("Loaded vasprun.xml: {} ionic steps, {} atoms",
                    vr.ionic_steps.len(),
                    vr.atominfo.atoms.len(),
                );

                // Initialize LDOS UI state from DOS data
                if let Some(ref dos) = vr.dos {
                    vasprun_ui.ldos_energy_min = *dos.total.energies.first().unwrap_or(&-10.0) as f32;
                    vasprun_ui.ldos_energy_max = *dos.total.energies.last().unwrap_or(&10.0) as f32;
                    vasprun_ui.ldos_energy = dos.efermi as f32;

                    if let Some(ref pdos) = dos.partial {
                        vasprun_ui.orbital_labels = pdos.orbitals.clone();
                        vasprun_ui.selected_orbitals = vec![true; pdos.orbitals.len()];
                    }
                }

                // If no POSCAR was loaded, use the vasprun's initial structure
                if loaded.0.is_none() {
                    let vr_struct = &vr.initial_structure;
                    let structure = convert_vasprun_structure(vr_struct);
                    load_structure_into_scene(
                        &structure, &mut commands, &mut meshes, &mut materials,
                        &elements, &mut camera_state, &mut sym_axes, &mut extent,
                        symprec.0, true,
                    );
                    loaded.0 = Some(structure);
                }

                vasprun_data.0 = Some(vr);
            }
            Err(e) => eprintln!("Error loading vasprun.xml: {e}"),
        }
    }

    // Load volumetric data if provided
    if let Some(ref path) = volumetric_path.0 {
        println!("Loading volumetric data from {path}...");
        match io::parse_volumetric(Path::new(path)) {
            Ok((vol_structure, grid)) => {
                println!("Loaded {}x{}x{} grid ({} points)",
                    grid.dims[0], grid.dims[1], grid.dims[2],
                    grid.data.len(),
                );
                println!("  min={:.6e}, max={:.6e}, std={:.6e}",
                    grid.min(), grid.max(), grid.std_dev());

                let suggested = grid.suggest_isovalue();
                println!("  suggested isovalue: {:.6e}", suggested);
                iso_state.isovalue = suggested as f32;
                iso_state.changed = true;

                // Use the volumetric structure if no POSCAR was loaded
                if loaded.0.is_none() {
                    load_structure_into_scene(
                        &vol_structure, &mut commands, &mut meshes, &mut materials,
                        &elements, &mut camera_state, &mut sym_axes, &mut extent,
                        symprec.0, true,
                    );
                    loaded.0 = Some(vol_structure);
                }

                loaded_volume.0 = Some(grid);
            }
            Err(e) => eprintln!("Error loading volumetric data: {e}"),
        }
    }

    // Main camera
    let offset = Vec3::new(10.0, 10.0, 10.0);
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(camera_state.pivot + offset)
            .looking_at(camera_state.pivot, Vec3::Y),
    ));

    // Axes gizmo
    vis::setup_axes_gizmo(&mut commands, &mut meshes, &mut materials);
}

/// Shared logic to load a structure into the scene (used on startup and supercell creation).
fn load_structure_into_scene(
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
) {
    // Compute pivot and extent from the lattice (not atom positions) so it
    // correctly encompasses the full displayed cell including periodic images.
    let lat = &structure.lattice;
    let a = Vec3::new(lat[(0, 0)] as f32, lat[(0, 1)] as f32, lat[(0, 2)] as f32);
    let b = Vec3::new(lat[(1, 0)] as f32, lat[(1, 1)] as f32, lat[(1, 2)] as f32);
    let c = Vec3::new(lat[(2, 0)] as f32, lat[(2, 1)] as f32, lat[(2, 2)] as f32);

    // Centroid of the parallelepiped = (a + b + c) / 2
    camera_state.pivot = (a + b + c) / 2.0;

    // Extent: half-diagonal of the parallelepiped
    let corners = [
        Vec3::ZERO, a, b, c, a + b, a + c, b + c, a + b + c,
    ];
    let max_dist = corners.iter()
        .map(|corner| (*corner - camera_state.pivot).length())
        .fold(0.0f32, f32::max);
    extent.0 = max_dist.max(2.0) * 1.2;

    *sym_axes = analysis::symmetry::detect_symmetry(structure, symprec);

    vis::spawn_structure(commands, meshes, materials, elements, structure, periodic_images);
    vis::spawn_unit_cell(commands, meshes, materials, structure);
}

fn update_gizmo_viewport(
    windows: Query<&Window>,
    mut gizmo_cam: Query<&mut Camera, With<vis::axes_gizmo::GizmoCamera>>,
) {
    let window = windows.single();
    let mut camera = gizmo_cam.single_mut();

    let height = window.physical_height();
    let size = 140u32;
    let margin = 10u32;

    camera.viewport = Some(bevy::render::camera::Viewport {
        physical_position: UVec2::new(margin, height.saturating_sub(size + margin)),
        physical_size: UVec2::new(size, size),
        ..default()
    });
}

fn orbit_camera(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut scroll: EventReader<bevy::input::mouse::MouseWheel>,
    mut query: Query<&mut Transform, (With<Camera3d>, Without<vis::axes_gizmo::GizmoCamera>)>,
    camera_state: Res<CameraState>,
    mut contexts: EguiContexts,
) {
    let egui_wants_input = contexts.ctx_mut().wants_pointer_input();
    let mut transform = query.single_mut();
    let pivot = camera_state.pivot;

    // Left-drag: orbit
    if mouse.pressed(MouseButton::Left) && !egui_wants_input {
        for ev in motion.read() {
            if let Some(axis) = camera_state.locked_axis {
                let angle = -ev.delta.x * 0.005;
                let rotation = Quat::from_axis_angle(axis, angle);
                let arm = transform.translation - pivot;
                transform.translation = pivot + rotation.mul_vec3(arm);
                transform.rotation = rotation * transform.rotation;
            } else {
                let yaw = Quat::from_rotation_y(-ev.delta.x * 0.005);
                let right = transform.right();
                let pitch = Quat::from_axis_angle(right.into(), -ev.delta.y * 0.005);
                let rotation = yaw * pitch;
                let arm = transform.translation - pivot;
                transform.translation = pivot + rotation.mul_vec3(arm);
                transform.rotation = rotation * transform.rotation;
            }
        }
    }
    // Right-drag or middle-drag: pan
    else if (mouse.pressed(MouseButton::Right) || mouse.pressed(MouseButton::Middle)) && !egui_wants_input {
        for ev in motion.read() {
            let right: Vec3 = transform.right().into();
            let up: Vec3 = transform.up().into();
            let dist = (transform.translation - pivot).length();
            let speed = dist * 0.002;
            transform.translation += (-ev.delta.x * speed) * right + (ev.delta.y * speed) * up;
        }
    } else {
        motion.clear();
    }

    // Scroll: zoom
    if !egui_wants_input {
        for ev in scroll.read() {
            let arm = transform.translation - pivot;
            let dist = arm.length();
            let factor = 1.0 - ev.y * 0.1;
            let new_dist = (dist * factor).max(1.0);
            transform.translation = pivot + arm.normalize() * new_dist;
        }
    } else {
        scroll.clear();
    }
}

/// X/Y/Z keys snap the camera to view along that axis.
/// Works with locked symmetry axis — if locked, snaps perpendicular to the lock axis
/// in the plane containing the pressed axis direction.
fn axis_view_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    camera_state: Res<CameraState>,
    mut query: Query<&mut Transform, (With<Camera3d>, Without<vis::axes_gizmo::GizmoCamera>)>,
) {
    // Spacebar: snap back to default view (looking at pivot from diagonal)
    if keys.just_pressed(KeyCode::Space) {
        let mut transform = query.single_mut();
        let pivot = camera_state.pivot;
        let dist = (transform.translation - pivot).length();
        let offset = Vec3::new(1.0, 1.0, 1.0).normalize() * dist;
        transform.translation = pivot + offset;
        transform.look_at(pivot, Vec3::Y);
        return;
    }

    let dir = if keys.just_pressed(KeyCode::KeyX) {
        Some(Vec3::X)
    } else if keys.just_pressed(KeyCode::KeyY) {
        Some(Vec3::Y)
    } else if keys.just_pressed(KeyCode::KeyZ) {
        Some(Vec3::Z)
    } else {
        None
    };

    let Some(view_dir) = dir else { return };

    let mut transform = query.single_mut();
    let pivot = camera_state.pivot;
    let dist = (transform.translation - pivot).length();

    transform.translation = pivot + view_dir * dist;

    // Up vector perpendicular to viewing direction
    let up = if view_dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let up = (up - view_dir * view_dir.dot(up)).normalize();
    transform.look_at(pivot, up);
}

/// N: cycle symmetry axis. Shift+N: lock/unlock.
fn cycle_symmetry_axis(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut camera_state: ResMut<CameraState>,
    mut cam_query: Query<&mut Transform, (With<Camera3d>, Without<vis::axes_gizmo::GizmoCamera>)>,
    indicator_query: Query<Entity, With<AxisIndicator>>,
    extent: Res<StructureExtent>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    if !keys.just_pressed(KeyCode::KeyN) {
        return;
    }

    if shift {
        if camera_state.locked_axis.is_some() {
            camera_state.locked_axis = None;
            vis::despawn_axis_indicator(&mut commands, &indicator_query);
            println!("Rotation unlocked");
        } else if let Some(axis) = sym_axes.current() {
            let dir = axis.direction;
            let label = axis.label.clone();
            camera_state.locked_axis = Some(dir);

            vis::despawn_axis_indicator(&mut commands, &indicator_query);
            vis::spawn_axis_indicator(
                &mut commands, &mut meshes, &mut materials,
                camera_state.pivot, dir, &label, extent.0,
            );
            println!("Rotation locked to {label} axis ({:.3}, {:.3}, {:.3})", dir.x, dir.y, dir.z);
        } else {
            println!("No symmetry axis to lock to");
        }
        return;
    }

    let Some(axis) = sym_axes.next() else {
        println!("No symmetry axes detected");
        return;
    };

    let label = axis.label.clone();
    let dir = axis.direction;

    vis::despawn_axis_indicator(&mut commands, &indicator_query);
    vis::spawn_axis_indicator(
        &mut commands, &mut meshes, &mut materials,
        camera_state.pivot, dir, &label, extent.0,
    );

    println!("Viewing along {label} axis: ({:.3}, {:.3}, {:.3})", dir.x, dir.y, dir.z);

    let mut transform = cam_query.single_mut();
    let pivot = camera_state.pivot;
    let dist = (transform.translation - pivot).length();
    transform.translation = pivot + dir * dist;

    let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let up = (up - dir * dir.dot(up)).normalize();
    transform.look_at(pivot, up);
}

/// U key or menu: toggle unit cell outline.
fn toggle_unit_cell(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut events: EventReader<MenuAction>,
    mut visible: ResMut<UnitCellVisible>,
    outline_query: Query<Entity, With<UnitCellOutline>>,
    loaded: Res<LoadedStructure>,
) {
    let toggle_pressed = keys.just_pressed(KeyCode::KeyU);
    let toggle_menu = events.read().any(|e| matches!(e, MenuAction::ToggleUnitCell));

    if !toggle_pressed && !toggle_menu {
        return;
    }

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
fn handle_rerun_symmetry(
    mut events: EventReader<MenuAction>,
    loaded: Res<LoadedStructure>,
    symprec: Res<SymmetryTolerance>,
    mut sym_axes: ResMut<SymmetryAxes>,
) {
    let rerun = events.read().any(|e| matches!(e, MenuAction::RerunSymmetry));
    if !rerun {
        return;
    }

    if let Some(ref structure) = loaded.0 {
        *sym_axes = analysis::symmetry::detect_symmetry(structure, symprec.0);
    }
}

/// Create supercell, despawn old scene, spawn new.
fn handle_create_supercell(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventReader<MenuAction>,
    mut loaded: ResMut<LoadedStructure>,
    elements: Res<data::ElementData>,
    mut camera_state: ResMut<CameraState>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut extent: ResMut<StructureExtent>,
    symprec: Res<SymmetryTolerance>,
    periodic: Res<PeriodicImages>,
    atom_query: Query<Entity, With<vis::atoms::AtomMarker>>,
    outline_query: Query<Entity, With<UnitCellOutline>>,
    wyckoff_query: Query<Entity, With<WyckoffHighlight>>,
    mirror_query: Query<Entity, With<MirrorPlaneVis>>,
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

    // Despawn old scene entities
    for entity in atom_query.iter() {
        commands.entity(entity).despawn();
    }
    vis::despawn_unit_cell(&mut commands, &outline_query);
    vis::despawn_wyckoff_highlights(&mut commands, &wyckoff_query);
    vis::despawn_mirror_planes(&mut commands, &mirror_query);

    load_structure_into_scene(
        &new_structure, &mut commands, &mut meshes, &mut materials,
        &elements, &mut camera_state, &mut sym_axes, &mut extent,
        symprec.0, periodic.0,
    );

    // Reposition camera for new structure
    let mut cam = cam_query.single_mut();
    let offset = Vec3::new(1.0, 1.0, 1.0).normalize() * extent.0 * 2.0;
    cam.translation = camera_state.pivot + offset;
    cam.look_at(camera_state.pivot, Vec3::Y);

    loaded.0 = Some(new_structure);
}

/// W key or menu: toggle Wyckoff position highlights.
fn toggle_wyckoff(
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

    if !toggle {
        return;
    }

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
fn toggle_mirror_planes(
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
    if !keys.just_pressed(KeyCode::KeyM) {
        return;
    }

    if visible.0 {
        vis::despawn_mirror_planes(&mut commands, &mirror_query);
        for v in mirror_vis.0.iter_mut() { *v = false; }
        visible.0 = false;
        println!("Mirror planes hidden");
    } else {
        // Ensure vec is sized
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
fn handle_individual_mirror_toggle(
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
                // Spawn this plane
                if let Some(mirror) = sym_axes.mirrors.get(*index) {
                    vis::mirror_plane::spawn_single_mirror(
                        &mut commands, &mut meshes, &mut materials,
                        camera_state.pivot, mirror, *index, extent.0,
                    );
                    visible.0 = true;
                }
            } else {
                // Despawn this plane
                vis::mirror_plane::despawn_mirror_by_index(&mut commands, &mirror_query, *index);
            }
        }
    }
}

/// P key or menu: toggle periodic boundary images.
fn toggle_periodic_images(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut events: EventReader<MenuAction>,
    mut periodic: ResMut<PeriodicImages>,
    elements: Res<data::ElementData>,
    loaded: Res<LoadedStructure>,
    atom_query: Query<Entity, With<vis::atoms::AtomMarker>>,
) {
    let toggle = keys.just_pressed(KeyCode::KeyP)
        || events.read().any(|e| matches!(e, MenuAction::TogglePeriodicImages));

    if !toggle {
        return;
    }

    periodic.0 = !periodic.0;
    println!("Periodic images: {}", if periodic.0 { "on" } else { "off" });

    // Despawn all atoms and respawn with new setting
    for entity in atom_query.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(ref structure) = loaded.0 {
        vis::spawn_structure(&mut commands, &mut meshes, &mut materials, &elements, structure, periodic.0);
    }
}

/// Convert a vasprun Structure to our canonical Structure.
fn convert_vasprun_structure(vs: &io::vasprun::types::Structure) -> data::Structure {
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

/// React to ionic step changes: update atom positions, forces, and magnetic moments.
fn update_trajectory_step(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    vasprun: Res<ui::VasprunData>,
    mut ui_state: ResMut<ui::VasprunUiState>,
    elements: Res<data::ElementData>,
    periodic: Res<PeriodicImages>,
    mut loaded: ResMut<LoadedStructure>,
    mut camera_state: ResMut<CameraState>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut extent: ResMut<StructureExtent>,
    symprec: Res<SymmetryTolerance>,
    atom_query: Query<Entity, With<vis::atoms::AtomMarker>>,
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

    // Convert and load the structure for this step
    let structure = convert_vasprun_structure(&step.structure);

    // Despawn old atoms and unit cell
    for entity in atom_query.iter() {
        commands.entity(entity).despawn();
    }
    vis::despawn_unit_cell(&mut commands, &outline_query);

    // Recompute pivot/extent from lattice
    let lat = &structure.lattice;
    let a = Vec3::new(lat[(0, 0)] as f32, lat[(0, 1)] as f32, lat[(0, 2)] as f32);
    let b = Vec3::new(lat[(1, 0)] as f32, lat[(1, 1)] as f32, lat[(1, 2)] as f32);
    let c = Vec3::new(lat[(2, 0)] as f32, lat[(2, 1)] as f32, lat[(2, 2)] as f32);
    camera_state.pivot = (a + b + c) / 2.0;
    let corners = [Vec3::ZERO, a, b, c, a+b, a+c, b+c, a+b+c];
    extent.0 = corners.iter()
        .map(|corner| (*corner - camera_state.pivot).length())
        .fold(0.0f32, f32::max)
        .max(2.0) * 1.2;

    vis::spawn_structure(&mut commands, &mut meshes, &mut materials, &elements, &structure, periodic.0);
    vis::spawn_unit_cell(&mut commands, &mut meshes, &mut materials, &structure);

    // Despawn old arrows
    vis::arrows::despawn_by_marker(&mut commands, &force_query);
    vis::arrows::despawn_by_marker(&mut commands, &mag_query);

    // Spawn force arrows if enabled
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

    // Spawn magnetic moment arrows if enabled
    if ui_state.show_mag_moments {
        // Try direct magnetization varray first, then compute from PDOS
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
fn update_ldos_coloring(
    vasprun: Res<ui::VasprunData>,
    mut ui_state: ResMut<ui::VasprunUiState>,
    atom_query: Query<(&vis::atoms::AtomMarker, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    elements: Res<data::ElementData>,
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
        0, // spin 0
    );

    vis::ldos_coloring::apply_ldos_coloring(&weights, &atom_query, &mut materials);
}

/// Isosurface control panel.
fn isosurface_panel(
    mut contexts: EguiContexts,
    loaded_volume: Res<LoadedVolume>,
    mut iso_state: ResMut<IsoState>,
) {
    let Some(ref grid) = loaded_volume.0 else { return };

    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("isosurface_panel")
        .default_width(220.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Isosurface");

            let has_negative = grid.min() < 0.0;
            if has_negative {
                ui.label("Wavefunction / density difference");
                ui.label(egui::RichText::new("Blue: +level  Red: −level").small().color(egui::Color32::GRAY));
            }

            ui.label(format!("Grid: {}×{}×{}", grid.dims[0], grid.dims[1], grid.dims[2]));
            ui.label(format!("Range: [{:.2e}, {:.2e}]", grid.min(), grid.max()));
            ui.label(format!("σ = {:.2e}", grid.std_dev()));

            ui.separator();

            if ui.checkbox(&mut iso_state.show, "Show isosurface").changed() {
                iso_state.changed = true;
            }

            if iso_state.show {
                ui.label("Isovalue:");
                // Text input for precise values
                let mut iso_str = format!("{:.4e}", iso_state.isovalue);
                if ui.add(
                    egui::TextEdit::singleline(&mut iso_str)
                        .desired_width(100.0)
                ).lost_focus() {
                    if let Ok(v) = iso_str.trim().parse::<f32>() {
                        iso_state.isovalue = v;
                        iso_state.changed = true;
                    }
                }

                // Slider for quick adjustment (log scale)
                let abs_max = grid.min().abs().max(grid.max().abs()) as f32;
                let log_min = (abs_max * 1e-4).log10();
                let log_max = abs_max.log10();
                let mut log_val = iso_state.isovalue.abs().max(1e-20).log10();
                if ui.add(egui::Slider::new(&mut log_val, log_min..=log_max)
                    .text("log₁₀")
                ).changed() {
                    iso_state.isovalue = 10.0f32.powf(log_val);
                    iso_state.changed = true;
                }

                ui.separator();

                ui.label("Opacity:");
                if ui.add(egui::Slider::new(&mut iso_state.opacity, 0.05..=1.0)
                    .text("α")
                ).changed() {
                    iso_state.changed = true;
                }

                if ui.button("Reset to suggested").clicked() {
                    iso_state.isovalue = grid.suggest_isovalue() as f32;
                    iso_state.changed = true;
                }
            }
        });
}

/// Rebuild isosurface mesh when isovalue or visibility changes.
fn update_isosurface(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut iso_state: ResMut<IsoState>,
    loaded_volume: Res<LoadedVolume>,
    iso_query: Query<Entity, With<vis::isosurface::IsosurfaceEntity>>,
) {
    if !iso_state.changed {
        return;
    }
    iso_state.changed = false;

    // Despawn old isosurface
    vis::isosurface::despawn_isosurface(&mut commands, &iso_query);

    if !iso_state.show {
        return;
    }

    let Some(ref grid) = loaded_volume.0 else { return };

    println!("Computing isosurface at level {:.4e}...", iso_state.isovalue);
    vis::isosurface::spawn_isosurface(
        &mut commands, &mut meshes, &mut materials,
        grid, iso_state.isovalue as f64, iso_state.opacity,
    );
}

/// Handle opening a structure file from the File menu.
fn handle_open_structure(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventReader<MenuAction>,
    mut loaded: ResMut<LoadedStructure>,
    elements: Res<data::ElementData>,
    mut camera_state: ResMut<CameraState>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut extent: ResMut<StructureExtent>,
    symprec: Res<SymmetryTolerance>,
    periodic: Res<PeriodicImages>,
    atom_query: Query<Entity, With<vis::atoms::AtomMarker>>,
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

            // Despawn old scene
            for entity in atom_query.iter() {
                commands.entity(entity).despawn();
            }
            vis::despawn_unit_cell(&mut commands, &outline_query);

            load_structure_into_scene(
                &structure, &mut commands, &mut meshes, &mut materials,
                &elements, &mut camera_state, &mut sym_axes, &mut extent,
                symprec.0, periodic.0,
            );

            // Reposition camera
            let mut cam = cam_query.single_mut();
            let offset = Vec3::new(1.0, 1.0, 1.0).normalize() * extent.0 * 2.0;
            cam.translation = camera_state.pivot + offset;
            cam.look_at(camera_state.pivot, Vec3::Y);

            loaded.0 = Some(structure);
        }
        Err(e) => eprintln!("Error loading structure: {e}"),
    }
}

/// Handle opening a vasprun.xml from the File menu.
fn handle_open_vasprun(
    mut events: EventReader<MenuAction>,
    mut vasprun_data: ResMut<ui::VasprunData>,
    mut vasprun_ui: ResMut<ui::VasprunUiState>,
    mut loaded: ResMut<LoadedStructure>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    elements: Res<data::ElementData>,
    mut camera_state: ResMut<CameraState>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut extent: ResMut<StructureExtent>,
    symprec: Res<SymmetryTolerance>,
    atom_query: Query<Entity, With<vis::atoms::AtomMarker>>,
    outline_query: Query<Entity, With<UnitCellOutline>>,
    mut cam_query: Query<&mut Transform, (With<Camera3d>, Without<vis::axes_gizmo::GizmoCamera>)>,
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

            // Initialize LDOS UI state
            if let Some(ref dos) = vr.dos {
                vasprun_ui.ldos_energy_min = *dos.total.energies.first().unwrap_or(&-10.0) as f32;
                vasprun_ui.ldos_energy_max = *dos.total.energies.last().unwrap_or(&10.0) as f32;
                vasprun_ui.ldos_energy = dos.efermi as f32;
                if let Some(ref pdos) = dos.partial {
                    vasprun_ui.orbital_labels = pdos.orbitals.clone();
                    vasprun_ui.selected_orbitals = vec![true; pdos.orbitals.len()];
                }
            }
            vasprun_ui.current_step = 0;

            // Load initial structure
            let vr_struct = &vr.initial_structure;
            let structure = convert_vasprun_structure(vr_struct);

            for entity in atom_query.iter() {
                commands.entity(entity).despawn();
            }
            vis::despawn_unit_cell(&mut commands, &outline_query);

            load_structure_into_scene(
                &structure, &mut commands, &mut meshes, &mut materials,
                &elements, &mut camera_state, &mut sym_axes, &mut extent,
                symprec.0, true,
            );

            let mut cam = cam_query.single_mut();
            let offset = Vec3::new(1.0, 1.0, 1.0).normalize() * extent.0 * 2.0;
            cam.translation = camera_state.pivot + offset;
            cam.look_at(camera_state.pivot, Vec3::Y);

            loaded.0 = Some(structure);
            vasprun_data.0 = Some(vr);
        }
        Err(e) => eprintln!("Error loading vasprun.xml: {e}"),
    }
}

/// Handle opening a volumetric file from the File menu.
fn handle_open_volumetric(
    mut events: EventReader<MenuAction>,
    mut loaded_volume: ResMut<LoadedVolume>,
    mut iso_state: ResMut<IsoState>,
    mut loaded: ResMut<LoadedStructure>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    elements: Res<data::ElementData>,
    mut camera_state: ResMut<CameraState>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut extent: ResMut<StructureExtent>,
    symprec: Res<SymmetryTolerance>,
    atom_query: Query<Entity, With<vis::atoms::AtomMarker>>,
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

            // Despawn old isosurface
            vis::isosurface::despawn_isosurface(&mut commands, &iso_query);

            // Load structure if none loaded
            if loaded.0.is_none() {
                for entity in atom_query.iter() {
                    commands.entity(entity).despawn();
                }
                vis::despawn_unit_cell(&mut commands, &outline_query);

                load_structure_into_scene(
                    &vol_structure, &mut commands, &mut meshes, &mut materials,
                    &elements, &mut camera_state, &mut sym_axes, &mut extent,
                    symprec.0, true,
                );

                let mut cam = cam_query.single_mut();
                let offset = Vec3::new(1.0, 1.0, 1.0).normalize() * extent.0 * 2.0;
                cam.translation = camera_state.pivot + offset;
                cam.look_at(camera_state.pivot, Vec3::Y);

                loaded.0 = Some(vol_structure);
            }

            loaded_volume.0 = Some(grid);
        }
        Err(e) => eprintln!("Error loading volumetric data: {e}"),
    }
}

/// Click to select an atom. Esc to deselect.
fn atom_pick_system(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut selected: ResMut<vis::selection::SelectedAtom>,
    cam_query: Query<&Transform, (With<Camera3d>, Without<vis::axes_gizmo::GizmoCamera>)>,
    atom_query: Query<(Entity, &vis::atoms::AtomMarker, &Transform), Without<Camera3d>>,
    highlight_query: Query<Entity, With<vis::selection::SelectionHighlight>>,
    mut contexts: EguiContexts,
) {
    // Esc to deselect
    if keys.just_pressed(KeyCode::Escape) {
        selected.index = None;
        selected.entity = None;
        vis::selection::despawn_selection_highlight(&mut commands, &highlight_query);
        return;
    }

    // Only pick on left click release (not drag)
    if !mouse.just_released(MouseButton::Left) {
        return;
    }

    // Don't pick if egui wants input
    if contexts.ctx_mut().wants_pointer_input() {
        return;
    }

    let window = windows.single();
    let Some(cursor_pos) = window.cursor_position() else { return };
    let cam_transform = cam_query.single();

    // Collect atoms
    let atoms: Vec<_> = atom_query.iter().collect();

    if let Some((_entity, atom_idx)) = vis::selection::pick_atom(
        cam_transform, window, cursor_pos, &atoms,
    ) {
        // Despawn old highlight
        vis::selection::despawn_selection_highlight(&mut commands, &highlight_query);

        // Find the atom's transform for positioning
        if let Some((_, _, atom_transform)) = atoms.iter().find(|(_, m, _)| m.index == atom_idx) {
            let highlight_entity = vis::selection::spawn_selection_highlight(
                &mut commands, &mut meshes, &mut materials,
                atom_transform.translation,
                atom_transform.scale.x,
            );
            selected.index = Some(atom_idx);
            selected.entity = Some(highlight_entity);
        }
    }
}

/// F12 or menu: save screenshot.
fn screenshot_system(
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
