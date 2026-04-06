use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use crate::data::Structure;
use crate::vis::selection::SelectedAtom;
use crate::analysis::SymmetryAxes;

/// Show atom info panel when an atom is selected.
pub fn atom_info_panel(
    mut contexts: EguiContexts,
    selected: Res<SelectedAtom>,
    loaded: Res<crate::scene::LoadedStructure>,
    sym_axes: Res<SymmetryAxes>,
) {
    let Some(atom_idx) = selected.index else { return };
    let Some(ref structure) = loaded.0 else { return };
    if atom_idx >= structure.num_atoms() { return; }

    let ctx = contexts.ctx_mut();

    egui::Window::new("Atom Info")
        .default_pos([10.0, 400.0])
        .resizable(false)
        .collapsible(true)
        .show(ctx, |ui| {
            let species = &structure.species[atom_idx];
            let z = structure.atomic_numbers[atom_idx];

            ui.heading(format!("{} (#{atom_idx})", species));

            ui.separator();

            // Coordinates
            let frac = structure.to_fractional();
            let cart = structure.to_cartesian();

            if atom_idx < frac.len() {
                let f = &frac[atom_idx];
                ui.label(format!("Fractional: ({:.6}, {:.6}, {:.6})", f.x, f.y, f.z));
            }
            if atom_idx < cart.len() {
                let c = &cart[atom_idx];
                ui.label(format!("Cartesian:  ({:.4}, {:.4}, {:.4}) Å", c.x, c.y, c.z));
            }

            ui.label(format!("Atomic number: {z}"));

            // Wyckoff site
            for site in &sym_axes.wyckoff_sites {
                if site.atom_indices.contains(&atom_idx) {
                    ui.separator();
                    ui.label(format!("Wyckoff site: {} (mult {})", site.letter, site.multiplicity));
                    ui.label(format!("Site symmetry: {}", site.site_symmetry));
                    break;
                }
            }

            ui.separator();
            if ui.small_button("Deselect").clicked() {
                // Can't mutate selected here, but the pick system handles it
            }
            ui.label(egui::RichText::new("Click another atom or Esc to deselect").small().color(egui::Color32::GRAY));
        });
}
