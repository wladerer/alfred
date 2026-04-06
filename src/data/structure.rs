use nalgebra::{Matrix3, Vector3};

/// Canonical atomistic structure representation.
/// All file formats convert into this.
#[derive(Debug, Clone)]
pub struct Structure {
    pub lattice: Matrix3<f64>,
    pub positions: Vec<Vector3<f64>>,
    pub atomic_numbers: Vec<u8>,
    pub species: Vec<String>,
    pub comment: String,
    pub is_cartesian: bool,
}

impl Structure {
    /// Convert fractional positions to Cartesian.
    /// Cartesian = lattice^T * fractional (column convention).
    pub fn to_cartesian(&self) -> Vec<Vector3<f64>> {
        if self.is_cartesian {
            return self.positions.clone();
        }
        let lat = self.lattice.transpose();
        self.positions.iter().map(|frac| lat * frac).collect()
    }

    /// Convert Cartesian positions to fractional.
    pub fn to_fractional(&self) -> Vec<Vector3<f64>> {
        if !self.is_cartesian {
            return self.positions.clone();
        }
        let inv = self.lattice.transpose().try_inverse().expect("Singular lattice matrix");
        self.positions.iter().map(|cart| inv * cart).collect()
    }

    pub fn num_atoms(&self) -> usize {
        self.positions.len()
    }

    /// Create a supercell by repeating the structure na x nb x nc times.
    pub fn supercell(&self, na: usize, nb: usize, nc: usize) -> Structure {
        let frac = self.to_fractional();
        let n_orig = self.num_atoms();
        let n_total = n_orig * na * nb * nc;

        let mut new_positions = Vec::with_capacity(n_total);
        let mut new_atomic_numbers = Vec::with_capacity(n_total);
        let mut new_species = Vec::with_capacity(n_total);

        for ia in 0..na {
            for ib in 0..nb {
                for ic in 0..nc {
                    let offset = Vector3::new(ia as f64, ib as f64, ic as f64);
                    for i in 0..n_orig {
                        let scaled = Vector3::new(
                            (frac[i].x + offset.x) / na as f64,
                            (frac[i].y + offset.y) / nb as f64,
                            (frac[i].z + offset.z) / nc as f64,
                        );
                        new_positions.push(scaled);
                        new_atomic_numbers.push(self.atomic_numbers[i]);
                        new_species.push(self.species[i].clone());
                    }
                }
            }
        }

        // Scale lattice vectors
        let mut new_lattice = self.lattice;
        for j in 0..3 {
            new_lattice[(0, j)] *= na as f64;
        }
        for j in 0..3 {
            new_lattice[(1, j)] *= nb as f64;
        }
        for j in 0..3 {
            new_lattice[(2, j)] *= nc as f64;
        }

        Structure {
            lattice: new_lattice,
            positions: new_positions,
            atomic_numbers: new_atomic_numbers,
            species: new_species,
            comment: format!("{} ({}x{}x{} supercell)", self.comment, na, nb, nc),
            is_cartesian: false,
        }
    }
}

pub fn symbol_to_z(symbol: &str) -> u8 {
    match symbol.trim() {
        "H" => 1, "He" => 2, "Li" => 3, "Be" => 4, "B" => 5, "C" => 6,
        "N" => 7, "O" => 8, "F" => 9, "Ne" => 10, "Na" => 11, "Mg" => 12,
        "Al" => 13, "Si" => 14, "P" => 15, "S" => 16, "Cl" => 17, "Ar" => 18,
        "K" => 19, "Ca" => 20, "Sc" => 21, "Ti" => 22, "V" => 23, "Cr" => 24,
        "Mn" => 25, "Fe" => 26, "Co" => 27, "Ni" => 28, "Cu" => 29, "Zn" => 30,
        "Ga" => 31, "Ge" => 32, "As" => 33, "Se" => 34, "Br" => 35, "Kr" => 36,
        "Rb" => 37, "Sr" => 38, "Y" => 39, "Zr" => 40, "Nb" => 41, "Mo" => 42,
        "Tc" => 43, "Ru" => 44, "Rh" => 45, "Pd" => 46, "Ag" => 47, "Cd" => 48,
        "In" => 49, "Sn" => 50, "Sb" => 51, "Te" => 52, "I" => 53, "Xe" => 54,
        "Cs" => 55, "Ba" => 56, "La" => 57, "Ce" => 58, "Pr" => 59, "Nd" => 60,
        "Pm" => 61, "Sm" => 62, "Eu" => 63, "Gd" => 64, "Tb" => 65, "Dy" => 66,
        "Ho" => 67, "Er" => 68, "Tm" => 69, "Yb" => 70, "Lu" => 71, "Hf" => 72,
        "Ta" => 73, "W" => 74, "Re" => 75, "Os" => 76, "Ir" => 77, "Pt" => 78,
        "Au" => 79, "Hg" => 80, "Tl" => 81, "Pb" => 82, "Bi" => 83, "Po" => 84,
        "At" => 85, "Rn" => 86, "Fr" => 87, "Ra" => 88, "Ac" => 89, "Th" => 90,
        "Pa" => 91, "U" => 92, "Np" => 93, "Pu" => 94,
        _ => 0,
    }
}
