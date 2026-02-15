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
    _other: HashMap<String, serde_json::Value>,
}

#[derive(Component, Clone, Default)]
pub struct SceneBakeName(pub String);

pub fn blender_cascades(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    gltf_extras: Query<(Entity, &Name, &Transform, &GltfExtras)>,
    scene_names: Query<&SceneBakeName>,
) {
    let name_prefix = scene_names
        .get(scene_ready.entity)
        .cloned()
        .unwrap_or_default()
        .0;
    for entity in children.iter_descendants(scene_ready.entity) {
        if let Ok((entity, name, trans, extras)) = gltf_extras.get(entity)
            && name.contains("BAKE") {
                let extras: ProbeBakeExtras = serde_json::from_str(&extras.value).unwrap();
                if let Some(bake_res) = extras.probe_bake_res {
                    let scale: Vec3A = trans.scale.into();
                    let position = trans.translation.to_vec3a();
                    let start = position - scale;
                    let end = position + scale;
                    let mut full_name = name_prefix.clone();
                    full_name.push_str(name);
                    let mut ecmds = commands.entity(entity);
                    ecmds
                        .insert(CascadeInput {
                            name: full_name,
                            ws_aabb: obvhs::aabb::Aabb::new(start, end),
                            resolution: vec3a(bake_res[0], bake_res[1], bake_res[2]),
                        })
                        .remove::<CascadeUniform>();

                    #[cfg(feature = "asset_baking")]
                    ecmds.remove::<CascadeData>();
                }
            }
    }
}

#[cfg(feature = "asset_baking")]
use light_volume_baker::CascadeData;
use obvhs::ray::Ray;
use serde::Deserialize;
use uniform_set_derive::UniformSet;

use crate::draw_debug::DebugLines;

pub fn transform_aabb(world_from_local: Mat4, aabb: obvhs::aabb::Aabb) -> obvhs::aabb::Aabb {
    let min = aabb.min;
    let max = aabb.max;
    let mut aabb = obvhs::aabb::Aabb::empty();
    aabb.extend(world_from_local.transform_point3a(vec3a(min.x, min.y, min.z)));
    aabb.extend(world_from_local.transform_point3a(vec3a(max.x, min.y, min.z)));
    aabb.extend(world_from_local.transform_point3a(vec3a(min.x, max.y, min.z)));
    aabb.extend(world_from_local.transform_point3a(vec3a(min.x, min.y, max.z)));
    aabb.extend(world_from_local.transform_point3a(vec3a(max.x, max.y, max.z)));
    aabb.extend(world_from_local.transform_point3a(vec3a(min.x, max.y, max.z)));
    aabb.extend(world_from_local.transform_point3a(vec3a(max.x, min.y, max.z)));
    aabb.extend(world_from_local.transform_point3a(vec3a(max.x, max.y, min.z)));
    aabb
}

pub fn select_cascade<'a, I>(
    cascades: I,
    draw_aabb: obvhs::aabb::Aabb,
    #[allow(unused)] mut debug: &mut DebugLines,
) -> u32
where
    I: IntoIterator<Item = &'a CascadeUniform>,
{
    // TODO cache selected cascade

    let draw_size = draw_aabb.diagonal().length();
    let draw_center = draw_aabb.center();
    let mut draw_dist_to_cascade = f32::MAX;
    let mut best_cascade = 0;
    let mut best_relative_res = 0.0;
    for (i, cascade) in cascades.into_iter().enumerate() {
        let cascade_aabb = obvhs::aabb::Aabb::new(
            cascade.cascade_position.into(),
            (cascade.cascade_position + cascade.cascade_res * cascade.cascade_spacing).into(),
        );
        // Oversize the cascade_aabb to include the full infulence range
        let spacing = cascade.cascade_spacing.to_vec3a() * 2.0;
        let mut enlarged_cascade_aabb = cascade_aabb;
        enlarged_cascade_aabb.min -= spacing;
        enlarged_cascade_aabb.max += spacing;

        let outside_weight = 100.0; // TODO do better
        let cascade_intersection = cascade_aabb.intersection(&draw_aabb);
        let enlarged_cascade_intersection = enlarged_cascade_aabb.intersection(&draw_aabb);
        let relative_res = (cascade.cascade_res / cascade.cascade_spacing).max_element();
        let dist_to_cascade = if enlarged_cascade_intersection.valid() {
            draw_size / enlarged_cascade_intersection.diagonal().length()
        } else {
            cascade_aabb.intersect_ray(&Ray::new_inf(
                draw_center,
                (cascade_aabb.center() - draw_center).normalize_or_zero(),
            )) * outside_weight
        };
        //debug.aabb(cascade_aabb, vec3(1.0, 0.0, 1.0));
        //debug.aabb(draw_aabb, vec3(1.0, 0.0, 0.0));

        if dist_to_cascade < draw_dist_to_cascade
            || (dist_to_cascade < draw_dist_to_cascade * 1.1
                && cascade_intersection.valid()
                && relative_res > best_relative_res)
        {
            draw_dist_to_cascade = dist_to_cascade;
            best_cascade = i;
            best_relative_res = relative_res;
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

// TODO UniformSet for newtype?
#[derive(UniformSet, Component, Clone)]
#[uniform_set(prefix = "ubv_")]
pub struct CascadeViewUniform {
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
        let cascade_res = (self.ws_aabb.diagonal() / self.resolution).ceil();
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

    // Note: if making changes copy into_uniform() and rename to CascadeViewUniform
    pub fn into_view_uniform(&self, asset_server: &AssetServer) -> CascadeViewUniform {
        let cascade_res = (self.ws_aabb.diagonal() / self.resolution).ceil();
        CascadeViewUniform {
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
        let cascade_res = (self.ws_aabb.diagonal() / self.resolution).ceil();
        CascadeData {
            name: self.name.clone(),
            ws_aabb: self.ws_aabb,
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
        ecmds.insert(input.into_view_uniform(&asset_server));
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
