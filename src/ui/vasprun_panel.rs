use bevy::prelude::*;
use bevy_egui::egui;

use crate::io::vasprun;
use crate::io::vasprun::types::IncarValue;

/// Resource holding a loaded vasprun.xml dataset.
#[derive(Resource, Default)]
pub struct VasprunData(pub Option<vasprun::Vasprun>);

/// State for the trajectory scrubber and visualization options.
#[derive(Resource)]
pub struct VasprunUiState {
    pub current_step: usize,
    pub show_forces: bool,
    pub force_scale: f32,
    pub show_mag_moments: bool,
    pub mag_scale: f32,
    pub show_ldos: bool,
    pub ldos_energy: f32,
    pub ldos_energy_min: f32,
    pub ldos_energy_max: f32,
    pub selected_orbitals: Vec<bool>,
    pub orbital_labels: Vec<String>,
    pub step_changed: bool,
    pub ldos_changed: bool,
}

impl Default for VasprunUiState {
    fn default() -> Self {
        Self {
            current_step: 0,
            show_forces: false,
            force_scale: 2.0,
            show_mag_moments: false,
            mag_scale: 1.0,
            show_ldos: false,
            ldos_energy: 0.0,
            ldos_energy_min: -10.0,
            ldos_energy_max: 10.0,
            selected_orbitals: Vec::new(),
            orbital_labels: Vec::new(),
            step_changed: false,
            ldos_changed: false,
        }
    }
}

impl VasprunUiState {
    /// Initialize LDOS UI state from parsed DOS data.
    pub fn init_from_dos(&mut self, dos: &vasprun::types::Dos) {
        self.ldos_energy_min = *dos.total.energies.first().unwrap_or(&-10.0) as f32;
        self.ldos_energy_max = *dos.total.energies.last().unwrap_or(&10.0) as f32;
        self.ldos_energy = dos.efermi as f32;
        if let Some(ref pdos) = dos.partial {
            self.orbital_labels = pdos.orbitals.clone();
            self.selected_orbitals = vec![true; pdos.orbitals.len()];
        }
    }
}

/// Check if a vasprun is non-collinear (LNONCOLLINEAR or LSORBIT).
fn is_noncollinear(vr: &vasprun::Vasprun) -> bool {
    let lnc = vr.incar.get("LNONCOLLINEAR")
        .map(|v| matches!(v, IncarValue::Bool(true)))
        .unwrap_or(false);
    let lso = vr.incar.get("LSORBIT")
        .map(|v| matches!(v, IncarValue::Bool(true)))
        .unwrap_or(false);
    lnc || lso
}

