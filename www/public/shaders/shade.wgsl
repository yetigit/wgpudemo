struct HitRecord {
  // Vec4 containing vec3 as normal and an extra float as the t value
  point: vec4<f32>,
  normal: vec3<f32>,
  _pad0: u32,
}

@group(0) @binding(1) 
var outputTexture: texture_storage_2d<rgba8unorm, write>;

@group(1) @binding(4) 
var<storage, read_write> rec: array<HitRecord>;

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {

  let width = dims.x;
  let idx = global_id.y * width + global_id.x;

  var color = vec3<f32>(0.0,0.0,0.0);

  if rec[idx].point.w > 0.0 {
      // Hit Color
      color = abs(rec[idx].normal);
  }

  let pixel_color = vec4<f32>(color, 1.0);
  textureStore(outputTexture, vec2<i32>((global_id.xy)), pixel_color);
}
