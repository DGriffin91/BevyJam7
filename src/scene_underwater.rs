use avian3d::prelude::*;
use bevy::{prelude::*, scene::SceneInstanceReady};
use bevy_fps_controller::controller::LogicalPlayer;

use crate::{
    SceneContents, SceneState,
    cascade::{self, SceneBakeName},
    physics::tri_mesh_collider,
    post_process::PostProcessSettings,
    prepare_lighting::DynamicLight,
    std_mat_render::Fog,
};

#[derive(Resource, Default)]
pub struct UnderwaterGameplayPlugin;

impl Plugin for UnderwaterGameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerUnderwaterState>();
        //    .add_systems(
        //    Update,
        //    ().run_if(in_state(SceneState::Underwater),
        //);
    }
}

#[derive(Resource, Default)]
pub struct PlayerUnderwaterState {}

#[derive(Component)]
pub struct UnderwaterScene;

pub fn load_underwater(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut fog: ResMut<Fog>,
    mut sun: Single<&mut DirectionalLight>,
    #[cfg(feature = "asset_baking")] mut rt_env_color: ResMut<
        light_volume_baker::rt_scene::RtEnvColor,
    >,
    player: Single<(&mut Transform, &mut LinearVelocity), With<LogicalPlayer>>,
    mut post_process: ResMut<PostProcessSettings>,
    mut next_state: ResMut<NextState<SceneState>>,
    mut state: ResMut<PlayerUnderwaterState>,
    mut clear: ResMut<ClearColor>,
) {
    #[cfg(feature = "asset_baking")]
    {
        rt_env_color.0 = vec3a(1.0, 1.5, 1.8) * 0.05;
    }
    next_state.set(SceneState::Underwater);
    post_process.enable = false;
    *state = Default::default();
    clear.0 = Color::srgb(0.25, 0.3, 0.4);

    let (mut player_trans, mut player_vel) = player.into_inner();
    *player_trans =
        Transform::from_xyz(0.0, 2.5, -1.5).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y);
    *player_vel = LinearVelocity::ZERO;

    sun.illuminance = 0.0;
    sun.shadows_enabled = false;

    fog.fog_color = vec4(0.1, 0.2, 0.5, 1.0);
    fog.caustics = vec4(0.3, 0.6, 1.0, 1.0);

    commands
        .spawn((
            SceneRoot(
                asset_server
                    .load(GltfAssetLabel::Scene(0).from_asset("testing/models/Underwater.gltf")),
            ),
            UnderwaterScene,
            SceneContents,
            SceneBakeName(String::from("Underwater")),
        ))
        .observe(cascade::blender_cascades)
        .observe(tri_mesh_collider)
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
                        spot_light.intensity *= 10000.0;
                        spot_light.range = 10000.0;
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

    let mut ecmds = commands.spawn((
        SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("testing/models/underwater_skybox.gltf")),
        ),
        UnderwaterScene,
        SceneContents,
        SceneBakeName(String::from("Underwater")),
    ));

    #[cfg(feature = "asset_baking")]
    ecmds.observe(
        |scene_ready: On<SceneInstanceReady>,
         mut commands: Commands,
         children: Query<&Children>,
         mesh_entities: Query<Entity, With<Mesh3d>>| {
            for entity in children.iter_descendants(scene_ready.entity) {
                if let Ok(entity) = mesh_entities.get(entity) {
                    commands
                        .entity(entity)
                        .insert(light_volume_baker::rt_scene::NoBake);
                }
            }
        },
    );
}
