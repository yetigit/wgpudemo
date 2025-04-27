#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Sphere {
    pub position: [f32; 3],
     _pad0: u32,
    pub color: [f32; 3],
     _pad1: u32,
    pub radius: f32,
     _pad2: [u32; 3],
}

const _: () = assert!(std::mem::size_of::<Sphere>() % 16 == 0);
impl Sphere {
    pub fn new (position: [f32; 3], color: [f32; 3], radius: f32) -> Self {
        Self{
            position,
            _pad0: 0,
            color,
            _pad1: 1,
            radius,
            _pad2: [0,0,0],
        }
    }
}
