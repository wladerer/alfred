use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::analysis::SymmetryAxes;
use crate::camera::CameraState;
use crate::scene::{LoadedStructure, LoadedVolume, IsoState, WyckoffVisible, MirrorsVisible, UnitCellVisible};
use crate::vis::symmetry_anim::AnimateSymmetry;
use super::menu::{MenuAction, MirrorVisibility};
use super::vasprun_panel::{VasprunData, VasprunUiState, vasprun_section_ui};

/// Unified right-side panel with collapsible sections.
pub fn info_panel_system(
    mut contexts: EguiContexts,
    loaded: Res<LoadedStructure>,
    sym_axes: Res<SymmetryAxes>,
    vasprun: Res<VasprunData>,
    mut ui_state: ResMut<VasprunUiState>,
    loaded_volume: Res<LoadedVolume>,
    mut iso_state: ResMut<IsoState>,
    mut mirror_vis: ResMut<MirrorVisibility>,
    mut menu_events: EventWriter<MenuAction>,
    wyckoff_visible: Res<WyckoffVisible>,
    mut mirrors_visible: ResMut<MirrorsVisible>,
    unit_cell_visible: Res<UnitCellVisible>,
    mut camera_state: ResMut<CameraState>,
    mut cam_query: Query<&mut Transform, (With<Camera3d>, Without<crate::vis::axes_gizmo::GizmoCamera>)>,
    mut anim_events: EventWriter<AnimateSymmetry>,
) {
    let has_structure = loaded.0.is_some();
    let has_vasprun = vasprun.0.is_some();
    let has_volume = loaded_volume.0.is_some();

    if !has_structure && !has_vasprun && !has_volume {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("info_panel")
        .default_width(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // --- Structure ---
                if let Some(ref structure) = loaded.0 {
                    egui::CollapsingHeader::new(egui::RichText::new("Structure").heading())
                        .default_open(true)
                        .show(ui, |ui| {
                            structure_section_ui(
                                ui, structure,
                                &unit_cell_visible,
                                &mut menu_events,
                            );
                        });

                    ui.separator();
                }

                // --- Symmetry ---
                if has_structure && (!sym_axes.spacegroup.is_empty() || !sym_axes.axes.is_empty()) {
                    egui::CollapsingHeader::new(egui::RichText::new("Symmetry").heading())
                        .default_open(true)
                        .show(ui, |ui| {
                            symmetry_section_ui(
                                ui, &sym_axes,
                                &mut mirror_vis,
                                &mut menu_events,
                                &wyckoff_visible,
                                &mut mirrors_visible,
                                &mut camera_state,
                                &mut cam_query,
                                &mut anim_events,
                            );
                        });

                    ui.separator();
                }

                // --- Electronic Structure ---
                if has_vasprun {
                    let vr = vasprun.0.as_ref().unwrap();
                    egui::CollapsingHeader::new(egui::RichText::new("Electronic Structure").heading())
                        .default_open(false)
                        .show(ui, |ui| {
                            vasprun_section_ui(ui, vr, &mut ui_state);
                        });

                    ui.separator();
                }

                // --- Models (Isosurface) ---
                if has_volume {
                    let grid = loaded_volume.0.as_ref().unwrap();
                    egui::CollapsingHeader::new(egui::RichText::new("Models").heading())
                        .default_open(false)
                        .show(ui, |ui| {
                            isosurface_section_ui(ui, grid, &mut iso_state);
                        });
                }
            });
        });
}

// ---------------------------------------------------------------------------
// Structure section
// ---------------------------------------------------------------------------

