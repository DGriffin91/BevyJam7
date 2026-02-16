use avian3d::prelude::*;
use bevy::{prelude::*, scene::SceneInstanceReady};
use bevy_fps_controller::controller::{FpsController, LogicalPlayer};
use bevy_seedling::prelude::*;

use crate::{
    SceneContents, SceneState,
    assets::{AudioAssets, SceneAssets},
    cascade::{self, SceneBakeName},
    despawn_scene_contents,
    physics::tri_mesh_collider,
    post_process::PostProcessSettings,
    std_mat_render::Fog,
};

#[derive(Resource, Default)]
pub struct FallingGameplayPlugin;

impl Plugin for FallingGameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerFallingState>()
            .add_systems(Update, (check_rings).run_if(in_state(SceneState::Falling)));
    }
}

#[derive(Resource, Default)]
pub struct PlayerFallingState {
    pub ghost_up_timer: f32,
    pub has_box: bool,
}

#[derive(Component)]
pub struct FallingScene;

pub fn load_falling(
    mut commands: Commands,
    mut fog: ResMut<Fog>,
    mut sun: Single<&mut DirectionalLight>,
    #[cfg(feature = "asset_baking")] mut rt_env_color: ResMut<
        light_volume_baker::rt_scene::RtEnvColor,
    >,
    player: Single<(&mut Transform, &mut LinearVelocity, &mut FpsController), With<LogicalPlayer>>,
    mut post_process: ResMut<PostProcessSettings>,
    mut next_state: ResMut<NextState<SceneState>>,
    mut state: ResMut<PlayerFallingState>,
    assets: Res<SceneAssets>,
    audio: Res<AudioAssets>,
) {
    #[cfg(feature = "asset_baking")]
    {
        rt_env_color.0 = vec3a(1.0, 1.5, 1.8);
    }
    next_state.set(SceneState::Falling);
    post_process.enable = false;
    *state = Default::default();

    commands.spawn((
        SamplePlayer::new(audio.end_music.clone())
            .with_volume(Volume::Decibels(-8.0))
            .looping(),
        FallingScene,
        SceneContents,
    ));

    let (mut player_trans, mut player_vel, mut player_ctrl) = player.into_inner();
    *player_trans =
        Transform::from_xyz(0.0, 10.5, 4.0).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y);
    *player_vel = LinearVelocity::ZERO;
    player_ctrl.walk_speed = 3.0;
    player_ctrl.run_speed = 4.0;
    player_ctrl.gravity = 1.0;
    player_ctrl.jump_speed = 4.0;
    player_ctrl.air_acceleration = 100.0;

    sun.illuminance = 100000.0;
    sun.shadows_enabled = false;

    fog.fog_color = vec4(0.0, 0.0, 0.0, 0.0);
    fog.caustics = vec4(0.0, 0.0, 0.0, 0.0);

    commands
        .spawn((
            SceneRoot(assets.falling.clone()),
            FallingScene,
            SceneContents,
            SceneBakeName(String::from("Falling")),
        ))
        .observe(cascade::blender_cascades)
        .observe(
            |scene_ready: On<SceneInstanceReady>,
             children: Query<&Children>,
             mut commands: Commands,
             named: Query<(Entity, &Name)>| {
                for entity in children.iter_descendants(scene_ready.entity) {
                    if let Ok((entity, name)) = named.get(entity)
                        && name.contains("Ring")
                    {
                        commands.entity(entity).insert(Ring);
                    }
                }
            },
        )
        .observe(tri_mesh_collider);
}

#[derive(Component)]
struct Ring;

fn check_rings(
    mut commands: Commands,
    rings: Query<(Entity, &GlobalTransform), With<Ring>>,
    camera: Single<&GlobalTransform, With<Camera>>,
) {
    for (entity, trans) in &rings {
        let cam_pos = camera.translation();
        let ring_pos = trans.translation();

        if cam_pos.y < ring_pos.y {
            if cam_pos.distance(ring_pos) < 3.0 {
                commands.entity(entity).despawn();
            } else {
                commands.run_system_cached(despawn_scene_contents);
                commands.run_system_cached(load_falling);
            }
        }
    }
}
