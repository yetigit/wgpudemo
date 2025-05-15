struct HitRecord {
  point: vec4<f32>,
  normal: vec3<f32>,
  flags: u32,
  material_id: i32,
  _pad0x: u32,
  _pad0y: u32,
  _pad0z: u32,
}

struct Material {
 albedo: vec4<f32>
}


@group(0) @binding(4) 
var<storage, read_write> rec: array<HitRecord>;

@group(1) @binding(5) 
var<uniform> dims: vec2<u32>;

@group(2) @binding(1) 
var outputTexture: texture_storage_2d<rgba8unorm, write>;

@group(3) @binding(6) 
var<storage> materials: array<Material>;

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {

  let width = dims.x;
  let height = dims.y;

  if (global_id.x >= width || global_id.y >= height) {
    return;
  }
  let idx = global_id.y * width + global_id.x;

  var color = vec4<f32>(1.0, 1.0, 1.0, 1.0);

  if rec[idx].point.w > 0.0 && rec[idx].material_id != -1 {
      // Hit Color
      color = materials[rec[idx].material_id].albedo;
  }

  textureStore(outputTexture, vec2<i32>((global_id.xy)), color);
}
