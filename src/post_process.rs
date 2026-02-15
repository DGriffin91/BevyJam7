use bevy::prelude::*;
use bgl2::{
    BevyGlContext, Tex,
    command_encoder::CommandEncoder,
    prepare_image::{GpuImages, TextureRef},
    render::RenderSet,
    shader_cached,
};
use bytemuck::cast_slice;
use glow::{HasContext, PixelUnpackData};
use uniform_set_derive::UniformSet;

#[derive(Resource, Default)]
pub struct PostProcessPlugin;

impl Plugin for PostProcessPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (copy_render_target, render_post_process)
                .chain()
                .in_set(RenderSet::RenderDebug),
        );
    }
}
#[derive(Resource, Default)]
pub struct PostProcessSettings {
    pub enable: bool,
}

#[derive(Resource)]
struct PostProcessBuffers {
    positions_vbo: glow::Buffer,
}

fn render_post_process(
    mut enc: ResMut<CommandEncoder>,
    render_texture: If<Res<RenderTexture>>,
    settings: Res<PostProcessSettings>,
) {
    if !settings.enable {
        return;
    }
    let render_texture = render_texture.clone();
    enc.record(|ctx, world| {
        #[allow(unexpected_cfgs)]
        let shader_index = shader_cached!(
            ctx,
            "../assets/shaders/post_process.vert",
            "../assets/shaders/post_process.frag",
            &[],
            &[]
        )
        .unwrap();
        unsafe {
            let positions_vbo = if let Some(buffers) = world.get_resource::<PostProcessBuffers>() {
                buffers.positions_vbo
            } else {
                let positions_vbo = ctx.gl.create_buffer().unwrap();
                let positions = [-1.0f32, -1.0, 3.0, -1.0, -1.0, 3.0];
                ctx.gl.bind_buffer(glow::ARRAY_BUFFER, Some(positions_vbo));
                ctx.gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    cast_slice(&positions),
                    glow::STATIC_DRAW,
                );
                world.insert_resource(PostProcessBuffers {
                    positions_vbo,
                });
                positions_vbo
            };

            ctx.use_cached_program(shader_index);

            ctx.start_alpha_blend();
            ctx.gl.disable(glow::DEPTH_TEST);

            let pos_loc = ctx.get_attrib_location(shader_index, "a_position").unwrap();

            ctx.gl.bind_buffer(glow::ARRAY_BUFFER, Some(positions_vbo));
            ctx.gl.enable_vertex_attrib_array(pos_loc);
            ctx.gl
                .vertex_attrib_pointer_f32(pos_loc, 2, glow::FLOAT, false, 8, 0);

            ctx.load_tex(
                world.resource::<GpuImages>(),
                "render_target",
                &Tex::Ref(render_texture.texture),
            );

            ctx.gl.draw_arrays(glow::TRIANGLES, 0, 3);

            ctx.set_cull_mode(None);
        };
    });
}

pub fn copy_render_target(
    mut commands: Commands,
    bevy_window: Single<&Window>,
    mut enc: ResMut<CommandEncoder>,
    render_target: Option<ResMut<RenderTexture>>,
    settings: Res<PostProcessSettings>,
) {
    if !settings.enable {
        return;
    }
    let width = bevy_window.physical_width().max(1);
    let height = bevy_window.physical_height().max(1);

    let mut just_init = false;
    let mut render_target_texture = if let Some(render_target) = render_target {
        render_target.clone()
    } else {
        just_init = true;
        let texture_ref = TextureRef::new();
        RenderTexture {
            texture: texture_ref.clone(),
            width,
            height,
        }
    };

    if just_init || render_target_texture.width != width || render_target_texture.height != height {
        let texture_ref = render_target_texture.texture.clone();
        render_target_texture.width = width;
        render_target_texture.height = height;

        enc.record(move |ctx, world| unsafe {
            if let Some((tex, _target)) = world
                .resource_mut::<GpuImages>()
                .texture_from_ref(&texture_ref)
            {
                ctx.gl.delete_texture(tex);
            }
            RenderTexture::init(
                ctx,
                &mut world.resource_mut::<GpuImages>(),
                &texture_ref,
                width,
                height,
            )
        });
    }

    let render_target = render_target_texture.clone();
    enc.record(move |ctx, world| {
        unsafe {
            if let Some((tex, _target)) = &mut world
                .resource_mut::<GpuImages>()
                .texture_from_ref(&render_target.texture)
            {
                ctx.gl.bind_texture(glow::TEXTURE_2D, Some(*tex));
                ctx.gl.copy_tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    0,
                );
            }
        };
    });

    commands.insert_resource(render_target_texture.clone());
}

#[derive(UniformSet, Resource, Clone, Default)]
#[uniform_set(prefix = "ub_")]
pub struct RenderTexture {
    pub texture: TextureRef,
    #[exclude]
    pub width: u32,
    #[exclude]
    pub height: u32,
}

impl RenderTexture {
    fn init(
        ctx: &mut BevyGlContext,
        images: &mut GpuImages,
        texture_ref: &TextureRef,
        width: u32,
        height: u32,
    ) {
        unsafe {
            let texture = ctx.gl.create_texture().unwrap();
            images.add_texture_set_ref(texture, glow::TEXTURE_2D, texture_ref);
            ctx.gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            ctx.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            ctx.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            ctx.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            ctx.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            ctx.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                PixelUnpackData::Slice(None),
            );
        }
    }
}
