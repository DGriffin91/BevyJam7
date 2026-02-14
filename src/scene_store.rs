use std::f32::consts::PI;

use avian3d::prelude::*;
use bevy::{camera::primitives::Aabb, prelude::*, scene::SceneInstanceReady};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use bevy_fps_controller::controller::LogicalPlayer;

#[derive(Resource, Default)]
pub struct StoreSceneGameplayPlugin;

impl Plugin for StoreSceneGameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerStoreState>()
            .add_systems(
                Update,
                (
                    pickup_box,
                    throw_box,
                    move_big_mac_box_forward,
                    timed_events,
                )
                    .run_if(in_state(SceneState::Store)),
            )
            .add_systems(
                EguiPrimaryContextPass,
                count_box.run_if(in_state(SceneState::Store)),
            );
    }
}

use crate::{
    SceneContents, SceneState,
    cascade::{self, SceneBakeName},
    despawn_scene_contents,
    draw_debug::DebugLines,
    physics::{
        convex_hull_collider, convex_hull_dyn_collider_indv, tri_mesh_collider,
        trimesh_dyn_collider_scene,
    },
    post_process::PostProcessSettings,
    std_mat_render::Fog,
};

pub fn load_store(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut fog: ResMut<Fog>,
    mut sun: Single<&mut DirectionalLight>,
    #[cfg(feature = "asset_baking")] mut rt_env_color: ResMut<
        light_volume_baker::rt_scene::RtEnvColor,
    >,
    player: Single<(&mut Transform, &mut LinearVelocity), With<LogicalPlayer>>,
    mut post_process: ResMut<PostProcessSettings>,
    mut state: ResMut<PlayerStoreState>,
    mut next_state: ResMut<NextState<SceneState>>,
) {
    #[cfg(feature = "asset_baking")]
    {
        rt_env_color.0 = Vec3A::ZERO;
    }
    next_state.set(SceneState::Store);
    post_process.enable = false;
    *state = Default::default();

    let (mut player_trans, mut player_vel) = player.into_inner();
    *player_trans =
        Transform::from_xyz(0.0, 3.0, 0.0).looking_at(Vec3::new(10.0, 0.0, 0.0), Vec3::Y);
    *player_vel = LinearVelocity::ZERO;

    sun.illuminance = 0.0;

    //fog.fog_color = vec4(0.01, 0.01, 0.01, 1.0);
    fog.fog_color = Vec4::ZERO;

    let shelf =
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("testing/models/store_shelf.gltf"));

    let max = 39;
    for i in 0..max {
        commands
            .spawn((
                Transform::from_xyz(i as f32 * -2.47182, 0.0, 0.0),
                SceneRoot(shelf.clone()),
                StoreScene,
                SceneContents,
                StoreShelf(max - i),
            ))
            .observe(convex_hull_collider);
        commands
            .spawn((
                Transform::from_xyz(i as f32 * -2.47182 + 42.9162 * 2.0, 0.0, 0.0)
                    .with_rotation(Quat::from_rotation_y(PI)),
                SceneRoot(shelf.clone()),
                StoreScene,
                SceneContents,
                StoreShelf(max - i),
            ))
            .observe(convex_hull_collider);
    }

    for i in 1..4 {
        commands
            .spawn((
                Transform::from_xyz(i as f32 * -8.0, 1.0, 0.0)
                    .with_rotation(Quat::from_rotation_y(i as f32)),
                SceneRoot(
                    asset_server.load(
                        GltfAssetLabel::Scene(0).from_asset("testing/models/store_cart.gltf"),
                    ),
                ),
                StoreScene,
                SceneContents,
            ))
            .observe(trimesh_dyn_collider_scene);
    }

    commands
        .spawn((
            SceneRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("testing/models/store_boxes_on_floor.gltf"),
            )),
            StoreScene,
            SceneContents,
            Transform::from_xyz(0.0, 0.2, 0.0),
        ))
        .observe(convex_hull_dyn_collider_indv)
        .observe(
            |scene_ready: On<SceneInstanceReady>,
             mut commands: Commands,
             children: Query<&Children>,
             mesh_entities: Query<Entity, With<Mesh3d>>| {
                for entity in children.iter_descendants(scene_ready.entity) {
                    if let Ok(entity) = mesh_entities.get(entity) {
                        commands.entity(entity).insert(MacBox::default());
                    }
                }
            },
        );

    commands
        .spawn((
            SceneRoot(
                asset_server.load(GltfAssetLabel::Scene(0).from_asset("testing/models/Store.gltf")),
            ),
            StoreScene,
            SceneContents,
            SceneBakeName(String::from("Store")),
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
        )
        .observe(tri_mesh_collider);
}

#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

#[derive(Component, Default)]
pub struct MacBox;

#[derive(Component)]
pub struct BigMacBox(Entity);

#[derive(Component)]
pub struct HeldBox;

#[derive(Component)]
pub struct StoreScene;

#[derive(Resource, Default, Debug)]
pub struct PlayerStoreState {
    pub has_box: bool,
    pub timer: f32,
    pub big_box_has_been_spawned: bool,
    pub boxes_in_aisle: u32,
}

