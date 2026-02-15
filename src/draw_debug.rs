use std::mem;

use bevy::prelude::*;
use bgl2::UniformSet;
use bgl2::prepare_image::GpuImages;
use bgl2::{
    bevy_standard_material::ViewUniforms, command_encoder::CommandEncoder, render::RenderSet,
    shader_cached,
};
use bytemuck::cast_slice;
use glow::HasContext;

#[derive(Resource, Default)]
pub struct DrawDebugPlugin;

impl Plugin for DrawDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugLines>()
            .add_systems(PostUpdate, draw_debug_lines.in_set(RenderSet::RenderDebug));
    }
}

/// Immediate mode, cleared every frame. For retained use the DebugLine Component.
#[derive(Resource, Default)]
pub struct DebugLines {
    /// Append to verts, they are cleared each frame
    pub positions: Vec<Vec3>,
    /// Append to colors, they are cleared each frame
    pub colors: Vec<Vec4>,
}

impl DebugLines {
    pub fn line(&mut self, a: Vec3, b: Vec3, color: Vec3) {
        self.positions.push(a);
        self.positions.push(b);
        let c = color.extend(1.0);
        self.colors.push(c);
        self.colors.push(c);
    }
    pub fn aabb(&mut self, aabb: obvhs::aabb::Aabb, color: Vec3) {
        let a = aabb.min;
        let b = aabb.max;
        self.line(vec3(a.x, a.y, a.z), vec3(b.x, a.y, a.z), color);
        self.line(vec3(a.x, a.y, b.z), vec3(b.x, a.y, b.z), color);
        self.line(vec3(a.x, b.y, a.z), vec3(b.x, b.y, a.z), color);
        self.line(vec3(a.x, b.y, b.z), vec3(b.x, b.y, b.z), color);
        self.line(vec3(a.x, a.y, a.z), vec3(a.x, b.y, a.z), color);
        self.line(vec3(a.x, a.y, b.z), vec3(a.x, b.y, b.z), color);
        self.line(vec3(b.x, a.y, a.z), vec3(b.x, b.y, a.z), color);
        self.line(vec3(b.x, a.y, b.z), vec3(b.x, b.y, b.z), color);
        self.line(vec3(a.x, a.y, a.z), vec3(a.x, a.y, b.z), color);
        self.line(vec3(a.x, b.y, a.z), vec3(a.x, b.y, b.z), color);
        self.line(vec3(b.x, a.y, a.z), vec3(b.x, a.y, b.z), color);
        self.line(vec3(b.x, b.y, a.z), vec3(b.x, b.y, b.z), color);
    }
}

#[derive(Component, Default)]
pub struct DebugLine {
    pub positions: [Vec3; 2],
    pub colors: [Vec4; 2],
}

impl DebugLine {
    pub fn new(a: Vec3, b: Vec3, color: Vec3) -> DebugLine {
        let c = color.extend(1.0);
        DebugLine {
            positions: [a, b],
            colors: [c, c],
        }
    }
}

#[derive(Resource)]
struct DrawDebugBuffers {
    positions_vbo: glow::Buffer,
    colors_vbo: glow::Buffer,
}

fn draw_debug_lines(
    mut enc: ResMut<CommandEncoder>,
    mut debug_lines: ResMut<DebugLines>,
    entities: Query<&DebugLine>,
) {
    for line in &entities {
        debug_lines.positions.push(line.positions[0]);
        debug_lines.positions.push(line.positions[1]);
        debug_lines.colors.push(line.colors[0]);
        debug_lines.colors.push(line.colors[1]);
    }

    assert_eq!(debug_lines.positions.len(), debug_lines.colors.len());
    if debug_lines.positions.is_empty() {
        return;
    }

    let mut lines = DebugLines::default();
    mem::swap(&mut *debug_lines, &mut lines);
    enc.record(move |ctx, world| {
        #[allow(unexpected_cfgs)]
        let shader_index = shader_cached!(
            ctx,
            "../assets/shaders/debug_lines.vert",
            "../assets/shaders/debug_lines.frag",
            &[],
            &[ViewUniforms::bindings()]
        )
        .unwrap();
        unsafe {
            let vert_count = lines.positions.len() as i32;
            ctx.start_alpha_blend();
            ctx.gl.disable(glow::DEPTH_TEST);

            if let Some(b) = world.remove_resource::<DrawDebugBuffers>() {
                ctx.gl.delete_buffer(b.positions_vbo);
                ctx.gl.delete_buffer(b.colors_vbo);
            }
            ctx.use_cached_program(shader_index);
            ctx.map_uniform_set_locations::<ViewUniforms>();
            ctx.bind_uniforms_set(
                world.resource::<GpuImages>(),
                world.resource::<ViewUniforms>(),
            );

            let pos_loc = ctx.get_attrib_location(shader_index, "a_position").unwrap();
            let col_loc = ctx.get_attrib_location(shader_index, "a_color").unwrap();

            let positions_vbo = ctx.gl.create_buffer().unwrap();
            ctx.gl.bind_buffer(glow::ARRAY_BUFFER, Some(positions_vbo));
            ctx.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                cast_slice(&lines.positions),
                glow::STATIC_DRAW,
            );
            ctx.gl.enable_vertex_attrib_array(pos_loc);
            ctx.gl
                .vertex_attrib_pointer_f32(pos_loc, 3, glow::FLOAT, false, 12, 0);

            let colors_vbo = ctx.gl.create_buffer().unwrap();
            ctx.gl.bind_buffer(glow::ARRAY_BUFFER, Some(colors_vbo));
            ctx.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                cast_slice(&lines.colors),
                glow::STATIC_DRAW,
            );
            ctx.gl.enable_vertex_attrib_array(col_loc);
            ctx.gl
                .vertex_attrib_pointer_f32(col_loc, 4, glow::FLOAT, false, 16, 0);

            ctx.set_cull_mode(None);
            ctx.gl.draw_arrays(glow::LINES, 0, vert_count);

            ctx.gl.bind_buffer(glow::ARRAY_BUFFER, None);

            world.insert_resource(DrawDebugBuffers {
                positions_vbo,
                colors_vbo,
            });
        };
    });
}
