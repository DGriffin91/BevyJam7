pub mod cascade;
pub mod copy_depth_prepass;
pub mod draw_debug;
pub mod post_process;
pub mod prepare_lighting;
pub mod scene_hallway;
pub mod scene_store;
pub mod scene_temple;
pub mod std_mat_render;

use std::f32::consts::PI;

use argh::FromArgs;
#[cfg(feature = "dev")]
use bevy::camera_controller::free_camera::FreeCameraState;
use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    core_pipeline::prepass::DepthPrepass,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    light::light_consts::lux::DIRECT_SUNLIGHT,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::WindowMode,
    winit::WinitSettings,
};
#[cfg(feature = "dev")]
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use bevy_mod_mipmap_generator::{MipmapGeneratorPlugin, generate_mipmaps};
use bgl2::{
    bevy_standard_material::{
        DrawsSortedByMaterial, init_std_shader_includes, sort_std_mat_by_material,
        standard_material_prepare_view,
    },
    phase_shadow::ShadowBounds,
    render::{OpenGLRenderPlugins, RenderSet, register_prepare_system},
};
use bgl2::{egui_plugin::GlowEguiPlugin, render::register_render_system};

#[cfg(feature = "asset_baking")]
use light_volume_baker::{
    CascadeData,
    cpu_probes::{CpuProbesPlugin, RunProbeDebug},
    pt_reference_camera::PtReferencePlugin,
    rt_scene::{RtEnvColor, RtScenePlugin},
    softbuffer_plugin::SoftBufferPlugin,
};

use crate::{
    cascade::ConvertCascadePlugin,
    draw_debug::DrawDebugPlugin,
    post_process::{PostProcessPlugin, PostProcessSettings},
    prepare_lighting::PrepareLightingPlugin,
    std_mat_render::{Fog, generate_tangets},
};

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
    /// temple test scene
    #[argh(switch)]
    temple: bool,
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    #[allow(unused)]
    let args: Args = Default::default();
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(unused)]
    let args: Args = argh::from_env();

    let mut app = App::new();
    app.insert_resource(args.clone());
    #[cfg(feature = "asset_baking")]
    app.insert_resource(RtEnvColor(vec3a(0.32, 0.4, 0.47) * 0.0));
    app.init_resource::<PostProcessSettings>()
        .insert_resource(ClearColor(Color::srgb(0.32, 0.4, 0.47)))
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
                        present_mode: bevy::window::PresentMode::Immediate,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    #[cfg(feature = "dev")]
                    unapproved_path_mode: bevy::asset::UnapprovedPathMode::Allow,
                    #[cfg(not(feature = "dev"))]
                    unapproved_path_mode: bevy::asset::UnapprovedPathMode::Forbid,
                    ..default()
                }),
            FreeCameraPlugin,
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            MipmapGeneratorPlugin,
        ));

    #[cfg(feature = "asset_baking")]
    {
        use light_volume_baker::{cpu_probes::CpuProbeSamples, gpu_rt::GpuRtPlugin};

        app.add_plugins(RtScenePlugin);
        if args.probe_debug {
            app.init_resource::<RunProbeDebug>();
        }
        app.insert_resource(CpuProbeSamples(0))
            .add_plugins((CpuProbesPlugin, GpuRtPlugin));
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
            .add_plugins((
                GlowEguiPlugin::default(),
                OpenGLRenderPlugins,
                PrepareLightingPlugin,
                DrawDebugPlugin,
                PostProcessPlugin,
            ))
            .add_systems(
                PostUpdate,
                sort_std_mat_by_material.in_set(RenderSet::Prepare),
            )
            .add_systems(
                Startup,
                init_std_shader_includes.in_set(RenderSet::Pipeline),
            );
        register_prepare_system(app.world_mut(), standard_material_prepare_view);
        //register_prepare_system(app.world_mut(), copy_depth_prepass);
        register_render_system::<StandardMaterial, _>(
            app.world_mut(),
            std_mat_render::standard_material_render,
        );

        #[cfg(feature = "dev")]
        app.add_systems(EguiPrimaryContextPass, (dev_ui, drag_drop_gltf));
    }

    app.init_resource::<Fog>()
        .add_plugins((
            ConvertCascadePlugin, //PostProcessPlugin
        ))
        .add_systems(Startup, (setup, scene_store::load_store).chain())
        .add_systems(Update, generate_mipmaps::<StandardMaterial>)
        .add_systems(Update, window_control)
        .add_systems(Update, generate_tangets)
        .run();
}

#[cfg(feature = "dev")]
fn dev_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    #[cfg(feature = "asset_baking")] cascades: Query<Entity, With<CascadeData>>,
    mut camera: Single<&mut FreeCameraState>,
    scene_contents: Query<Entity, With<SceneContents>>,
) {
    if let Ok(ctx) = contexts.ctx_mut() {
        camera.enabled = !(ctx.wants_pointer_input() || ctx.wants_keyboard_input());
    }

    egui::Window::new("Dev Utils").show(contexts.ctx_mut().unwrap(), |ui| {
        #[cfg(feature = "asset_baking")]
        {
            use light_volume_baker::gpu_rt::NeedsGpuBake;
            use light_volume_baker::{NeedsCourseBake, NeedsFineBake};
            if ui.button("Rebake All").clicked() {
                for entity in &cascades {
                    commands
                        .entity(entity)
                        .insert((NeedsGpuBake, NeedsCourseBake, NeedsFineBake));
                }
            }
            let mut despawn_scene_contents = false;
            if ui.button("Load Store").clicked() {
                use crate::scene_store::load_store;
                let id = commands.register_system(load_store);
                commands.run_system(id);
                despawn_scene_contents = true;
            }
            if ui.button("Load hallway").clicked() {
                use crate::scene_hallway::load_hallway;
                let id = commands.register_system(load_hallway);
                commands.run_system(id);
                despawn_scene_contents = true;
            }
            if ui.button("Load Temple").clicked() {
                use crate::scene_temple::load_temple;
                let id = commands.register_system(load_temple);
                commands.run_system(id);
                despawn_scene_contents = true;
            }

            if despawn_scene_contents {
                for entity in &scene_contents {
                    commands.entity(entity).despawn();
                }
            }
        }
    });
}

fn setup(mut commands: Commands, args: Res<Args>) {
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
        ShadowBounds::cube(if args.temple { 250.0 } else { 100.0 }),
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
        DepthPrepass,
    ));
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

#[cfg(feature = "dev")]
fn drag_drop_gltf(
    mut drag_and_drop_reader: MessageReader<FileDragAndDrop>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut added: Local<bevy::platform::collections::HashSet<std::path::PathBuf>>,
) {
    for e in drag_and_drop_reader.read() {
        match e {
            FileDragAndDrop::DroppedFile { path_buf, .. } => {
                if added.insert(path_buf.clone()) {
                    use crate::cascade::SceneBakeName;

                    let path = relative_to_assets(&path_buf).unwrap();
                    let scene_bake_name = path
                        .file_prefix()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                        .to_string();
                    commands
                        .spawn((
                            SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path))),
                            SceneBakeName(scene_bake_name),
                        ))
                        .observe(cascade::blender_cascades);
                }
            }
            _ => (),
        }
    }
}

#[cfg(feature = "dev")]
fn relative_to_assets(path: &std::path::Path) -> Option<std::path::PathBuf> {
    pathdiff::diff_paths(path, std::env::current_dir().ok()?.join("assets"))
}

#[derive(Component)]
pub struct SceneContents;
