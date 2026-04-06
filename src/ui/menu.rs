use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::analysis::SymmetryAxes;
use crate::scene::SymmetryTolerance;

/// Events emitted by menu actions.
#[derive(Event)]
pub enum MenuAction {
    OpenStructure(std::path::PathBuf),
    OpenVasprun(std::path::PathBuf),
    OpenVolumetric(std::path::PathBuf),
    TakeScreenshot,
    Quit,
    ResetCamera,
    ToggleUnitCell,
    ToggleAxesGizmo,
    ToggleWyckoff,
    ToggleMirrorPlanes,
    ToggleMirror(usize, bool),
    TogglePeriodicImages,
    CycleSymmetryAxis,
    RerunSymmetry,
    CreateSupercell(usize, usize, usize),
}

/// Persistent state for the supercell input fields.
#[derive(Resource)]
pub struct SupercellInput {
    pub na: u32,
    pub nb: u32,
    pub nc: u32,
}

impl Default for SupercellInput {
    fn default() -> Self {
        Self { na: 1, nb: 1, nc: 1 }
    }
}

/// Persistent text buffer for the symprec input field.
#[derive(Resource)]
pub struct SymprecInput(pub String);

impl Default for SymprecInput {
    fn default() -> Self {
        Self("1e-5".to_string())
    }
}

/// Per-mirror visibility state.
#[derive(Resource, Default)]
pub struct MirrorVisibility(pub Vec<bool>);

/// Top menu bar rendered with egui.
pub fn menu_bar_system(
    mut contexts: EguiContexts,
    mut menu_events: EventWriter<MenuAction>,
    sym_axes: Res<SymmetryAxes>,
    mut symprec: ResMut<SymmetryTolerance>,
    mut sc_input: ResMut<SupercellInput>,
    mut symprec_input: ResMut<SymprecInput>,
    mut mirror_vis: ResMut<MirrorVisibility>,
) {
    // Ensure mirror visibility vec matches number of mirrors
    if mirror_vis.0.len() != sym_axes.mirrors.len() {
        mirror_vis.0.resize(sym_axes.mirrors.len(), false);
    }

    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            // File menu
            ui.menu_button("File", |ui| {
                if ui.button("Open Structure...").clicked() {
                    ui.close_menu();
                    // POSCAR, CONTCAR have no extension; .vasp and .poscar do
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Structure (POSCAR / CONTCAR / *.vasp / *.poscar)")
                        .add_filter("Structure files", &["vasp", "poscar"])
                        .add_filter("All files", &["*"])
                        .pick_file()
                    {
                        menu_events.send(MenuAction::OpenStructure(path));
                    }
                }
                if ui.button("Open vasprun.xml...").clicked() {
                    ui.close_menu();
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open vasprun.xml")
                        .add_filter("XML files", &["xml"])
                        .add_filter("Gzipped XML", &["gz"])
                        .add_filter("All files", &["*"])
                        .pick_file()
                    {
                        menu_events.send(MenuAction::OpenVasprun(path));
                    }
                }
                if ui.button("Open Volumetric...").clicked() {
                    ui.close_menu();
                    // CHGCAR, LOCPOT, etc. have no extension; wavefunction files use .vasp
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Volumetric (CHGCAR / LOCPOT / *.vasp)")
                        .add_filter("Volumetric files", &["vasp"])
                        .add_filter("All files", &["*"])
                        .pick_file()
                    {
                        menu_events.send(MenuAction::OpenVolumetric(path));
                    }
                }
                ui.separator();
                if ui.button("Screenshot  [F12]").clicked() {
                    menu_events.send(MenuAction::TakeScreenshot);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    menu_events.send(MenuAction::Quit);
                    ui.close_menu();
                }
            });

            // Edit menu
            ui.menu_button("Edit", |ui| {
                if ui.button("Reset Camera").clicked() {
                    menu_events.send(MenuAction::ResetCamera);
                    ui.close_menu();
                }
                ui.separator();
                ui.menu_button("Create Supercell", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("a:");
                        ui.add(egui::DragValue::new(&mut sc_input.na).range(1..=10).speed(0.1));
                        ui.label("b:");
                        ui.add(egui::DragValue::new(&mut sc_input.nb).range(1..=10).speed(0.1));
                        ui.label("c:");
                        ui.add(egui::DragValue::new(&mut sc_input.nc).range(1..=10).speed(0.1));
                    });
                    if ui.button("Create").clicked() {
                        menu_events.send(MenuAction::CreateSupercell(
                            sc_input.na as usize,
                            sc_input.nb as usize,
                            sc_input.nc as usize,
                        ));
                        ui.close_menu();
                    }
                });
            });

            // View menu
            ui.menu_button("View", |ui| {
                if ui.button("Toggle Unit Cell  [U]").clicked() {
                    menu_events.send(MenuAction::ToggleUnitCell);
                    ui.close_menu();
                }
                if ui.button("Toggle Periodic Images  [P]").clicked() {
                    menu_events.send(MenuAction::TogglePeriodicImages);
                    ui.close_menu();
                }
                if ui.button("Toggle Axes Gizmo").clicked() {
                    menu_events.send(MenuAction::ToggleAxesGizmo);
                    ui.close_menu();
                }
                if ui.button("Toggle Wyckoff Positions  [W]").clicked() {
                    menu_events.send(MenuAction::ToggleWyckoff);
                    ui.close_menu();
                }

                ui.separator();

                // Mirror planes with per-plane checkboxes
                if !sym_axes.mirrors.is_empty() {
                    ui.menu_button("Mirror Planes", |ui| {
                        // Toggle all
                        if ui.button("Show All").clicked() {
                            for (i, vis) in mirror_vis.0.iter_mut().enumerate() {
                                if !*vis {
                                    *vis = true;
                                    menu_events.send(MenuAction::ToggleMirror(i, true));
                                }
                            }
                        }
                        if ui.button("Hide All").clicked() {
                            for (i, vis) in mirror_vis.0.iter_mut().enumerate() {
                                if *vis {
                                    *vis = false;
                                    menu_events.send(MenuAction::ToggleMirror(i, false));
                                }
                            }
                        }
                        ui.separator();

                        // Individual checkboxes
                        for (i, mirror) in sym_axes.mirrors.iter().enumerate() {
                            if i < mirror_vis.0.len() {
                                let (r, g, b) = crate::vis::mirror_plane::MIRROR_COLORS
                                    [i % crate::vis::mirror_plane::MIRROR_COLORS.len()];
                                let color = egui::Color32::from_rgb(
                                    (r * 255.0) as u8,
                                    (g * 255.0) as u8,
                                    (b * 255.0) as u8,
                                );
                                let label = egui::RichText::new(format!(
                                    "■ {} ({:.2}, {:.2}, {:.2})",
                                    mirror.label,
                                    mirror.normal.x, mirror.normal.y, mirror.normal.z,
                                )).color(color);

                                let old_val = mirror_vis.0[i];
                                if ui.checkbox(&mut mirror_vis.0[i], label).changed() {
                                    menu_events.send(MenuAction::ToggleMirror(i, !old_val));
                                }
                            }
                        }
                    });
                }

                ui.separator();

                // Symmetry tolerance input
                ui.label("Symmetry tolerance:");
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut symprec_input.0)
                            .desired_width(80.0)
                            .hint_text("e.g. 1e-3"),
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if let Ok(val) = symprec_input.0.trim().parse::<f64>() {
                            if val > 0.0 {
                                symprec.0 = val;
                            }
                        }
                    }
                });
                ui.label(format!("  current: {:.1e}", symprec.0));
                if ui.button("Re-detect symmetry").clicked() {
                    if let Ok(val) = symprec_input.0.trim().parse::<f64>() {
                        if val > 0.0 {
                            symprec.0 = val;
                        }
                    }
                    menu_events.send(MenuAction::RerunSymmetry);
                    ui.close_menu();
                }

                ui.separator();

                // Symmetry axes submenu
                if !sym_axes.axes.is_empty() {
                    ui.menu_button("Symmetry Axes", |ui| {
                        for (i, axis) in sym_axes.axes.iter().enumerate() {
                            let current = if i == sym_axes.current_index { " ●" } else { "" };
                            let text = format!("{} ({:.2}, {:.2}, {:.2}){current}",
                                axis.label, axis.direction.x, axis.direction.y, axis.direction.z);
                            if ui.button(text).clicked() {
                                menu_events.send(MenuAction::CycleSymmetryAxis);
                                ui.close_menu();
                            }
                        }
                    });
                }
            });

            // Shortcuts hint + spacegroup on the right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !sym_axes.spacegroup.is_empty() {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} (#{})",
                            sym_axes.spacegroup, sym_axes.spacegroup_number
                        ))
                        .small()
                        .color(egui::Color32::LIGHT_GRAY),
                    );
                    ui.separator();
                }
                ui.label(
                    egui::RichText::new("X/Y/Z: view  N: sym axis  Shift+N: lock  U: cell  W: wyckoff  M: mirrors")
                        .small()
                        .color(egui::Color32::GRAY),
                );
            });
        });
    });
}

