#import bevy_ecs_tilemap::common::process_fragment
#import bevy_ecs_tilemap::common::tilemap_data
#import bevy_ecs_tilemap::vertex_output::MeshVertexOutput

const DIAMOND_BASIS_X: vec2<f32> = vec2<f32>(0.5, -0.5);
const DIAMOND_BASIS_Y: vec2<f32> = vec2<f32>(0.5, 0.5);

struct WaterFoamParams {
    foam_color: vec4<f32>,
    foam_settings: vec4<f32>,
};

@group(3) @binding(0)
var mask_texture: texture_2d<f32>;

@group(3) @binding(1)
var mask_sampler: sampler;

@group(3) @binding(2)
var<uniform> params: WaterFoamParams;

fn diamond_tile_pos_to_world_pos(pos: vec2<f32>, grid_width: f32, grid_height: f32) -> vec2<f32> {
    let unscaled_pos = pos.x * DIAMOND_BASIS_X + pos.y * DIAMOND_BASIS_Y;
    return vec2<f32>(grid_width * unscaled_pos.x, grid_height * unscaled_pos.y);
}

fn foam_factor(in: MeshVertexOutput) -> f32 {
    let tile_uv = in.uv.zw;
    let tile_pos = tilemap_data.chunk_pos + vec2<f32>(in.storage_position);
    let center = diamond_tile_pos_to_world_pos(
        tile_pos,
        tilemap_data.grid_size.x,
        tilemap_data.grid_size.y
    );
    let bot_left = center - 0.5 * tilemap_data.tile_size;
    let top_right = bot_left + tilemap_data.tile_size;
    let world_pos = mix(bot_left, top_right, tile_uv);
    let pos = world_pos / tilemap_data.tile_size;
    let phase = tilemap_data.time * params.foam_settings.z;
    let wave_a = sin((pos.x * params.foam_settings.y) * 6.283 + phase);
    let wave_b = sin((pos.y * (params.foam_settings.y * 1.3)) * 6.283 - phase * 0.8);
    let wave = (wave_a + wave_b) * 0.5;
    let wave01 = wave * 0.5 + 0.5;
    return wave01 * params.foam_settings.x;
}

@fragment
fn fragment(in: MeshVertexOutput) -> @location(0) vec4<f32> {
    let tile_uv = in.uv.zw;
    let pos = tilemap_data.chunk_pos + vec2<f32>(in.storage_position) + tile_uv;
    let v = fract((pos.x + pos.y) * 0.2);
    let mask = textureSample(mask_texture, mask_sampler, in.uv.xy).a;
    let shaded = v * mask;
    return vec4<f32>(shaded, shaded, shaded, mask);
}