pub fn pickup_box(
    mut commands: Commands,
    player: Single<(Entity, &Transform), With<LogicalPlayer>>,
    camera: Single<(Entity, &Transform), With<Camera3d>>,
    boxes: Query<
        (Entity, &GlobalTransform, &LinearVelocity),
        (With<MacBox>, Without<LogicalPlayer>),
    >,
    mut state: ResMut<PlayerStoreState>,
    asset_server: Res<AssetServer>,
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
                        SceneRoot(
                            asset_server.load(
                                GltfAssetLabel::Scene(0)
                                    .from_asset("testing/models/store_single_box.gltf"),
                            ),
                        ),
                        StoreScene,
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
    mut state: ResMut<PlayerStoreState>,
    asset_server: Res<AssetServer>,
    btn: Res<ButtonInput<MouseButton>>,
    #[allow(unused)] mut debug: ResMut<DebugLines>,
) {
    if btn.just_pressed(MouseButton::Left) && state.has_box {
        state.has_box = false;
        let camera = camera.clone();
        for box_entity in &boxes {
            commands.entity(box_entity).despawn();
            commands
                .spawn((
                    SceneRoot(asset_server.load(
                        GltfAssetLabel::Scene(0).from_asset("testing/models/store_single_box.gltf"),
                    )),
                    StoreScene,
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
                                    LinearVelocity(camera.forward().as_vec3() * 10.0),
                                    MacBox,
                                    //Mass(0.001),
                                ));
                            }
                        }
                    },
                );
        }
    }
}

pub fn count_box(
    mut contexts: EguiContexts,
    boxes: Query<&GlobalTransform, With<MacBox>>,
    #[allow(unused)] mut debug: ResMut<DebugLines>,
    mut state: ResMut<PlayerStoreState>,
) {
    let aisle_aabb = obvhs::aabb::Aabb::new(vec3a(-52.0, -1.0, -2.5), vec3a(52.0, 4.0, 2.5));
    state.boxes_in_aisle = if state.has_box { 1 } else { 0 };
    for box_global_trans in &boxes {
        if aisle_aabb.contains_point(box_global_trans.translation().into()) {
            state.boxes_in_aisle += 1;
        }
    }

    egui::Window::new("").show(contexts.ctx_mut().unwrap(), |ui| {
        ui.label(format!("Boxes found by query {}", boxes.iter().len()));
        ui.label(format!("Boxes remaining: {}", state.boxes_in_aisle));
        ui.label(format!("Timer: {}", state.timer));
    });
}

fn move_big_mac_box_forward(
    mut commands: Commands,
    transforms: Query<(&GlobalTransform, &Aabb)>,
    mut boxes: Query<(&mut Transform, &BigMacBox)>,
    time: Res<Time>,
    camera: Single<&GlobalTransform, With<Camera>>,
) {
    let camera_pos = camera.translation();
    for (mut trans, big_box) in &mut boxes {
        trans.translation.x += time.delta_secs() * 8.0;
        if let Ok((global_trans, aabb)) = transforms.get(big_box.0) {
            let box_pos = global_trans.transform_point(aabb.center.into()).x;
            if box_pos > 65.0 || box_pos > camera_pos.x || camera_pos.y < -10.0 {
                commands.run_system_cached(despawn_scene_contents);
                commands.run_system_cached(load_store);
                break;
            }
        }
    }
}

fn play_animation_when_ready(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
    meshes: Res<Assets<Mesh>>,
    mesh_entities: Query<(Entity, &Mesh3d)>,
) {
    let mut mesh_entity = None;
    if let Ok(animation_to_play) = animations_to_play.get(scene_ready.entity) {
        for entity in children.iter_descendants(scene_ready.entity) {
            if let Ok(mut player) = players.get_mut(entity) {
                player.play(animation_to_play.index).repeat();
                commands
                    .entity(entity)
                    .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));
            }
            if let Ok((entity, mesh)) = mesh_entities.get(entity) {
                let mesh = meshes.get(mesh).unwrap();
                mesh_entity = Some(entity);
                commands.entity(entity).insert((
                    Collider::convex_hull_from_mesh(mesh).unwrap(),
                    RigidBody::Static,
                ));
            }
        }
    }
    commands
        .entity(scene_ready.entity)
        .insert((SceneContents, BigMacBox(mesh_entity.unwrap())));
}

fn timed_events(
    mut state: ResMut<PlayerStoreState>,
    mut commands: Commands,
    time: Res<Time>,
    shelves: Query<(Entity, &Transform, &StoreShelf)>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    if state.boxes_in_aisle < 25 {
        state.timer += time.delta_secs();
    }

    let shelves_swap_start = 4.0;
    let spawn_big_box = shelves_swap_start + 20.0;

    if state.timer > shelves_swap_start {
        if !shelves.is_empty() {
            let shelf = asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("testing/models/store_mac_shelf.gltf"));
            for (shelf_entity, shelf_trans, shelf_index) in &shelves {
                if (state.timer - shelves_swap_start) * 3.0 > shelf_index.0 as f32 {
                    commands.entity(shelf_entity).despawn();
                    commands
                        .spawn((
                            shelf_trans.clone(),
                            SceneRoot(shelf.clone()),
                            StoreScene,
                            SceneContents,
                            StoreMacShelf,
                        ))
                        .observe(convex_hull_collider);
                }
            }
        }
    }

    if !state.big_box_has_been_spawned && state.timer > spawn_big_box {
        state.big_box_has_been_spawned = true;
        let (graph, index) =
            AnimationGraph::from_clip(asset_server.load(
                GltfAssetLabel::Animation(0).from_asset("testing/models/store_mac_anim.gltf"),
            ));
        let graph_handle = graphs.add(graph);
        let animation_to_play = AnimationToPlay {
            graph_handle,
            index,
        };
        let mesh_scene = SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("testing/models/store_mac_anim.gltf")),
        );
        commands
            .spawn((
                animation_to_play,
                mesh_scene,
                Transform::from_scale(Vec3::ONE),
                SceneContents,
            ))
            .observe(play_animation_when_ready);
    }
}

#[derive(Component)]
pub struct StoreShelf(i32);

#[derive(Component)]
pub struct StoreMacShelf;
