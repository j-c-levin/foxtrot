use crate::player_control::actions::{Actions, ActionsFrozen};
use crate::util::trait_extension::{Vec2Ext, Vec3Ext};
use crate::GameState;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};

const MAX_DISTANCE: f32 = 10.0;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiCamera>()
            .register_type::<MainCamera>()
            .add_startup_system(spawn_ui_camera)
            // Enables the system that synchronizes your `Transform`s and `LookTransform`s.
            .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(despawn_ui_camera))
            .add_system_set(
                SystemSet::on_update(GameState::Playing)
                    .with_system(follow_target.label("follow_target"))
                    .with_system(
                        handle_camera_controls
                            .label("handle_camera_controls")
                            .after("follow_target"),
                    )
                    .with_system(
                        update_camera_transform
                            .label("update_camera_transform")
                            .after("handle_camera_controls"),
                    )
                    .with_system(cursor_grab_system),
            );
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Component, Reflect, Serialize, Deserialize, Default)]
#[reflect(Component, Serialize, Deserialize)]
pub struct UiCamera;

#[derive(Debug, Clone, PartialEq, Component, Reflect, Serialize, Deserialize, Default)]
#[reflect(Component, Serialize, Deserialize)]
pub struct MainCamera {
    pub current: CameraPosition,
    pub new: CameraPosition,
}

impl MainCamera {
    pub fn set_target(&mut self, target: Vec3) -> &mut Self {
        self.new.target = target;
        self
    }

    pub fn look_at_new_target(&mut self) -> &mut Self {
        let target = self.new.target;
        if !(target - self.new.eye.translation).is_approx_zero() {
            self.new.eye.look_at(target, Vect::Y);
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Component, Reflect, Serialize, Deserialize, Default)]
#[reflect(Component, Serialize, Deserialize)]
pub struct CameraPosition {
    pub eye: Transform,
    pub target: Vec3,
}

fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), UiCamera, Name::new("Camera")));
}

fn despawn_ui_camera(mut commands: Commands, query: Query<Entity, With<UiCamera>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn follow_target(mut camera_query: Query<&mut MainCamera>) {
    for mut camera in &mut camera_query {
        let target_movement = camera.new.target - camera.current.target;
        camera.new.eye.translation = camera.current.eye.translation + target_movement;
        camera.look_at_new_target();
    }
}

fn handle_camera_controls(mut camera_query: Query<&mut MainCamera>, actions: Res<Actions>) {
    let mouse_sensitivity = 5e-3;
    let camera_movement = match actions.camera_movement {
        Some(vector) => vector * mouse_sensitivity,
        None => return,
    };

    if camera_movement.is_approx_zero() {
        return;
    }
    for mut camera in camera_query.iter_mut() {
        let direction = camera.new.eye.forward();
        let mut transform = camera.new.eye.with_rotation(default());

        let horizontal_angle = -camera_movement.x.clamp(-PI + 1e-5, PI - 1e-5);
        transform.rotate_axis(Vec3::Y, horizontal_angle);
        let vertical_angle = -camera_movement.y;
        let vertical_angle = clamp_vertical_rotation(direction, vertical_angle);
        transform.rotate_local_x(vertical_angle);

        let pivot = camera.new.target;
        let rotation = transform.rotation;
        camera.new.eye.rotate_around(pivot, rotation);
    }
}

fn clamp_vertical_rotation(current_direction: Vec3, angle: f32) -> f32 {
    let current_angle = current_direction.angle_between(Vect::Y);
    let new_angle = current_angle - angle;

    const ANGLE_FROM_EXTREMES: f32 = TAU / 32.;
    const MAX_ANGLE: f32 = TAU / 2.0 - ANGLE_FROM_EXTREMES;
    const MIN_ANGLE: f32 = 0.0 + ANGLE_FROM_EXTREMES;

    let clamped_angle = if new_angle > MAX_ANGLE {
        MAX_ANGLE - current_angle
    } else if new_angle < MIN_ANGLE {
        MIN_ANGLE - current_angle
    } else {
        angle
    };

    if clamped_angle.abs() < 0.01 {
        // This smooths user experience
        0.
    } else {
        clamped_angle
    }
}

fn update_camera_transform(
    time: Res<Time>,
    mut camera_query: Query<(&mut Transform, &mut MainCamera)>,
    rapier_context: Res<RapierContext>,
) {
    let dt = time.delta_seconds();
    for (mut transform, mut camera) in camera_query.iter_mut() {
        let scale = (5. * dt).min(1.);
        let line_of_sight_result = keep_line_of_sight(&camera, &rapier_context);
        if line_of_sight_result.correction == LineOfSightCorrection::Closer {
            transform.translation = line_of_sight_result.location;
        } else {
            let direction = line_of_sight_result.location - transform.translation;
            transform.translation += direction * scale;
        }
        transform.rotation = transform.rotation.slerp(camera.new.eye.rotation, scale);

        camera.current = camera.new.clone();
    }
}

fn keep_line_of_sight(camera: &MainCamera, rapier_context: &RapierContext) -> LineOfSightResult {
    let origin = camera.new.target;
    let desired_direction = camera.new.eye.translation - camera.new.target;
    let norm_direction = desired_direction.try_normalize().unwrap_or(Vec3::Z);

    let distance = get_raycast_distance(origin, norm_direction, rapier_context, MAX_DISTANCE);
    let location = origin + norm_direction * distance;
    let correction = if distance * distance < desired_direction.length_squared() {
        LineOfSightCorrection::Closer
    } else {
        LineOfSightCorrection::Further
    };
    LineOfSightResult {
        location,
        correction,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LineOfSightResult {
    pub location: Vec3,
    pub correction: LineOfSightCorrection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineOfSightCorrection {
    Closer,
    Further,
}

pub fn get_raycast_distance(
    origin: Vec3,
    direction: Vec3,
    rapier_context: &RapierContext,
    max_distance: f32,
) -> f32 {
    let max_toi = max_distance;
    let solid = true;
    let mut filter = QueryFilter::only_fixed();
    filter.flags |= QueryFilterFlags::EXCLUDE_SENSORS;

    let min_distance_to_objects = 0.001;
    rapier_context
        .cast_ray(origin, direction, max_toi, solid, filter)
        .map(|(_entity, toi)| toi - min_distance_to_objects)
        .unwrap_or(max_distance)
}

fn cursor_grab_system(
    mut windows: ResMut<Windows>,
    key: Res<Input<KeyCode>>,
    frozen: Option<Res<ActionsFrozen>>,
) {
    let window = windows.get_primary_mut().unwrap();
    if frozen.is_some() {
        window.set_cursor_grab_mode(CursorGrabMode::None);
        window.set_cursor_visibility(true);
        return;
    }
    if key.just_pressed(KeyCode::Escape) {
        if matches!(window.cursor_grab_mode(), CursorGrabMode::None) {
            // if you want to use the cursor, but not let it leave the window,
            // use `Confined` mode:
            window.set_cursor_grab_mode(CursorGrabMode::Confined);

            // for a game that doesn't use the cursor (like a shooter):
            // use `Locked` mode to keep the cursor in one place
            window.set_cursor_grab_mode(CursorGrabMode::Locked);
            // also hide the cursor
            window.set_cursor_visibility(false);
        } else {
            window.set_cursor_grab_mode(CursorGrabMode::None);
            window.set_cursor_visibility(true);
        }
    }
}
