#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Sphere {
    position: [f32; 3],
    _pad0: u32,
    color: [f32; 3],
    _pad1: u32,
    radius: f32,
    _pad2: [u32; 3],
}

const _: () = assert!(std::mem::size_of::<Sphere>() % 16 == 0);
