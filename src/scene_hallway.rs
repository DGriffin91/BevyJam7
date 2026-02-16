use avian3d::prelude::*;
use bevy::{prelude::*, scene::SceneInstanceReady};
use bevy_fps_controller::controller::{FpsController, LogicalPlayer};
use bevy_seedling::prelude::*;

use crate::{
    SceneContents, SceneState,
    assets::{AudioAssets, SceneAssets},
    cascade::{self, SceneBakeName},
    despawn_scene_contents,
    draw_debug::DebugLines,
    physics::{convex_hull_dyn_collider_indv, tri_mesh_collider},
    post_process::PostProcessSettings,
    scene_falling::load_falling,
    scene_store::{HeldBox, MacBox, ThrownBox},
    std_mat_render::Fog,
};

#[derive(Resource, Default)]
pub struct HallwayGameplayPlugin;

impl Plugin for HallwayGameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerHallwayState>().add_systems(
            Update,
            (ghost_movement, pickup_box, throw_box).run_if(in_state(SceneState::Hallway)),
        );
    }
}

#[derive(Resource, Default)]
pub struct PlayerHallwayState {
    pub ghost_up_timer: f32,
    pub has_box: bool,
}

#[derive(Component)]
pub struct HallwayScene;

pub fn load_hallway(
    mut commands: Commands,
    mut fog: ResMut<Fog>,
    mut sun: Single<&mut DirectionalLight>,
    #[cfg(feature = "asset_baking")] mut rt_env_color: ResMut<
        light_volume_baker::rt_scene::RtEnvColor,
    >,
    player: Single<(&mut Transform, &mut LinearVelocity, &mut FpsController), With<LogicalPlayer>>,
    mut post_process: ResMut<PostProcessSettings>,
    mut next_state: ResMut<NextState<SceneState>>,
    mut state: ResMut<PlayerHallwayState>,
    assets: Res<SceneAssets>,
    audio: Res<AudioAssets>,
) {
    #[cfg(feature = "asset_baking")]
    {
        rt_env_color.0 = Vec3A::ZERO;
    }
    next_state.set(SceneState::Hallway);
    post_process.enable = true;
    *state = Default::default();

    commands.spawn((
        SamplePlayer::new(audio.hallway_music.clone())
            .with_volume(Volume::Decibels(-8.0))
            .looping(),
        HallwayScene,
        SceneContents,
    ));

    let (mut player_trans, mut player_vel, mut player_ctrl) = player.into_inner();
    *player_trans =
        Transform::from_xyz(0.0, 2.5, 4.0).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y);
    *player_vel = LinearVelocity::ZERO;
    player_ctrl.walk_speed = 3.0;
    player_ctrl.run_speed = 4.0;
    player_ctrl.gravity = 23.0;
    player_ctrl.jump_speed = 4.0;
    player_ctrl.air_acceleration = 80.0;

    sun.illuminance = 0.0;
    sun.shadows_enabled = false;

    fog.fog_color = vec4(5.0, 5.0, 5.0, 0.02);
    fog.caustics = vec4(0.6, 0.0, 0.0, 0.0);

    commands
        .spawn((
            SceneRoot(assets.hallway.clone()),
            HallwayScene,
            SceneContents,
            SceneBakeName(String::from("Hallway")),
        ))
        .observe(cascade::blender_cascades)
        .observe(
            |scene_ready: On<SceneInstanceReady>,
             children: Query<&Children>,
             mut point_lights: Query<&mut PointLight>,
             mut spot_lights: Query<&mut SpotLight>| {
                for entity in children.iter_descendants(scene_ready.entity) {
                    if let Ok(mut point_light) = point_lights.get_mut(entity) {
                        point_light.shadows_enabled = true;
                    } else if let Ok(mut spot_light) = spot_lights.get_mut(entity) {
                        spot_light.shadows_enabled = true;
                    }
                }
            },
        );

    commands
        .spawn((
            SceneRoot(assets.hallway_collider_mesh.clone()),
            HallwayScene,
            SceneContents,
        ))
        .observe(cascade::blender_cascades)
        .observe(
            |scene_ready: On<SceneInstanceReady>,
             mut commands: Commands,
             children: Query<&Children>,
             material_entites: Query<Entity, With<MeshMaterial3d<StandardMaterial>>>| {
                for entity in children.iter_descendants(scene_ready.entity) {
                    if let Ok(entity) = material_entites.get(entity) {
                        commands
                            .entity(entity)
                            .remove::<MeshMaterial3d<StandardMaterial>>();
                    }
                }
            },
        )
        .observe(tri_mesh_collider);

    commands
        .spawn((
            SceneRoot(assets.store_single_box.clone()),
            HallwayScene,
            SceneContents,
            Transform::from_translation(vec3(1.0, 0.2, -10.0)),
        ))
        .observe(convex_hull_dyn_collider_indv)
        .observe(
            |scene_ready: On<SceneInstanceReady>,
             mut commands: Commands,
             children: Query<&Children>,
             mesh_entities: Query<Entity, With<Mesh3d>>| {
                for entity in children.iter_descendants(scene_ready.entity) {
                    if let Ok(entity) = mesh_entities.get(entity) {
                        commands.entity(entity).insert(MacBox);
                    }
                }
            },
        );

    commands.spawn((
        SceneRoot(assets.hallway_ghost.clone()),
        HallwayScene,
        SceneContents,
        Transform::from_xyz(0.0, 1.666, -19.05),
        Ghost,
    ));
}

