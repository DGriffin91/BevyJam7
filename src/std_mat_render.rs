use bevy::{camera::primitives::Aabb, prelude::*};
use bgl2::bevy_standard_lighting::DEFAULT_MAX_LIGHTS_DEF;
use bgl2::{
    UniformSet, UniformValue,
    bevy_standard_material::{
        DrawsSortedByMaterial, ReadReflection, SkipReflection, StandardMaterialUniforms,
        ViewUniforms,
    },
    command_encoder::CommandEncoder,
    flip_cull_mode,
    phase_shadow::DirectionalLightShadow,
    phase_transparent::DeferredAlphaBlendDraws,
    plane_reflect::ReflectionUniforms,
    prepare_image::GpuImages,
    prepare_joints::JointData,
    prepare_mesh::GpuMeshes,
    render::{RenderPhase, set_blend_func_from_alpha_mode, transparent_draw_from_alpha_mode},
    shader_cached,
};
use itertools::Either;
use uniform_set_derive::UniformSet;

use crate::cascade::{CascadeUniform, CascadeViewUniform, select_cascade, transform_aabb};
use crate::copy_depth_prepass::PrepassTexture;
use crate::draw_debug::DebugLines;
use crate::prepare_lighting::GameLightingUniforms;

#[derive(UniformSet, Resource, Clone, Default)]
#[uniform_set(prefix = "ub_")]
pub struct Fog {
    pub fog_color: Vec4,
    pub caustics: Vec4,
}

