// Ether Compute Shader
// 3D grid simulation of Fluid Ether

@group(0) @binding(0) var input_texture: texture_3d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_3d<rgba32float, write>;

@group(1) @binding(0) var<uniform> grid_size: vec3<u32>;
@group(1) @binding(1) var<uniform> dt: f32;
@group(1) @binding(2) var<uniform> center_sink: vec3<f32>; // position of central sink

const PHI_INV_4: f32 = 0.146446609406726237799577818947237528;

fn sample(pos: vec3<i32>) -> vec4<f32> {
    let clamped = clamp(pos, vec3<i32>(0), vec3<i32>(grid_size) - 1);
    return textureLoad(input_texture, clamped, 0);
}

@compute @workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pos = vec3<i32>(global_id);

    if any(pos >= vec3<i32>(grid_size)) {
        return;
    }

    let current = sample(pos);
    let w = current.r;
    let flow = current.gba; // x,y,z

    // Step 1: Diffusion - blur pressure (w)
    var w_sum = w;
    var count = 1.0;
    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            for (var dz = -1; dz <= 1; dz++) {
                if dx == 0 && dy == 0 && dz == 0 { continue; }
                let neighbor = sample(pos + vec3<i32>(dx, dy, dz));
                w_sum += neighbor.r;
                count += 1.0;
            }
        }
    }
    let diffused_w = w_sum / count;

    // Step 2: Advection - move flow based on curl
    // Compute curl at pos
    let dx = 1.0;
    let curl_x = (sample(pos + vec3<i32>(0,1,0)).a - sample(pos + vec3<i32>(0,-1,0)).a) / (2.0 * dx) -
                 (sample(pos + vec3<i32>(0,0,1)).b - sample(pos + vec3<i32>(0,0,-1)).b) / (2.0 * dx);
    let curl_y = (sample(pos + vec3<i32>(1,0,0)).a - sample(pos + vec3<i32>(-1,0,0)).a) / (2.0 * dx) -
                 (sample(pos + vec3<i32>(0,0,1)).g - sample(pos + vec3<i32>(0,0,-1)).g) / (2.0 * dx);
    let curl_z = (sample(pos + vec3<i32>(1,0,0)).b - sample(pos + vec3<i32>(-1,0,0)).b) / (2.0 * dx) -
                 (sample(pos + vec3<i32>(0,1,0)).g - sample(pos + vec3<i32>(0,-1,0)).g) / (2.0 * dx);

    let curl = vec3<f32>(curl_x, curl_y, curl_z);
    let advected_flow = flow + curl * dt;

    // Step 3: Sink - if star in this voxel, reduce w
    let world_pos = vec3<f32>(pos) / vec3<f32>(grid_size) * 100.0; // assume grid from -50 to 50 or something
    let distance_to_center = length(world_pos - center_sink);
    var sinked_w = diffused_w;
    if distance_to_center < 1.0 { // within 1 unit
        sinked_w -= 0.1 * dt; // reduce pressure
    }

    // Step 4: Drag - apply golden ratio rotation to flow
    let cos_mu = cos(PHI_INV_4);
    let sin_mu = sin(PHI_INV_4);
    let rotated_x = advected_flow.x * cos_mu - advected_flow.y * sin_mu;
    let rotated_y = advected_flow.x * sin_mu + advected_flow.y * cos_mu;
    let dragged_flow = vec3<f32>(rotated_x, rotated_y, advected_flow.z);

    // Write output
    textureStore(output_texture, pos, vec4<f32>(sinked_w, dragged_flow.x, dragged_flow.y, dragged_flow.z));
}