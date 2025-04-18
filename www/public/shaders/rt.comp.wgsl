struct Camera {
    up_vector: vec3<f32>,
    _pad0: u32,
    position: vec3<f32>,
    _pad1: u32,
    look_at: vec3<f32>,
    _pad2: u32,

    forward: vec3<f32>,
    _pad3: u32,
    right: vec3<f32>,
    _pad4: u32,
    up: vec3<f32>,
    _pad5: u32,

    focal_length: f32,
    focus_distance: f32,
    aperture: f32,

    sensor_height: f32,
    aspect_ratio: f32,
    picture_width: u32,

    aperture_radius: f32,
    fovy: f32,

    min_coc: f32,

    _pad6x: u32,
    _pad6y: u32,
    _pad6z: u32,
    
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage, read_write> outputBuffer: array<vec4<f32>>;  // Output buffer

@compute @workgroup_size(1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let width = camera.picture_width;
  let index = global_id.y * width + global_id.x;
  let pixel_color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
  outputBuffer[index] = pixel_color;
}
