use bevy::prelude::*;
use crate::vis::atoms::AtomMarker;

/// Component added to the currently selected atom's highlight ring.
#[derive(Component)]
pub struct SelectionHighlight;

/// Resource tracking the selected atom.
#[derive(Resource, Default)]
pub struct SelectedAtom {
    pub index: Option<usize>,
    pub entity: Option<Entity>,
}

/// Cast a ray from the camera through the mouse position, return the closest atom hit.
pub fn pick_atom(
    camera_transform: &Transform,
    window: &Window,
    cursor_pos: Vec2,
    atoms: &[(Entity, &AtomMarker, &Transform)],
) -> Option<(Entity, usize)> {
    // Build ray from camera position through cursor
    let viewport_size = Vec2::new(window.width(), window.height());

    // Cursor to normalized coordinates [-1, 1]
    let ndc_x = (cursor_pos.x / viewport_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (cursor_pos.y / viewport_size.y) * 2.0; // flip y

    // Simple perspective ray: use camera's basis vectors
    let fov_half = std::f32::consts::FRAC_PI_4; // ~45 degree FOV
    let aspect = viewport_size.x / viewport_size.y;

    let forward: Vec3 = camera_transform.forward().into();
    let right: Vec3 = camera_transform.right().into();
    let up: Vec3 = camera_transform.up().into();

    let ray_dir = (forward + right * ndc_x * aspect * fov_half.tan() + up * ndc_y * fov_half.tan()).normalize();
    let ray_origin = camera_transform.translation;

    let mut best: Option<(Entity, usize, f32)> = None;

    for &(entity, marker, atom_transform) in atoms {
        let center = atom_transform.translation;
        let radius = atom_transform.scale.x * 0.4; // base sphere radius is 0.4

        // Ray-sphere intersection
        let oc = ray_origin - center;
        let b = oc.dot(ray_dir);
        let c = oc.dot(oc) - radius * radius;
        let discriminant = b * b - c;

        if discriminant >= 0.0 {
            let t = -b - discriminant.sqrt();
            if t > 0.0 {
                if best.is_none() || t < best.unwrap().2 {
                    best = Some((entity, marker.index, t));
                }
            }
        }
    }

    best.map(|(e, idx, _)| (e, idx))
}

/// Spawn a highlight ring around the selected atom.
pub fn spawn_selection_highlight(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    radius: f32,
) -> Entity {
    let ring = meshes.add(Sphere::new(0.42).mesh().ico(2).unwrap());
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 0.3, 0.4),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(ring),
        MeshMaterial3d(material),
        Transform::from_translation(position)
            .with_scale(Vec3::splat(radius * 1.3)),
        SelectionHighlight,
    )).id()
}

pub fn despawn_selection_highlight(commands: &mut Commands, query: &Query<Entity, With<SelectionHighlight>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
