use bevy::prelude::*;
use crate::data::Structure;

/// Marker for unit cell outline entities.
#[derive(Component)]
pub struct UnitCellOutline;

/// Spawn 12 thin cylinders forming the edges of the unit cell parallelepiped.
pub fn spawn_unit_cell(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    structure: &Structure,
) {
    let lat = &structure.lattice;
    let a = Vec3::new(lat[(0, 0)] as f32, lat[(0, 1)] as f32, lat[(0, 2)] as f32);
    let b = Vec3::new(lat[(1, 0)] as f32, lat[(1, 1)] as f32, lat[(1, 2)] as f32);
    let c = Vec3::new(lat[(2, 0)] as f32, lat[(2, 1)] as f32, lat[(2, 2)] as f32);

    let o = Vec3::ZERO;

    // 8 corners of the parallelepiped
    let corners = [
        o,         a,         b,         c,
        a + b,     a + c,     b + c,     a + b + c,
    ];

    // 12 edges: pairs of corner indices
    let edges: [(usize, usize); 12] = [
        (0, 1), (0, 2), (0, 3),   // from origin
        (1, 4), (1, 5),           // from a
        (2, 4), (2, 6),           // from b
        (3, 5), (3, 6),           // from c
        (4, 7), (5, 7), (6, 7),  // to a+b+c
    ];

    let edge_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.8, 0.8, 0.8, 0.6),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    for (i0, i1) in edges {
        let start = corners[i0];
        let end = corners[i1];
        let mid = (start + end) / 2.0;
        let diff = end - start;
        let length = diff.length();

        if length < 1e-6 {
            continue;
        }

        let dir = diff / length;
        let rotation = Quat::from_rotation_arc(Vec3::Y, dir);
        let cylinder = meshes.add(Cylinder::new(0.02, length));

        commands.spawn((
            Mesh3d(cylinder),
            MeshMaterial3d(edge_material.clone()),
            Transform::from_translation(mid).with_rotation(rotation),
            UnitCellOutline,
        ));
    }
}

/// Despawn all unit cell outline entities.
pub fn despawn_unit_cell(commands: &mut Commands, query: &Query<Entity, With<UnitCellOutline>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