/// Render vasprun electronic structure UI into an existing `Ui` context.
/// Called from the info panel's "Electronic Structure" collapsing section.
pub fn vasprun_section_ui(ui: &mut egui::Ui, vr: &vasprun::Vasprun, ui_state: &mut VasprunUiState) {
    if vr.ionic_steps.is_empty() { return; }

    let n_steps = vr.ionic_steps.len();
    let step = &vr.ionic_steps[ui_state.current_step.min(n_steps - 1)];

    // --- Trajectory scrubber ---
    ui.label(egui::RichText::new("Trajectory").strong());
    ui.horizontal(|ui| {
        ui.label("Step:");
        let mut s = ui_state.current_step as u32;
        if ui.add(egui::DragValue::new(&mut s)
            .range(0..=(n_steps as u32 - 1))
            .speed(0.2)
        ).changed() {
            ui_state.current_step = s as usize;
            ui_state.step_changed = true;
        }
        ui.label(format!("/ {}", n_steps - 1));
    });

    if n_steps > 1 {
        let mut s = ui_state.current_step;
        if ui.add(egui::Slider::new(&mut s, 0..=(n_steps - 1))
            .text("ionic step")
        ).changed() {
            ui_state.current_step = s;
            ui_state.step_changed = true;
        }
    }

    ui.label(format!("E = {:.6} eV", step.energy.e_fr_energy));
    ui.label(format!("E\u{2080} = {:.6} eV", step.energy.e_0_energy));

    ui.separator();

    // --- Forces ---
    ui.label(egui::RichText::new("Forces").strong());
    if ui.checkbox(&mut ui_state.show_forces, "Show force vectors").changed() {
        ui_state.step_changed = true;
    }
    if ui_state.show_forces {
        if ui.add(egui::Slider::new(&mut ui_state.force_scale, 0.5..=10.0)
            .text("scale")
        ).changed() {
            ui_state.step_changed = true;
        }
        let max_f = step.forces.iter()
            .map(|f| (f[0]*f[0] + f[1]*f[1] + f[2]*f[2]).sqrt())
            .fold(0.0f64, f64::max);
        ui.label(format!("Max |F| = {:.4} eV/\u{00C5}", max_f));
    }

    ui.separator();

    // --- Magnetic moments ---
    ui.label(egui::RichText::new("Magnetic Moments").strong());
    let has_mag = step.magnetization.is_some();
    let can_compute_mag = !has_mag
        && vr.dos.as_ref().and_then(|d| d.partial.as_ref()).is_some()
        && vr.dos.as_ref().map(|d| d.total.densities.shape()[0] >= 2).unwrap_or(false);
    let mag_available = has_mag || can_compute_mag;

    if mag_available {
        let label = if can_compute_mag { "Show moments (from PDOS)" } else { "Show moments" };
        if ui.checkbox(&mut ui_state.show_mag_moments, label).changed() {
            ui_state.step_changed = true;
        }
        if ui_state.show_mag_moments {
            if ui.add(egui::Slider::new(&mut ui_state.mag_scale, 0.5..=5.0)
                .text("scale")
            ).changed() {
                ui_state.step_changed = true;
            }
        }
    } else {
        ui.label(egui::RichText::new("No magnetization data").color(egui::Color32::GRAY));
    }

    ui.separator();

    // --- DOS Plot + LDOS ---
    ui.label(egui::RichText::new("Density of States").strong());

    let has_dos = vr.dos.is_some();
    let has_pdos = vr.dos.as_ref().and_then(|d| d.partial.as_ref()).is_some();

    if has_dos {
        dos_plot_ui(ui, vr, ui_state, has_pdos);
    } else {
        ui.label(egui::RichText::new("No DOS data").color(egui::Color32::GRAY));
    }
}

