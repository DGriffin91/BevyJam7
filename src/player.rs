use std::f32::consts::{PI, TAU};

use avian3d::prelude::*;
use bevy::{
    core_pipeline::prepass::DepthPrepass,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use bevy_fps_controller::controller::*;

#[derive(Resource, Default)]
pub struct PlayerControllerPlugin;

const SPAWN_POINT: Vec3 = Vec3::new(0.0, 1.625, 0.0);
impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .add_plugins(FpsControllerPlugin)
            .add_systems(Startup, setup_player_controller)
            .add_systems(EguiPrimaryContextPass, manage_cursor);
    }
}

fn setup_player_controller(mut commands: Commands) {
    let logical_entity = commands
        .spawn((
            Collider::cylinder(0.4, 2.0),
            // A capsule can be used but is NOT recommended
            // If you use it, you have to make sure each segment point is
            // equidistant from the translation of the player transform
            // Collider::capsule(0.5, height),
            Friction {
                dynamic_coefficient: 0.0,
                static_coefficient: 0.0,
                combine_rule: CoefficientCombine::Min,
            },
            Restitution {
                coefficient: 0.0,
                combine_rule: CoefficientCombine::Min,
            },
            LinearVelocity::ZERO,
            RigidBody::Dynamic,
            LockedAxes::ROTATION_LOCKED,
            Mass(1.0),
            GravityScale(0.0),
            Transform::from_translation(SPAWN_POINT),
            LogicalPlayer,
            FpsControllerInput {
                pitch: -TAU / 12.0,
                yaw: TAU * 5.0 / 8.0,
                ..default()
            },
            FpsController {
                air_acceleration: 80.0,
                jump_speed: 4.0,
                run_speed: 5.0,
                walk_speed: 4.0,
                enable_input: false,
                key_fly: KeyCode::KeyL,
                crouch_height: 2.7,
                ..default()
            },
        ))
        .insert(CameraConfig {
            height_offset: -1.5,
        })
        .id();

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-45.0, 4.0, 0.0).looking_at(Vec3::new(0.0, 18.0, 0.0), Vec3::Y),
        //FreeCamera {
        //    walk_speed: 5.0,
        //    run_speed: 30.0,
        //    ..default()
        //},
        Projection::Perspective(PerspectiveProjection {
            fov: PI / 3.0,
            ..default()
        }),
        DepthPrepass,
        RenderPlayer { logical_entity },
    ));
}

fn manage_cursor(
    btn: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
    mut cursor: Single<&mut CursorOptions>,
    mut contexts: EguiContexts,
    mut controller_query: Query<&mut FpsController>,
) {
    let mut wants_input = false;
    if let Ok(ctx) = contexts.ctx_mut() {
        wants_input = ctx.wants_pointer_input() || ctx.wants_keyboard_input();
    }
    if !wants_input {
        if btn.just_pressed(MouseButton::Left) {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
            for mut controller in &mut controller_query {
                controller.enable_input = true;
            }
        }
    }
    if key.just_pressed(KeyCode::Escape) || key.just_pressed(KeyCode::Tab) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
        for mut controller in &mut controller_query {
            controller.enable_input = false;
        }
    }
}
