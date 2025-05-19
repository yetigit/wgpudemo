pub fn uniform_bind_group_lay(device: &wgpu::Device, binding: u32) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

pub fn buf_bind_group_lay(
    device: &wgpu::Device,
    binding: u32,
    read_only: bool,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

pub fn mira(device: &wgpu::Device, hit_record_bind: u32, rays_bind: u32) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Shade stage hits and rays"),
        entries: &[
            // HitRecord buffer
            wgpu::BindGroupLayoutEntry {
                binding: hit_record_bind,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Rays buffer
            wgpu::BindGroupLayoutEntry {
                binding: rays_bind,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

pub fn mira_bind<'a, 'b>(
    device: &wgpu::Device,
    rec_rs: wgpu::BindingResource<'a>,
    rays_rs: wgpu::BindingResource<'b>,
    hit_record_bind: u32,
    rays_bind: u32,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        // Get it from our compute pipeline
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: hit_record_bind,
                resource: rec_rs,
            },
            wgpu::BindGroupEntry {
                binding: rays_bind,
                resource: rays_rs,
            },
        ],
    })
}

pub fn sphere_n_dim_group_lay(
    device: &wgpu::Device,
    sphere_bind: u32,
    dim_bind: u32,
    read_only: bool,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Shade stage spheres"),
        // Sphere buffer
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: sphere_bind,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Dim uniform
            wgpu::BindGroupLayoutEntry {
                binding: dim_bind,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

pub fn sphere_n_dim_bind_group<'a, 'b>(
    device: &wgpu::Device,
    sphere_rs: wgpu::BindingResource<'a>,
    dim_rs: wgpu::BindingResource<'b>,
    sphere_bind: u32,
    dim_bind: u32,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        // Get it from our compute pipeline
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: sphere_bind,
                resource: sphere_rs,
            },
            wgpu::BindGroupEntry {
                binding: dim_bind,
                resource: dim_rs,
            },
        ],
    })
}

pub fn material_n_seed_group_lay(
    device: &wgpu::Device,
    material_bind: u32,
    seed_bind: u32,
    read_only: bool,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        // Materials buffer
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: material_bind,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Seed uniform
            wgpu::BindGroupLayoutEntry {
                binding: seed_bind,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

pub fn material_n_seed_bind_group<'a, 'b>(
    device: &wgpu::Device,
    materials_rs: wgpu::BindingResource<'a>,
    seed_rs: wgpu::BindingResource<'b>,
    material_bind: u32,
    seed_bind: u32,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        // Get it from our compute pipeline
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: material_bind,
                resource: materials_rs,
            },
            wgpu::BindGroupEntry {
                binding: seed_bind,
                resource: seed_rs,
            },
        ],
    })
}

pub fn bind_group_from<'a>(
    device: &wgpu::Device,
    resource: wgpu::BindingResource<'a>,
    binding: u32,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        // Get it from our compute pipeline
        layout,
        entries: &[wgpu::BindGroupEntry { binding, resource }],
    })
}

pub fn img_texture_bind_group_lay(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    binding: u32,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    })
}

pub fn img_texture_bind_group<'a>(
    device: &wgpu::Device,
    texture_view: &'a wgpu::TextureView,
    binding: u32,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Main bind group"),
        // Get it from our compute pipeline
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(texture_view),
        }],
    })
}
