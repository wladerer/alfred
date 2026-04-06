use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use crate::analysis::symmetry::MirrorPlane;

/// Marker for mirror plane entities, with the index into the mirrors list.
#[derive(Component)]
pub struct MirrorPlaneVis(pub usize);

/// Color palette for mirror planes.
pub const MIRROR_COLORS: &[(f32, f32, f32)] = &[
    (0.3, 0.7, 1.0),   // light blue
    (1.0, 0.6, 0.3),   // light orange
    (0.5, 1.0, 0.5),   // light green
    (1.0, 0.5, 1.0),   // light pink
    (1.0, 1.0, 0.5),   // light yellow
    (0.5, 1.0, 1.0),   // light cyan
];

/// Spawn a single mirror plane quad at the given index.
pub fn spawn_single_mirror(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pivot: Vec3,
    mirror: &MirrorPlane,
    index: usize,
    extent: f32,
) {
    let size = extent * 1.2;
    let (r, g, b) = MIRROR_COLORS[index % MIRROR_COLORS.len()];
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(r, g, b, 0.15),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    let normal = mirror.normal;
    let arbitrary = if normal.y.abs() > 0.99 { Vec3::X } else { Vec3::Y };
    let u = normal.cross(arbitrary).normalize() * size;
    let v = normal.cross(u).normalize() * size;

    let corners = [
        pivot - u - v,
        pivot + u - v,
        pivot + u + v,
        pivot - u + v,
    ];

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, bevy::render::render_asset::RenderAssetUsages::default());
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            corners[0].to_array(),
            corners[1].to_array(),
            corners[2].to_array(),
            corners[3].to_array(),
        ],
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![
            normal.to_array(),
            normal.to_array(),
            normal.to_array(),
            normal.to_array(),
        ],
    );
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        MirrorPlaneVis(index),
    ));
}

/// Spawn all mirror planes.
pub fn spawn_mirror_planes(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pivot: Vec3,
    mirrors: &[MirrorPlane],
    extent: f32,
    visible_mask: &[bool],
) {
    for (i, mirror) in mirrors.iter().enumerate() {
        if i < visible_mask.len() && visible_mask[i] {
            spawn_single_mirror(commands, meshes, materials, pivot, mirror, i, extent);
        }
    }
}

/// Despawn all mirror plane entities.
pub fn despawn_mirror_planes(commands: &mut Commands, query: &Query<Entity, With<MirrorPlaneVis>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Despawn mirror planes with a specific index.
pub fn despawn_mirror_by_index(commands: &mut Commands, query: &Query<(Entity, &MirrorPlaneVis)>, index: usize) {
    for (entity, vis) in query.iter() {
        if vis.0 == index {
            commands.entity(entity).despawn();
        }
    }
}
