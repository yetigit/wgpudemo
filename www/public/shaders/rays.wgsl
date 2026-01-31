struct Camera {
  right: vec3<f32>,
  _pad0: u32,
  up: vec3<f32>,
  _pad1: u32,
  forward: vec3<f32>,
  _pad2: u32,
  pos: vec3<f32>,
  _pad3: u32,

  sensor_h: f32,

  aspect_ratio: f32,
  focal_length: f32,
  _pad4x: u32,
}

struct Controller {
   speed: f32,
   sensitivity: f32,
   fwd_move: f32,
   right_move: f32,
   pan_x: f32,
   pan_y: f32,
   yaw: f32,
   pitch: f32,
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

@group(3) @binding(8) 
var<uniform> controls: Controller;


struct CameraSensor {
  pixeloo: vec3<f32>,
  pixel_delta_u: vec3<f32>,
  pixel_delta_v: vec3<f32>,
  pos: vec3<f32>,
}

fn compute_sensor (width: u32 , height: u32) -> CameraSensor { 
  let sensor_height = camera.sensor_h;
  let sensor_width = sensor_height * camera.aspect_ratio;

  let yaw = controls.yaw;
  let pitch = controls.pitch;

  var fp: vec3<f32> = camera.forward * sin(yaw) + camera.right * cos(yaw);
  let rp = -cross(camera.up, fp);

  fp = camera.up * -sin(pitch) + fp * -cos(pitch);
  let upp= cross(fp, rp);

  let right = normalize(rp); 
  let forward = normalize(fp); 
  let up = normalize(upp);

  var pos: vec3<f32> = camera.pos;

  // apply WASD
  pos += forward * controls.fwd_move;
  pos += right * controls.right_move;
  
  // apply PAN
  pos += right * controls.pan_x;
  pos += up * controls.pan_y;
  
  //////////

  let sensor_u = right * -sensor_width;
  let sensor_v = up * -sensor_height;
  let pixel_delta_u = sensor_u / f32(width);
  let pixel_delta_v = sensor_v / f32(height);
  let sensor_corner =
      pos + forward * camera.focal_length - ((sensor_u + sensor_v) * 0.5);

  let pixeloo = sensor_corner + ((pixel_delta_u + pixel_delta_v) * 0.5);

  return CameraSensor ( 
    pixeloo,
    pixel_delta_u,
    pixel_delta_v,
    pos
  );

}


@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let width = dims.x;
  let height = dims.y;
  if (global_id.x >= width || global_id.y >= height) {
    return;
  }

  // compute sensor 
  let sensor= compute_sensor(width, height);


  // Create ray
  let pixel_pos = sensor.pixeloo + 
  sensor.pixel_delta_u * f32(global_id.x) + 
  sensor.pixel_delta_v * f32(global_id.y);

  let ray = pixel_pos - sensor.pos;

  let index = global_id.y * width + global_id.x;
  rays[index].dir = ray;
  rays[index].o = sensor.pos;

}
