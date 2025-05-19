struct HitRecord {
  point: vec4<f32>,
  normal: vec3<f32>,
  flags: u32,
  material_id: i32,
  _pad0x: u32,
  _pad0y: u32,
  _pad0z: u32,
}

struct Sphere {
   position: vec3<f32>,
   radius: f32,
   material_id: i32,
   _pad0x: u32,
   _pad0y: u32,
   _pad0z: u32,
}

struct Material {
 albedo: vec4<f32>,
 kind: u32,
 fuzz: f32,
 _pad0x : u32,
 _pad0y : u32,
}

struct Ray {
  dir: vec3<f32>,
  _pad0: u32,
  o: vec3<f32>,
  _pad1: u32,
}


@group(0) @binding(4) 
var<storage, read_write> rec: array<HitRecord>;
@group(0) @binding(2) 
var<storage, read_write> rays: array<Ray>;

@group(1) @binding(3) 
var<storage> world_spheres: array<Sphere>;
@group(1) @binding(5) 
var<uniform> dims: vec2<u32>;

@group(2) @binding(1) 
var outputTexture: texture_storage_2d<rgba8unorm, write>;

@group(3) @binding(6) 
var<storage> materials: array<Material>;
@group(3) @binding(7) 
var<uniform> u_seed: f32;


fn is_near_zero(v: vec3<f32>) -> bool {
  let epsilon: f32 = 0.0001;
  return dot(v, v) < (epsilon * epsilon);
}

// [0, 1] Random value
fn rand(seed: ptr<function, f32>, pixel: vec2<f32>) -> f32
{
    let result: f32 = fract(sin(*seed / 100.0f * dot(pixel, vec2<f32>(12.9898f, 78.233f))) * 43758.5453f);
    *seed = *seed + 1.0f;
    return result;
}

fn rand3(seed: ptr<function, f32>, pixel: vec2<f32>) -> vec3<f32>
{
    let x: f32 = fract(sin(*seed / 100.0f * dot(pixel, vec2<f32>(12.9898f, 78.233f))) * 43758.5453f);
    let y: f32 = fract(sin((*seed+1) / 100.0f * dot(pixel, vec2<f32>(12.9898f, 78.233f))) * 43758.5453f);
    let z: f32 = fract(sin((*seed+2) / 100.0f * dot(pixel, vec2<f32>(12.9898f, 78.233f))) * 43758.5453f);
    *seed = *seed + 3;
    return vec3<f32>(x, y, z);
}

fn random_in_sphere(seed: ptr<function, f32>, pixel: vec2<f32>) -> vec3<f32> {
  while (true){
    var rand_vec = rand3(seed, pixel);
    rand_vec *= 2.0;
    rand_vec += vec3<f32>(-1.0, -1.0, -1.0);
    let len_sq = dot(rand_vec, rand_vec);
    let eps = 0.0001;
    if len_sq > eps {
      return rand_vec / sqrt(len_sq);
    }
  }
  return vec3<f32>();
}

fn random_in_hemisphere(normal: vec3<f32>, seed: ptr<function, f32>, pixel: vec2<f32>) -> vec3<f32> {
  let dir = random_in_sphere(seed, pixel);
  if (dot(normal, dir) > 0.0){
    return dir;
  }else{
    return -dir;
  }
}

fn reflect(dir: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
   return dir - (2.0 * (dot(dir, normal) * normal));
}

fn hit_sphere(center:vec3<f32>, radius: f32, ro: vec3<f32>, rv: vec3<f32>, 
  tmin: f32, tmax: f32) -> f32 {

  let oc = center - ro;
  let a = dot(rv, rv) ; 
  let h = dot(rv, oc);
  let c = dot(oc, oc) - radius*radius;
  let discriminant = h*h - a*c;

  if discriminant < 0.0 {
      return -1.0;
  }
  let sqroot = sqrt(discriminant);

  var root = (h - sqroot) / a;
  if (root <= tmin || root >= tmax){
    root = (h + sqroot) / a;
    if (root <= tmin || root >= tmax){
      return -1.0;
    }
  }
  return root;
}

// ray_dir is normalized
fn set_hit_orientation (ray_dir: vec3<f32>,
  irec : ptr<function, HitRecord>) {

  // Ray started from inside and hit the surface from the back
  if dot(irec.normal, ray_dir) > 0.0 {
    // NOTE: Make it point outward
    irec.normal = -irec.normal;
    irec.flags = irec.flags | 0x1;
  }
}