fn structure_section_ui(
    ui: &mut egui::Ui,
    structure: &crate::data::Structure,
    unit_cell_visible: &UnitCellVisible,
    menu_events: &mut EventWriter<MenuAction>,
) {
    // Formula
    let mut seen = Vec::new();
    for s in &structure.species {
        if !seen.contains(s) {
            seen.push(s.clone());
        }
    }

    // Build formula with subscript numbers
    let formula: String = seen.iter().map(|s| {
        let count = structure.species.iter().filter(|x| *x == s).count();
        if count > 1 {
            let sub: String = count.to_string().chars().map(|c| match c {
                '0' => '\u{2080}', '1' => '\u{2081}', '2' => '\u{2082}',
                '3' => '\u{2083}', '4' => '\u{2084}', '5' => '\u{2085}',
                '6' => '\u{2086}', '7' => '\u{2087}', '8' => '\u{2088}',
                '9' => '\u{2089}', other => other,
            }).collect();
            format!("{s}{sub}")
        } else {
            s.clone()
        }
    }).collect::<Vec<_>>().join("");

    ui.label(egui::RichText::new(&formula).strong().size(16.0));
    ui.label(format!("{} atoms", structure.num_atoms()));

    ui.add_space(4.0);

    // Lattice parameters
    let lat = &structure.lattice;
    let a_vec = lat.row(0);
    let b_vec = lat.row(1);
    let c_vec = lat.row(2);
    let a_len = a_vec.norm();
    let b_len = b_vec.norm();
    let c_len = c_vec.norm();
    let alpha = (b_vec.dot(&c_vec) / (b_len * c_len)).acos().to_degrees();
    let beta = (a_vec.dot(&c_vec) / (a_len * c_len)).acos().to_degrees();
    let gamma = (a_vec.dot(&b_vec) / (a_len * b_len)).acos().to_degrees();
    let volume = lat.determinant().abs();

    egui::Grid::new("lattice_grid")
        .spacing([6.0, 2.0])
        .show(ui, |ui| {
            ui.label(egui::RichText::new("a").strong());
            ui.label(format!("{:.4} \u{00C5}", a_len));
            ui.label(egui::RichText::new("\u{03B1}").strong());
            ui.label(format!("{:.2}\u{00B0}", alpha));
            ui.end_row();

            ui.label(egui::RichText::new("b").strong());
            ui.label(format!("{:.4} \u{00C5}", b_len));
            ui.label(egui::RichText::new("\u{03B2}").strong());
            ui.label(format!("{:.2}\u{00B0}", beta));
            ui.end_row();

            ui.label(egui::RichText::new("c").strong());
            ui.label(format!("{:.4} \u{00C5}", c_len));
            ui.label(egui::RichText::new("\u{03B3}").strong());
            ui.label(format!("{:.2}\u{00B0}", gamma));
            ui.end_row();
        });

    ui.label(format!("V = {:.2} \u{00C5}\u{00B3}", volume));

    ui.add_space(4.0);

    // Unit cell toggle (local copy — toggle_unit_cell system handles the actual state)
    let mut cell_vis = unit_cell_visible.0;
    if ui.checkbox(&mut cell_vis, "Unit cell").changed() {
        menu_events.send(MenuAction::ToggleUnitCell);
    }
}

// ---------------------------------------------------------------------------
// Symmetry section
// ---------------------------------------------------------------------------