#[derive(Component)]
struct Ghost;

fn ghost_movement(
    mut commands: Commands,
    ghost: Single<&mut Transform, With<Ghost>>,
    time: Res<Time>,
    camera: Single<&GlobalTransform, With<Camera>>,
    boxes: Query<(Entity, &GlobalTransform), (With<ThrownBox>, Without<LogicalPlayer>)>,
    mut state: ResMut<PlayerHallwayState>,
) {
    let camera_pos = camera.translation();
    let camera_pos_high = camera_pos + Vec3::Y;
    let camera_pos_low = camera_pos - Vec3::Y;
    let mut ghost = ghost.into_inner();

    let ghost_pos = ghost.translation;
    let mut box_is_thrown = false;
    if let Some((_box_entity, trans)) = boxes.iter().next() {
        state.ghost_up_timer += time.delta_secs();
        if state.ghost_up_timer > 2.0 {
            let thrown_box_pos = trans.translation();
            ghost.look_to(-(thrown_box_pos - ghost_pos).normalize(), Vec3::Y);
            if (thrown_box_pos - Vec3::Y).distance(ghost_pos) > 3.0 {
                ghost.translation +=
                    (thrown_box_pos - ghost_pos).normalize() * time.elapsed_secs() * 0.001;
            }
        } else {
            ghost.translation += Vec3::Y * time.delta_secs() * 10.0;
        }
        box_is_thrown = true;
    } else {
        ghost.translation.y = (time.elapsed_secs() * 2.0).sin() * 0.5 + 1.0;
        state.ghost_up_timer = 0.0;
    }

    if camera_pos.z < -13.5 && !box_is_thrown {
        ghost.translation.y = (time.elapsed_secs() * 100.0).sin() * 0.4 + 1.3;
        ghost.translation.z += time.delta_secs() * 10.0;
    }

    if ghost.translation.distance(camera_pos) < 5.0 {
        ghost.look_to(-(camera_pos_low - ghost_pos).normalize(), Vec3::Y);
        ghost.translation +=
            (camera_pos_high - ghost_pos).normalize() * time.elapsed_secs() * 0.0015;
    }
    if ghost.translation.distance(camera_pos) < 1.5 || camera_pos.y < -10.0 {
        commands.run_system_cached(despawn_scene_contents);
        commands.run_system_cached(load_hallway);
    }
    if camera_pos.z < -20.0 && box_is_thrown {
        commands.run_system_cached(despawn_scene_contents);
        commands.run_system_cached(load_falling);
    }
}

pub fn pickup_box(
    mut commands: Commands,
    player: Single<(Entity, &Transform), With<LogicalPlayer>>,
    camera: Single<(Entity, &Transform), With<Camera3d>>,
    boxes: Query<
        (Entity, &GlobalTransform, &LinearVelocity),
        (With<MacBox>, Without<LogicalPlayer>),
    >,
    mut state: ResMut<PlayerHallwayState>,
    assets: Res<SceneAssets>,
) {
    let (_player_entity, player_trans) = player.into_inner();
    let (camera_entity, _camera_trans) = camera.into_inner();
    if !state.has_box {
        for (mac_box_entity, mac_global_trans, vel) in boxes {
            if player_trans
                .translation
                .distance(mac_global_trans.translation())
                < 1.8
                && vel.length() < 2.0
            {
                commands.entity(mac_box_entity).despawn();
                state.has_box = true;

                commands.entity(camera_entity).with_children(|parent| {
                    parent.spawn((
                        SceneRoot(assets.store_single_box.clone()),
                        HallwayScene,
                        SceneContents,
                        Transform::from_translation(vec3(0.0, -0.3, -0.6)),
                        HeldBox,
                    ));
                });

                break;
            }
        }
    }
}

pub fn throw_box(
    mut commands: Commands,
    camera: Single<&GlobalTransform, With<Camera>>,
    boxes: Query<Entity, With<HeldBox>>,
    mut state: ResMut<PlayerHallwayState>,
    btn: Res<ButtonInput<MouseButton>>,
    #[allow(unused)] mut debug: ResMut<DebugLines>,
    assets: Res<SceneAssets>,
) {
    if btn.just_pressed(MouseButton::Left) && state.has_box {
        state.has_box = false;
        let camera = **camera;
        for box_entity in &boxes {
            commands.entity(box_entity).despawn();
            commands
                .spawn((
                    SceneRoot(assets.store_single_box.clone()),
                    HallwayScene,
                    SceneContents,
                    Transform::from_translation(camera.translation() + *camera.forward()),
                ))
                .observe(
                    move |scene_ready: On<SceneInstanceReady>,
                          mut commands: Commands,
                          children: Query<&Children>,
                          mesh_entities: Query<(Entity, &Mesh3d)>,
                          meshes: Res<Assets<Mesh>>| {
                        for entity in children.iter_descendants(scene_ready.entity) {
                            if let Ok((entity, mesh)) = mesh_entities.get(entity) {
                                let mesh = meshes.get(mesh).unwrap();
                                commands.entity(entity).insert((
                                    Collider::convex_hull_from_mesh(mesh).unwrap(),
                                    RigidBody::Dynamic,
                                    LinearVelocity(camera.forward().as_vec3() * 5.0),
                                    MacBox,
                                    ThrownBox,
                                    //Mass(0.001),
                                ));
                            }
                        }
                    },
                );
        }
    }
}
