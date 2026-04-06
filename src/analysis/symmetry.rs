use crate::data::Structure;
use bevy::prelude::*;
use nalgebra::Vector3;
use spglib::cell::Cell;
use spglib::dataset::Dataset;

/// A detected rotational symmetry axis with its fold order.
#[derive(Debug, Clone)]
pub struct RotationAxis {
    pub direction: Vec3,
    pub fold: u8,
    pub label: String,
}

/// A detected mirror plane defined by its normal vector.
#[derive(Debug, Clone)]
pub struct MirrorPlane {
    pub normal: Vec3,
    pub label: String,
}

/// Wyckoff site info for a group of equivalent atoms.
#[derive(Debug, Clone)]
pub struct WyckoffSite {
    pub letter: char,
    pub multiplicity: usize,
    pub atom_indices: Vec<usize>,
    pub site_symmetry: String,
}

/// Resource holding all detected symmetry information.
#[derive(Resource, Default)]
pub struct SymmetryAxes {
    pub axes: Vec<RotationAxis>,
    pub mirrors: Vec<MirrorPlane>,
    pub wyckoff_sites: Vec<WyckoffSite>,
    pub current_index: usize,
    pub spacegroup: String,
    pub spacegroup_number: i32,
}

impl SymmetryAxes {
    pub fn current(&self) -> Option<&RotationAxis> {
        self.axes.get(self.current_index)
    }

    pub fn next(&mut self) -> Option<&RotationAxis> {
        if self.axes.is_empty() {
            return None;
        }
        self.current_index = (self.current_index + 1) % self.axes.len();
        self.axes.get(self.current_index)
    }
}

/// Detect symmetry axes, mirror planes, and Wyckoff positions from a Structure.
pub fn detect_symmetry(structure: &Structure, symprec: f64) -> SymmetryAxes {
    let frac_positions = structure.to_fractional();

    let lattice: [[f64; 3]; 3] = [
        [structure.lattice[(0, 0)], structure.lattice[(0, 1)], structure.lattice[(0, 2)]],
        [structure.lattice[(1, 0)], structure.lattice[(1, 1)], structure.lattice[(1, 2)]],
        [structure.lattice[(2, 0)], structure.lattice[(2, 1)], structure.lattice[(2, 2)]],
    ];

    let positions: Vec<[f64; 3]> = frac_positions
        .iter()
        .map(|p| [p.x, p.y, p.z])
        .collect();

    let types: Vec<i32> = structure.atomic_numbers.iter().map(|&z| z as i32).collect();

    let mut cell = Cell::new(&lattice, &positions, &types);
    let dataset = Dataset::new(&mut cell, symprec);

    let spacegroup = dataset.international_symbol.clone();
    let spacegroup_number = dataset.spacegroup_number;

    println!("Detected spacegroup: {} (#{spacegroup_number})", spacegroup);
    println!("Found {} symmetry operations", dataset.n_operations);

    let lat_t = structure.lattice.transpose();
    let axes = extract_rotation_axes(&dataset.rotations, &lat_t);
    let mirrors = extract_mirror_planes(&dataset.rotations, &lat_t);
    let wyckoff_sites = extract_wyckoff_sites(&dataset);

    println!("Found {} unique rotation axes:", axes.len());
    for ax in &axes {
        println!("  {} — ({:.3}, {:.3}, {:.3})", ax.label, ax.direction.x, ax.direction.y, ax.direction.z);
    }
    println!("Found {} mirror planes:", mirrors.len());
    for m in &mirrors {
        println!("  {} — normal ({:.3}, {:.3}, {:.3})", m.label, m.normal.x, m.normal.y, m.normal.z);
    }
    println!("Found {} Wyckoff sites:", wyckoff_sites.len());
    for w in &wyckoff_sites {
        println!("  {}({}) — {} atoms, site sym: {}", w.letter, w.multiplicity, w.atom_indices.len(), w.site_symmetry);
    }

    SymmetryAxes {
        axes,
        mirrors,
        wyckoff_sites,
        current_index: 0,
        spacegroup,
        spacegroup_number,
    }
}

/// Extract Wyckoff site information from the spglib dataset.
fn extract_wyckoff_sites(dataset: &Dataset) -> Vec<WyckoffSite> {
    use std::collections::BTreeMap;

    // Group atoms by (wyckoff_letter, equivalent_atom_index)
    let mut groups: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();

    for (i, (&wyck, &equiv)) in dataset.wyckoffs.iter().zip(dataset.equivalent_atoms.iter()).enumerate() {
        groups.entry((wyck, equiv)).or_default().push(i);
    }

    // Deduplicate: merge groups with the same Wyckoff letter
    let mut by_letter: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for ((wyck, _), indices) in &groups {
        by_letter.entry(*wyck).or_default().extend(indices);
    }

    let mut sites = Vec::new();
    for (wyck_int, atom_indices) in by_letter {
        let letter = (b'a' + wyck_int as u8) as char;
        let site_sym = if !atom_indices.is_empty() && atom_indices[0] < dataset.site_symmetry_symbols.len() {
            dataset.site_symmetry_symbols[atom_indices[0]].clone()
        } else {
            String::from("1")
        };

        sites.push(WyckoffSite {
            letter,
            multiplicity: atom_indices.len(),
            atom_indices,
            site_symmetry: site_sym,
        });
    }

    sites
}

