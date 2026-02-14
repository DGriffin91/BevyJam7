use bevy::{light::light_consts::lux::DIRECT_SUNLIGHT, prelude::*, scene::SceneInstanceReady};
use bgl2::phase_shadow::ShadowBounds;

use crate::{
    SceneContents, cascade::CascadeInput, post_process::PostProcessSettings,
    prepare_lighting::DynamicLight, std_mat_render::Fog,
};

#[derive(Component)]
pub struct TempleScene;

pub fn load_temple(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut fog: ResMut<Fog>,
    sun: Single<(&mut DirectionalLight, &mut ShadowBounds)>,
    #[cfg(feature = "asset_baking")] mut rt_env_color: ResMut<
        light_volume_baker::rt_scene::RtEnvColor,
    >,
    camera: Single<&mut Transform, With<Camera3d>>,
    mut settings: ResMut<PostProcessSettings>,
) {
    #[cfg(feature = "asset_baking")]
    {
        rt_env_color.0 = vec3a(0.32, 0.4, 0.47) * 2.0;
    }

    settings.enable = false;

    *camera.into_inner() =
        Transform::from_xyz(-45.0, 4.0, 0.0).looking_at(Vec3::new(0.0, 18.0, 0.0), Vec3::Y);

    let (mut sun, mut shadow_bounds) = sun.into_inner();
    sun.illuminance = DIRECT_SUNLIGHT;
    sun.shadows_enabled = true;
    *shadow_bounds = ShadowBounds::cube(250.0);

    fog.fog_color = vec4(1.0, 1.0, 1.0, 1.0);
    let start = vec3a(-47.5, 0.1, -25.5);
    let end = vec3a(36.0, 56.0, 34.0) * 2.0 + start;
    commands.spawn((
        CascadeInput {
            name: String::from("nave"),
            ws_aabb: obvhs::aabb::Aabb::new(start, end),
            resolution: vec3a(1.5, 1.5, 1.5),
        },
        SceneContents,
        TempleScene,
    ));

    let start = vec3a(10.5, 0.1, -25.5);
    let end = vec3a(26.0, 86.0, 34.0) * 2.0 + start;
    commands.spawn((
        CascadeInput {
            name: String::from("tower"),
            ws_aabb: obvhs::aabb::Aabb::new(start, end),
            resolution: vec3a(2.0, 2.0, 2.0),
        },
        SceneContents,
        TempleScene,
    ));

    commands.spawn((
        SceneRoot(asset_server.load(
            //GltfAssetLabel::Scene(0).from_asset("testing/models/temple/temple.gltf"),
            GltfAssetLabel::Scene(0).from_asset("testing/models/temple_test/temple_test.gltf"),
        )),
        SceneContents,
        TempleScene,
    ));

    commands
        .spawn((
            SceneRoot(
                asset_server
                    .load(GltfAssetLabel::Scene(0).from_asset("testing/temple_lights_test.gltf")),
            ),
            SceneContents,
            TempleScene,
        ))
        .observe(
            |scene_ready: On<SceneInstanceReady>,
             children: Query<&Children>,
             mut point_lights: Query<&mut PointLight>,
             mut spot_lights: Query<&mut SpotLight>,
             mut commands: Commands| {
                for entity in children.iter_descendants(scene_ready.entity) {
                    if let Ok(mut point_light) = point_lights.get_mut(entity) {
                        point_light.shadows_enabled = true;
                        point_light.intensity *= 50.0;
                    } else if let Ok(mut spot_light) = spot_lights.get_mut(entity) {
                        spot_light.shadows_enabled = true;
                        spot_light.intensity *= 50.0;
                    } else {
                        continue;
                    };
                    let mut ecmds = commands.entity(entity);
                    ecmds.insert(DynamicLight);
                    #[cfg(feature = "asset_baking")]
                    ecmds.insert(light_volume_baker::rt_scene::NoBake);
                }
            },
        );
}
