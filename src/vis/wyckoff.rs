use bevy::prelude::*;
use crate::analysis::symmetry::WyckoffSite;
use crate::data::Structure;

/// Marker for Wyckoff highlight entities.
#[derive(Component)]
pub struct WyckoffHighlight;

/// Color palette for Wyckoff sites (distinct, translucent).
pub const WYCKOFF_COLORS: &[(f32, f32, f32)] = &[
    (0.2, 0.6, 1.0),   // blue
    (1.0, 0.4, 0.2),   // orange
    (0.2, 0.9, 0.4),   // green
    (0.9, 0.2, 0.8),   // magenta
    (1.0, 0.9, 0.2),   // yellow
    (0.4, 0.9, 0.9),   // cyan
    (0.9, 0.5, 0.5),   // salmon
    (0.6, 0.4, 1.0),   // purple
];

/// Spawn translucent orbs around atoms at each Wyckoff site.
pub fn spawn_wyckoff_highlights(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    structure: &Structure,
    wyckoff_sites: &[WyckoffSite],
) {
    let cart_positions = structure.to_cartesian();

    for (site_idx, site) in wyckoff_sites.iter().enumerate() {
        let (r, g, b) = WYCKOFF_COLORS[site_idx % WYCKOFF_COLORS.len()];
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, 0.25),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });

        // Slightly larger than the atom sphere
        let sphere = meshes.add(Sphere::new(0.6).mesh().ico(2).unwrap());

        for &atom_idx in &site.atom_indices {
            if atom_idx >= cart_positions.len() {
                continue;
            }
            let pos = &cart_positions[atom_idx];
            commands.spawn((
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32))
                    .with_scale(Vec3::splat(1.8)),
                WyckoffHighlight,
            ));
        }
    }
}

pub fn despawn_wyckoff_highlights(commands: &mut Commands, query: &Query<Entity, With<WyckoffHighlight>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