/// Extract unique mirror plane normals from symmetry operations.
/// Mirrors have det = -1 and trace = 1 (in fractional coordinates).
fn extract_mirror_planes(
    rotations: &[[[i32; 3]; 3]],
    lattice_t: &nalgebra::Matrix3<f64>,
) -> Vec<MirrorPlane> {
    let mut found: Vec<MirrorPlane> = Vec::new();

    for rot in rotations {
        let r = nalgebra::Matrix3::new(
            rot[0][0] as f64, rot[0][1] as f64, rot[0][2] as f64,
            rot[1][0] as f64, rot[1][1] as f64, rot[1][2] as f64,
            rot[2][0] as f64, rot[2][1] as f64, rot[2][2] as f64,
        );

        let trace = r.trace();
        let det = r.determinant();

        // Mirror: det = -1, trace = 1 (i.e. one eigenvalue is -1, two are +1)
        if (det + 1.0).abs() > 0.1 || (trace - 1.0).abs() > 0.1 {
            continue;
        }

        // The mirror normal is the eigenvector with eigenvalue -1
        // Solve (R + I)v = 0
        let rpi = r + nalgebra::Matrix3::identity();
        if let Some(normal_frac) = null_space_vector(&rpi) {
            let normal_cart = lattice_t * normal_frac;
            let norm = normal_cart.norm();
            if norm < 1e-10 {
                continue;
            }
            let normal_cart = normal_cart / norm;
            let normal_cart = canonical_direction(normal_cart);

            let dir = Vec3::new(normal_cart.x as f32, normal_cart.y as f32, normal_cart.z as f32);

            // Deduplicate
            let is_dup = found.iter().any(|existing| {
                existing.normal.dot(dir).abs() > 0.999
            });

            if !is_dup {
                // Label based on alignment to lattice axes
                let label = if dir.dot(Vec3::X).abs() > 0.99 {
                    "σ_v(yz)".to_string()
                } else if dir.dot(Vec3::Y).abs() > 0.99 {
                    "σ_v(xz)".to_string()
                } else if dir.dot(Vec3::Z).abs() > 0.99 {
                    "σ_h(xy)".to_string()
                } else {
                    "σ_d".to_string()
                };

                found.push(MirrorPlane { normal: dir, label });
            }
        }
    }

    found
}

/// Extract unique rotation axes and their fold orders from integer rotation matrices.
fn extract_rotation_axes(
    rotations: &[[[i32; 3]; 3]],
    lattice_t: &nalgebra::Matrix3<f64>,
) -> Vec<RotationAxis> {
    let mut found: Vec<RotationAxis> = Vec::new();

    for rot in rotations {
        let r = nalgebra::Matrix3::new(
            rot[0][0] as f64, rot[0][1] as f64, rot[0][2] as f64,
            rot[1][0] as f64, rot[1][1] as f64, rot[1][2] as f64,
            rot[2][0] as f64, rot[2][1] as f64, rot[2][2] as f64,
        );

        let trace = r.trace();
        let det = r.determinant();

        // Only proper rotations (det = +1), skip identity (trace = 3)
        if (det - 1.0).abs() > 0.1 || (trace - 3.0).abs() < 0.1 {
            continue;
        }

        let fold = fold_from_trace(trace);
        if fold < 2 {
            continue;
        }

        let ri = r - nalgebra::Matrix3::identity();
        if let Some(axis_frac) = null_space_vector(&ri) {
            let axis_cart = lattice_t * axis_frac;
            let norm = axis_cart.norm();
            if norm < 1e-10 {
                continue;
            }
            let axis_cart = axis_cart / norm;
            let axis_cart = canonical_direction(axis_cart);

            let dir = Vec3::new(axis_cart.x as f32, axis_cart.y as f32, axis_cart.z as f32);

            let is_duplicate = found.iter_mut().any(|existing| {
                let dot = existing.direction.dot(dir).abs();
                if dot > 0.999 {
                    if fold > existing.fold {
                        existing.fold = fold;
                        existing.label = format!("C{fold}");
                    }
                    true
                } else {
                    false
                }
            });

            if !is_duplicate {
                found.push(RotationAxis {
                    direction: dir,
                    fold,
                    label: format!("C{fold}"),
                });
            }
        }
    }

    found.sort_by(|a, b| b.fold.cmp(&a.fold));
    found
}

fn fold_from_trace(trace: f64) -> u8 {
    let t = trace.round() as i32;
    match t {
        -1 => 2,
        0 => 3,
        1 => 4,
        2 => 6,
        _ => 0,
    }
}

fn null_space_vector(m: &nalgebra::Matrix3<f64>) -> Option<Vector3<f64>> {
    let svd = m.svd(true, true);
    let v_t = svd.v_t?;
    let min_idx = svd.singular_values
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)?;

    if svd.singular_values[min_idx] > 0.1 {
        return None;
    }

    Some(Vector3::new(v_t[(min_idx, 0)], v_t[(min_idx, 1)], v_t[(min_idx, 2)]))
}

fn canonical_direction(v: Vector3<f64>) -> Vector3<f64> {
    for &val in &[v.z, v.y, v.x] {
        if val.abs() > 1e-10 {
            if val < 0.0 {
                return -v;
            } else {
                return v;
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_from_trace() {
        assert_eq!(fold_from_trace(-1.0), 2);
        assert_eq!(fold_from_trace(0.0), 3);
        assert_eq!(fold_from_trace(1.0), 4);
        assert_eq!(fold_from_trace(2.0), 6);
        assert_eq!(fold_from_trace(3.0), 0);
    }
}
