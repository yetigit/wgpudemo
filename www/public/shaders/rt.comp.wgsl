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

fn hit_sphere(center:vec3<f32>, radius: f32, ro: vec3<f32>, rv: vec3<f32>) ->f32 {
    let oc = center - ro;
    let a = dot(rv, rv) ; 
    let h = dot(rv, oc);
    let c = dot(oc, oc ) - radius*radius;
    let discriminant = h*h - a*c;

    if discriminant < 0 {
        return -1.0;
    } else {
        return (h - sqrt(discriminant)) / a;
    }
}

@group(0) @binding(0) 
var<uniform> camera: Camera;
@group(0) @binding(1) 
var outputTexture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let pic_width = camera.picture_width;
  let pic_height = u32(f32(pic_width) / camera.aspect_ratio);
  if (global_id.x >= pic_width || global_id.y >= pic_height) {
    return;
  }

  let sensor_height = camera.sensor_height;
  let sensor_width = sensor_height * camera.aspect_ratio;

  let sensor_u = camera.right * -sensor_width;
  let sensor_v = camera.up * -sensor_height;
  let pixel_delta_u = sensor_u / f32(pic_width);
  let pixel_delta_v = sensor_v / f32(pic_height);
  let sensor_corner = 
  camera.position + camera.forward * camera.focal_length
  - ((sensor_u + sensor_v) * 0.5);

  let pixeloo = sensor_corner + ((pixel_delta_u + pixel_delta_v) * 0.5);

  let pixel_pos = pixeloo + 
  (pixel_delta_u * f32(global_id.x) + pixel_delta_v * f32(global_id.y));
  let ray = pixel_pos - camera.position;

  let sphere_center = vec3<f32>(0.0, 0.0, 500.0);
  let sphere_radius = 50.0;

  let s = hit_sphere(sphere_center, sphere_radius, camera.position, ray);

  var ray_color = vec3<f32>(0.0, 0.0, 0.0);

  if s > 0.0 {
    ray_color = abs(normalize((camera.position + ray * s) - sphere_center));
  }else{
    let unit_direction = normalize(ray);
    let a = 0.5 * (unit_direction.y + 1.0);
    ray_color = (1.0 - a) * vec3<f32>(1.0, 1.0, 1.0) + a * vec3<f32>(0.2, 0.3, 0.8);
  }

  let index = global_id.y * pic_width + global_id.x;
  let pixel_color = vec4<f32>(ray_color, 1.0);
  textureStore(outputTexture, vec2<i32>((global_id.xy)), pixel_color);
}
