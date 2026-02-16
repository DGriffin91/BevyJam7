use bevy::{prelude::*, window::WindowMode};
use bevy_egui::{
    EguiContexts, EguiPrimaryContextPass,
    egui::{self, TextStyle},
};
use bevy_fps_controller::controller::FpsController;

use crate::{SceneState, despawn_scene_contents, scene_store::load_store};

pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, window_control)
            .add_systems(EguiPrimaryContextPass, menu_ui); //.run_if(in_state(GameLoading::Loaded))
    }
}

pub fn menu_ui(
    mut commands: Commands,
    fps_controller: Single<&mut FpsController>,
    window: Single<&mut Window>,
    mut contexts: EguiContexts,
    mut app_exit: MessageWriter<AppExit>,
    state: Res<State<SceneState>>,
    #[cfg(feature = "asset_baking")] cascades: Query<Entity, With<CascadeData>>,
    #[cfg(feature = "dev")] mut camera: Option<
        Single<&mut bevy::camera_controller::free_camera::FreeCameraState>,
    >,
) {
    let mut window = window.into_inner();
    let mut fps_controller = fps_controller.into_inner();

    if fps_controller.enable_input {
        return;
    }
    let height = window.height();
    let width = 250.0;

    let Ok(context) = contexts.ctx_mut() else {
        return;
    };

    #[cfg(feature = "dev")]
    {
        let wants_input = context.wants_pointer_input() || context.wants_keyboard_input();
        if let Some(camera) = &mut camera {
            camera.enabled = !wants_input;
        }
    }

    let loading = matches!(state.get(), SceneState::Loading);
    egui::Window::new("SETTINGS")
        .fixed_pos(egui::Pos2::ZERO)
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .fixed_size(egui::vec2(width, height))
        .frame(
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(12, 12, 12))
                .stroke(egui::Stroke::NONE)
                .corner_radius(0.0)
                .inner_margin(16.0)
                .outer_margin(0.0),
        )
        .show(context, |ui| {
            let style = ui.style_mut();
            let font = egui::FontId::new(
                if loading { 20.0 } else { 18.0 },
                egui::FontFamily::Proportional,
            );
            style.text_styles.insert(TextStyle::Body, font.clone());
            style.text_styles.insert(TextStyle::Button, font.clone());

            ui.allocate_space(egui::vec2(width, 40.0));
            ui.spacing_mut().slider_width = ui.available_width();

            if loading {
                ui.label("LOADING...");
                ui.label("");
                ui.allocate_space(egui::vec2(width, height));
                return;
            }

            ui.label("");
            ui.label("MOUSE SENSITIVITY");
            let mut sens = fps_controller.sensitivity * 1000.0;
            if ui.add(egui::Slider::new(&mut sens, 0.1..=10.0)).changed() {
                fps_controller.sensitivity = sens / 1000.0;
            }

            //if ui
            //    .add(egui::Slider::new(&mut music.volume, 0.0..=2.0).text("MUSIC VOLUME"))
            //    .changed()
            //{
            //    if let Some(track) = tracks.get_mut(&music.handle) {
            //        track
            //            .0
            //            .set_volume(music.volume as f64, kira::tween::Tween::default());
            //    }
            //}

            //if ui
            //    .add(egui::Slider::new(&mut sfx.volume, 0.0..=2.0).text("SFX VOLUME"))
            //    .changed()
            //{
            //    if let Some(track) = tracks.get_mut(&sfx.handle) {
            //        track
            //            .0
            //            .set_volume(sfx.volume as f64, kira::tween::Tween::default());
            //    }
            //}

            ui.allocate_space(egui::vec2(width, 40.0));
            ui.label("WINDOW MODE");
            if ui
                .radio(
                    window.mode == WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    "BORDERLESS FULLSCREEN",
                )
                .clicked()
            {
                window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
            }
            if ui
                .radio(
                    window.mode
                        == WindowMode::Fullscreen(
                            MonitorSelection::Current,
                            VideoModeSelection::Current,
                        ),
                    "FULLSCREEN",
                )
                .clicked()
            {
                window.mode =
                    WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current);
            }
            if ui
                .radio(window.mode == WindowMode::Windowed, "WINDOWED")
                .clicked()
            {
                window.mode = WindowMode::Windowed;
            }

            ui.allocate_space(egui::vec2(width, 40.0));

            if ui.button("RESTART GAME").clicked() {
                commands.run_system_cached(despawn_scene_contents);
                commands.run_system_cached(load_store);
            }
            if ui.button("EXIT GAME").clicked() {
                app_exit.write(AppExit::Success);
            }

            #[cfg(feature = "dev")]
            {
                #[cfg(feature = "asset_baking")]
                {
                    use light_volume_baker::gpu_rt::NeedsGpuBake;
                    use light_volume_baker::{NeedsCourseBake, NeedsFineBake};
                    if ui.button("Rebake All").clicked() {
                        for entity in &cascades {
                            commands.entity(entity).insert((
                                NeedsGpuBake,
                                NeedsCourseBake,
                                NeedsFineBake,
                            ));
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
            }

            ui.allocate_space(egui::vec2(width, height));
        });
}

fn window_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut window: Single<&mut Window>,
    fps_controller: Single<&mut FpsController>,
) {
    if keyboard_input.just_pressed(KeyCode::F11) || keyboard_input.just_pressed(KeyCode::KeyF) {
        if window.mode == WindowMode::Windowed {
            window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
        } else {
            window.mode = WindowMode::Windowed;
        }
    }
    if !fps_controller.enable_input && keyboard_input.just_pressed(KeyCode::Escape) {
        window.mode = WindowMode::Windowed;
    }
}
