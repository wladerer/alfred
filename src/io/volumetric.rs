use crate::data::{Structure, VolumeGrid};
use crate::io::poscar::parse_poscar_str;
use nalgebra::Matrix3;
use std::fs;
use std::path::Path;

/// Parse a VASP volumetric data file (CHGCAR, LOCPOT, wavefunction .vasp, etc.).
/// Format: POSCAR header + blank line + grid dims + volumetric data.
/// Returns both the structure and the volume grid.
pub fn parse_volumetric(path: &Path) -> Result<(Structure, VolumeGrid), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;
    parse_volumetric_str(&content)
}

pub fn parse_volumetric_str(content: &str) -> Result<(Structure, VolumeGrid), String> {
    let lines: Vec<&str> = content.lines().collect();

    // Find the grid dimension line after the atomic coordinates.
    // The POSCAR part ends after the positions block.
    // We need to find the line with 3 integers (grid dims).
    let (poscar_end, grid_dims) = find_grid_dims(&lines)?;

    // Parse the POSCAR header portion
    let poscar_text: String = lines[..poscar_end].join("\n");
    let structure = parse_poscar_str(&poscar_text)?;

    let nx = grid_dims[0];
    let ny = grid_dims[1];
    let nz = grid_dims[2];
    let total = nx * ny * nz;

    // Parse volumetric data — values are whitespace-separated floats after the grid line
    let mut data = Vec::with_capacity(total);
    for line in &lines[(poscar_end + 1)..] {
        // Stop if we hit another grid line (CHGCAR can have multiple datasets)
        if data.len() >= total {
            break;
        }
        for token in line.split_whitespace() {
            if data.len() >= total {
                break;
            }
            match token.parse::<f64>() {
                Ok(v) => data.push(v),
                Err(_) => {
                    // Could be augmentation data or another section — stop here
                    if data.len() > 0 {
                        break;
                    }
                }
            }
        }
        if data.len() >= total {
            break;
        }
    }

    if data.len() < total {
        return Err(format!(
            "Expected {} grid values ({}x{}x{}), got {}",
            total, nx, ny, nz, data.len()
        ));
    }

    let lattice = structure.lattice;

    Ok((
        structure,
        VolumeGrid {
            lattice,
            dims: [nx, ny, nz],
            data,
        },
    ))
}

/// Scan lines to find the grid dimension line (3 integers on a line).
/// Returns (line_index_of_grid_dims, [nx, ny, nz]).
fn find_grid_dims(lines: &[&str]) -> Result<(usize, [usize; 3]), String> {
    // Skip the first 8 lines minimum (POSCAR header), then look for a line
    // with exactly 3 integers (possibly after blank lines).
    for (i, line) in lines.iter().enumerate().skip(8) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.len() == 3 {
            if let (Ok(a), Ok(b), Ok(c)) = (
                tokens[0].parse::<usize>(),
                tokens[1].parse::<usize>(),
                tokens[2].parse::<usize>(),
            ) {
                if a > 1 && b > 1 && c > 1 {
                    return Ok((i, [a, b, c]));
                }
            }
        }
    }
    Err("Could not find grid dimensions line".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_small_volumetric() {
        let content = "\
test
1.0
3.0 0.0 0.0
0.0 3.0 0.0
0.0 0.0 3.0
Si
1
Direct
0.0 0.0 0.0

   2   2   2
 1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0";

        let (structure, grid) = parse_volumetric_str(content).unwrap();
        assert_eq!(structure.num_atoms(), 1);
        assert_eq!(grid.dims, [2, 2, 2]);
        assert_eq!(grid.data.len(), 8);
        assert!((grid.get(0, 0, 0) - 1.0).abs() < 1e-10);
        assert!((grid.get(1, 0, 0) - 2.0).abs() < 1e-10);
        assert!((grid.get(0, 1, 0) - 3.0).abs() < 1e-10);
    }
}
