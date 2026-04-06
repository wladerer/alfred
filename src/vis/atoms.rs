use bevy::prelude::*;
use crate::data::{ElementData, Structure};
use nalgebra::Vector3;

#[derive(Component)]
pub struct AtomMarker {
    pub index: usize,
    pub atomic_number: u8,
}

/// Spawn spheres for each atom in the structure.
/// If `periodic_images` is true, atoms near fractional 0 are duplicated
/// at the opposite face/edge/corner to fill the unit cell visually.
pub fn spawn_structure(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    elements: &ElementData,
    structure: &Structure,
    periodic_images: bool,
) {
    let frac_positions = structure.to_fractional();
    let lat_t = structure.lattice.transpose();
    let sphere = meshes.add(Sphere::new(0.4).mesh().ico(2).unwrap());

    let tol = 0.02; // fractional tolerance for boundary detection

    for (i, frac) in frac_positions.iter().enumerate() {
        let z = structure.atomic_numbers[i];
        let props = elements.by_z(z);
        let material = materials.add(StandardMaterial {
            base_color: props.color,
            perceptual_roughness: 0.6,
            metallic: 0.1,
            ..default()
        });

        // Generate all periodic images for this atom
        let images = if periodic_images {
            periodic_copies(frac, tol)
        } else {
            vec![*frac]
        };

        for image_frac in &images {
            let cart = lat_t * image_frac;
            commands.spawn((
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(Vec3::new(cart.x as f32, cart.y as f32, cart.z as f32))
                    .with_scale(Vec3::splat(props.radius)),
                AtomMarker { index: i, atomic_number: z },
            ));
        }
    }
}

/// Generate periodic copies of a fractional position.
/// If a coordinate is near 0, add a copy with that coordinate at 1.
/// This handles faces (1 coord near 0 → 1 extra), edges (2 → 3 extra),
/// and corners (3 → 7 extra).
fn periodic_copies(frac: &Vector3<f64>, tol: f64) -> Vec<Vector3<f64>> {
    // Wrap into [0, 1)
    let wrapped = Vector3::new(
        frac.x.rem_euclid(1.0),
        frac.y.rem_euclid(1.0),
        frac.z.rem_euclid(1.0),
    );

    // For each axis, determine offsets: just [0] if interior, [0, 1] if on boundary
    let offsets_x: Vec<f64> = if wrapped.x < tol { vec![0.0, 1.0] } else { vec![0.0] };
    let offsets_y: Vec<f64> = if wrapped.y < tol { vec![0.0, 1.0] } else { vec![0.0] };
    let offsets_z: Vec<f64> = if wrapped.z < tol { vec![0.0, 1.0] } else { vec![0.0] };

    let mut copies = Vec::new();
    for &dx in &offsets_x {
        for &dy in &offsets_y {
            for &dz in &offsets_z {
                copies.push(Vector3::new(wrapped.x + dx, wrapped.y + dy, wrapped.z + dz));
            }
        }
    }
    copies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_periodic_copies_interior() {
        let frac = Vector3::new(0.25, 0.5, 0.75);
        let copies = periodic_copies(&frac, 0.02);
        assert_eq!(copies.len(), 1);
    }

    #[test]
    fn test_periodic_copies_face() {
        // Atom on the x=0 face → 2 copies
        let frac = Vector3::new(0.0, 0.5, 0.5);
        let copies = periodic_copies(&frac, 0.02);
        assert_eq!(copies.len(), 2);
    }

    #[test]
    fn test_periodic_copies_edge() {
        // Atom on x=0, y=0 edge → 4 copies
        let frac = Vector3::new(0.0, 0.0, 0.5);
        let copies = periodic_copies(&frac, 0.02);
        assert_eq!(copies.len(), 4);
    }

    #[test]
    fn test_periodic_copies_corner() {
        // Atom at origin → 8 copies (all corners)
        let frac = Vector3::new(0.0, 0.0, 0.0);
        let copies = periodic_copies(&frac, 0.02);
        assert_eq!(copies.len(), 8);
    }
}
