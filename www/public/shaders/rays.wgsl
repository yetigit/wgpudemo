struct Camera {
  pixeloo: vec3<f32>,
  _pad0: u32,
  pixel_delta_u: vec3<f32>,
  _pad1: u32,
  pixel_delta_v: vec3<f32>,
  _pad2: u32,
  pos: vec3<f32>,
  _pad3: u32,
}

struct Ray {
  dir: vec3<f32>,
  _pad0: u32,
  o: vec3<f32>,
  _pad1: u32,
}

@group(0) @binding(0) 
var<uniform> camera: Camera;

@group(1) @binding(2) 
var<storage, read_write> rays: array<Ray>;

@group(2) @binding(5) 
var<uniform> dims: vec2<u32>;

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let width = dims.x;
  let height = dims.y;
  if (global_id.x >= width || global_id.y >= height) {
    return;
  }

  // Create ray
  let pixel_pos = camera.pixeloo + 
  camera.pixel_delta_u * f32(global_id.x) + 
  camera.pixel_delta_v * f32(global_id.y);

  let ray = pixel_pos - camera.pos;

  let index = global_id.y * width + global_id.x;
  rays[index].dir = ray;
  rays[index].o = camera.pos;

}
