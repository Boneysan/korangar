#language slang 2026

import postprocessing;

public struct EffectVertexOutput {
    public float4 position : SV_Position;
    [[vk::location(0)]] public var texture_coordinates: float2;
    [[vk::location(1)]] public var color: float4;
    [[vk::location(2)]] public var texture_index: int;
}

[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var instance_data: StructuredBuffer<EffectInstanceData>;
[[vk::binding(1, 1)]] var textures: Texture2D[];

[[shader("vertex")]]
func vs_main(input: PostprocessingVertexInput) -> EffectVertexOutput {
    let instance = instance_data[input.instance_index];

    let positions = effect_position_data(instance, input.vertex_index);

    var output: EffectVertexOutput;
    output.position = float4(positions.xy, 0.0, 1.0);
    output.texture_coordinates = positions.zw;
    output.color = float4(instance.color0, instance.color1);
    output.texture_index = instance.texture_index;
    return output;
}

/// Below this sampled alpha a texel is treated as keyed-out and never written.
static const var EFFECT_ALPHA_TEST: float = 0.004;

[[shader("pixel")]]
func fs_main(input: EffectVertexOutput) -> float4 {
    let color = textures[input.texture_index].Sample(linear_sampler, input.texture_coordinates) * input.color;

    // RO effect textures are alpha-keyed and the original client relies on alpha
    // TESTING, not just blending. Most STR layers blend additively, where a
    // transparent texel is black and therefore invisible for free — which is why
    // this was never noticed. But layers that blend One/Zero (silence.str layer
    // 6, every layer of sleep.str) write the source verbatim, painting the
    // texture's black backing as an opaque square. Discarding keyed-out texels
    // restores the original behaviour for those layers and changes nothing for
    // the additive ones.
    if (color.a <= EFFECT_ALPHA_TEST) {
        discard;
    }

    return color;
}
