use std::f32::consts::PI;

use bevy::{prelude::*, scene::SceneInstanceReady};

use crate::{
    SceneContents,
    cascade::{self, SceneBakeName},
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
        ))
        .observe(convex_hull_dyn_collider_indv);

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