/// Render the DOS plot and LDOS controls.
fn dos_plot_ui(ui: &mut egui::Ui, vr: &vasprun::Vasprun, ui_state: &mut VasprunUiState, has_pdos: bool) {
    let dos = vr.dos.as_ref().unwrap();
    let nspins = dos.total.densities.shape()[0];
    let nedos = dos.total.energies.len();
    let noncollinear = is_noncollinear(vr);
    let spin_polarized = nspins == 2 && !noncollinear;
    let efermi = dos.efermi as f32;

    let e_min = ui_state.ldos_energy_min - efermi;
    let e_max = ui_state.ldos_energy_max - efermi;
    let plot_height = 250.0;
    let plot_width = ui.available_width().max(200.0);

    let (response, painter) = ui.allocate_painter(
        egui::Vec2::new(plot_width, plot_height),
        egui::Sense::click_and_drag(),
    );
    let rect = response.rect;

    painter.rect_filled(rect, 2.0, egui::Color32::from_gray(30));

    let e_to_y = |e_rel: f32| -> f32 {
        let t = (e_rel - e_min) / (e_max - e_min);
        rect.bottom() - t * rect.height()
    };
    let y_to_e_abs = |y: f32| -> f32 {
        let t = (rect.bottom() - y) / rect.height();
        let e_rel = e_min + t * (e_max - e_min);
        e_rel + efermi
    };

    let max_dos = (0..nedos)
        .map(|i| {
            let mut m = dos.total.densities[[0, i]].abs();
            if nspins > 1 { m = m.max(dos.total.densities[[1, i]].abs()); }
            m
        })
        .fold(0.0f64, f64::max);
    let dos_scale = if max_dos > 1e-10 { max_dos } else { 1.0 };

    let x_center = if spin_polarized { rect.center().x } else { rect.left() + 2.0 };

    let dos_to_x = |d: f64, spin: usize| -> f32 {
        let t = (d / dos_scale) as f32;
        if spin_polarized {
            if spin == 0 {
                x_center + t * (rect.right() - x_center - 2.0)
            } else {
                x_center - t * (x_center - rect.left() - 2.0)
            }
        } else {
            rect.left() + 2.0 + t * (rect.width() - 4.0)
        }
    };

    // Center line for spin-polarized
    if spin_polarized {
        painter.line_segment(
            [egui::pos2(x_center, rect.top()), egui::pos2(x_center, rect.bottom())],
            egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
        );
        painter.text(
            egui::pos2(x_center + 4.0, rect.top() + 2.0),
            egui::Align2::LEFT_TOP, "\u{2191}",
            egui::FontId::proportional(10.0), egui::Color32::from_rgb(100, 180, 255),
        );
        painter.text(
            egui::pos2(x_center - 4.0, rect.top() + 2.0),
            egui::Align2::RIGHT_TOP, "\u{2193}",
            egui::FontId::proportional(10.0), egui::Color32::from_rgb(255, 130, 100),
        );
    } else {
        painter.text(
            egui::pos2(rect.right() - 4.0, rect.bottom() - 4.0),
            egui::Align2::RIGHT_BOTTOM, "DOS \u{2192}",
            egui::FontId::proportional(10.0), egui::Color32::from_gray(100),
        );
    }

    // Fermi level
    if 0.0 >= e_min && 0.0 <= e_max {
        let y_fermi = e_to_y(0.0);
        painter.line_segment(
            [egui::pos2(rect.left(), y_fermi), egui::pos2(rect.right(), y_fermi)],
            egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(100, 100, 100, 120)),
        );
        painter.text(
            egui::pos2(rect.right() - 2.0, y_fermi - 2.0),
            egui::Align2::RIGHT_BOTTOM, "E_F",
            egui::FontId::proportional(9.0), egui::Color32::from_gray(100),
        );
    }

    // DOS curves
    let dos_color_up = egui::Color32::from_rgb(100, 180, 255);
    let dos_color_dn = egui::Color32::from_rgb(255, 130, 100);
    let spins_to_plot = if noncollinear { 1 } else { nspins };

    for spin in 0..spins_to_plot {
        let color = if spin == 0 { dos_color_up } else { dos_color_dn };
        let points: Vec<egui::Pos2> = (0..nedos)
            .filter_map(|i| {
                let e_rel = dos.total.energies[i] as f32 - efermi;
                if e_rel < e_min || e_rel > e_max { return None; }
                let d = dos.total.densities[[spin, i]];
                Some(egui::pos2(dos_to_x(d, spin), e_to_y(e_rel)))
            })
            .collect();

        if points.len() > 1 {
            let zero_x = dos_to_x(0.0, spin);
            let fill_color = egui::Color32::from_rgba_premultiplied(
                color.r(), color.g(), color.b(), 35,
            );
            let mut mesh = egui::Mesh::default();
            for p in &points {
                mesh.colored_vertex(egui::pos2(zero_x, p.y), fill_color);
                mesh.colored_vertex(*p, fill_color);
            }
            for i in 0..(points.len() - 1) {
                let base = (i * 2) as u32;
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base + 1, base + 3, base + 2);
            }
            painter.add(egui::Shape::mesh(mesh));

            let stroke = egui::Stroke::new(1.5, color);
            for w in points.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
    }

    // PDOS overlay
    if ui_state.show_ldos && has_pdos {
        let pdos = dos.partial.as_ref().unwrap();
        let nions = pdos.data.shape()[1];
        let norbitals = pdos.data.shape()[2];

        let mut summed = vec![0.0f64; nedos];
        for ion in 0..nions {
            for orb in 0..norbitals {
                if orb < ui_state.selected_orbitals.len() && ui_state.selected_orbitals[orb] {
                    for ie in 0..nedos {
                        summed[ie] += pdos.data[[0, ion, orb, ie]];
                    }
                }
            }
        }

        let pdos_color = egui::Color32::from_rgb(255, 200, 50);
        let points: Vec<egui::Pos2> = (0..nedos)
            .filter_map(|i| {
                let e_rel = dos.total.energies[i] as f32 - efermi;
                if e_rel < e_min || e_rel > e_max { return None; }
                Some(egui::pos2(dos_to_x(summed[i], 0), e_to_y(e_rel)))
            })
            .collect();

        if points.len() > 1 {
            let stroke = egui::Stroke::new(1.5, pdos_color);
            for w in points.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
    }

    // Energy cursor
    let cursor_e_rel = ui_state.ldos_energy - efermi;
    if cursor_e_rel >= e_min && cursor_e_rel <= e_max {
        let y_cursor = e_to_y(cursor_e_rel);
        painter.line_segment(
            [egui::pos2(rect.left(), y_cursor), egui::pos2(rect.right(), y_cursor)],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 220, 50)),
        );
        painter.text(
            egui::pos2(rect.right() - 2.0, y_cursor + 2.0),
            egui::Align2::RIGHT_TOP,
            format!("{:.2} eV", cursor_e_rel),
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(255, 220, 50),
        );
    }

    // Click/drag to move energy cursor
    if response.dragged() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let new_e = y_to_e_abs(pos.y).clamp(ui_state.ldos_energy_min, ui_state.ldos_energy_max);
            ui_state.ldos_energy = new_e;
            ui_state.ldos_changed = true;
        }
    }

    // Energy axis ticks
    let n_ticks = 5;
    for i in 0..=n_ticks {
        let e = e_min + (e_max - e_min) * (i as f32 / n_ticks as f32);
        let y = e_to_y(e);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.left() + 3.0, y)],
            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
        );
        painter.text(
            egui::pos2(rect.left() + 4.0, y),
            egui::Align2::LEFT_CENTER,
            format!("{:.1}", e),
            egui::FontId::proportional(9.0),
            egui::Color32::from_gray(100),
        );
    }

    // Border
    let bs = egui::Stroke::new(1.0, egui::Color32::from_gray(60));
    painter.line_segment([rect.left_top(), rect.right_top()], bs);
    painter.line_segment([rect.right_top(), rect.right_bottom()], bs);
    painter.line_segment([rect.right_bottom(), rect.left_bottom()], bs);
    painter.line_segment([rect.left_bottom(), rect.left_top()], bs);

    // Legend
    ui.horizontal(|ui| {
        if spin_polarized {
            ui.colored_label(dos_color_up, "\u{25a0} Spin \u{2191}");
            ui.colored_label(dos_color_dn, "\u{25a0} Spin \u{2193}");
        } else {
            ui.colored_label(dos_color_up, "\u{25a0} Total DOS");
            if noncollinear {
                ui.label(egui::RichText::new("(non-collinear)").small().color(egui::Color32::GRAY));
            }
        }
        if ui_state.show_ldos && has_pdos {
            ui.colored_label(egui::Color32::from_rgb(255, 200, 50), "\u{25a0} PDOS");
        }
    });

    ui.separator();

    // --- LDOS controls ---
    ui.label(egui::RichText::new("LDOS on Structure").strong());
    if ui.checkbox(&mut ui_state.show_ldos, "Color atoms by DOS").changed() {
        ui_state.ldos_changed = true;
    }
    if ui_state.show_ldos {
        let abs_min = ui_state.ldos_energy_min;
        let abs_max = ui_state.ldos_energy_max;
        if ui.add(egui::Slider::new(
            &mut ui_state.ldos_energy,
            abs_min..=abs_max,
        ).text("E (eV)").step_by(0.01)
         .custom_formatter(|v, _| format!("{:.2}", v as f32 - efermi))
        ).changed() {
            ui_state.ldos_changed = true;
        }

        ui.label("Orbitals:");
        ui.horizontal_wrapped(|ui| {
            let labels: Vec<String> = ui_state.orbital_labels.clone();
            for (i, label) in labels.iter().enumerate() {
                if i < ui_state.selected_orbitals.len() {
                    if ui.checkbox(&mut ui_state.selected_orbitals[i], label.as_str()).changed() {
                        ui_state.ldos_changed = true;
                    }
                }
            }
        });

        ui.horizontal(|ui| {
            if ui.small_button("All").clicked() {
                for s in ui_state.selected_orbitals.iter_mut() { *s = true; }
                ui_state.ldos_changed = true;
            }
            if ui.small_button("None").clicked() {
                for s in ui_state.selected_orbitals.iter_mut() { *s = false; }
                ui_state.ldos_changed = true;
            }
            let labels = ui_state.orbital_labels.clone();
            for group in &["s", "p", "d", "f"] {
                if ui.small_button(*group).clicked() {
                    for (i, label) in labels.iter().enumerate() {
                        if i < ui_state.selected_orbitals.len() {
                            ui_state.selected_orbitals[i] = label.starts_with(group);
                        }
                    }
                    ui_state.ldos_changed = true;
                }
            }
        });
    }
}
