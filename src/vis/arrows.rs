use bevy::prelude::*;

/// Marker for force arrow entities.
#[derive(Component, Clone)]
pub struct ForceArrow;

/// Marker for magnetic moment arrow entities.
#[derive(Component, Clone)]
pub struct MagMomentArrow;

/// Spawn arrow glyphs (cylinder + cone tip) for per-atom vector data.
/// `vectors` is indexed by atom, each is a 3D vector in Cartesian coords.
/// `scale` controls the visual length scaling.
pub fn spawn_arrows<M: Component + Clone>(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    positions: &[Vec3],
    vectors: &[[f64; 3]],
    scale: f32,
    marker: M,
    color_fn: fn(f32) -> Color,
) {
    let shaft_mesh = meshes.add(Cylinder::new(0.03, 1.0));
    let tip_mesh = meshes.add(Sphere::new(0.06).mesh().ico(1).unwrap());

    for (i, pos) in positions.iter().enumerate() {
        if i >= vectors.len() {
            break;
        }
        let v = Vec3::new(vectors[i][0] as f32, vectors[i][1] as f32, vectors[i][2] as f32);
        let magnitude = v.length();
        if magnitude < 1e-6 {
            continue;
        }

        let dir = v / magnitude;
        let length = magnitude * scale;
        let color = color_fn(magnitude);

        let material = materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            ..default()
        });

        // Shaft: cylinder from atom position along the vector direction
        let shaft_center = *pos + dir * (length / 2.0);
        let rotation = Quat::from_rotation_arc(Vec3::Y, dir);

        commands.spawn((
            Mesh3d(shaft_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(shaft_center)
                .with_rotation(rotation)
                .with_scale(Vec3::new(1.0, length, 1.0)),
            marker.clone(),
        ));

        // Tip sphere at the end
        let tip_pos = *pos + dir * length;
        commands.spawn((
            Mesh3d(tip_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(tip_pos)
                .with_scale(Vec3::splat(1.5)),
            marker.clone(),
        ));
    }
}

pub fn despawn_by_marker<M: Component>(commands: &mut Commands, query: &Query<Entity, With<M>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Color by magnitude: blue (low) → red (high).
pub fn force_color(magnitude: f32) -> Color {
    let t = (magnitude / 2.0).min(1.0); // 2 eV/A = fully red
    Color::srgb(t, 0.2, 1.0 - t)
}

/// Color for magnetic moments: blue (negative/down) → white (zero) → red (positive/up).
pub fn mag_color(magnitude: f32) -> Color {
    let t = (magnitude / 3.0).min(1.0); // 3 µB = saturated
    Color::srgb(0.5 + 0.5 * t, 0.3, 0.5 + 0.5 * t)
}
