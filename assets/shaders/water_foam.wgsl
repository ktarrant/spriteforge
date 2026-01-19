#import bevy_ecs_tilemap::common::process_fragment
#import bevy_ecs_tilemap::common::tilemap_data
#import bevy_ecs_tilemap::vertex_output::MeshVertexOutput

struct WaterFoamParams {
    foam_color: vec4<f32>,
    foam_settings: vec4<f32>,
};

const TAU: f32 = 6.28318530718;

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
    let lx = tile_pos.x + in.uv.z;
    let ly = (tile_pos.y + in.uv.w) * 2.0;
    let freq = params.foam_settings.y;
    let phase = tilemap_data.time * params.foam_settings.z;
    let wave = sin((lx * freq + phase) * TAU) * sin((ly * freq + phase) * TAU);
    let shimmer = wave * params.foam_settings.x;
    return vec4<f32>(base.rgb + shimmer, base.a);
}
