use bevy::prelude::*;

/// Marker for the symmetry axis visual indicator entities.
#[derive(Component)]
pub struct AxisIndicator;

/// Marker for the axis label text.
#[derive(Component)]
pub struct AxisLabel;

/// Despawn any existing axis indicator entities.
pub fn despawn_axis_indicator(commands: &mut Commands, query: &Query<Entity, With<AxisIndicator>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Spawn a cylinder + cones along the given axis direction through pivot, with a label.
pub fn spawn_axis_indicator(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pivot: Vec3,
    direction: Vec3,
    label: &str,
    extent: f32,
) {
    let half_len = extent;
    let start = pivot - direction * half_len;
    let end = pivot + direction * half_len;
    let midpoint = (start + end) / 2.0;

    // Axis line as a thin cylinder
    let cylinder = meshes.add(Cylinder::new(0.03, half_len * 2.0));
    let axis_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.85, 0.0, 0.7),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // Cylinder defaults to Y-axis aligned, so we need to rotate it to match `direction`
    let rotation = Quat::from_rotation_arc(Vec3::Y, direction);

    commands.spawn((
        Mesh3d(cylinder),
        MeshMaterial3d(axis_material.clone()),
        Transform::from_translation(midpoint).with_rotation(rotation),
        AxisIndicator,
    ));

    // Small cone at the positive end
    let cone = meshes.add(Sphere::new(0.1).mesh().ico(1).unwrap());
    commands.spawn((
        Mesh3d(cone),
        MeshMaterial3d(axis_material.clone()),
        Transform::from_translation(end + direction * 0.15).with_rotation(rotation),
        AxisIndicator,
    ));

    // Label billboard — spawn a small sphere as a marker at the tip, with the label in console
    // (True 3D text requires bevy's Text2d or a font atlas; for now print to console)
    println!("  Axis: {label}");

    // Small marker sphere at the tip
    let marker = meshes.add(Sphere::new(0.08).mesh().ico(1).unwrap());
    let marker_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.85, 0.0, 0.9),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.spawn((
        Mesh3d(marker),
        MeshMaterial3d(marker_material),
        Transform::from_translation(end + direction * 0.4),
        AxisIndicator,
    ));
}
