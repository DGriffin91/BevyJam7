use avian3d::prelude::*;
use bevy::{prelude::*, scene::SceneInstanceReady};
use bgl2::mesh_util::get_attribute_f32x3;

pub fn tri_mesh_collider(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    mesh_entities: Query<(Entity, &Mesh3d)>,
    meshes: Res<Assets<Mesh>>,
) {
    for entity in children.iter_descendants(scene_ready.entity) {
        if let Ok((entity, mesh)) = mesh_entities.get(entity) {
            let mesh = meshes.get(mesh).unwrap();
            commands.entity(entity).insert((
                Collider::trimesh_from_mesh(mesh).unwrap(),
                //Collider::convex_hull_from_mesh(mesh).unwrap(),
                RigidBody::Static,
            ));
        }
    }
}

pub fn convex_hull_collider(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    mesh_entities: Query<(Entity, &Mesh3d)>,
    meshes: Res<Assets<Mesh>>,
) {
    for entity in children.iter_descendants(scene_ready.entity) {
        if let Ok((entity, mesh)) = mesh_entities.get(entity) {
            let mesh = meshes.get(mesh).unwrap();
            commands.entity(entity).insert((
                Collider::convex_hull_from_mesh(mesh).unwrap(),
                RigidBody::Static,
            ));
        }
    }
}

pub fn convex_hull_dyn_collider_scene(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    mesh_entities: Query<(Entity, &Mesh3d)>,
    meshes: Res<Assets<Mesh>>,
) {
    let mut points = Vec::new();
    for entity in children.iter_descendants(scene_ready.entity) {
        if let Ok((_entity, mesh)) = mesh_entities.get(entity) {
            let mesh = meshes.get(mesh).unwrap();
            let positions = get_attribute_f32x3(mesh, Mesh::ATTRIBUTE_POSITION)
                .expect("Meshes vertex positions are required");
            for pos in positions {
                points.push(Vec3::from_array(*pos))
            }
        }
    }
    commands
        .entity(scene_ready.entity)
        .insert((Collider::convex_hull(points).unwrap(), RigidBody::Dynamic));
}

pub fn convex_hull_dyn_collider_indv(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    mesh_entities: Query<(Entity, &Mesh3d)>,
    meshes: Res<Assets<Mesh>>,
) {
    for entity in children.iter_descendants(scene_ready.entity) {
        if let Ok((entity, mesh)) = mesh_entities.get(entity) {
            let mesh = meshes.get(mesh).unwrap();
            commands.entity(entity).insert((
                Collider::convex_hull_from_mesh(mesh).unwrap(),
                RigidBody::Dynamic,
            ));
        }
    }
}