fn symmetry_section_ui(
    ui: &mut egui::Ui,
    sym_axes: &SymmetryAxes,
    mirror_vis: &mut MirrorVisibility,
    menu_events: &mut EventWriter<MenuAction>,
    wyckoff_visible: &WyckoffVisible,
    mirrors_visible: &mut MirrorsVisible,
    camera_state: &mut CameraState,
    cam_query: &mut Query<&mut Transform, (With<Camera3d>, Without<crate::vis::axes_gizmo::GizmoCamera>)>,
    anim_events: &mut EventWriter<AnimateSymmetry>,
) {
    // Space group
    if !sym_axes.spacegroup.is_empty() {
        ui.label(egui::RichText::new(
            format!("{} (#{}) ", sym_axes.spacegroup, sym_axes.spacegroup_number)
        ).strong().size(14.0));
        ui.add_space(4.0);
    }

    // Rotation axes — clickable to snap camera, animatable
    if !sym_axes.axes.is_empty() {
        ui.label(egui::RichText::new("Rotation Axes").strong());

        egui::Grid::new("axes_grid")
            .striped(true)
            .spacing([6.0, 2.0])
            .show(ui, |ui| {
                for axis in &sym_axes.axes {
                    let d = axis.direction;

                    if ui.small_button(&axis.label).on_hover_text("View along this axis").clicked() {
                        let mut transform = cam_query.single_mut();
                        let pivot = camera_state.pivot;
                        let dist = (transform.translation - pivot).length();
                        transform.translation = pivot + d * dist;
                        let up = if d.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
                        let up = (up - d * d.dot(up)).normalize();
                        transform.look_at(pivot, up);
                    }

                    if ui.small_button("\u{25B6}").on_hover_text("Animate rotation").clicked() {
                        anim_events.send(AnimateSymmetry::Rotation { axis: d, fold: axis.fold });
                    }

                    ui.label(
                        egui::RichText::new(format!("[{:.2}, {:.2}, {:.2}]", d.x, d.y, d.z))
                            .small()
                            .color(egui::Color32::GRAY)
                    );
                    ui.end_row();
                }
            });

        ui.add_space(4.0);
    }

    // Mirror planes — toggleable checkboxes, animatable
    if !sym_axes.mirrors.is_empty() {
        ui.label(egui::RichText::new("Mirror Planes").strong());

        // Ensure visibility vec is sized
        mirror_vis.0.resize(sym_axes.mirrors.len(), false);

        // Show all / Hide all
        ui.horizontal(|ui| {
            if ui.small_button("Show all").clicked() {
                for (i, v) in mirror_vis.0.iter_mut().enumerate() {
                    if !*v {
                        *v = true;
                        menu_events.send(MenuAction::ToggleMirror(i, true));
                    }
                }
                mirrors_visible.0 = true;
            }
            if ui.small_button("Hide all").clicked() {
                for (i, v) in mirror_vis.0.iter_mut().enumerate() {
                    if *v {
                        *v = false;
                        menu_events.send(MenuAction::ToggleMirror(i, false));
                    }
                }
                mirrors_visible.0 = false;
            }
        });

        egui::Grid::new("mirrors_grid")
            .striped(true)
            .spacing([6.0, 2.0])
            .show(ui, |ui| {
                for (i, mirror) in sym_axes.mirrors.iter().enumerate() {
                    let n = mirror.normal;
                    let (r, g, b) = crate::vis::mirror_plane::MIRROR_COLORS
                        [i % crate::vis::mirror_plane::MIRROR_COLORS.len()];
                    let color = egui::Color32::from_rgb(
                        (r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8,
                    );

                    let mut vis = mirror_vis.0[i];
                    if ui.checkbox(&mut vis, egui::RichText::new(&mirror.label).color(color)).changed() {
                        mirror_vis.0[i] = vis;
                        menu_events.send(MenuAction::ToggleMirror(i, vis));
                        if vis { mirrors_visible.0 = true; }
                    }

                    if ui.small_button("\u{25B6}").on_hover_text("Animate reflection").clicked() {
                        anim_events.send(AnimateSymmetry::Reflection { normal: n });
                    }

                    ui.label(
                        egui::RichText::new(format!("n=[{:.2}, {:.2}, {:.2}]", n.x, n.y, n.z))
                            .small()
                            .color(egui::Color32::GRAY)
                    );
                    ui.end_row();
                }
            });

        ui.add_space(4.0);
    }

    // Inversion center
    if sym_axes.has_inversion {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Inversion Center").strong());
            ui.label(egui::RichText::new("i").italics());
            if ui.small_button("\u{25B6}").on_hover_text("Animate inversion").clicked() {
                anim_events.send(AnimateSymmetry::Inversion);
            }
        });
        ui.add_space(4.0);
    }

    // Wyckoff sites — toggleable, with colored table
    if !sym_axes.wyckoff_sites.is_empty() {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Wyckoff Sites").strong());
            let mut wk_vis = wyckoff_visible.0;
            if ui.checkbox(&mut wk_vis, "")
                .on_hover_text("Toggle 3D highlights")
                .changed()
            {
                menu_events.send(MenuAction::ToggleWyckoff);
            }
        });

        egui::Grid::new("wyckoff_info_grid")
            .striped(true)
            .spacing([12.0, 3.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Site").strong());
                ui.label(egui::RichText::new("Mult").strong());
                ui.label(egui::RichText::new("Symmetry").strong());
                ui.label(egui::RichText::new("N").strong());
                ui.end_row();

                for (i, site) in sym_axes.wyckoff_sites.iter().enumerate() {
                    let (r, g, b) = crate::vis::wyckoff::WYCKOFF_COLORS
                        [i % crate::vis::wyckoff::WYCKOFF_COLORS.len()];
                    let color = egui::Color32::from_rgb(
                        (r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8,
                    );

                    ui.label(egui::RichText::new(
                        format!("\u{25CF} {}", site.letter)
                    ).color(color).strong());
                    ui.label(format!("{}", site.multiplicity));
                    ui.label(egui::RichText::new(&site.site_symmetry).monospace());
                    ui.label(format!("{}", site.atom_indices.len()));
                    ui.end_row();
                }
            });
    }
}

