use bevy::prelude::*;
use bevy::render::view::RenderLayers;

/// Render layer for the axes gizmo (separate from the main scene).
pub const GIZMO_LAYER: usize = 1;

/// Marker for the gizmo camera.
#[derive(Component)]
pub struct GizmoCamera;

/// Marker for the gizmo axis meshes.
#[derive(Component)]
pub struct GizmoAxis;

/// Spawn the axes gizmo: a small camera in the bottom-left corner
/// with three colored arrows for X (red), Y (green), Z (blue).
pub fn setup_axes_gizmo(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let layer = RenderLayers::layer(GIZMO_LAYER);

    // Gizmo camera — small viewport in bottom-left, orthographic so arrows stay constant size
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 1, // render after main camera
            clear_color: ClearColorConfig::None,
            viewport: Some(bevy::render::camera::Viewport {
                physical_position: UVec2::new(10, 10),
                physical_size: UVec2::new(120, 120),
                ..default()
            }),
            ..default()
        },
        Projection::from(OrthographicProjection {
            scaling_mode: bevy::render::camera::ScalingMode::Fixed {
                width: 3.0,
                height: 3.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        GizmoCamera,
        layer.clone(),
    ));

    // Shared geometries
    let shaft = meshes.add(Cylinder::new(0.04, 1.0));
    let tip = meshes.add(Sphere::new(0.08).mesh().ico(1).unwrap());

    let red = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.2, 0.2), unlit: true, ..default() });
    let green = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.9, 0.2), unlit: true, ..default() });
    let blue = materials.add(StandardMaterial { base_color: Color::srgb(0.3, 0.4, 1.0), unlit: true, ..default() });

    // X axis (red) — default cylinder is along Y, rotate to X
    let rot_x = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    commands.spawn((
        Mesh3d(shaft.clone()), MeshMaterial3d(red.clone()),
        Transform::from_translation(Vec3::new(0.5, 0.0, 0.0)).with_rotation(rot_x),
        GizmoAxis, layer.clone(),
    ));
    commands.spawn((
        Mesh3d(tip.clone()), MeshMaterial3d(red),
        Transform::from_translation(Vec3::new(1.1, 0.0, 0.0)).with_rotation(rot_x),
        GizmoAxis, layer.clone(),
    ));

    // Y axis (green) — no rotation needed, cylinder defaults to Y
    commands.spawn((
        Mesh3d(shaft.clone()), MeshMaterial3d(green.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        GizmoAxis, layer.clone(),
    ));
    commands.spawn((
        Mesh3d(tip.clone()), MeshMaterial3d(green),
        Transform::from_translation(Vec3::new(0.0, 1.1, 0.0)),
        GizmoAxis, layer.clone(),
    ));

    // Z axis (blue) — rotate to Z
    let rot_z = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    commands.spawn((
        Mesh3d(shaft), MeshMaterial3d(blue.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.5)).with_rotation(rot_z),
        GizmoAxis, layer.clone(),
    ));
    commands.spawn((
        Mesh3d(tip), MeshMaterial3d(blue),
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.1)).with_rotation(rot_z),
        GizmoAxis, layer.clone(),
    ));

    // Ambient light for the gizmo layer
    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
        layer,
    ));
}

/// Sync the gizmo camera's rotation with the main camera so the axes
/// reflect the current viewing orientation.
pub fn sync_gizmo_camera(
    main_cam: Query<&Transform, (With<Camera3d>, Without<GizmoCamera>)>,
    mut gizmo_cam: Query<&mut Transform, With<GizmoCamera>>,
) {
    let main_transform = main_cam.single();
    let mut gizmo_transform = gizmo_cam.single_mut();

    // Copy the main camera's rotation but keep the gizmo camera at a fixed distance
    gizmo_transform.rotation = main_transform.rotation;
    // Position the gizmo camera looking at origin from the same direction
    let back = gizmo_transform.rotation * Vec3::new(0.0, 0.0, 3.5);
    gizmo_transform.translation = back;
}