pub fn standard_material_render(
    mesh_entities: Query<(
        Entity,
        &ViewVisibility,
        &GlobalTransform,
        &Mesh3d,
        &Aabb,
        &MeshMaterial3d<StandardMaterial>,
        Has<SkipReflection>,
        Has<ReadReflection>,
        Option<&JointData>,
    )>,
    view_uniforms: Single<&ViewUniforms>,
    materials: Res<Assets<StandardMaterial>>,
    phase: Res<RenderPhase>,
    mut transparent_draws: ResMut<DeferredAlphaBlendDraws>,
    reflect_uniforms: Option<Res<ReflectionUniforms>>,
    sorted: Res<DrawsSortedByMaterial>,
    mut enc: ResMut<CommandEncoder>,
    shadow: Option<Res<DirectionalLightShadow>>,
    cascades: Query<&CascadeUniform>,
    view_cascades: Query<&CascadeViewUniform>,
    prepass: Option<ResMut<PrepassTexture>>,
    fog: Option<Res<Fog>>,
    mut debug: ResMut<DebugLines>,
) {
    let view_uniforms = view_uniforms.clone();
    if cascades.iter().len() == 0 {
        warn_once!("No cascades");
    }

    let phase = *phase;

    let iter = if phase.transparent() {
        Either::Right(mesh_entities.iter_many(transparent_draws.take()))
    } else {
        Either::Left(mesh_entities.iter_many(&**sorted))
    };

    struct Draw {
        world_from_local: Mat4,
        joint_data: Option<JointData>,
        material_h: AssetId<StandardMaterial>,
        material_idx: u32,
        read_reflect: bool,
        mesh: Handle<Mesh>,
        cascade_idx: u32,
    }

    let mut draws = Vec::new();
    let mut render_materials: Vec<StandardMaterialUniforms> = Vec::new();

    let v_pos = view_uniforms.view_position.to_vec3a();
    let view_cascade_idx = select_cascade(
        cascades,
        obvhs::aabb::Aabb::new(v_pos - 0.01, v_pos + 0.01),
        &mut debug,
    );

    let mut last_material = None;
    let mut current_material_idx = 0;
    for (
        entity,
        view_vis,
        transform,
        mesh,
        aabb,
        material_h,
        skip_reflect,
        read_reflect,
        joint_data,
    ) in iter
    {
        if (phase.can_use_camera_frustum_cull() && !view_vis.get())
            || (skip_reflect && phase.reflection())
        {
            continue;
        }

        let Some(material) = materials.get(material_h) else {
            continue;
        };

        let world_from_local = transform.to_matrix();

        // If in opaque phase we must defer any alpha blend draws so they can be sorted and run in order.
        if !transparent_draws.maybe_defer::<StandardMaterial>(
            transparent_draw_from_alpha_mode(&material.alpha_mode),
            phase,
            entity,
            transform,
            aabb,
            &view_uniforms.view_from_world,
            &world_from_local,
        ) {
            continue;
        }

        if last_material != Some(material_h) {
            current_material_idx = render_materials.len() as u32;
            last_material = Some(material_h);
            render_materials.push(material.into());
        }

        let draw_aabb = transform_aabb(
            world_from_local,
            obvhs::aabb::Aabb::new(aabb.min(), aabb.max()),
        );
        let cascade_idx = select_cascade(cascades, draw_aabb, &mut debug);

        draws.push(Draw {
            // TODO don't copy full material
            material_idx: current_material_idx,
            world_from_local,
            joint_data: joint_data.cloned(),
            material_h: material_h.id(),
            read_reflect,
            mesh: mesh.0.clone(),
            cascade_idx,
        });
    }

    let reflect_uniforms = reflect_uniforms.as_deref().cloned();

    let shadow = shadow.as_deref().cloned();
    let prepass = prepass.as_deref().cloned();
    let fog = fog.as_deref().cloned();

    let cascades = cascades.iter().cloned().collect::<Vec<_>>();
    let view_cascades = view_cascades.iter().cloned().collect::<Vec<_>>();
    enc.record(move |ctx, world| {
        let can_read_prepass = match phase {
            RenderPhase::ReflectOpaque
            | RenderPhase::ReflectTransparent
            | RenderPhase::Opaque
            | RenderPhase::Transparent => prepass.is_some(),
            _ => false,
        };

        let theres_fog = if let Some(fog) = &fog {
            fog.fog_color != Vec4::ZERO
        } else {
            false
        };

        let theres_caustics = if let Some(fog) = &fog {
            fog.caustics != Vec4::ZERO
        } else {
            false
        };

        let lighting_uniforms = world.resource::<GameLightingUniforms>().clone();
        #[allow(unexpected_cfgs)]
        let shader_index = shader_cached!(
            ctx,
            "../assets/shaders/std_mat.vert",
            "../assets/shaders/std_mat.frag",
            [
                DEFAULT_MAX_LIGHTS_DEF,
                if cascades.is_empty() {
                    ("", "")
                } else {
                    ("CASCADE", "")
                },
                if can_read_prepass {
                    ("READ_PREPASS", "")
                } else {
                    ("", "")
                },
                if theres_fog {
                    ("THERES_FOG", "")
                } else {
                    ("", "")
                },
                if theres_caustics {
                    ("THERES_CAUSTICS", "")
                } else {
                    ("", "")
                }
            ]
            .iter()
            .chain(
                lighting_uniforms
                    .shader_defs(true, shadow.is_some(), &phase)
                    .iter()
            )
            .chain(phase.shader_defs().iter()),
            &[
                ViewUniforms::bindings(),
                StandardMaterialUniforms::bindings(),
                GameLightingUniforms::bindings(),
                CascadeUniform::bindings(),
                CascadeViewUniform::bindings(),
                PrepassTexture::bindings(),
                Fog::bindings(),
            ]
        )
        .unwrap();

        world.resource_mut::<GpuMeshes>().reset_mesh_bind_cache();
        ctx.use_cached_program(shader_index);

        ctx.map_uniform_set_locations::<ViewUniforms>();
        ctx.map_uniform_set_locations::<StandardMaterialUniforms>();
        ctx.map_uniform_set_locations::<CascadeUniform>();
        ctx.map_uniform_set_locations::<CascadeViewUniform>();
        ctx.map_uniform_set_locations::<PrepassTexture>();
        ctx.map_uniform_set_locations::<Fog>();

        ctx.bind_uniforms_set(
            world.resource::<GpuImages>(),
            world.resource::<ViewUniforms>(),
        );

        let mut reflect_bool_location = None;
        if !phase.depth_only() {
            ctx.map_uniform_set_locations::<GameLightingUniforms>();
            ctx.bind_uniforms_set(world.resource::<GpuImages>(), &lighting_uniforms);

            reflect_bool_location = ctx.get_uniform_location("read_reflection");
            ctx.map_uniform_set_locations::<ReflectionUniforms>();
            ctx.bind_uniforms_set(
                world.resource::<GpuImages>(),
                reflect_uniforms.as_ref().unwrap_or(&Default::default()),
            );
            ctx.bind_uniforms_set(
                world.resource::<GpuImages>(),
                fog.as_ref().unwrap_or(&Default::default()),
            );
        }

        if can_read_prepass {
            ctx.bind_uniforms_set(world.resource::<GpuImages>(), &prepass.unwrap());
        }

        let mut last_material = None;
        for draw in &draws {
            let material = &render_materials[draw.material_idx as usize];
            set_blend_func_from_alpha_mode(&ctx.gl, &material.alpha_mode);

            ctx.load("world_from_local", draw.world_from_local);

            if let Some(joint_data) = &draw.joint_data {
                ctx.load("joint_data", joint_data.as_slice());
            }
            ctx.load("has_joint_data", draw.joint_data.is_some());

            let images = world.resource::<GpuImages>();

            if let Some(cascade) = cascades.get(draw.cascade_idx as usize) {
                ctx.bind_uniforms_set(images, cascade);
            } else {
                warn_once!("cascade {} not found", draw.cascade_idx);
            }

            if let Some(cascade) = view_cascades.get(view_cascade_idx as usize) {
                ctx.bind_uniforms_set(images, cascade);
            } else {
                warn_once!("cascade {} not found", draw.cascade_idx);
            }

            if let Some(ref loc) = reflect_bool_location {
                (draw.read_reflect && phase.read_reflect() && reflect_uniforms.is_some())
                    .load(&ctx.gl, &loc)
            }

            // Only re-bind if the material has changed.
            if last_material != Some(draw.material_h) {
                ctx.set_cull_mode(flip_cull_mode(material.cull_mode, phase.reflection()));
                ctx.bind_uniforms_set(world.resource::<GpuImages>(), material);
            }

            world
                .resource_mut::<GpuMeshes>()
                .draw_mesh(ctx, draw.mesh.id(), shader_index);
            last_material = Some(draw.material_h);
        }
    });
}

pub fn generate_tangets(
    mut bevy_meshes: ResMut<Assets<Mesh>>,
    mut mesh_events: MessageReader<AssetEvent<Mesh>>,
) {
    for event in mesh_events.read() {
        let mesh_h = match event {
            AssetEvent::LoadedWithDependencies { id } | AssetEvent::Added { id } => id,
            _ => continue,
        }; // | AssetEvent::Modified { id }
        if let Some(mesh) = bevy_meshes.get_mut(*mesh_h) {
            if mesh.attribute(Mesh::ATTRIBUTE_TANGENT).is_none() {
                mesh.generate_tangents().unwrap();
            }
        }
    }
}
