pub mod cascade;

use std::f32::consts::PI;

use argh::FromArgs;
use bevy::{
    camera::primitives::Aabb,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    light::light_consts::lux::DIRECT_SUNLIGHT,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{PresentMode, WindowMode},
    winit::WinitSettings,
};
use bevy_mod_mipmap_generator::{MipmapGeneratorPlugin, generate_mipmaps};
use bgl2::{
    UniformSet, UniformValue,
    bevy_standard_lighting::{OpenGLStandardLightingPlugin, StandardLightingUniforms},
    bevy_standard_material::{
        DrawsSortedByMaterial, ReadReflection, SkipReflection, StandardMaterialUniforms,
        ViewUniforms, init_std_shader_includes, sort_std_mat_by_material,
        standard_material_prepare_view,
    },
    command_encoder::CommandEncoder,
    flip_cull_mode,
    phase_shadow::{DirectionalLightShadow, ShadowBounds},
    phase_transparent::DeferredAlphaBlendDraws,
    plane_reflect::ReflectionUniforms,
    prepare_image::GpuImages,
    prepare_joints::JointData,
    prepare_mesh::GpuMeshes,
    render::{
        OpenGLRenderPlugins, RenderPhase, RenderSet, register_prepare_system,
        set_blend_func_from_alpha_mode, transparent_draw_from_alpha_mode,
    },
    shader_cached,
};
use bgl2::{bevy_standard_lighting::DEFAULT_MAX_LIGHTS_DEF, render::register_render_system};
use itertools::Either;

#[cfg(feature = "asset_baking")]
use light_volume_baker::{
    cpu_probes::{CpuProbesPlugin, RunProbeDebug},
    pt_reference_camera::PtReferencePlugin,
    rt_scene::{RtEnvColor, RtScenePlugin},
    softbuffer_plugin::SoftBufferPlugin,
};

use crate::cascade::{CascadeInput, CascadeUniform, generate_cascade_data, select_cascade};

