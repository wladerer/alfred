use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::analysis::SymmetryAxes;
use crate::scene::{LoadedStructure, StructureExtent};
use crate::vis;

#[derive(Resource)]
pub struct CameraState {
    pub pivot: Vec3,
    pub locked_axis: Option<Vec3>,
    /// Pan step size for arrow key panning (Angstroms).
    pub pan_step: f32,
    /// Fine rotation step for IJKL keys (degrees).
    pub rotation_step: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            pivot: Vec3::ZERO,
            locked_axis: None,
            pan_step: 0.1,
            rotation_step: 5.0,
        }
    }
}

/// Main camera query filter: excludes the gizmo camera.
type MainCamera = (With<Camera3d>, Without<vis::axes_gizmo::GizmoCamera>);

pub fn orbit_camera(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut scroll: EventReader<bevy::input::mouse::MouseWheel>,
    mut query: Query<&mut Transform, MainCamera>,
    camera_state: Res<CameraState>,
    mut contexts: EguiContexts,
) {
    let egui_wants_input = contexts.ctx_mut().wants_pointer_input();
    let mut transform = query.single_mut();
    let pivot = camera_state.pivot;

    if mouse.pressed(MouseButton::Left) && !egui_wants_input {
        for ev in motion.read() {
            if let Some(axis) = camera_state.locked_axis {
                let angle = -ev.delta.x * 0.005;
                let rotation = Quat::from_axis_angle(axis, angle);
                let arm = transform.translation - pivot;
                transform.translation = pivot + rotation.mul_vec3(arm);
                transform.rotation = rotation * transform.rotation;
            } else {
                let yaw = Quat::from_rotation_y(-ev.delta.x * 0.005);
                let right = transform.right();
                let pitch = Quat::from_axis_angle(right.into(), -ev.delta.y * 0.005);
                let rotation = yaw * pitch;
                let arm = transform.translation - pivot;
                transform.translation = pivot + rotation.mul_vec3(arm);
                transform.rotation = rotation * transform.rotation;
            }
        }
    } else if (mouse.pressed(MouseButton::Right) || mouse.pressed(MouseButton::Middle))
        && !egui_wants_input
    {
        for ev in motion.read() {
            let right: Vec3 = transform.right().into();
            let up: Vec3 = transform.up().into();
            let dist = (transform.translation - pivot).length();
            let speed = dist * 0.002;
            transform.translation += (-ev.delta.x * speed) * right + (ev.delta.y * speed) * up;
        }
    } else {
        motion.clear();
    }

    if !egui_wants_input {
        for ev in scroll.read() {
            let arm = transform.translation - pivot;
            let dist = arm.length();
            let factor = 1.0 - ev.y * 0.1;
            let new_dist = (dist * factor).max(1.0);
            transform.translation = pivot + arm.normalize() * new_dist;
        }
    } else {
        scroll.clear();
    }
}

/// X/Y/Z keys snap the camera to view along that axis.
/// Spacebar resets to the default diagonal view.
pub fn axis_view_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    camera_state: Res<CameraState>,
    mut query: Query<&mut Transform, MainCamera>,
) {
    if keys.just_pressed(KeyCode::Space) {
        let mut transform = query.single_mut();
        let pivot = camera_state.pivot;
        let dist = (transform.translation - pivot).length();
        let offset = Vec3::new(1.0, 1.0, 1.0).normalize() * dist;
        transform.translation = pivot + offset;
        transform.look_at(pivot, Vec3::Y);
        return;
    }

    let dir = if keys.just_pressed(KeyCode::KeyX) {
        Some(Vec3::X)
    } else if keys.just_pressed(KeyCode::KeyY) {
        Some(Vec3::Y)
    } else if keys.just_pressed(KeyCode::KeyZ) {
        Some(Vec3::Z)
    } else {
        None
    };

    let Some(view_dir) = dir else { return };

    let mut transform = query.single_mut();
    let pivot = camera_state.pivot;
    let dist = (transform.translation - pivot).length();

    transform.translation = pivot + view_dir * dist;
    let up = if view_dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let up = (up - view_dir * view_dir.dot(up)).normalize();
    transform.look_at(pivot, up);
}