// ---------------------------------------------------------------------------
// Isosurface section
// ---------------------------------------------------------------------------

fn isosurface_section_ui(ui: &mut egui::Ui, grid: &crate::data::VolumeGrid, iso_state: &mut IsoState) {
    let has_negative = grid.min() < 0.0;
    if has_negative {
        ui.label("Wavefunction / density difference");
        ui.label(egui::RichText::new("Blue: +level  Red: \u{2212}level")
            .small().color(egui::Color32::GRAY));
    }

    ui.label(format!("Grid: {}\u{00D7}{}\u{00D7}{}", grid.dims[0], grid.dims[1], grid.dims[2]));
    ui.label(format!("Range: [{:.2e}, {:.2e}]", grid.min(), grid.max()));
    ui.label(format!("\u{03C3} = {:.2e}", grid.std_dev()));

    ui.separator();

    if ui.checkbox(&mut iso_state.show, "Show isosurface").changed() {
        iso_state.changed = true;
    }

    if iso_state.show {
        ui.label("Isovalue:");
        let mut iso_str = format!("{:.4e}", iso_state.isovalue);
        if ui.add(
            egui::TextEdit::singleline(&mut iso_str).desired_width(100.0)
        ).lost_focus() {
            if let Ok(v) = iso_str.trim().parse::<f32>() {
                iso_state.isovalue = v;
                iso_state.changed = true;
            }
        }

        let abs_max = grid.min().abs().max(grid.max().abs()) as f32;
        let log_min = (abs_max * 1e-4).log10();
        let log_max = abs_max.log10();
        let mut log_val = iso_state.isovalue.abs().max(1e-20).log10();
        if ui.add(egui::Slider::new(&mut log_val, log_min..=log_max)
            .text("log\u{2081}\u{2080}")
        ).changed() {
            iso_state.isovalue = 10.0f32.powf(log_val);
            iso_state.changed = true;
        }

        ui.separator();

        ui.label("Opacity:");
        if ui.add(egui::Slider::new(&mut iso_state.opacity, 0.05..=1.0)
            .text("\u{03B1}")
        ).changed() {
            iso_state.changed = true;
        }

        if ui.button("Reset to suggested").clicked() {
            iso_state.isovalue = grid.suggest_isovalue() as f32;
            iso_state.changed = true;
        }
    }
}
