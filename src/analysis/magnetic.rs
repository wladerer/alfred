use crate::io::vasprun::types::{Dos, PartialDos};

/// Compute per-atom magnetic moments from partial DOS by integrating
/// spin-up minus spin-down up to E_Fermi.
/// Returns a Vec of [0, 0, moment_z] per atom (collinear along z).
pub fn moments_from_pdos(dos: &Dos) -> Option<Vec<[f64; 3]>> {
    let pdos = dos.partial.as_ref()?;
    let nspins = pdos.data.shape()[0];
    if nspins < 2 {
        return None; // Non-magnetic
    }

    let nions = pdos.data.shape()[1];
    let norbitals = pdos.data.shape()[2];
    let nedos = pdos.data.shape()[3];

    let energies = &dos.total.energies;
    let efermi = dos.efermi;

    let mut moments = Vec::with_capacity(nions);

    for ion in 0..nions {
        let mut moment = 0.0;

        for ie in 1..nedos {
            if energies[ie] > efermi {
                break;
            }
            // Trapezoidal integration of (spin_up - spin_down)
            let de = energies[ie] - energies[ie - 1];
            for orb in 0..norbitals {
                let up_prev = pdos.data[[0, ion, orb, ie - 1]];
                let up_curr = pdos.data[[0, ion, orb, ie]];
                let dn_prev = pdos.data[[1, ion, orb, ie - 1]];
                let dn_curr = pdos.data[[1, ion, orb, ie]];

                let diff_prev = up_prev - dn_prev;
                let diff_curr = up_curr - dn_curr;
                moment += 0.5 * (diff_prev + diff_curr) * de;
            }
        }

        moments.push([0.0, 0.0, moment]); // Collinear: along z
    }

    Some(moments)
}
