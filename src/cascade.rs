use bevy::{
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    platform::collections::HashMap,
    prelude::*,
    scene::SceneInstanceReady,
};

#[derive(Resource, Default)]
pub struct ConvertCascadePlugin;

impl Plugin for ConvertCascadePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (generate_cascade_data).chain());
    }
}

#[derive(Debug, Deserialize)]
struct ProbeBakeExtras {
    #[serde(rename = "probe bake res")]
    probe_bake_res: Option<Vec<f32>>,

    #[serde(flatten)]
    other: HashMap<String, serde_json::Value>,
}

pub fn blender_cascades(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    gltf_extras: Query<(Entity, &GlobalTransform, &Name, &GltfExtras)>,
) {
    for entity in children.iter_descendants(scene_ready.entity) {
        if let Ok((entity, trans, name, extras)) = gltf_extras.get(entity) {
            if name.contains("BAKE") {
                let extras: ProbeBakeExtras = serde_json::from_str(&extras.value).unwrap();
                if let Some(bake_res) = extras.probe_bake_res {
                    let scale: Vec3A = trans.scale().into();
                    let start = trans.translation_vec3a() - scale;
                    let end = trans.translation_vec3a() + scale;
                    commands
                        .entity(entity)
                        .insert(CascadeInput {
                            name: String::from("test_gltf"),
                            ws_aabb: obvhs::aabb::Aabb::new(start, end),
                            resolution: vec3a(bake_res[0], bake_res[1], bake_res[2]),
                        })
                        .remove::<CascadeData>()
                        .remove::<CascadeUniform>();
                }
            }
        }
    }
}

#[cfg(feature = "asset_baking")]
use light_volume_baker::CascadeData;
use obvhs::ray::Ray;
use serde::Deserialize;
use uniform_set_derive::UniformSet;

pub fn select_cascade<'a, I>(cascades: I, draw_aabb: obvhs::aabb::Aabb) -> u32
where
    I: IntoIterator<Item = &'a CascadeUniform>,
{
    let mut draw_dist_to_cascade = f32::MAX;
    let mut best_cascade = 0;
    for (i, cascade) in cascades.into_iter().enumerate() {
        let cascade_aabb = obvhs::aabb::Aabb::new(
            cascade.cascade_position.into(),
            (cascade.cascade_position + cascade.cascade_res * cascade.cascade_spacing).into(),
        );
        let outside_weight = 10.0; // TODO do better
        let cascade_intersection = cascade_aabb.intersection(&draw_aabb);
        let dist_to_cascade = if cascade_intersection.valid() {
            draw_aabb.diagonal().length() / cascade_intersection.diagonal().length()
        } else {
            cascade_aabb.intersect_ray(&Ray::new_inf(
                draw_aabb.center(),
                (cascade_aabb.center() - draw_aabb.center()).normalize_or_zero(),
            )) * outside_weight
        };
        if dist_to_cascade < draw_dist_to_cascade {
            draw_dist_to_cascade = dist_to_cascade;
            best_cascade = i;
        }
    }
    best_cascade as u32
}

#[derive(Component, Clone)]
pub struct CascadeInput {
    pub name: String,
    pub ws_aabb: obvhs::aabb::Aabb,
    pub resolution: Vec3A,
}

#[derive(UniformSet, Component, Clone)]
#[uniform_set(prefix = "ub_")]
pub struct CascadeUniform {
    pub probes_gi: Handle<Image>,
    pub probes_id: Handle<Image>,
    pub cascade_position: Vec3,
    pub cascade_res: Vec3,
    pub cascade_spacing: Vec3,
    /// Before padding
    pub probe_size: f32,
    pub gi_texel: f32,
    pub id_texel: Vec2,
}

impl CascadeInput {
    pub fn into_uniform(&self, asset_server: &AssetServer) -> CascadeUniform {
        let cascade_res = self.ws_aabb.diagonal() / self.resolution;
        CascadeUniform {
            probes_gi: asset_server.load(format!("bake/probes_gi_{}.png", self.name)),
            probes_id: asset_server.load_with_settings(
                format!("bake/probes_id_{}.png", self.name),
                |settings: &mut ImageLoaderSettings| settings.sampler = sampler_nearest_clamp(),
            ),
            cascade_position: self.ws_aabb.min.into(),
            cascade_res: cascade_res.into(),
            cascade_spacing: self.resolution.into(),
            probe_size: 6.0,
            gi_texel: 1.0 / 2048.0,
            id_texel: vec2(1.0 / cascade_res.x, 1.0 / (cascade_res.y * cascade_res.z)),
        }
    }

    #[cfg(feature = "asset_baking")]
    pub fn into_cascade_data(&self) -> CascadeData {
        let cascade_res = self.ws_aabb.diagonal() / self.resolution;
        CascadeData {
            name: self.name.clone(),
            ws_aabb: self.ws_aabb,
            resolution: self.resolution,
            cascade_position: self.ws_aabb.min.into(),
            cascade_res: cascade_res.into(),
            cascade_spacing: self.resolution.into(),
            probe_size: 6.0,
            gi_texel: 1.0 / 2048.0,
            id_texel: vec2(1.0 / cascade_res.x, 1.0 / (cascade_res.y * cascade_res.z)),
        }
    }
}

pub fn generate_cascade_data(
    mut commands: Commands,
    input_probes: Query<(Entity, &CascadeInput), Without<CascadeUniform>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, input) in &input_probes {
        let mut ecmds = commands.entity(entity);
        ecmds.insert(input.into_uniform(&asset_server));
        #[cfg(feature = "asset_baking")]
        ecmds.insert(input.into_cascade_data());
    }
}

pub fn sampler_nearest_clamp() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        address_mode_w: ImageAddressMode::ClampToEdge,
        ..Default::default()
    })
}
