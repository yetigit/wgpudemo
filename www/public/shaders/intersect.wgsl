
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
  _pad0: u32,
}

@group(0) @binding(2) 
var<storage, read_write> rays: array<Ray>;

@group(1) @binding(4) 
var<storage, read_write> rec: array<HitRecord>;

@group(2) @binding(3) 
var<storage> world_spheres: array<Sphere>;


@group(3) @binding(5) 
var<uniform> dims: vec2<u32>;

fn hit_sphere(center:vec3<f32>, radius: f32, ro: vec3<f32>, rv: vec3<f32>) ->f32 {
    let oc = center - ro;
    let a = dot(rv, rv) ; 
    let h = dot(rv, oc);
    let c = dot(oc, oc ) - radius*radius;
    let discriminant = h*h - a*c;

    if discriminant < 0.0 {
        return -1.0;
    } else {
        return (h - sqrt(discriminant)) / a;
    }
}

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let width = dims.x;
  let ray_id = global_id.y * width + global_id.x;


  var closest_hit: f32 = -1.0;
  var closest_sphere = 0u;
  let dir = rays[ray_id].dir;
  let o = rays[ray_id].o;

  for (var i = 0u; i < arrayLength(&world_spheres); i++) {
    let sphere_center = world_spheres[i].position;
    let sphere_radius = world_spheres[i].radius;

    // Cast ray
    let s = hit_sphere(sphere_center, sphere_radius, o, dir);

    if s > 0.0 {

      if closest_hit < 0.0 {
        closest_hit = s;
        closest_sphere = i;
      }
      if s < closest_hit {
        closest_hit = s;
        closest_sphere = i;
      }

    }
  }

  if closest_hit > 0.0 {
      let hit_point = o + dir * closest_hit;
      let normal = normalize(hit_point - world_spheres[closest_sphere].position);
      rec[ray_id].point = vec4<f32>(hit_point.xyz, closest_hit);
      rec[ray_id].normal = normal;
  }
  
}
