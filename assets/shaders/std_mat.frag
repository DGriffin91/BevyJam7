//#define NO_DIRECTIONAL

#include std::math
#include std::pbr
#include std::agx
#include std::shadow_sampling

vec3 apply_pbr_lighting(vec3 V, vec3 diffuse_color, vec3 F0, vec3 vert_normal, vec3 normal, float perceptual_roughness,
    float environment_occlusion, float diffuse_transmission, vec2 screen_uv, vec2 view_resolution, vec3 ws_position, float dir_shadow) {
    float roughness = perceptual_roughness * perceptual_roughness;
    vec3 output_color = vec3(0.0);

    #ifndef NO_DIRECTIONAL
    #ifdef SAMPLE_SHADOW
    float bias = 0.002;
    float normal_bias = 0.05;
    vec4 shadow_clip = ub_shadow_clip_from_world * vec4(ws_position + vert_normal * normal_bias, 1.0);
    vec3 shadow_uvz = (shadow_clip.xyz / shadow_clip.w) * 0.5 + 0.5;

    if (shadow_uvz.x > 0.0 && shadow_uvz.x < 1.0 && shadow_uvz.y > 0.0 && shadow_uvz.y < 1.0 && shadow_uvz.z > 0.0 && shadow_uvz.z < 1.0) {
        dir_shadow = bilinear_shadow2(ub_shadow_texture, shadow_uvz.xy, shadow_uvz.z, bias, view_resolution);
        //dir_shadow *= sample_shadow_map_castano_thirteen(ub_shadow_texture, shadow_uvz.xy, shadow_uvz.z, bias, view_resolution);
    }
    dir_shadow = hardenedKernel(dir_shadow);
    #endif // SAMPLE_SHADOW

    output_color += directional_light(V, F0, diffuse_color, normal, roughness, diffuse_transmission, dir_shadow, ub_directional_light_dir, ub_directional_light_color);
    #endif // NO_DIRECTIONAL

    #ifndef NO_ENV
    // Environment map
    float NoV = abs(dot(normal, V)) + 1e-5;
    float mip_levels = 8.0; // TODO put in uniform
    vec3 dir = reflect(-V, normal);
    vec3 env_diffuse = rgbe2rgb(textureCubeLod(ub_diffuse_map, vec3(normal.xy, -normal.z), 0.0)) * ub_env_intensity;
    vec3 env_specular = rgbe2rgb(textureCubeLod(ub_specular_map, vec3(dir.xy, -dir.z), perceptual_roughness * mip_levels)) * ub_env_intensity;
    output_color += environment_light(NoV, F0, perceptual_roughness, diffuse_color, env_diffuse, env_specular) * environment_occlusion;
    #endif // NO_ENV

    return output_color;
}

varying vec4 clip_position;
varying vec3 ws_position;
varying vec4 tangent;
varying vec3 vert_normal;
varying vec2 uv_0;
varying vec2 uv_1;

uniform sampler2D reflect_texture;
uniform bool read_reflection;
uniform vec3 reflection_plane_position;
uniform vec3 reflection_plane_normal;

int sampleTrilinearCorner(vec3 f, float u) {
    float fx = f.x, fy = f.y, fz = f.z;

    float ax = 1.0 - fx;
    float ay = 1.0 - fy;
    float az = 1.0 - fz;

    // 8 corner weights (sum to 1)
    float w0 = ax * ay * az; // (0,0,0)
    float w1 = fx * ay * az; // (1,0,0)
    float w2 = ax * fy * az; // (0,1,0)
    float w3 = fx * fy * az; // (1,1,0)
    float w4 = ax * ay * fz; // (0,0,1)
    float w5 = fx * ay * fz; // (1,0,1)
    float w6 = ax * fy * fz; // (0,1,1)
    float w7 = fx * fy * fz; // (1,1,1)

    // CDF walk
    float c = w0;
    if (u < c) return 0;
    c += w1; if (u < c) return 1;
    c += w2; if (u < c) return 2;
    c += w3; if (u < c) return 3;
    c += w4; if (u < c) return 4;
    c += w5; if (u < c) return 5;
    c += w6; if (u < c) return 6;
    return 7;
}