/// N: cycle symmetry axis. Shift+N: lock/unlock rotation to current axis.
pub fn cycle_symmetry_axis(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut sym_axes: ResMut<SymmetryAxes>,
    mut camera_state: ResMut<CameraState>,
    mut cam_query: Query<&mut Transform, MainCamera>,
    indicator_query: Query<Entity, With<vis::AxisIndicator>>,
    extent: Res<StructureExtent>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    if !keys.just_pressed(KeyCode::KeyN) {
        return;
    }

    if shift {
        if camera_state.locked_axis.is_some() {
            camera_state.locked_axis = None;
            vis::despawn_axis_indicator(&mut commands, &indicator_query);
            println!("Rotation unlocked");
        } else if let Some(axis) = sym_axes.current() {
            let dir = axis.direction;
            let label = axis.label.clone();
            camera_state.locked_axis = Some(dir);

            vis::despawn_axis_indicator(&mut commands, &indicator_query);
            vis::spawn_axis_indicator(
                &mut commands, &mut meshes, &mut materials,
                camera_state.pivot, dir, &label, extent.0,
            );
            println!("Rotation locked to {label} axis ({:.3}, {:.3}, {:.3})", dir.x, dir.y, dir.z);
        } else {
            println!("No symmetry axis to lock to");
        }
        return;
    }

    let Some(axis) = sym_axes.next() else {
        println!("No symmetry axes detected");
        return;
    };

    let label = axis.label.clone();
    let dir = axis.direction;

    vis::despawn_axis_indicator(&mut commands, &indicator_query);
    vis::spawn_axis_indicator(
        &mut commands, &mut meshes, &mut materials,
        camera_state.pivot, dir, &label, extent.0,
    );

    println!("Viewing along {label} axis: ({:.3}, {:.3}, {:.3})", dir.x, dir.y, dir.z);

    let mut transform = cam_query.single_mut();
    let pivot = camera_state.pivot;
    let dist = (transform.translation - pivot).length();
    transform.translation = pivot + dir * dist;

    let up = if dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let up = (up - dir * dir.dot(up)).normalize();
    transform.look_at(pivot, up);
}

/// Arrow keys: pan the camera in the view plane.
/// Inverted: arrow direction moves the scene (camera moves opposite).
pub fn arrow_pan_system(
    keys: Res<ButtonInput<KeyCode>>,
    camera_state: Res<CameraState>,
    mut query: Query<&mut Transform, MainCamera>,
    mut contexts: EguiContexts,
) {
    if contexts.ctx_mut().wants_keyboard_input() {
        return;
    }

    let mut delta = Vec2::ZERO;
    if keys.pressed(KeyCode::ArrowLeft)  { delta.x += 1.0; }
    if keys.pressed(KeyCode::ArrowRight) { delta.x -= 1.0; }
    if keys.pressed(KeyCode::ArrowUp)    { delta.y -= 1.0; }
    if keys.pressed(KeyCode::ArrowDown)  { delta.y += 1.0; }

    if delta == Vec2::ZERO {
        return;
    }

    let mut transform = query.single_mut();
    let right: Vec3 = transform.right().into();
    let up: Vec3 = transform.up().into();
    let step = camera_state.pan_step;
    transform.translation += right * delta.x * step + up * delta.y * step;
}

