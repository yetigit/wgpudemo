#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Sphere {
    pub position: [f32; 3],
    pub radius: f32,
    pub material_id: i32,
    _pad0: [u32; 3]
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Material {
    pub albedo: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<Material>() % 16 == 0);
const _: () = assert!(std::mem::size_of::<Sphere>() % 16 == 0);
impl Sphere {
    pub fn new (position: [f32; 3], material_id: i32, radius: f32) -> Self {
        Self{
            position,
            radius,
            material_id,
            _pad0: [0,0,0],
        }
    }
}