/// Wyckoff legend panel — shown when Wyckoff highlights are visible.
pub fn wyckoff_legend_system(
    mut contexts: EguiContexts,
    sym_axes: Res<SymmetryAxes>,
    visible: Res<crate::scene::WyckoffVisible>,
) {
    if !visible.0 || sym_axes.wyckoff_sites.is_empty() {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::Window::new("Wyckoff Sites")
        .default_pos([ctx.screen_rect().right() - 200.0, 40.0])
        .resizable(false)
        .collapsible(true)
        .default_open(true)
        .show(ctx, |ui| {
            egui::Grid::new("wyckoff_grid")
                .striped(true)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Site").strong());
                    ui.label(egui::RichText::new("Mult").strong());
                    ui.label(egui::RichText::new("Sym").strong());
                    ui.label(egui::RichText::new("Atoms").strong());
                    ui.end_row();

                    for (i, site) in sym_axes.wyckoff_sites.iter().enumerate() {
                        let (r, g, b) = crate::vis::wyckoff::WYCKOFF_COLORS
                            [i % crate::vis::wyckoff::WYCKOFF_COLORS.len()];
                        let color = egui::Color32::from_rgb(
                            (r * 255.0) as u8,
                            (g * 255.0) as u8,
                            (b * 255.0) as u8,
                        );

                        ui.label(egui::RichText::new(format!("■ {}", site.letter)).color(color).strong());
                        ui.label(format!("{}", site.multiplicity));
                        ui.label(&site.site_symmetry);
                        ui.label(format!("{}", site.atom_indices.len()));
                        ui.end_row();
                    }
                });
        });
}

/// Handle menu action events.
pub fn handle_menu_actions(
    mut events: EventReader<MenuAction>,
    mut exit: EventWriter<AppExit>,
) {
    for event in events.read() {
        match event {
            MenuAction::Quit => {
                exit.send(AppExit::Success);
            }
            MenuAction::ResetCamera => {
                println!("TODO: Reset camera");
            }
            // File loads and other actions handled by dedicated systems
            _ => {}
        }
    }
}
