use std::f32::consts::PI;

use avian3d::prelude::{Collider, LinearVelocity, RigidBody};
use bevy::{prelude::*, scene::SceneInstanceReady};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use bevy_fps_controller::controller::LogicalPlayer;
use obvhs::aabb::Aabb;

#[derive(Resource, Default)]
pub struct StoreSceneGameplayPlugin;

impl Plugin for StoreSceneGameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerStoreState>()
            .add_systems(Update, (pickup_box, throw_box))
            .add_systems(EguiPrimaryContextPass, count_box);
    }
}

use crate::{
    SceneContents,
    cascade::{self, SceneBakeName},
    draw_debug::DebugLines,
    physics::{
        convex_hull_collider, convex_hull_dyn_collider_indv, convex_hull_dyn_collider_scene,
        tri_mesh_collider,
    },
    post_process::PostProcessSettings,
    std_mat_render::Fog,
};

#[derive(Component)]
pub struct StoreScene;

pub fn load_store(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut fog: ResMut<Fog>,
    mut sun: Single<&mut DirectionalLight>,
    #[cfg(feature = "asset_baking")] mut rt_env_color: ResMut<
        light_volume_baker::rt_scene::RtEnvColor,
    >,
    camera: Single<&mut Transform, With<Camera3d>>,
    mut post_process: ResMut<PostProcessSettings>,
) {
    #[cfg(feature = "asset_baking")]
    {
        rt_env_color.0 = Vec3A::ZERO;
    }
    post_process.enable = false;

    *camera.into_inner() =
        Transform::from_xyz(0.0, 2.0, 0.0).looking_at(Vec3::new(10.0, 0.0, 0.0), Vec3::Y);

    sun.illuminance = 0.0;

    //fog.fog_color = vec4(0.01, 0.01, 0.01, 1.0);
    fog.fog_color = Vec4::ZERO;

    let shelf =
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("testing/models/store_shelf.gltf"));

    for i in 0..39 {
        commands
            .spawn((
                Transform::from_xyz(i as f32 * -2.47182, 0.0, 0.0),
                SceneRoot(shelf.clone()),
                StoreScene,
                SceneContents,
            ))
            .observe(convex_hull_collider);
        commands
            .spawn((
                Transform::from_xyz(i as f32 * -2.47182 + 42.9162 * 2.0, 0.0, 0.0)
                    .with_rotation(Quat::from_rotation_y(PI)),
                SceneRoot(shelf.clone()),
                StoreScene,
                SceneContents,
            ))
            .observe(convex_hull_collider);
    }

    commands
        .spawn((
            SceneRoot(
                asset_server
                    .load(GltfAssetLabel::Scene(0).from_asset("testing/models/store_cart.gltf")),
            ),
            StoreScene,
            SceneContents,
        ))
        .observe(convex_hull_dyn_collider_scene);

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

#[derive(Component, Default)]
pub struct MacBox;

#[derive(Component)]
pub struct HeldBox;

#[derive(Resource, Default)]
pub struct PlayerStoreState {
    has_box: bool,
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
    camera: Single<&Transform, With<Camera3d>>,
    boxes: Query<Entity, With<HeldBox>>,
    mut state: ResMut<PlayerStoreState>,
    asset_server: Res<AssetServer>,
    btn: Res<ButtonInput<MouseButton>>,
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
                    Transform::from_translation(camera.translation),
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
    boxes: Query<(&Transform, &GlobalTransform), With<MacBox>>,
    #[allow(unused)] mut debug: ResMut<DebugLines>,
    state: Res<PlayerStoreState>,
) {
    let aisle_aabb = Aabb::new(vec3a(-52.0, 0.0, -2.5), vec3a(52.0, 4.0, 2.5));
    let mut boxes_in_aisle = if state.has_box { 1 } else { 0 };
    for (box_trans, box_global_trans) in &boxes {
        if aisle_aabb.contains_point(box_global_trans.translation().into())
            || aisle_aabb.contains_point(box_trans.translation.into())
        {
            boxes_in_aisle += 1;
        }
    }

    egui::Window::new("").show(contexts.ctx_mut().unwrap(), |ui| {
        ui.label(format!("Boxes remaining: {boxes_in_aisle}"));
    });
}