#[derive(FromArgs, Resource, Clone, Default)]
/// Config
pub struct Args {
    #[cfg(feature = "asset_baking")]
    /// render using reference PT, see what the probes see
    #[argh(switch)]
    reference_pt: bool,
    #[cfg(feature = "asset_baking")]
    /// cpu render first cascade
    #[argh(switch)]
    probe_debug: bool,
    #[cfg(feature = "asset_baking")]
    /// include probe baking plugin
    #[argh(switch)]
    bake: bool,
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    #[allow(unused)]
    let args: Args = Default::default();
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(unused)]
    let args: Args = argh::from_env();

    let mut app = App::new();
    #[cfg(feature = "asset_baking")]
    app.insert_resource(RtEnvColor(vec3a(0.32, 0.4, 0.47) * 0.0));
    app.insert_resource(ClearColor(Color::srgb(0.32, 0.4, 0.47)))
        .insert_resource(WinitSettings::continuous())
        .insert_resource(GlobalAmbientLight::NONE)
        .add_plugins((
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: WgpuSettings {
                        backends: None,
                        ..default()
                    }
                    .into(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::Immediate,
                        ..default()
                    }),
                    ..default()
                }),
            FreeCameraPlugin,
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            MipmapGeneratorPlugin,
        ));

    #[cfg(feature = "asset_baking")]
    {
        if args.reference_pt || args.probe_debug || args.bake {
            app.add_plugins(RtScenePlugin);
        }
        if args.probe_debug {
            app.init_resource::<RunProbeDebug>();
        }
        if args.probe_debug || args.bake {
            app.add_plugins(CpuProbesPlugin);
        }
        if args.reference_pt {
            app.add_plugins(PtReferencePlugin);
        }
    }

    #[cfg(feature = "asset_baking")]
    let bgl2_render = !(args.reference_pt || args.probe_debug);
    #[cfg(not(feature = "asset_baking"))]
    let bgl2_render = true;

    #[cfg(feature = "asset_baking")]
    if args.reference_pt || args.probe_debug {
        app.add_plugins(SoftBufferPlugin);
    }

    if bgl2_render {
        app.init_resource::<DrawsSortedByMaterial>()
            .add_plugins((OpenGLRenderPlugins, OpenGLStandardLightingPlugin))
            .add_systems(
                PostUpdate,
                sort_std_mat_by_material.in_set(RenderSet::Prepare),
            )
            .add_systems(
                Startup,
                init_std_shader_includes.in_set(RenderSet::Pipeline),
            );
        register_prepare_system(app.world_mut(), standard_material_prepare_view);
        register_render_system::<StandardMaterial, _>(app.world_mut(), standard_material_render);
    }

    app.add_systems(Update, generate_cascade_data)
        .add_systems(Startup, setup)
        .add_systems(Update, generate_mipmaps::<StandardMaterial>)
        .add_systems(Update, window_control)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let start = vec3a(-47.5, 0.1, -25.5);
    let end = vec3a(36.0, 56.0, 34.0) * 2.0 + start;
    commands.spawn(CascadeInput {
        name: String::from("nave"),
        ws_aabb: obvhs::aabb::Aabb::new(start, end),
        resolution: vec3a(2.0, 2.0, 2.0),
    });

    let start = vec3a(10.5, 0.1, -25.5);
    let end = vec3a(26.0, 86.0, 34.0) * 2.0 + start;
    commands.spawn(CascadeInput {
        name: String::from("tower"),
        ws_aabb: obvhs::aabb::Aabb::new(start, end),
        resolution: vec3a(2.0, 2.0, 2.0),
    });

    // Sun
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(vec3(4.0, -10.0, 3.9), Vec3::Y),
        DirectionalLight {
            color: Color::srgb(1.0, 0.9, 0.8),
            illuminance: DIRECT_SUNLIGHT,
            shadows_enabled: true,
            shadow_depth_bias: 0.0,
            shadow_normal_bias: 0.0,
            ..default()
        },
        ShadowBounds::cube(250.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-45.0, 4.0, 0.0).looking_at(Vec3::new(0.0, 18.0, 0.0), Vec3::Y),
        FreeCamera {
            walk_speed: 5.0,
            run_speed: 30.0,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: PI / 3.0,
            ..default()
        }),
    ));

    commands.spawn(SceneRoot(asset_server.load(
        //GltfAssetLabel::Scene(0).from_asset("testing/models/temple/temple.gltf"),
        GltfAssetLabel::Scene(0).from_asset("testing/models/temple_test/temple_test.gltf"),
    )));
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
) {
    let view_uniforms = view_uniforms.clone();
    if cascades.iter().len() == 0 {
        warn!("No cascades");
        return; // TODO support no cascades
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

        let ws_radius = transform.radius_vec3a(aabb.half_extents);
        let ws_center = world_from_local.transform_point3a(aabb.center);
        let draw_aabb = obvhs::aabb::Aabb::new(ws_center, Vec3A::splat(ws_radius));

        draws.push(Draw {
            // TODO don't copy full material
            material_idx: current_material_idx,
            world_from_local,
            joint_data: joint_data.cloned(),
            material_h: material_h.id(),
            read_reflect,
            mesh: mesh.0.clone(),
            cascade_idx: select_cascade(cascades, draw_aabb),
        });
    }

    let reflect_uniforms = reflect_uniforms.as_deref().cloned();

    let shadow = shadow.as_deref().cloned();

    let cascades = cascades.iter().cloned().collect::<Vec<_>>();
    enc.record(move |ctx, world| {
        let lighting_uniforms = world.resource::<StandardLightingUniforms>().clone();
        #[allow(unexpected_cfgs)]
        let shader_index = shader_cached!(
            ctx,
            "../assets/shaders/temple_mat.vert",
            "../assets/shaders/temple_mat.frag",
            [DEFAULT_MAX_LIGHTS_DEF]
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
                StandardLightingUniforms::bindings(),
                CascadeUniform::bindings(),
            ]
        )
        .unwrap();

        world.resource_mut::<GpuMeshes>().reset_mesh_bind_cache();
        ctx.use_cached_program(shader_index);

        ctx.map_uniform_set_locations::<ViewUniforms>();
        ctx.map_uniform_set_locations::<StandardMaterialUniforms>();
        ctx.map_uniform_set_locations::<CascadeUniform>();
        ctx.bind_uniforms_set(
            world.resource::<GpuImages>(),
            world.resource::<ViewUniforms>(),
        );

        let mut reflect_bool_location = None;
        if !phase.depth_only() {
            ctx.map_uniform_set_locations::<StandardLightingUniforms>();
            ctx.bind_uniforms_set(world.resource::<GpuImages>(), &lighting_uniforms);

            reflect_bool_location = ctx.get_uniform_location("read_reflection");
            ctx.map_uniform_set_locations::<ReflectionUniforms>();
            ctx.bind_uniforms_set(
                world.resource::<GpuImages>(),
                reflect_uniforms.as_ref().unwrap_or(&Default::default()),
            );
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
            ctx.bind_uniforms_set(images, &cascades[draw.cascade_idx as usize]);

            if let Some(loc) = reflect_bool_location {
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

fn window_control(keyboard_input: Res<ButtonInput<KeyCode>>, mut window: Single<&mut Window>) {
    if keyboard_input.just_pressed(KeyCode::F11) || keyboard_input.just_pressed(KeyCode::KeyF) {
        if window.mode == WindowMode::Windowed {
            window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
        } else {
            window.mode = WindowMode::Windowed;
        }
    }
    if keyboard_input.just_pressed(KeyCode::Escape) {
        window.mode = WindowMode::Windowed;
    }
}
