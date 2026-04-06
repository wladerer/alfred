use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use crate::analysis::marching_cubes::{self, IsosurfaceMesh};
use crate::data::VolumeGrid;

/// Marker for isosurface mesh entities.
#[derive(Component)]
pub struct IsosurfaceEntity;

/// Spawn an isosurface mesh for the given volume grid and isovalue.
/// For wavefunctions (has negative values), renders both +level and -level
/// in different colors.
pub fn spawn_isosurface(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    grid: &VolumeGrid,
    level: f64,
    opacity: f32,
) {
    let has_negative = grid.min() < 0.0;

    // Positive isosurface
    let iso_pos = marching_cubes::marching_cubes(grid, level);
    if !iso_pos.vertices.is_empty() {
        let mesh = build_bevy_mesh(&iso_pos);
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.2, 0.5, 1.0, opacity),
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            perceptual_roughness: 0.8,
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            IsosurfaceEntity,
        ));
    }

    // Negative isosurface (for wavefunctions)
    if has_negative && level > 0.0 {
        let iso_neg = marching_cubes::marching_cubes(grid, -level);
        if !iso_neg.vertices.is_empty() {
            let mesh = build_bevy_mesh(&iso_neg);
            let material = materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.3, 0.2, opacity),
                alpha_mode: AlphaMode::Blend,
                double_sided: true,
                cull_mode: None,
                perceptual_roughness: 0.8,
                ..default()
            });
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::IDENTITY,
                IsosurfaceEntity,
            ));
        }
    }
}

fn build_bevy_mesh(iso: &IsosurfaceMesh) -> Mesh {
    let positions: Vec<[f32; 3]> = iso.vertices.iter().map(|v| v.to_array()).collect();
    let normals: Vec<[f32; 3]> = iso.normals.iter().map(|n| n.to_array()).collect();

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(iso.indices.clone()));
    mesh
}

pub fn despawn_isosurface(commands: &mut Commands, query: &Query<Entity, With<IsosurfaceEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
