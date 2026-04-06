use bevy::prelude::*;
use crate::io::vasprun::PartialDos;
use crate::vis::atoms::AtomMarker;

/// Compute per-atom DOS weight at a given energy, summing over selected orbitals.
/// Returns a Vec<f32> of length nions, normalized to [0, 1].
pub fn compute_atom_weights(
    pdos: &PartialDos,
    energies: &[f64],
    energy: f64,
    selected_orbitals: &[bool],
    spin: usize,
) -> Vec<f32> {
    let nions = pdos.data.shape()[1];
    let norbitals = pdos.data.shape()[2];
    let nedos = pdos.data.shape()[3];

    // Find the energy index closest to the target
    let energy_idx = energies.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            ((**a) - energy).abs().partial_cmp(&((**b) - energy).abs()).unwrap()
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    if energy_idx >= nedos {
        return vec![0.0; nions];
    }

    let mut weights = Vec::with_capacity(nions);
    for ion in 0..nions {
        let mut total = 0.0f64;
        for orb in 0..norbitals {
            if orb < selected_orbitals.len() && selected_orbitals[orb] {
                total += pdos.data[[spin, ion, orb, energy_idx]];
            }
        }
        weights.push(total as f32);
    }

    // Normalize
    let max_w = weights.iter().cloned().fold(0.0f32, f32::max);
    if max_w > 1e-10 {
        for w in weights.iter_mut() {
            *w /= max_w;
        }
    }

    weights
}

/// Apply LDOS coloring to existing atom entities.
/// weight=0 → base color (dim), weight=1 → bright highlight color.
pub fn apply_ldos_coloring(
    weights: &[f32],
    atom_query: &Query<(&AtomMarker, &MeshMaterial3d<StandardMaterial>)>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    for (marker, mat_handle) in atom_query.iter() {
        let w = if marker.index < weights.len() {
            weights[marker.index]
        } else {
            0.0
        };

        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            // Blend: dim gray → hot color based on weight
            let r = 0.15 + 0.85 * w;
            let g = 0.15 + 0.35 * w;
            let b = 0.15 + 0.05 * w * (1.0 - w) * 4.0; // slight blue at mid
            mat.base_color = Color::srgb(r, g, b);
        }
    }
}

/// Reset atom colors to their element defaults.
pub fn reset_atom_colors(
    atom_query: &Query<(&AtomMarker, &MeshMaterial3d<StandardMaterial>)>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    elements: &crate::data::ElementData,
) {
    for (marker, mat_handle) in atom_query.iter() {
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            let props = elements.by_z(marker.atomic_number);
            mat.base_color = props.color;
        }
    }
}