// No bit shift on glsl 120
float hash(float n) { return fract(sin(n) * 1e4); }
float hash(vec2 p) { return fract(1e4 * sin(17.0 * p.x + p.y * 0.1) * (0.1 + abs(sin(p.y * 13.0 + p.x)))); }

float signNotZero(in float k) {
    return (k >= 0.0) ? 1.0 : -1.0;
}

vec2 signNotZero(in vec2 v) {
    return vec2(signNotZero(v.x), signNotZero(v.y));
}

// https://github.com/RomkoSI/G3D/blob/master/data-files/shader/octahedral.glsl
/** Assumes that v is a unit vector. The result is an octahedral vector on the [-1, +1] square. */
vec2 octEncode(vec3 v) {
    float l1norm = abs(v.x) + abs(v.y) + abs(v.z);
    vec2 result = v.xy * (1.0 / l1norm);
    if (v.z < 0.0) {
        result = (1.0 - abs(result.yx)) * signNotZero(result.xy);
    }
    return result;
}

vec3 corner_offset(int i) {
    if (i == 0) return vec3(0.0, 0.0, 0.0);
    if (i == 1) return vec3(1.0, 0.0, 0.0);
    if (i == 2) return vec3(0.0, 1.0, 0.0);
    if (i == 3) return vec3(1.0, 1.0, 0.0);
    if (i == 4) return vec3(0.0, 0.0, 1.0);
    if (i == 5) return vec3(1.0, 0.0, 1.0);
    if (i == 6) return vec3(0.0, 1.0, 1.0);
    return vec3(1.0, 1.0, 1.0);

    // TODO in opengl 2 use something like:
    // (and bench to make sure it's actually faster)
    // const vec3 OFFS[8] = vec3[8](
    //     vec3(0.0, 0.0, 0.0),
    //     vec3(1.0, 0.0, 0.0),
    //     vec3(0.0, 1.0, 0.0),
    //     vec3(1.0, 1.0, 0.0),
    //     vec3(0.0, 0.0, 1.0),
    //     vec3(1.0, 0.0, 1.0),
    //     vec3(0.0, 1.0, 1.0),
    //     vec3(1.0, 1.0, 1.0)
    // );
    // And in webgl use bit shifting
}


vec4 sample_cascade_stochastic(vec3 ws_position, vec3 ws_normal, vec2 screen_uv, vec3 diffuse_color) {
    vec3 ls_position = (ws_position - ub_cascade_position) / ub_cascade_spacing;
    vec3 base = floor(ls_position);
    base = min(max(base, vec3(0.0)), ub_cascade_res - 2.0);
    vec3 alpha = saturate(ls_position - base);

    vec2 id_texel_half = ub_id_texel * 0.5;

    int i = sampleTrilinearCorner(alpha, hash(screen_uv + hash(ub_frame)));
    vec3 offset = corner_offset(i);
    vec3 probe_pos = base + offset;
    vec2 ls_id_position = vec2(probe_pos.x, probe_pos.y + probe_pos.z * ub_cascade_res.y);
    vec3 trilinear = mix(1.0 - alpha, alpha, offset);
    float probe_pad_size = ub_probe_size + 2.0;
    vec2 oct = octEncode(ws_normal) * 0.5 + 0.5;
    vec4 probe_id_info = texture2D(ub_probes_id, id_texel_half + ub_id_texel * ls_id_position);
    vec2 probe_xy_id = floor(probe_id_info.xy * 255.0 + 0.5);
    vec2 uv = ub_gi_texel + oct * ub_probe_size * ub_gi_texel + (ub_gi_texel * probe_pad_size * probe_xy_id);
    vec3 probe_irradiance = 0.5 * PI * rgbe2rgb(texture2D(ub_probes_gi, uv));
    vec3 color = probe_irradiance * diffuse_color * 1000.0 * 5.0;

    return vec4(color, probe_id_info.z);
}

