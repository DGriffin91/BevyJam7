use bevy::prelude::*;
use bgl2::{
    BevyGlContext,
    command_encoder::CommandEncoder,
    prepare_image::{GpuImages, TextureRef},
};
use glow::{HasContext, PixelUnpackData};
use uniform_set_derive::UniformSet;

pub fn copy_depth_prepass(
    mut commands: Commands,
    bevy_window: Single<&Window>,
    mut enc: ResMut<CommandEncoder>,
    prepass: Option<ResMut<PrepassTexture>>,
) {
    let width = bevy_window.physical_width().max(1);
    let height = bevy_window.physical_height().max(1);

    let mut just_init = false;
    let mut prepass = if let Some(prepass) = prepass {
        prepass.clone()
    } else {
        just_init = true;
        let texture_ref = TextureRef::new();
        
        PrepassTexture {
            texture: texture_ref.clone(),
            width,
            height,
        }
    };

    if just_init || prepass.width != width || prepass.height != height {
        let texture_ref = prepass.texture.clone();
        prepass.width = width;
        prepass.height = height;

        enc.record(move |ctx, world| unsafe {
            if let Some((tex, _target)) = world
                .resource_mut::<GpuImages>()
                .texture_from_ref(&texture_ref)
            {
                ctx.gl.delete_texture(tex);
                PrepassTexture::init(
                    ctx,
                    &mut world.resource_mut::<GpuImages>(),
                    &texture_ref,
                    width,
                    height,
                )
            }
        });
    }

    let enc_prepass = prepass.clone();
    enc.record(move |ctx, world| {
        if let Some((texture, target)) = world
            .resource_mut::<GpuImages>()
            .texture_from_ref(&enc_prepass.texture)
        {
            unsafe {
                ctx.gl.bind_texture(target, Some(texture));
                ctx.gl.copy_tex_image_2d(
                    target,
                    0,
                    glow::RGBA,
                    0,
                    0,
                    enc_prepass.width as i32,
                    enc_prepass.height as i32,
                    0,
                );
            };
        }
    });

    commands.insert_resource(prepass.clone());
}

#[derive(UniformSet, Resource, Clone, Default)]
#[uniform_set(prefix = "ub_")]
pub struct PrepassTexture {
    pub texture: TextureRef,
    #[exclude]
    pub width: u32,
    #[exclude]
    pub height: u32,
}

impl PrepassTexture {
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
