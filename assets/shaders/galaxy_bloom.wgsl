// Bright-pass, Kawase blur, and tonemap. Dust mask multiplies the disk so
// the lanes read as faint dark bands rather than extra stars.

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct Params {
    texel: vec2<f32>,
    offset: f32,
    mode: f32, // 0 extract, 1 blur, 2 composite
    exposure: f32,
    bloom: f32,
    dust: f32,
    _pad: f32,
}

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_samp: sampler;
@group(0) @binding(2) var bloom_tex: texture_2d<f32>;
@group(0) @binding(3) var dust_tex: texture_2d<f32>;
@group(0) @binding(4) var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    var p = vec2<f32>(-1.0, 3.0);
    switch vi {
        case 0u: { p = vec2<f32>(-1.0, -1.0); }
        case 1u: { p = vec2<f32>(3.0, -1.0); }
        default: {}
    }
    out.clip = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return out;
}

fn reinhard(c: vec3<f32>) -> vec3<f32> {
    return c / (vec3<f32>(1.0) + c);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    if (params.mode < 0.5) {
        let c = textureSample(src_tex, src_samp, in.uv).rgb;
        let l = max(c.r, max(c.g, c.b));
        let k = smoothstep(0.45, 1.35, l);
        return vec4<f32>(c * k, 1.0);
    }
    if (params.mode < 1.5) {
        let o = params.offset * params.texel;
        let c = (
            textureSample(src_tex, src_samp, in.uv + vec2<f32>(o.x, o.y))
            + textureSample(src_tex, src_samp, in.uv + vec2<f32>(-o.x, o.y))
            + textureSample(src_tex, src_samp, in.uv + vec2<f32>(o.x, -o.y))
            + textureSample(src_tex, src_samp, in.uv + vec2<f32>(-o.x, -o.y))
        ) * 0.25;
        return c;
    }
    let hdr = textureSample(src_tex, src_samp, in.uv).rgb;
    let bloom = textureSample(bloom_tex, src_samp, in.uv).rgb;
    let lane = textureSample(dust_tex, src_samp, in.uv).r;
    var c = (hdr + bloom * params.bloom) * (1.0 - params.dust * clamp(lane, 0.0, 1.0));
    let sky = vec3<f32>(0.0015, 0.002, 0.006) * (1.0 - smoothstep(0.0, 0.9, length(in.uv - vec2<f32>(0.5))));
    c = c * params.exposure + sky;
    return vec4<f32>(reinhard(c), 1.0);
}