vec3 sample_fog(float blend, float seed, vec3 atm_color, vec3 sample_normal, vec2 screen_uv, vec3 V) {
    vec3 sample_pos = ws_position * (1.0 - seed) + seed * ub_view_position;
    vec4 col_shad = sample_cascade_stochastic(sample_pos, sample_normal, screen_uv, vec3(blend));
    vec3 color = col_shad.rgb;
    //output_color /= blend;

    float atm_dir_shadow = col_shad.w;
    vec4 shadow_clip = ub_shadow_clip_from_world * vec4(sample_pos, 1.0);
    vec3 shadow_uvz = (shadow_clip.xyz / shadow_clip.w) * 0.5 + 0.5;

    if (shadow_uvz.x > 0.0 && shadow_uvz.x < 1.0 && shadow_uvz.y > 0.0 && shadow_uvz.y < 1.0 && shadow_uvz.z > 0.0 && shadow_uvz.z < 1.0) {
        atm_dir_shadow = bilinear_shadow2(ub_shadow_texture, shadow_uvz.xy, shadow_uvz.z, 0.0, ub_view_resolution);
    }
    color += directional_light(V, vec3(0.0), atm_color, sample_normal, 0.5, 0.0, atm_dir_shadow,
                                      ub_directional_light_dir, ub_directional_light_color);
    return color;
}

