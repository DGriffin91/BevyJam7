use bevy::{prelude::*, scene::SceneInstanceReady};

use crate::{SceneContents, cascade, post_process::PostProcessSettings, std_mat_render::Fog};

#[derive(Component)]
pub struct HallwayScene;

pub fn load_hallway(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut fog: ResMut<Fog>,
    mut sun: Single<&mut DirectionalLight>,
    #[cfg(feature = "asset_baking")] mut rt_env_color: ResMut<
        light_volume_baker::rt_scene::RtEnvColor,
    >,
    camera: Single<&mut Transform, With<Camera3d>>,
    mut settings: ResMut<PostProcessSettings>,
) {
    #[cfg(feature = "asset_baking")]
    {
        rt_env_color.0 = Vec3A::ZERO;
    }
    settings.enable = true;

    *camera.into_inner() =
        Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y);

    sun.illuminance = 0.0;

    fog.fog_color = vec4(5.0, 5.0, 5.0, 1.0);

    commands
        .spawn((
            SceneRoot(
                asset_server
                    .load(GltfAssetLabel::Scene(0).from_asset("testing/models/Hallway.gltf")),
            ),
            HallwayScene,
            SceneContents,
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
}
