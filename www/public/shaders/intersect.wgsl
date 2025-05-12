
struct Ray {
  dir: vec3<f32>,
  _pad0: u32,
  o: vec3<f32>,
  _pad1: u32,
}
struct Sphere {
   position: vec3<f32>,

   _pad0: u32,
   color: vec3<f32>,

   _pad1: u32,

   radius: f32,

   _pad2x: u32,
   _pad2y: u32,
   _pad2z: u32,
}

struct HitRecord {
  // Vec4 containing vec3 as normal and an extra float as the t value
  point: vec4<f32>,
  normal: vec3<f32>,
  flags: u32,
}

@group(0) @binding(2) 
var<storage, read_write> rays: array<Ray>;

@group(1) @binding(4) 
var<storage, read_write> rec: array<HitRecord>;

@group(2) @binding(3) 
var<storage> world_spheres: array<Sphere>;


@group(3) @binding(5) 
var<uniform> dims: vec2<u32>;

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

fn set_hit_orientation (ray: ptr<storage, vec3<f32>, read_write>,
  irec : ptr<storage, HitRecord, read_write>) {

  // Ray started from inside and hit the surface from the back
  if dot(irec.normal, *ray) > 0.0 {
    // NOTE: Make it point outward
    irec.normal = -irec.normal;
    irec.flags = irec.flags | 0x1;
  }
}

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let width = dims.x;
  let height = dims.y;

  if (global_id.x >= width || global_id.y >= height) {
    return;
  }

  let ray_id = global_id.y * width + global_id.x;

  var closest_hit: f32 = -1.0;
  var closest_sphere = 0u;
  let dir = rays[ray_id].dir;
  let o = rays[ray_id].o;

  for (var i = 0u; i < arrayLength(&world_spheres); i++) {
    let sphere_center = world_spheres[i].position;
    let sphere_radius = world_spheres[i].radius;

    // Cast ray
    let s = hit_sphere(sphere_center, sphere_radius, o, dir, 0.001, 99999.0);

    let is_valid_hit = s > 0.0 && (closest_hit < 0.0 || s < closest_hit);

    closest_hit = select(closest_hit, s, is_valid_hit);
    closest_sphere = select(closest_sphere, i, is_valid_hit);
  }

  let hit_point = o + dir * closest_hit;
  let normal = normalize(hit_point - world_spheres[closest_sphere].position);
  rec[ray_id].point = vec4<f32>(hit_point.xyz, closest_hit);
  rec[ray_id].normal = normal;
  set_hit_orientation(&rays[ray_id].dir, &rec[ray_id]);
  
}
