pub mod cascade;
pub mod std_mat_render;

use std::f32::consts::PI;

use argh::FromArgs;
use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    light::light_consts::lux::DIRECT_SUNLIGHT,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{PresentMode, WindowMode},
    winit::WinitSettings,
};
use bevy_mod_mipmap_generator::{MipmapGeneratorPlugin, generate_mipmaps};
use bgl2::render::register_render_system;
use bgl2::{
    bevy_standard_lighting::OpenGLStandardLightingPlugin,
    bevy_standard_material::{
        DrawsSortedByMaterial, init_std_shader_includes, sort_std_mat_by_material,
        standard_material_prepare_view,
    },
    phase_shadow::ShadowBounds,
    render::{OpenGLRenderPlugins, RenderSet, register_prepare_system},
};

#[cfg(feature = "asset_baking")]
use light_volume_baker::{
    cpu_probes::{CpuProbesPlugin, RunProbeDebug},
    pt_reference_camera::PtReferencePlugin,
    rt_scene::{RtEnvColor, RtScenePlugin},
    softbuffer_plugin::SoftBufferPlugin,
};

use crate::{
    cascade::{CascadeInput, generate_cascade_data},
    std_mat_render::standard_material_render,
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
