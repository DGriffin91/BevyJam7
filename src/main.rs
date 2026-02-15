pub mod cascade;
pub mod copy_depth_prepass;
pub mod draw_debug;
pub mod physics;
pub mod player;
pub mod post_process;
pub mod prepare_lighting;
pub mod scene_falling;
pub mod scene_hallway;
pub mod scene_store;
pub mod scene_temple;
pub mod scene_underwater;
pub mod std_mat_render;

use argh::FromArgs;
#[cfg(feature = "dev")]
use bevy::camera_controller::free_camera::FreeCameraState;
use bevy::{
    asset::AssetMetaCheck,
    camera_controller::free_camera::FreeCameraPlugin,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::WindowMode,
    winit::WinitSettings,
};
#[cfg(feature = "dev")]
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

#[cfg(not(target_arch = "wasm32"))]
use bevy_mod_mipmap_generator::{MipmapGeneratorPlugin, generate_mipmaps};

use bgl2::{
    bevy_standard_material::{
        DrawsSortedByMaterial, init_std_shader_includes, sort_std_mat_by_material,
        standard_material_prepare_view,
    },
    command_encoder::CommandEncoder,
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
    player::PlayerControllerPlugin,
    post_process::{PostProcessPlugin, PostProcessSettings},
    prepare_lighting::PrepareLightingPlugin,
    scene_falling::FallingGameplayPlugin,
    scene_hallway::HallwayGameplayPlugin,
    scene_store::StoreSceneGameplayPlugin,
    scene_underwater::UnderwaterGameplayPlugin,
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
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SceneState {
    Init,
    Hallway,
    Store,
    Temple,
    Underwater,
    Falling,
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
                    meta_check: AssetMetaCheck::Never,
                    #[cfg(feature = "dev")]
                    unapproved_path_mode: bevy::asset::UnapprovedPathMode::Allow,
                    #[cfg(not(feature = "dev"))]
                    unapproved_path_mode: bevy::asset::UnapprovedPathMode::Forbid,
                    ..default()
                }),
            FreeCameraPlugin,
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .insert_state(SceneState::Init);

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
                GlowEguiPlugin,
                OpenGLRenderPlugins,
                PrepareLightingPlugin,
                DrawDebugPlugin,
                PostProcessPlugin,
                PlayerControllerPlugin,
                StoreSceneGameplayPlugin,
                HallwayGameplayPlugin,
                UnderwaterGameplayPlugin,
                FallingGameplayPlugin,
            ))
            .add_systems(
                PostUpdate,
                sort_std_mat_by_material.in_set(RenderSet::Prepare),
            )
            .add_systems(
                Startup,
                init_std_shader_includes.in_set(RenderSet::Pipeline),
            )
            .add_systems(Startup, pre_load_some_assets);
        register_prepare_system(app.world_mut(), standard_material_prepare_view);
        //register_prepare_system(app.world_mut(), copy_depth_prepass);
        register_render_system::<StandardMaterial, _>(
            app.world_mut(),
            std_mat_render::standard_material_render,
        );

        app.add_systems(
            Startup,
            (|mut enc: ResMut<CommandEncoder>| {
                enc.record(|ctx, _world| {
                    ctx.add_shader_include("game::caustics", include_str!("shaders/caustics.glsl"));
                });
            })
            .in_set(RenderSet::Pipeline),
        );
        #[cfg(feature = "dev")]
        app.add_systems(EguiPrimaryContextPass, (dev_ui, drag_drop_gltf));
    }

    app.init_resource::<Fog>()
        .add_plugins(ConvertCascadePlugin)
        .add_systems(
            Startup,
            (setup, scene_store::load_store)
                .chain()
                .after(init_std_shader_includes),
        )
        .add_systems(Update, window_control)
        .add_systems(Update, generate_tangets);

    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(MipmapGeneratorPlugin)
        .add_systems(Update, generate_mipmaps::<StandardMaterial>);

    #[cfg(target_arch = "wasm32")]
    app.add_systems(
        Update,
        sync_canvas_and_window_size
            .in_set(RenderSet::Present)
            .after(bgl2::render::present),
    );

    app.run();
}