fn hit_any(ray: ptr<function, Ray>, new_rec : ptr<function, HitRecord>) -> bool {
  var closest_hit: f32 = -1.0;
  var closest_sphere = 0u;

  for (var i = 0u; i < arrayLength(&world_spheres); i++) {
    let sphere_center = world_spheres[i].position;
    let sphere_radius = world_spheres[i].radius;

    let eps = 0.01;
    // Cast ray
    let s = hit_sphere(sphere_center, sphere_radius, ray.o, ray.dir, eps, 99999.0);

    let is_valid_hit = s > 0.0 && (closest_hit < 0.0 || s < closest_hit);

    closest_hit = select(closest_hit, s, is_valid_hit);
    closest_sphere = select(closest_sphere, i, is_valid_hit);
  }

  if closest_hit < 0.0 {
    return false;
  }

  let hit_point = ray.o + ray.dir * closest_hit;
  let normal = normalize(hit_point - world_spheres[closest_sphere].position);
  new_rec.point = vec4<f32>(hit_point.xyz, closest_hit);
  new_rec.normal = normal;
  new_rec.material_id = world_spheres[closest_sphere].material_id;
  
  set_hit_orientation(ray.dir, new_rec);
  return true;
}

fn linear_to_srgb(linear: f32) -> f32{
    if (linear <= 0.0031308f){
        return linear * 12.92f;
    }
    else {
        return 1.055f * pow(linear, 1.0f / 2.4f) - 0.055f;
    }
}

fn to_srgb (color: vec4<f32>) -> vec4<f32> {
  return vec4<f32> (
  linear_to_srgb(color.x),
  linear_to_srgb(color.y),
  linear_to_srgb(color.z), 
  color.w);
}

fn scatter_lambert(hit_info: ptr<function, HitRecord>, 
  seed: ptr<function, f32>, 
  pixel: vec2<f32>,
  ray :ptr<function, Ray>) -> bool {

  let rand_vec = random_in_hemisphere(hit_info.normal, seed, pixel);
  var dir = normalize(rand_vec + hit_info.normal);
  if is_near_zero(dir) {
    dir = hit_info.normal;
  }

  ray.dir = dir;
  ray.o = hit_info.point.xyz;
  return true;
}

fn scatter_metal(vec: vec3<f32>, 
  fuzz: f32, 
  hit_info: ptr<function, HitRecord>,
  seed: ptr<function, f32>, 
  pixel: vec2<f32>,
  ray: ptr<function, Ray>) -> bool {
  var dir = reflect(vec, hit_info.normal);
  dir = normalize(normalize(dir) + fuzz * random_in_hemisphere(hit_info.normal, seed, pixel));

  ray.dir = dir;
  ray.o = hit_info.point.xyz;
  // NOTE: In theory never false since I use random in hemisphere
  let ret = dot(dir, hit_info.normal) > 0.0;
  return ret;
}


@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {

  let width = dims.x;
  let height = dims.y;

  if (global_id.x >= width || global_id.y >= height) {
    return;
  }
  let idx = global_id.y * width + global_id.x;
  let pixel = vec2<f32>(global_id.xy);

  var color = vec4<f32>(0.0, 0.0, 0.0, 1.0);

  // How much ping-pong ===========
  let max_bounce = 100; 
  // ==============================

  var seed = u_seed;
  let pseed: ptr<function, f32> = &seed;

  if rec[idx].point.w > 0.0 && rec[idx].material_id != -1 {
    // Hit Color
    var attenuation = materials[rec[idx].material_id].albedo.xyz;
    var depth = 0;
    var b_loop = true;

    var bounce_rec: HitRecord;
    let bounce_rec_ptr: ptr<function, HitRecord> = &bounce_rec;
    bounce_rec = rec[idx];

    while (depth < max_bounce && b_loop) {

      let material_kind = materials[bounce_rec.material_id].kind;
      let fuzz_value = materials[bounce_rec.material_id].fuzz;

      var ray: Ray;
      let ray_ptr: ptr<function, Ray> = &ray;
      var bounces = false;

      if (material_kind == 0){
        bounces = scatter_lambert(bounce_rec_ptr, pseed, pixel, ray_ptr);
      } else if (material_kind == 1) {
        bounces = scatter_metal(rays[idx].dir, fuzz_value, bounce_rec_ptr, pseed, pixel, ray_ptr);
        if (bounces == false) {
          b_loop = false;
          break;
        }
      }

      // Reset to initial value
      bounce_rec.point.w = -1.0;
      bounce_rec.flags = 0x0;
      bounce_rec.material_id = -1;

      if (hit_any(ray_ptr, bounce_rec_ptr)){
        attenuation *= materials[bounce_rec.material_id].albedo.xyz;
        rays[idx] = ray;
      }else {
        attenuation *= vec3<f32>(0.7, 0.7, 0.7);
        b_loop = false;
      }
      depth += 1;
    }

    color = vec4<f32>(attenuation, 1.0);

  }
  textureStore(outputTexture, vec2<i32>(global_id.xy), to_srgb(color));
}
