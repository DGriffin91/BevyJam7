use avian3d::prelude::*;
use bevy::{prelude::*, scene::SceneInstanceReady};
use bevy_fps_controller::controller::{FpsController, LogicalPlayer};

use crate::{
    SceneContents, SceneState,
    assets::SceneAssets,
    cascade::{self, SceneBakeName},
    despawn_scene_contents,
    physics::tri_mesh_collider,
    post_process::PostProcessSettings,
    prepare_lighting::DynamicLight,
    scene_hallway::load_hallway,
    std_mat_render::Fog,
};

#[derive(Resource, Default)]
pub struct UnderwaterGameplayPlugin;

impl Plugin for UnderwaterGameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerUnderwaterState>().add_systems(
            Update,
            (teleporter, move_airships, move_searchlights)
                .chain()
                .run_if(in_state(SceneState::Underwater)),
        );
    }
}

#[derive(Resource, Default)]
pub struct PlayerUnderwaterState {}

#[derive(Component)]
pub struct UnderwaterScene;

pub fn load_underwater(
    mut commands: Commands,
    mut fog: ResMut<Fog>,
    mut sun: Single<&mut DirectionalLight>,
    #[cfg(feature = "asset_baking")] mut rt_env_color: ResMut<
        light_volume_baker::rt_scene::RtEnvColor,
    >,
    player: Single<(&mut Transform, &mut LinearVelocity, &mut FpsController), With<LogicalPlayer>>,
    mut post_process: ResMut<PostProcessSettings>,
    mut next_state: ResMut<NextState<SceneState>>,
    mut state: ResMut<PlayerUnderwaterState>,
    mut clear: ResMut<ClearColor>,
    assets: Res<SceneAssets>,
) {
    #[cfg(feature = "asset_baking")]
    {
        rt_env_color.0 = vec3a(1.0, 1.5, 1.8) * 0.05;
    }
    next_state.set(SceneState::Underwater);
    post_process.enable = false;
    *state = Default::default();
    clear.0 = Color::srgb(0.25, 0.3, 0.4);

    let (mut player_trans, mut player_vel, mut player_ctrl) = player.into_inner();
    *player_trans =
        Transform::from_xyz(0.0, 2.5, -1.5).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y);
    *player_vel = LinearVelocity::ZERO;
    player_ctrl.walk_speed = 2.0;
    player_ctrl.run_speed = 2.5;
    player_ctrl.gravity = 3.0;
    player_ctrl.jump_speed = 2.0;
    player_ctrl.air_acceleration = 80.0;

    sun.illuminance = 0.0;
    sun.shadows_enabled = false;

    fog.fog_color = vec4(0.1, 0.2, 0.5, 1.0);
    fog.caustics = vec4(0.3, 0.6, 1.0, 1.0);

    commands
        .spawn((
            SceneRoot(assets.underwater.clone()),
            UnderwaterScene,
            SceneContents,
            SceneBakeName(String::from("Underwater")),
        ))
        .observe(cascade::blender_cascades)
        .observe(tri_mesh_collider);

    #[allow(unused)]
    let mut ecmds = commands.spawn((
        SceneRoot(assets.underwater_skybox.clone()),
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

    let ship_scene = &assets.underwater_airship;
    for i in 0..3 {
        let pos = SHIP_DESTINATIONS[i];
        commands
            .spawn((
                SceneRoot(ship_scene.clone()),
                UnderwaterScene,
                SceneContents,
                Transform::from_translation(vec3(pos.x, pos.y + i as f32 * 10.0, pos.z)),
                Airship {
                    destination: i,
                    speed: i as f32 * 0.5 + 1.0,
                    index: i as u32,
                },
            ))
            .observe(proc_ship);
    }

    commands
        .spawn((
            SceneRoot(assets.underwater_collider_mesh.clone()),
            UnderwaterScene,
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
}

fn teleporter(mut commands: Commands, camera: Single<&GlobalTransform, With<Camera>>) {
    if camera.translation().z < -77.0 {
        commands.run_system_cached(despawn_scene_contents);
        commands.run_system_cached(load_hallway);
    }
}

fn move_airships(mut airships: Query<(&mut Transform, &mut Airship)>, time: Res<Time>) {
    for (mut trans, mut ship) in &mut airships {
        let mut old_dest = SHIP_DESTINATIONS[ship.destination];
        old_dest.y = trans.translation.y;
        if old_dest.distance(trans.translation) < 5.0 {
            ship.destination = (ship.destination + 1) % SHIP_DESTINATIONS.len();
        }
        let mut current_dest = SHIP_DESTINATIONS[ship.destination];
        current_dest.y = trans.translation.y;
        let current_pos = trans.translation;
        let to = current_dest - current_pos;

        if to.length_squared() < 0.0001 {
            ship.destination = (ship.destination + 1) % SHIP_DESTINATIONS.len();
            continue;
        }
        let dest_vec = to.normalize();
        let desired = trans.looking_at(current_dest, Vec3::Y).rotation;
        let turn = 0.08;
        trans.rotation = trans
            .rotation
            .slerp(desired, 1.0 - (-turn * time.delta_secs()).exp());
        let align = dest_vec.dot(*trans.forward()).clamp(0.0, 1.0);
        trans.translation += dest_vec * (6.0 * align * ship.speed) * time.delta_secs();
    }
}

fn move_searchlights(
    mut search_lights: Query<(&ChildOf, &GlobalTransform, &mut Transform, &Searchlight)>,
    parents: Query<&GlobalTransform>,
    camera: Single<&GlobalTransform, With<Camera>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let camera_pos = camera.translation();
    let sweep_speed = 0.3;
    let phase_strength = 1.0;
    let sweep_distance = 100.0;
    for (parent, light_global, mut light_trans, search_light) in &mut search_lights {
        let z = SWEEP_REGIONS[search_light.index as usize];
        let ws_aim_point = vec3(
            (time.elapsed_secs() * sweep_speed + phase_strength * search_light.index as f32).sin()
                * sweep_distance,
            0.0,
            z,
        );
        let ws_light_pos = light_global.translation();
        let ws_dir = (ws_aim_point - ws_light_pos).normalize();
        if point_in_cone(camera_pos, ws_light_pos, ws_dir, 9.0f32.to_radians()) {
            commands.run_system_cached(despawn_scene_contents);
            commands.run_system_cached(load_underwater);
        }
        let desired_global = Transform::IDENTITY.looking_to(ws_dir, Vec3::Y).rotation;
        let parent_global_rot = parents.get(parent.0).unwrap().rotation();
        light_trans.rotation = parent_global_rot.inverse() * desired_global;
    }
}

#[derive(Component, Clone, Debug, Default)]
struct Airship {
    destination: usize,
    speed: f32,
    index: u32,
}
#[derive(Component, Clone, Debug, Default)]
struct Searchlight {
    index: u32,
}

const SHIP_DESTINATIONS: [Vec3; 6] = [
    vec3(-40.0, 40.0, -40.0),
    vec3(-45.0, 40.0, -20.0),
    vec3(50.0, 40.0, -80.0),
    vec3(45.0, 40.0, -20.0),
    vec3(-40.0, 40.0, -120.0),
    vec3(-50.0, 40.0, -90.0),
];

const SWEEP_REGIONS: [f32; 3] = [-10.0, -40.0, -70.0];

fn proc_ship(
    scene_ready: On<SceneInstanceReady>,
    children: Query<&Children>,
    mut spot_lights: Query<&mut SpotLight>,
    mut commands: Commands,
    named: Query<(Entity, &Name)>,
    airships: Query<&Airship>,
) {
    if let Ok(airship) = airships.get(scene_ready.entity) {
        for entity in children.iter_descendants(scene_ready.entity) {
            if let Ok(mut spot_light) = spot_lights.get_mut(entity) {
                spot_light.shadows_enabled = true;
                spot_light.intensity *= 1000.0;
                spot_light.range = 10000.0;
                let mut ecmds = commands.entity(entity);
                ecmds.insert(DynamicLight);
                #[cfg(feature = "asset_baking")]
                ecmds.insert(light_volume_baker::rt_scene::NoBake);
            };
            if let Ok((entity, name)) = named.get(entity)
                && name.contains("SEARCH_LIGHT")
            {
                commands.entity(entity).insert(Searchlight {
                    index: airship.index,
                });
            }
        }
    }
}

fn point_in_cone(p: Vec3, origin: Vec3, normal: Vec3, opening: f32) -> bool {
    let v = p - origin;
    let v_len = v.length();
    if v_len <= 1e-8 {
        return true;
    };
    let dir = v / v_len;
    let half_angle_rad = opening * 0.5;
    let cos_half = half_angle_rad.cos();
    dir.dot(normal) >= cos_half
}