#[cfg(feature = "dev")]
fn dev_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    #[cfg(feature = "asset_baking")] cascades: Query<Entity, With<CascadeData>>,
    mut camera: Option<Single<&mut FreeCameraState>>,
) {
    if let Ok(ctx) = contexts.ctx_mut() {
        let wants_input = ctx.wants_pointer_input() || ctx.wants_keyboard_input();
        if let Some(camera) = &mut camera {
            camera.enabled = !wants_input;
        }
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
        }
        if ui.button("Load Store").clicked() {
            use crate::scene_store::load_store;
            commands.run_system_cached(despawn_scene_contents);
            commands.run_system_cached(load_store);
        }
        if ui.button("Load hallway").clicked() {
            use crate::scene_hallway::load_hallway;
            commands.run_system_cached(despawn_scene_contents);
            commands.run_system_cached(load_hallway);
        }
        if ui.button("Load Temple").clicked() {
            use crate::scene_temple::load_temple;
            commands.run_system_cached(despawn_scene_contents);
            commands.run_system_cached(load_temple);
        }
        if ui.button("Load Underwater").clicked() {
            use crate::scene_underwater::load_underwater;
            commands.run_system_cached(despawn_scene_contents);
            commands.run_system_cached(load_underwater);
        }
        if ui.button("Load Falling").clicked() {
            use crate::scene_falling::load_falling;
            commands.run_system_cached(despawn_scene_contents);
            commands.run_system_cached(load_falling);
        }
    });
}

fn setup(mut commands: Commands) {
    // Sun
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(vec3(4.0, -10.0, 3.9), Vec3::Y),
        DirectionalLight {
            color: Color::srgb(1.0, 0.9, 0.8),
            illuminance: 0.0,
            shadows_enabled: false,
            shadow_depth_bias: 0.0,
            shadow_normal_bias: 0.0,
            ..default()
        },
        ShadowBounds::cube(100.0),
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
    if keyboard_input.just_pressed(KeyCode::Escape) || keyboard_input.just_pressed(KeyCode::Tab) {
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
        if let FileDragAndDrop::DroppedFile { path_buf, .. } = e
            && added.insert(path_buf.clone())
        {
            use crate::cascade::SceneBakeName;

            let path = relative_to_assets(path_buf).unwrap();
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
}

#[cfg(feature = "dev")]
fn relative_to_assets(path: &std::path::Path) -> Option<std::path::PathBuf> {
    pathdiff::diff_paths(path, std::env::current_dir().ok()?.join("assets"))
}

#[derive(Component)]
pub struct SceneContents;

pub fn despawn_scene_contents(
    mut commands: Commands,
    scene_contents: Query<Entity, With<SceneContents>>,
) {
    for entity in &scene_contents {
        commands.entity(entity).despawn();
    }
}

fn pre_load_some_assets(asset_server: Res<AssetServer>, mut scenes: Local<Vec<Handle<Scene>>>) {
    // Usually I use bevy_asset_loader but I ran out of time and forgot at the start
    for path in [
        "testing/models/Falling.gltf",
        "testing/models/Hallway.gltf",
        "testing/models/hallway_collider_mesh.gltf",
        "testing/models/store_single_box.gltf",
        "testing/models/hallway_ghost.gltf",
        "testing/models/store_single_box.gltf",
        "testing/models/store_shelf.gltf",
        "testing/models/store_cart.gltf",
        "testing/models/store_boxes_on_floor.gltf",
        "testing/models/Store.gltf",
        "testing/models/store_single_box.gltf",
        "testing/models/store_mac_shelf.gltf",
        "testing/models/store_mac_anim.gltf",
        "testing/models/store_mac_anim.gltf",
        "testing/models/Underwater.gltf",
        "testing/models/underwater_skybox.gltf",
        "testing/models/underwater_airship.gltf",
        "testing/models/underwater_collider_mesh.gltf",
    ] {
        scenes.push(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path)));
    }
}

#[cfg(target_arch = "wasm32")]
fn sync_canvas_and_window_size(mut bevy_window: Single<(Entity, &mut Window)>) {
    use winit::platform::web::WindowExtWebSys;
    let (bevy_window_entity, window) = &mut *bevy_window;
    bevy::winit::WINIT_WINDOWS.with_borrow(|winit_windows| {
        let Some(winit_window) = winit_windows.get_window(*bevy_window_entity) else {
            return;
        };
        let canvas = winit_window.canvas().unwrap();
        let rect = canvas.get_bounding_client_rect();
        let css_w = rect.width().max(1.0);
        let css_h = rect.height().max(1.0);
        let dpr = web_sys::window().unwrap().device_pixel_ratio().max(1.0);
        let phys_w = (css_w * dpr).round().max(1.0) as u32;
        let phys_h = (css_h * dpr).round().max(1.0) as u32;
        if canvas.width() != phys_w || canvas.height() != phys_h {
            canvas.set_width(phys_w);
            canvas.set_height(phys_h);
            window.resolution.set_physical_resolution(phys_w, phys_h);
            window.resolution.set_scale_factor(dpr as f32);
        }
    });
}
