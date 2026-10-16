// Soft camera-facing sprites. Size is in pixels and grows when the camera
// comes closer, so a tilted view reads as a disk with thickness.

struct Star {
    pos: vec4<f32>,
    vel: vec4<f32>,
}

struct Camera {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    cam_pos: vec4<f32>,
    // xy viewport pixels, z pattern angle, w pass (0 stars, 1 dust)
    viewport: vec4<f32>,
    // x star brightness, y dust-mask gain
    extra: vec4<f32>,
}

@group(0) @binding(0) var<storage, read> stars: array<Star>;
@group(0) @binding(1) var<uniform> camera: Camera;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) glow: f32,
}

const GOLDEN: f32 = 2.399963;

fn star_rgb(p: vec3<f32>, v: vec3<f32>, dust: f32) -> vec3<f32> {
    let r = length(p.xz);
    let spd = length(v);
    let energy = clamp(spd / 12.0, 0.0, 1.0);
    if (dust > 0.5) {
        return vec3<f32>(0.62, 0.38, 0.16);
    }
    if (r < 20.0) {
        return mix(vec3<f32>(1.0, 0.95, 0.78), vec3<f32>(1.0, 0.62, 0.28), clamp(r / 20.0, 0.0, 1.0));
    }
    let warm = vec3<f32>(1.0, 0.74, 0.42);
    let cool = vec3<f32>(0.38, 0.72, 1.0);
    var rgb = mix(warm, cool, smoothstep(28.0, 150.0, r));
    let ang = atan2(p.z, p.x);
    let phase = ang - GOLDEN * log(max(r, 1.0)) - camera.viewport.z;
    let arm = exp(-sin(phase) * sin(phase) * 7.0);
    rgb = rgb * (0.62 + 0.7 * arm) * (0.45 + energy);
    return rgb;
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VsOut {
    var out: VsOut;
    // Dynamic indexing of a private array is not portable. vertex_index is 0..5.
    var q = vec2<f32>(1.0, 1.0);
    switch vi {
        case 0u: { q = vec2<f32>(-1.0, -1.0); }
        case 1u: { q = vec2<f32>(1.0, -1.0); }
        case 2u: { q = vec2<f32>(-1.0, 1.0); }
        case 3u: { q = vec2<f32>(-1.0, 1.0); }
        case 4u: { q = vec2<f32>(1.0, -1.0); }
        default: {}
    }
    var p = stars[ii].pos.xyz;
    let dust = stars[ii].pos.w;
    let pass_dust = camera.viewport.w > 0.5;
    if (pass_dust && dust > 0.5) {
        let r = max(length(p.xz), 1.0);
        let ang = atan2(p.z, p.x) - 0.22;
        p = vec3<f32>(r * cos(ang), p.y * 0.35, r * sin(ang));
    }
    let clip = camera.proj * camera.view * vec4<f32>(p, 1.0);
    let dist = length(camera.cam_pos.xyz - p);
    let near = clamp(420.0 / max(dist, 30.0), 0.4, 2.6);
    var px = (4.2 + stars[ii].vel.w * 2.4) * near;
    if (dust > 0.5) {
        px = 13.0 * near;
    }
    if (length(p.xz) < 22.0 && dust < 0.5) {
        px = px * 1.35;
    }
    let ndc = vec2<f32>(px) / max(camera.viewport.xy, vec2<f32>(1.0)) * 2.0;
    out.clip = vec4<f32>(clip.xy + q * ndc * clip.w, clip.z, clip.w);
    out.uv = q;
    out.color = star_rgb(stars[ii].pos.xyz, stars[ii].vel.xyz, dust);
    out.glow = dust;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let r2 = dot(in.uv, in.uv);
    if (r2 > 1.0) {
        discard;
    }
    let pass_dust = camera.viewport.w > 0.5;
    if (pass_dust && in.glow < 0.5) {
        discard;
    }
    if (!pass_dust && in.glow > 0.5) {
        discard;
    }
    let soft = exp(-r2 * 3.4);
    if (pass_dust) {
        return vec4<f32>(soft * camera.extra.y, 0.0, 0.0, soft);
    }
    let core = exp(-r2 * 14.0);
    let intensity = (soft * 0.85 + core * 1.6) * camera.extra.x;
    return vec4<f32>(in.color * intensity, soft);
}
