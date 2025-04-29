#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Ray {
    dir: [f32; 3],
    _pad0: u32,
    o: [f32; 3],
    _pad1: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HitRecord {
    // Vec4 containing vec3 as normal and an extra float as the t value
    point: [f32; 4],
    normal: [f32; 3],
    _pad0: u32,
}

const _: () = assert!(std::mem::size_of::<HitRecord>() % 16 == 0);
const _: () = assert!(std::mem::size_of::<Ray>() % 16 == 0);
