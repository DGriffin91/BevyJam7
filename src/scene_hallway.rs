use avian3d::prelude::LinearVelocity;
use bevy::{prelude::*, scene::SceneInstanceReady};
use bevy_fps_controller::controller::LogicalPlayer;

use crate::{
    SceneContents, SceneState,
    cascade::{self, SceneBakeName},
    despawn_scene_contents,
    physics::{convex_hull_dyn_collider_indv, tri_mesh_collider},
    post_process::PostProcessSettings,
    scene_store::{MacBox, pickup_box, throw_box},
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
    pub timer: f32,
}

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
    player: Single<(&mut Transform, &mut LinearVelocity), With<LogicalPlayer>>,
    mut post_process: ResMut<PostProcessSettings>,
    mut next_state: ResMut<NextState<SceneState>>,
    mut state: ResMut<PlayerHallwayState>,
) {
    #[cfg(feature = "asset_baking")]
    {
        rt_env_color.0 = Vec3A::ZERO;
    }
    next_state.set(SceneState::Hallway);
    post_process.enable = true;
    *state = Default::default();

    let (mut player_trans, mut player_vel) = player.into_inner();
    *player_trans =
        Transform::from_xyz(0.0, 2.0, 4.0).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y);
    *player_vel = LinearVelocity::ZERO;

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
            SceneRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("testing/models/hallway_collider_mesh.gltf"),
            )),
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
            SceneRoot(
                asset_server.load(
                    GltfAssetLabel::Scene(0).from_asset("testing/models/store_single_box.gltf"),
                ),
            ),
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
                        commands.entity(entity).insert(MacBox::default());
                    }
                }
            },
        );

    commands.spawn((
        SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("testing/models/hallway_ghost.gltf")),
        ),
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
    mut ghost: Single<&mut Transform, With<Ghost>>,
    time: Res<Time>,
    camera: Single<&GlobalTransform, With<Camera>>,
) {
    ghost.translation.y = (time.elapsed_secs() * 2.0).sin() * 0.5 + 1.0;
    let camera_pos = camera.translation();
    if camera_pos.z < -13.5 {
        ghost.translation.y = (time.elapsed_secs() * 100.0).sin() * 0.4 + 1.3;
        ghost.translation.z += time.delta_secs() * 10.0;
        if ghost.translation.z > camera_pos.z {
            commands.run_system_cached(despawn_scene_contents);
            commands.run_system_cached(load_hallway);
        }
    }
}
