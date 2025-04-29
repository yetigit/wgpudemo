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

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let pic_width = camera.picture_width;
  let pic_height = u32(f32(pic_width) / camera.aspect_ratio);
  if (global_id.x >= pic_width || global_id.y >= pic_height) {
    return;
  }
  // Right hand system
  // NOTE: This should be done outside
  let forward = normalize(camera.look_at - camera.position);
  let right = normalize(cross(camera.up_vector, forward));
  let up = cross(forward, right);

  // Building the sensor frame
  let sensor_height = camera.sensor_height;
  let sensor_width = sensor_height * camera.aspect_ratio;

  let sensor_u = right * -sensor_width;
  let sensor_v = up * -sensor_height;
  let pixel_delta_u = sensor_u / f32(pic_width);
  let pixel_delta_v = sensor_v / f32(pic_height);
  let sensor_corner = 
  camera.position + forward * camera.focal_length
  - ((sensor_u + sensor_v) * 0.5);

  let pixeloo = sensor_corner + ((pixel_delta_u + pixel_delta_v) * 0.5);

  // Create ray
  let pixel_pos = pixeloo + 
  pixel_delta_u * f32(global_id.x) + 
  pixel_delta_v * f32(global_id.y);

  let ray = pixel_pos - camera.position;

  let index = global_id.y * pic_width + global_id.x;
  rays[index].dir = ray;
  rays[index].o = camera.position;

}
