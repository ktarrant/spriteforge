#import bevy_ecs_tilemap::common::process_fragment
#import bevy_ecs_tilemap::vertex_output::MeshVertexOutput

struct TreeLightParams {
    light_dir: vec4<f32>,
    ambient_strength: f32,
    diffuse_strength: f32,
    _pad0: vec2<f32>,
};

@group(3) @binding(0)
var normal_texture: texture_2d<f32>;

@group(3) @binding(1)
var normal_sampler: sampler;

@group(3) @binding(2)
var<uniform> params: TreeLightParams;

fn decode_normal(rgb: vec3<f32>) -> vec3<f32> {
    let n = rgb * 2.0 - vec3<f32>(1.0);
    return normalize(n);
}

@fragment
fn fragment(in: MeshVertexOutput) -> @location(0) vec4<f32> {
    let base = process_fragment(in);
    let normal_rgb = textureSample(normal_texture, normal_sampler, in.uv.xy).rgb;
    let normal = decode_normal(normal_rgb);
    let light_dir = normalize(params.light_dir.xyz);
    let ndotl = max(dot(normal, light_dir), 0.0);
    let gray = clamp(ndotl, 0.0, 1.0);
    return vec4<f32>(vec3<f32>(gray), base.a);
}
