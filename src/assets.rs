use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_seedling::sample::AudioSample;

#[derive(AssetCollection, Resource)]
pub struct SceneAssets {
    #[asset(path = "models/Falling.gltf#Scene0")]
    pub falling: Handle<Scene>,
    #[asset(path = "models/Hallway.gltf#Scene0")]
    pub hallway: Handle<Scene>,
    #[asset(path = "models/hallway_collider_mesh.gltf#Scene0")]
    pub hallway_collider_mesh: Handle<Scene>,
    #[asset(path = "models/store_single_box.gltf#Scene0")]
    pub store_single_box: Handle<Scene>,
    #[asset(path = "models/hallway_ghost.gltf#Scene0")]
    pub hallway_ghost: Handle<Scene>,
    #[asset(path = "models/store_shelf.gltf#Scene0")]
    pub store_shelf: Handle<Scene>,
    #[asset(path = "models/store_cart.gltf#Scene0")]
    pub store_cart: Handle<Scene>,
    #[asset(path = "models/store_boxes_on_floor.gltf#Scene0")]
    pub store_boxes_on_floor: Handle<Scene>,
    #[asset(path = "models/Store.gltf#Scene0")]
    pub store: Handle<Scene>,
    #[asset(path = "models/store_mac_shelf.gltf#Scene0")]
    pub store_mac_shelf: Handle<Scene>,
    #[asset(path = "models/store_mac_anim.gltf#Scene0")]
    pub store_mac_anim: Handle<Scene>,
    #[asset(path = "models/Underwater.gltf#Scene0")]
    pub underwater: Handle<Scene>,
    #[asset(path = "models/underwater_skybox.gltf#Scene0")]
    pub underwater_skybox: Handle<Scene>,
    #[asset(path = "models/underwater_airship.gltf#Scene0")]
    pub underwater_airship: Handle<Scene>,
    #[asset(path = "models/underwater_collider_mesh.gltf#Scene0")]
    pub underwater_collider_mesh: Handle<Scene>,
}

#[derive(AssetCollection, Resource)]
pub struct AudioAssets {
    #[asset(path = "audio/chug.ogg")]
    pub chug: Handle<AudioSample>,
    #[asset(path = "audio/big_box.ogg")]
    pub big_box: Handle<AudioSample>,
    #[asset(path = "audio/store_music.ogg")]
    pub store_music: Handle<AudioSample>,
    #[asset(path = "audio/store_ramp.ogg")]
    pub store_ramp: Handle<AudioSample>,

    #[asset(path = "audio/underwater_music.ogg")]
    pub underwater_music: Handle<AudioSample>,
    #[asset(path = "audio/underwater_beam.ogg")]
    pub underwater_beam: Handle<AudioSample>,

    #[asset(path = "audio/hallway_music.ogg")]
    pub hallway_music: Handle<AudioSample>,

    #[asset(path = "audio/end_music.ogg")]
    pub end_music: Handle<AudioSample>,
}
