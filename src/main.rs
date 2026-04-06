mod analysis;
mod camera;
mod data;
mod io;
mod scene;
mod ui;
mod vis;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use ui::menu::MenuAction;

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
        .add_event::<vis::symmetry_anim::AnimateSymmetry>()
        // File paths
        .insert_resource(scene::PoscarPath(poscar_path))
        .insert_resource(scene::VasprunPath(vasprun_path))
        .insert_resource(scene::VolumetricPath(volumetric_path))
        // Data resources
        .insert_resource(data::ElementData::load())
        .insert_resource(scene::LoadedVolume(None))
        .insert_resource(scene::LoadedStructure(None))
        .insert_resource(scene::IsoState::default())
        .insert_resource(scene::StructureExtent(5.0))
        .insert_resource(scene::SymmetryTolerance(1e-5))
        .insert_resource(scene::PrimitiveAtomCount(0))
        .insert_resource(scene::UnitCellVisible(true))
        .insert_resource(scene::WyckoffVisible(false))
        .insert_resource(scene::MirrorsVisible(false))
        .insert_resource(scene::PeriodicImages(true))
        // UI state
        .insert_resource(ui::VasprunData::default())
        .insert_resource(ui::VasprunUiState::default())
        .insert_resource(ui::menu::SupercellInput::default())
        .insert_resource(ui::menu::SymprecInput::default())
        .insert_resource(ui::menu::MirrorVisibility::default())
        // Camera
        .insert_resource(camera::CameraState::default())
        .insert_resource(analysis::SymmetryAxes::default())
        // Startup
        .add_systems(Startup, scene::setup_scene)
        // UI (ordering: top panel and view panel before right info panel)
        .add_systems(Update, ui::menu_bar_system.before(ui::info_panel_system))
        .add_systems(Update, camera::view_panel_system.before(ui::info_panel_system))
        .add_systems(Update, ui::info_panel_system)
        .add_systems(Update, ui::menu::handle_menu_actions)
        // Camera
        .add_systems(Update, camera::orbit_camera)
        .add_systems(Update, camera::axis_view_shortcuts)
        .add_systems(Update, camera::cycle_symmetry_axis)
        .add_systems(Update, camera::arrow_pan_system)
        .add_systems(Update, camera::fine_rotation_system)
        .add_systems(Update, camera::update_gizmo_viewport)
        .add_systems(Update, vis::sync_gizmo_camera)
        // Scene toggles
        .add_systems(Update, scene::toggle_unit_cell)
        .add_systems(Update, scene::toggle_periodic_images)
        .add_systems(Update, scene::toggle_wyckoff)
        .add_systems(Update, scene::toggle_mirror_planes)
        .add_systems(Update, scene::handle_individual_mirror_toggle)
        .add_systems(Update, scene::handle_rerun_symmetry)
        .add_systems(Update, scene::handle_create_supercell)
        // Trajectory and coloring (ldos must run after trajectory step)
        .add_systems(Update, scene::update_trajectory_step)
        .add_systems(Update, scene::update_ldos_coloring.after(scene::update_trajectory_step))
        .add_systems(Update, scene::update_isosurface)
        // File handlers
        .add_systems(Update, scene::handle_open_structure)
        .add_systems(Update, scene::handle_open_vasprun)
        .add_systems(Update, scene::handle_open_volumetric)
        .add_systems(Update, scene::apply_camera_reposition)
        // Symmetry animation
        .add_systems(Update, vis::symmetry_anim::handle_animate_symmetry)
        .add_systems(Update, vis::symmetry_anim::animate_symmetry)
        // Misc
        .add_systems(Update, scene::screenshot_system)
        .run();
}