void main() {
    vec4 base_color = ub_base_color * to_linear(texture2D(ub_base_color_texture, uv_0));
    float blender_exposure = 0.2; // TODO set on camera

    #ifdef WRITE_REFLECTION
    if (dot(ws_position - reflection_plane_position, reflection_plane_normal) < 0.0) {
        discard;
    }
    #endif // WRITE_REFLECTION

    vec3 ndc_position = clip_position.xyz / clip_position.w;
    vec2 screen_uv = ndc_position.xy * 0.5 + 0.5;

    #ifdef RENDER_DEPTH_ONLY
    gl_FragColor = EncodeFloatRGBA(saturate(ndc_position.z * 0.5 + 0.5));
    #else // RENDER_DEPTH_ONLY

    vec3 V = normalize(ub_view_position - ws_position);

    vec4 metallic_roughness = texture2D(ub_metallic_roughness_texture, uv_0);
    float perceptual_roughness = metallic_roughness.g * ub_perceptual_roughness;
    float metallic = ub_metallic * metallic_roughness.b;
    vec3 F0 = calculate_F0(base_color.rgb, metallic, ub_reflectance);
    vec3 diffuse_color = base_color.rgb * (1.0 - metallic);

    float emissive_exposure_factor = 1000.0; // TODO do something better
    vec3 emissive = emissive_exposure_factor * ub_emissive.rgb * to_linear(texture2D(ub_emissive_texture, uv_0).rgb);
    float emissive_v = saturate(emissive.r + emissive.g + emissive.b);
    if (ub_emissive.r == ub_emissive.g && ub_emissive.r == ub_emissive.b) {
        emissive_v = 0.0; // janky workaround for helmet having emissive
    }
    vec3 post_tonemap_emissive = ub_emissive.rgb;

    vec3 normal = vert_normal;
    if (ub_has_normal_map) {
        normal = apply_normal_mapping(ub_normal_map_texture, vert_normal, tangent, uv_0, ub_flip_normal_map_y, ub_double_sided);
    }

    vec3 output_color = emissive.rgb;
    float env_occ = 1.0;

    #ifdef READ_REFLECTION
    if (read_reflection && perceptual_roughness < 0.2) {
        vec3 sharp_reflection_color = reversible_tonemap_invert(texture2D(reflect_texture, screen_uv).rgb);
        output_color += sharp_reflection_color.rgb / ub_view_exposure; // TODO integrate brdf properly
        env_occ = 0.0;
    }
    #endif //READ_REFLECTION

    // float dir_shadow = 0.0;
    // {
    //     #ifdef CASCADE
    //     vec3 ls_position = (ws_position - ub_cascade_position) / ub_cascade_spacing;
    //     vec3 base = floor(ls_position);
    //     base = min(max(base, vec3(0.0)), ub_cascade_res - 2.0);
    //     vec3 alpha = saturate(ls_position - base);

    //     vec3 sum_irradiance = vec3(0.0);
    //     float sum_weight = 0.0;

    //     vec2 id_texel_half = ub_id_texel * 0.5;
    //     for (int i = 0; i < 8; ++i) {
    //         vec3 offset = corner_offset(i);
    //         vec3 probe_pos = base + offset;
    //         vec2 ls_id_position = vec2(probe_pos.x, probe_pos.y + probe_pos.z * ub_cascade_res.y);
    //         vec3 trilinear = mix(1.0 - alpha, alpha, offset);
    //         float weight = trilinear.x * trilinear.y * trilinear.z;
    //         float probe_pad_size = ub_probe_size + 2.0;
    //         vec2 oct = octEncode(normal) * 0.5 + 0.5;
    //         vec4 probe_id_info = texture2D(ub_probes_id, id_texel_half + ub_id_texel * ls_id_position);
    //         vec2 probe_xy_id = floor(probe_id_info.xy * 255.0 + 0.5);
    //         vec2 uv = ub_gi_texel + oct * ub_probe_size * ub_gi_texel + (ub_gi_texel * probe_pad_size * probe_xy_id);
    //         dir_shadow += weight * probe_id_info.z;
    //         sum_irradiance += weight * rgbe2rgb(texture2D(ub_probes_gi, uv));
    //         sum_weight += weight;
    //     }
    //     vec3 net_irradiance = sum_irradiance / sum_weight;
    //     dir_shadow = dir_shadow / sum_weight;
    //     vec3 probe_irradiance = 0.5 * PI * net_irradiance;
    //     output_color += probe_irradiance * diffuse_color * 1000.0 * 5.0;
    //     #else //CASCADE
    //     dir_shadow = 1.0;
    //     #endif //CASCADE
    // }

    float dir_shadow = 0.0;
    {
        #ifdef CASCADE
        vec4 col_shad = sample_cascade_stochastic(ws_position, normal, screen_uv, diffuse_color );
        output_color += col_shad.rgb;
        dir_shadow = col_shad.w;

        #else //CASCADE
        dir_shadow = 1.0;
        #endif //CASCADE
    }


    output_color += apply_pbr_lighting(V, diffuse_color, F0, vert_normal, normal, perceptual_roughness,
            env_occ, ub_diffuse_transmission, screen_uv, ub_view_resolution, ws_position, dir_shadow);

    {

        float seed = hash(screen_uv + hash(ub_frame - 123.456));
        float f = 0.5;
        vec3 fog_color = f * sample_fog(3.0, seed, vec3(1.0), vec3(0.0, 1.0, 0.0), screen_uv, V);
        seed = hash(screen_uv + 2.0 + hash(ub_frame - 567.345));
        fog_color += (1.0 - f) * sample_fog(3.0,seed, vec3(1.0), V, screen_uv, V);

        float frag_dist = length(ub_view_position - ws_position) * 0.02;
        f = min(frag_dist, 1.0);
        output_color = fog_color * f + (1.0 - f) * output_color;
    }


    gl_FragColor = vec4(ub_view_exposure * output_color * blender_exposure, base_color.a);
    #ifdef WRITE_REFLECTION
    gl_FragColor.rgb = reversible_tonemap(gl_FragColor.rgb);
    #else
    gl_FragColor.rgb = agx_tonemapping(gl_FragColor.rgb); // in: linear, out: srgb
    //gl_FragColor.rgb = from_linear(gl_FragColor.rgb); // in: linear, out: srgb
    //gl_FragColor.rgb = mix(gl_FragColor.rgb, post_tonemap_emissive, emissive_v);
    #endif // WRITE_REFLECTION
    gl_FragColor = clamp(gl_FragColor, vec4(0.0), vec4(1.0));

    #endif // NOT RENDER_DEPTH_ONLY
}