/// IJKL keys: fine rotation (d-pad style).
/// I/K = pitch up/down, J/L = yaw left/right.
pub fn fine_rotation_system(
    keys: Res<ButtonInput<KeyCode>>,
    camera_state: Res<CameraState>,
    mut query: Query<&mut Transform, MainCamera>,
    mut contexts: EguiContexts,
) {
    if contexts.ctx_mut().wants_keyboard_input() {
        return;
    }

    let step = camera_state.rotation_step.to_radians();
    let mut yaw = 0.0f32;
    let mut pitch = 0.0f32;

    if keys.just_pressed(KeyCode::KeyJ) { yaw += step; }
    if keys.just_pressed(KeyCode::KeyL) { yaw -= step; }
    if keys.just_pressed(KeyCode::KeyI) { pitch += step; }
    if keys.just_pressed(KeyCode::KeyK) { pitch -= step; }

    if yaw == 0.0 && pitch == 0.0 {
        return;
    }

    let mut transform = query.single_mut();
    let pivot = camera_state.pivot;
    let arm = transform.translation - pivot;

    let yaw_rot = Quat::from_rotation_y(yaw);
    let right: Vec3 = transform.right().into();
    let pitch_rot = Quat::from_axis_angle(right, pitch);
    let rotation = yaw_rot * pitch_rot;

    transform.translation = pivot + rotation.mul_vec3(arm);
    transform.rotation = rotation * transform.rotation;
}

/// Fixed bottom panel with view buttons (a/b/c lattice directions),
/// pan step, rotation step, and keyboard hint.
pub fn view_panel_system(
    mut contexts: EguiContexts,
    mut camera_state: ResMut<CameraState>,
    mut cam_query: Query<&mut Transform, MainCamera>,
    loaded: Res<LoadedStructure>,
) {
    let ctx = contexts.ctx_mut();

    egui::TopBottomPanel::bottom("view_controls")
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("View:");
                let axes = [("a", Vec3::X), ("b", Vec3::Y), ("c", Vec3::Z)];
                for (label, dir) in axes {
                    if ui.button(label).clicked() {
                        let mut transform = cam_query.single_mut();
                        let pivot = camera_state.pivot;
                        let dist = (transform.translation - pivot).length();

                        let view_dir = if let Some(ref structure) = loaded.0 {
                            let lat = &structure.lattice;
                            let idx = if dir == Vec3::X { 0 } else if dir == Vec3::Y { 1 } else { 2 };
                            let v = Vec3::new(
                                lat[(idx, 0)] as f32,
                                lat[(idx, 1)] as f32,
                                lat[(idx, 2)] as f32,
                            );
                            v.normalize()
                        } else {
                            dir
                        };

                        transform.translation = pivot + view_dir * dist;
                        let up = if view_dir.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
                        let up = (up - view_dir * view_dir.dot(up)).normalize();
                        transform.look_at(pivot, up);
                    }
                }

                ui.separator();
                ui.label("Pan:");
                ui.add(egui::DragValue::new(&mut camera_state.pan_step)
                    .range(0.01..=2.0)
                    .speed(0.01)
                    .suffix(" Å"));

                ui.separator();
                ui.label("Rot:");
                ui.add(egui::DragValue::new(&mut camera_state.rotation_step)
                    .range(1.0..=45.0)
                    .speed(0.5)
                    .suffix("°"));

                ui.separator();
                ui.label(
                    egui::RichText::new("Arrows: pan  |  IJKL: rotate  |  Space: reset")
                        .small()
                        .color(egui::Color32::GRAY)
                );
            });
        });
}

pub fn update_gizmo_viewport(
    windows: Query<&Window>,
    mut gizmo_cam: Query<&mut Camera, With<vis::axes_gizmo::GizmoCamera>>,
) {
    let window = windows.single();
    let mut camera = gizmo_cam.single_mut();

    let height = window.physical_height();
    let size = 140u32;
    let margin = 10u32;

    camera.viewport = Some(bevy::render::camera::Viewport {
        physical_position: UVec2::new(margin, height.saturating_sub(size + margin)),
        physical_size: UVec2::new(size, size),
        ..default()
    });
}
