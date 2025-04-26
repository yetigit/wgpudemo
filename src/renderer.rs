use wasm_bindgen::prelude::*;
use std::{iter, sync::Arc};
use wgpu::{include_wgsl, util::DeviceExt};

use winit::window::Window; 

use crate::camera::Camera;

pub struct Renderer {
    // Wgpu objects
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    render_pipeline: wgpu::RenderPipeline,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    compute_pipeline: Option<wgpu::ComputePipeline>,

    // Buffers and textures
    texture_rt: Option<wgpu::Texture>,
    texture_rt_view: Option<wgpu::TextureView>,
    camera_uniform: Option<wgpu::Buffer>,

    // Misc
    pub window: Arc<Window>,
    camera: Camera,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl Renderer {
    const CAMERA_UNIFORM_BIND:u32 = 0;
    const IMG_TEXTURE_BIND:u32 = 1;
    pub fn window(&self) -> &Window {
        &self.window
    }

    async fn fetch_shader(shader_path: &str) -> Result<String, JsValue> {
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Request, RequestInit, RequestMode, Response};

        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(shader_path, &opts).unwrap();

        let window = web_sys::window().expect("No web window");
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;

        let text = JsFuture::from(resp.text()?).await?;
        Ok(text.as_string().unwrap())
    }
    pub async fn new(window: Window, color_fmt: wgpu::TextureFormat) -> Self {
        let window = Arc::new(window);
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            flags: wgpu::InstanceFlags::ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER
                | wgpu::InstanceFlags::VALIDATION
                | wgpu::InstanceFlags::DEBUG,
            ..Default::default()
        });

        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let required_features = wgpu::Features::from_bits_truncate(wgpu::Features::empty().bits());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features,
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = color_fmt;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };

        let shader_code = Renderer::fetch_shader("public/shaders/simple.wgsl").await.unwrap();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // surface.configure(&device, &config);

        let camera = Camera::new();
        Self {
            surface,
            device,
            render_pipeline,
            queue,
            config,
            size,
            window,
            camera,
            compute_pipeline: None,
            camera_uniform: None,
            texture_rt: None,
            texture_rt_view: None,
        }
    }

    fn img_bytes_per_row(width: u32) -> u32 {
        let bytes_per_row = std::mem::size_of::<u32>() * width as usize;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        // NOTE: Humm there is a better way to do this with bits
        let pad = (alignment - (bytes_per_row % alignment)) % alignment;
        let num = bytes_per_row + pad;
        num as u32
    }

    fn create_img_texture(&mut self) {
        let width = self.size.width;
        let height = self.size.height;
        let buffer = vec![0 as u8; (width * height) as usize * std::mem::size_of::<u32>()];

        let texture = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: Some("Storage texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                format: self.config.format,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &buffer,
        );
        if let Some(tex) = self.texture_rt.as_ref() {
            tex.destroy();
        }
        self.texture_rt_view = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        self.texture_rt = Some(texture);
    }

    fn create_camera_uniform(&mut self) {
        let camera_uniform_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Camera uniform"),
                    contents: bytemuck::cast_slice(&[self.camera]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        // NOTE: If it's bound to the shader, what happens ?
        if let Some(cam_uniform_buf) = self.camera_uniform.as_ref() {
            cam_uniform_buf.destroy();
        }
        self.camera_uniform= Some(camera_uniform_buffer);
    }

    fn create_rt_bindgrp(&self, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main bind group"),
            // Get it from our compute pipeline
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: Renderer::CAMERA_UNIFORM_BIND,
                    resource: self
                        .camera_uniform
                        .as_ref()
                        .unwrap()
                        .as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: Renderer::IMG_TEXTURE_BIND,
                    resource: wgpu::BindingResource::TextureView(
                        self.texture_rt_view.as_ref().unwrap(),
                    ),
                },
            ],
        })
    }

    fn create_rt_pipeline(&mut self) {
        if self.compute_pipeline.is_none() {
            let layout = self
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        // camera
                        wgpu::BindGroupLayoutEntry {
                            binding: Renderer::CAMERA_UNIFORM_BIND,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // img storage buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: Renderer::IMG_TEXTURE_BIND,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: self.config.format,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });

            let compute_pipeline_layout =
                self.device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &[&layout],
                        push_constant_ranges: &[],
                    });

            {
                let shader_desc = include_wgsl!("../www/public/shaders/rt.comp.wgsl");
                let shader_mod = self.device.create_shader_module(shader_desc);
                let compute_pipeline =
                    self.device
                        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some("Compute pipeline"),
                            layout: Some(&compute_pipeline_layout),
                            module: &shader_mod,
                            entry_point: Some("main"),
                            compilation_options: Default::default(),
                            cache: None,
                        });
                self.compute_pipeline = Some(compute_pipeline);
            }
        }
    }

    pub fn on_resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) -> bool {
        
        log::warn!("w={}, h={}", new_size.width, new_size.height);

        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            log::warn!("Building or updating 🛠 buffers");
            self.camera.set_focal_length(50.0);
            self.camera.set_resolution(
                new_size.width,
                new_size.height,
                true,
            );
            self.create_img_texture();
            self.create_camera_uniform();
            self.create_rt_pipeline();
            return true;
        }
        false
    }

    pub fn render (&mut self) -> Result<(), wgpu::SurfaceError>{
        // log::warn!("Render") ; 
        let output = self.surface.get_current_texture()?;
        // let view = output
        //     .texture
        //     .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let width = self.size.width;
        let height = self.size.height;
        {
            let workgrp_x = width.div_ceil(8);
            let workgrp_y = height.div_ceil(8);
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute pass"),
                ..Default::default()
            });
            let compute_pipeline = self.compute_pipeline.as_ref().unwrap();
            let bind_grp_layout = compute_pipeline.get_bind_group_layout(0);
            let bind_grp = self.create_rt_bindgrp(&bind_grp_layout);
            compute_pass.set_pipeline(compute_pipeline);
            compute_pass.set_bind_group(0, &bind_grp, &[]);
            compute_pass.dispatch_workgroups(workgrp_x, workgrp_y, 1);
        }
        let texture = self.texture_rt.as_ref().unwrap();
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &output.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
