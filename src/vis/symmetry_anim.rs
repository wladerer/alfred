use bevy::prelude::*;

use crate::camera::CameraState;
use crate::vis::atoms::AtomMarker;

/// Event to request a symmetry animation from the UI.
#[derive(Event, Clone)]
pub enum AnimateSymmetry {
    Rotation { axis: Vec3, fold: u8 },
    Reflection { normal: Vec3 },
    Inversion,
}

/// Describes which symmetry operation to animate.
#[derive(Clone)]
pub enum SymmetryOp {
    /// Rotate around `axis` by `angle` radians, centered on `pivot`.
    Rotation { axis: Vec3, angle: f32, pivot: Vec3 },
    /// Reflect through the plane with `normal`, passing through `pivot`.
    Reflection { normal: Vec3, pivot: Vec3 },
    /// Invert through `pivot` (pos → 2*pivot - pos).
    Inversion { pivot: Vec3 },
}

/// Resource tracking an in-progress symmetry animation.
#[derive(Resource)]
pub struct SymmetryAnimation {
    pub op: SymmetryOp,
    /// Per-atom start positions (world space).
    pub start_positions: Vec<(Entity, Vec3)>,
    /// Progress 0.0 -> 1.0 (forward), then 1.0 -> 0.0 (return).
    pub t: f32,
    /// True while returning to start.
    pub returning: bool,
    /// Animation speed (full cycle in seconds).
    pub duration: f32,
    /// Pause at the midpoint (fully transformed) in seconds.
    pub hold: f32,
    pub hold_elapsed: f32,
}

impl SymmetryAnimation {
    pub fn new(op: SymmetryOp, start_positions: Vec<(Entity, Vec3)>) -> Self {
        Self {
            op,
            start_positions,
            t: 0.0,
            returning: false,
            duration: 0.6,
            hold: 0.3,
            hold_elapsed: 0.0,
        }
    }
}

/// Apply the symmetry operation at parameter `t` (0=identity, 1=full operation).
fn apply_op(op: &SymmetryOp, pos: Vec3, t: f32) -> Vec3 {
    match op {
        SymmetryOp::Rotation { axis, angle, pivot } => {
            let rel = pos - *pivot;
            let rot = Quat::from_axis_angle(*axis, *angle * t);
            *pivot + rot.mul_vec3(rel)
        }
        SymmetryOp::Reflection { normal, pivot } => {
            let rel = pos - *pivot;
            let d = rel.dot(*normal);
            let reflected = rel - 2.0 * d * *normal;
            *pivot + rel.lerp(reflected, t)
        }
        SymmetryOp::Inversion { pivot } => {
            let rel = pos - *pivot;
            let inverted = -rel;
            *pivot + rel.lerp(inverted, t)
        }
    }
}

/// Smooth ease-in-out curve.
fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// System: handle AnimateSymmetry events by starting a new animation.
pub fn handle_animate_symmetry(
    mut commands: Commands,
    mut events: EventReader<AnimateSymmetry>,
    camera_state: Res<CameraState>,
    atom_query: Query<(Entity, &Transform), With<AtomMarker>>,
    existing: Option<Res<SymmetryAnimation>>,
) {
    // Don't start a new animation while one is running
    if existing.is_some() {
        events.clear();
        return;
    }

    for event in events.read() {
        let pivot = camera_state.pivot;
        let start_positions: Vec<(Entity, Vec3)> = atom_query.iter()
            .map(|(e, t)| (e, t.translation))
            .collect();

        if start_positions.is_empty() {
            continue;
        }

        let op = match event {
            AnimateSymmetry::Rotation { axis, fold } => {
                let angle = std::f32::consts::TAU / *fold as f32;
                SymmetryOp::Rotation { axis: *axis, angle, pivot }
            }
            AnimateSymmetry::Reflection { normal } => {
                SymmetryOp::Reflection { normal: *normal, pivot }
            }
            AnimateSymmetry::Inversion => {
                SymmetryOp::Inversion { pivot }
            }
        };

        commands.insert_resource(SymmetryAnimation::new(op, start_positions));
        break; // Only start one animation per frame
    }
}

/// System: advance the symmetry animation each frame.
pub fn animate_symmetry(
    mut commands: Commands,
    time: Res<Time>,
    anim: Option<ResMut<SymmetryAnimation>>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(mut anim) = anim else { return };

    let dt = time.delta_secs();
    let speed = 1.0 / anim.duration;

    if !anim.returning {
        anim.t += dt * speed * 2.0;
        if anim.t >= 1.0 {
            anim.t = 1.0;
            anim.hold_elapsed += dt;
            if anim.hold_elapsed < anim.hold {
                // Hold at fully transformed position
                for (entity, start_pos) in &anim.start_positions {
                    if let Ok(mut transform) = transforms.get_mut(*entity) {
                        transform.translation = apply_op(&anim.op, *start_pos, 1.0);
                    }
                }
                return;
            }
            anim.returning = true;
        }
    } else {
        anim.t -= dt * speed * 2.0;
        if anim.t <= 0.0 {
            // Animation complete — snap back to exact start positions
            for (entity, start_pos) in &anim.start_positions {
                if let Ok(mut transform) = transforms.get_mut(*entity) {
                    transform.translation = *start_pos;
                }
            }
            commands.remove_resource::<SymmetryAnimation>();
            return;
        }
    }

    let eased = ease_in_out(anim.t);
    for (entity, start_pos) in &anim.start_positions {
        if let Ok(mut transform) = transforms.get_mut(*entity) {
            transform.translation = apply_op(&anim.op, *start_pos, eased);
        }
    }
}
