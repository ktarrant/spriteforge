#import bevy_ecs_tilemap::common::process_fragment
#import bevy_ecs_tilemap::common::tilemap_data
#import bevy_ecs_tilemap::vertex_output::MeshVertexOutput

struct WaterFoamParams {
    foam_color: vec4<f32>,
    foam_settings: vec4<f32>,
};

const TAU: f32 = 6.28318530718;

fn hash2(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

fn uv_from_xy(xf: f32, yf: f32) -> vec2<f32> {
    let lx = 0.0;
    let ly = 0.75;
    let dx = xf - lx;
    let dy = yf - ly;
    let u = dx + 2.0 * dy;
    let v = u - dy * 4.0;
    return vec2<f32>(u, v);
}

fn xy_from_uv(u: f32, v: f32) -> vec2<f32> {
    let x = (u + v) * 0.5;
    let y = (u - v) * 0.25 + 0.75;
    return vec2<f32>(x, y);
}

@group(3) @binding(0)
var mask_texture: texture_2d<f32>;

@group(3) @binding(1)
var mask_sampler: sampler;

@group(3) @binding(2)
var<uniform> params: WaterFoamParams;

@fragment
fn fragment(in: MeshVertexOutput) -> @location(0) vec4<f32> {
    let base = process_fragment(in);
    let tile_pos = vec2<f32>(in.storage_position);
    let local_xy = vec2<f32>(in.uv.z, in.uv.w);
    let iso_uv = uv_from_xy(local_xy.x, local_xy.y);
    let tile_uv = tile_pos + iso_uv;
    let iso_xy = xy_from_uv(tile_uv.x, tile_uv.y);
    let lx = iso_xy.x;
    let ly = iso_xy.y * 2.0;
    let freq = params.foam_settings.y;
    let phase = tilemap_data.time * params.foam_settings.z;
    let flow = vec2<f32>(0.12, 0.08);
    let pos = vec2<f32>(lx, ly) + flow * phase;
    let dir_a = normalize(vec2<f32>(0.8, 0.6));
    let dir_b = normalize(vec2<f32>(-0.4, 0.9));
    let dir_c = normalize(vec2<f32>(-0.9, -0.2));
    let wave_a = sin((dot(pos, dir_a) * freq + phase * 0.9) * TAU);
    let wave_b = sin((dot(pos, dir_b) * (freq * 0.8) - phase * 0.6) * TAU);
    let wave_c = sin((dot(pos, dir_c) * (freq * 1.3) + phase * 1.1) * TAU);
    let wave = (wave_a + 0.6 * wave_b + 0.4 * wave_c) / 2.0;
    let wave01 = wave * 0.5 + 0.5;
    let crest = smoothstep(0.45, 0.85, wave01);
    let noise = hash2(pos * 2.0 + phase);
    let shimmer = crest * params.foam_settings.x * mix(0.8, 1.2, noise) * 1.4;
    let mask = textureSample(mask_texture, mask_sampler, in.uv.xy).a;
    let shaded = base.rgb + params.foam_color.rgb * shimmer;
    let rgb = mix(base.rgb, shaded, mask);
    return vec4<f32>(rgb, base.a);
}
