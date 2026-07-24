#language slang 2026

// GL-fallback variant of the ground-decal pass (see `ground_decal_bindless.slang`
// for the full explanation). Here textures are bound one at a time in group 3;
// the drawer issues one draw per texture run, so there is no per-instance texture
// index.

import globals;
import forward;
import matrix;
import transform;

struct GroundDecalInstanceData {
    float4 corners[4];
    float4 texture_coordinates[2];
    float4 color;
    int texture_index;
};

struct GroundDecalVertexInput {
    uint vertex_index : SV_VulkanVertexID;
    uint instance_index : SV_VulkanInstanceID;
};

struct GroundDecalVertexOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] var texture_coordinates: float2;
    [[vk::location(1)]] var color: float4;
};

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(1, 0)]] var nearest_sampler: SamplerState;
[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 2)]] var instance_data: StructuredBuffer<GroundDecalInstanceData>;
[[vk::binding(0, 3)]] var texture: Texture2D;

// tl, tr, bl, bl, tr, br — two triangles from the four corners.
[ForceInline]
func corner_index(vertex_index: uint) -> uint {
    switch (vertex_index) {
        case 0: return 0;
        case 1: return 1;
        case 2: return 2;
        case 3: return 2;
        case 4: return 1;
        default: return 3;
    }
}

[ForceInline]
func decal_uv(instance: GroundDecalInstanceData, corner: uint) -> float2 {
    switch (corner) {
        case 0: return instance.texture_coordinates[0].xy;
        case 1: return instance.texture_coordinates[0].zw;
        case 2: return instance.texture_coordinates[1].xy;
        default: return instance.texture_coordinates[1].zw;
    }
}

[[shader("vertex")]]
func vs_main(input: GroundDecalVertexInput) -> GroundDecalVertexOutput {
    let instance = instance_data[input.instance_index];
    let corner = corner_index(input.vertex_index);
    let world_position = float4(instance.corners[corner].xyz, 1.0);

    var output: GroundDecalVertexOutput;
    output.position = mul(global_uniforms.view_projection, world_position);
    output.texture_coordinates = decal_uv(instance, corner);
    output.color = instance.color;
    return output;
}

[[shader("pixel")]]
func fs_main(input: GroundDecalVertexOutput) -> float4 {
    let sampled = texture.Sample(linear_sampler, input.texture_coordinates);
    let color = sampled * input.color;

    if (color.a < 0.02) {
        discard;
    }

    return color;
}
