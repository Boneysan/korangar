#language slang 2026

import screen_space;
import transform;

struct ColorGrade {
    float4 value;
};

[[vk::binding(0, 0)]] var texture: Texture2D;
[[vk::binding(0, 1)]] var color_grade: ConstantBuffer<ColorGrade>;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex::new(vertex_index);
}

func apply_grade(float3 linear) -> float3 {
    let gray = dot(linear, float3(0.2126, 0.7152, 0.0722));
    let packed = color_grade.value;
    let saturated = lerp(float3(gray, gray, gray), linear, packed.w);
    return saturate(saturated * packed.xyz);
}

[[shader("pixel")]]
func fs_main(FullscreenVertex input) -> float4 {
    let linear_color = texture.Load(int3(int2(input.position.xy), 0));
    let graded = apply_grade(linear_color.rgb);
    let srgb_color = color::linear_to_srgb(graded);
    return float4(srgb_color.rgb, linear_color.a);
}
