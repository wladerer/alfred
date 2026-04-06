use crate::data::Structure;
use crate::data::structure::symbol_to_z;
use nalgebra::{Matrix3, Vector3};
use std::fs;
use std::path::Path;

/// Parse a VASP POSCAR/CONTCAR file into a Structure.
pub fn parse_poscar(path: &Path) -> Result<Structure, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;
    parse_poscar_str(&content)
}

pub fn parse_poscar_str(content: &str) -> Result<Structure, String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 8 {
        return Err("POSCAR file too short".into());
    }

    let comment = lines[0].to_string();

    // Line 2: scaling factor
    let scale: f64 = lines[1]
        .trim()
        .parse()
        .map_err(|_| "Invalid scaling factor")?;

    // Lines 3-5: lattice vectors
    let lattice = parse_lattice(&lines[2..5], scale)?;

    // Line 6: species names (VASP 5+ format)
    // Line 7: ion counts
    // Detect whether line 6 is species names or ion counts
    let (species_names, counts_line) = if lines[5].trim().chars().next().map_or(false, |c| c.is_alphabetic()) {
        let names: Vec<String> = lines[5].split_whitespace().map(String::from).collect();
        (names, 6)
    } else {
        return Err("VASP 4 format (no species names) is not supported".into());
    };

    let counts: Vec<usize> = lines[counts_line]
        .split_whitespace()
        .map(|s| s.parse().map_err(|_| "Invalid ion count"))
        .collect::<Result<Vec<_>, _>>()?;

    if species_names.len() != counts.len() {
        return Err("Mismatch between species names and ion counts".into());
    }

    let total_atoms: usize = counts.iter().sum();

    // Next line: Selective dynamics (optional) or Direct/Cartesian
    let mut coord_line = counts_line + 1;
    if lines.get(coord_line).map_or(false, |l| {
        l.trim().starts_with('S') || l.trim().starts_with('s')
    }) {
        coord_line += 1; // skip selective dynamics
    }

    let is_cartesian = lines.get(coord_line).map_or(false, |l| {
        let first = l.trim().chars().next().unwrap_or(' ');
        first == 'C' || first == 'c' || first == 'K' || first == 'k'
    });

    let pos_start = coord_line + 1;
    if lines.len() < pos_start + total_atoms {
        return Err("Not enough position lines".into());
    }

    let mut positions = Vec::with_capacity(total_atoms);
    for line in &lines[pos_start..pos_start + total_atoms] {
        let coords: Vec<f64> = line
            .split_whitespace()
            .take(3)
            .map(|s| s.parse().map_err(|_| "Invalid coordinate"))
            .collect::<Result<Vec<_>, _>>()?;
        if coords.len() < 3 {
            return Err("Incomplete coordinate line".into());
        }
        positions.push(Vector3::new(coords[0], coords[1], coords[2]));
    }

    // Build species and atomic_numbers arrays
    let mut species = Vec::with_capacity(total_atoms);
    let mut atomic_numbers = Vec::with_capacity(total_atoms);
    for (name, &count) in species_names.iter().zip(counts.iter()) {
        for _ in 0..count {
            atomic_numbers.push(symbol_to_z(name));
            species.push(name.clone());
        }
    }

    Ok(Structure {
        lattice,
        positions,
        atomic_numbers,
        species,
        comment,
        is_cartesian,
    })
}

fn parse_lattice(lines: &[&str], scale: f64) -> Result<Matrix3<f64>, String> {
    let mut rows = [[0.0f64; 3]; 3];
    for (i, line) in lines.iter().enumerate() {
        let vals: Vec<f64> = line
            .split_whitespace()
            .map(|s| s.parse().map_err(|_| "Invalid lattice value"))
            .collect::<Result<Vec<_>, _>>()?;
        if vals.len() < 3 {
            return Err("Incomplete lattice vector".into());
        }
        rows[i] = [vals[0] * scale, vals[1] * scale, vals[2] * scale];
    }
    // nalgebra Matrix3 is column-major, but we want rows as lattice vectors
    // so row i of the matrix = lattice vector i
    Ok(Matrix3::from_rows(&[
        rows[0].into(),
        rows[1].into(),
        rows[2].into(),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NACL_POSCAR: &str = "\
NaCl rock salt
1.0
5.64 0.00 0.00
0.00 5.64 0.00
0.00 0.00 5.64
Na Cl
4 4
Direct
0.0 0.0 0.0
0.5 0.5 0.0
0.5 0.0 0.5
0.0 0.5 0.5
0.5 0.5 0.5
0.0 0.0 0.5
0.0 0.5 0.0
0.5 0.0 0.0";

    #[test]
    fn test_parse_nacl() {
        let s = parse_poscar_str(NACL_POSCAR).unwrap();
        assert_eq!(s.num_atoms(), 8);
        assert_eq!(s.species[0], "Na");
        assert_eq!(s.species[4], "Cl");
        assert_eq!(s.atomic_numbers[0], 11); // Na
        assert_eq!(s.atomic_numbers[4], 17); // Cl
        assert!(!s.is_cartesian);
    }

    #[test]
    fn test_cartesian_conversion() {
        let s = parse_poscar_str(NACL_POSCAR).unwrap();
        let cart = s.to_cartesian();
        // First atom at origin
        assert!((cart[0].x).abs() < 1e-10);
        // Second Na at (0.5, 0.5, 0.0) fractional = (2.82, 2.82, 0.0) Cartesian
        assert!((cart[1].x - 2.82).abs() < 1e-10);
        assert!((cart[1].y - 2.82).abs() < 1e-10);
    }

    #[test]
    fn test_supercell() {
        let s = parse_poscar_str(NACL_POSCAR).unwrap();
        let sc = s.supercell(2, 2, 2);
        assert_eq!(sc.num_atoms(), 8 * 8); // 8 atoms * 2*2*2
        // Lattice should be doubled
        assert!((sc.lattice[(0, 0)] - 11.28).abs() < 1e-10);
        assert!((sc.lattice[(1, 1)] - 11.28).abs() < 1e-10);
        assert!((sc.lattice[(2, 2)] - 11.28).abs() < 1e-10);
        // All fractional coords should be in [0, 1)
        for pos in &sc.positions {
            assert!(pos.x >= 0.0 && pos.x < 1.0, "x={} out of range", pos.x);
            assert!(pos.y >= 0.0 && pos.y < 1.0, "y={} out of range", pos.y);
            assert!(pos.z >= 0.0 && pos.z < 1.0, "z={} out of range", pos.z);
        }
    }

    #[test]
    fn test_roundtrip_coordinates() {
        let s = parse_poscar_str(NACL_POSCAR).unwrap();
        let cart = s.to_cartesian();
        let cart_struct = Structure {
            lattice: s.lattice,
            positions: cart,
            atomic_numbers: s.atomic_numbers.clone(),
            species: s.species.clone(),
            comment: s.comment.clone(),
            is_cartesian: true,
        };
        let frac = cart_struct.to_fractional();
        for (orig, recovered) in s.positions.iter().zip(frac.iter()) {
            assert!((orig - recovered).norm() < 1e-10);
        }
    }
}
