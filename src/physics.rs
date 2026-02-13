use avian3d::prelude::*;
use bevy::{prelude::*, scene::SceneInstanceReady};
use bevy_mod_mesh_tools::mesh_append;
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

pub fn trimesh_dyn_collider_scene(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    mesh_entities: Query<(Entity, &Mesh3d)>,
    meshes: Res<Assets<Mesh>>,
) {
    let mut combined_mesh = None;
    for entity in children.iter_descendants(scene_ready.entity) {
        if let Ok((_entity, mesh)) = mesh_entities.get(entity) {
            let mesh = meshes.get(mesh).unwrap();
            if let Some(combined_mesh) = &mut combined_mesh {
                mesh_append(combined_mesh, &mesh).unwrap();
            } else {
                combined_mesh = Some(mesh.clone());
            }
        }
    }
    if let Some(combined_mesh) = combined_mesh {
        commands.entity(scene_ready.entity).insert((
            Collider::trimesh_from_mesh(&combined_mesh).unwrap(),
            RigidBody::Dynamic,
        ));
    }
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
