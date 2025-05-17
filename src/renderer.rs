use wasm_bindgen::prelude::*;
use std::{iter, sync::Arc};
use wgpu::{include_wgsl, util::DeviceExt};

use winit::window::Window; 

use crate::camera::Camera;
use crate::sphere::{Sphere, Material};

use crate::intersection::{ Ray, HitRecord };
use crate::binding;

pub struct Renderer {
    // Wgpu objects
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    compute_pipeline: [Option<wgpu::ComputePipeline>; 4],

    // Buffers and textures
    // Ray pass
    camera_uniform: Option<wgpu::Buffer>,
    seed_uniform: Option<wgpu::Buffer>,
    dim_uniform: Option<wgpu::Buffer>,
    rays_buf: Option<wgpu::Buffer>,
    // Intersection pass
    hit_buf: Option<wgpu::Buffer>,
    materials_buf: Option<wgpu::Buffer>,
    spheres_buf: Option<wgpu::Buffer>,
    // Final texture
    frame_texture: Option<wgpu::Texture>,
    frame_texview: Option<wgpu::TextureView>,

    // Materials 
    materials: Vec<Material>,
    // Misc
    pub window: Arc<Window>,
    camera: Camera,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl Renderer {
    const CAMERA_UNIFORM_BIND: u32 = 0;
    const IMG_TEX_BIND: u32 = 1;
    const RAYS_BUF_BIND: u32 = 2;
    const SPHERE_BUF_BIND: u32 = 3;
    const HIT_REC_BUF_BIND: u32 = 4;
    const DIM_UNIFORM_BIND: u32 = 5;
    const MAT_BUF_BIND: u32 = 6;
    const SEED_UNIFORM_BIND: u32 = 7;

    fn ray_pipeline(&self) -> Option<&wgpu::ComputePipeline> {
        self.compute_pipeline[0].as_ref()
    }

    fn intersect_pipeline(&self) -> Option<&wgpu::ComputePipeline> {
        self.compute_pipeline[1].as_ref()
    }

    fn shade_pipeline(&self) -> Option<&wgpu::ComputePipeline> {
        self.compute_pipeline[2].as_ref()
    }

    fn set_ray_pipeline(
        &mut self,
        pipeline: wgpu::ComputePipeline,
    ) -> Option<wgpu::ComputePipeline> {
        self.compute_pipeline[0].replace(pipeline)
    }

    fn set_intersect_pipeline(
        &mut self,
        pipeline: wgpu::ComputePipeline,
    ) -> Option<wgpu::ComputePipeline> {
        self.compute_pipeline[1].replace(pipeline)
    }

    fn set_shade_pipeline(
        &mut self,
        pipeline: wgpu::ComputePipeline,
    ) -> Option<wgpu::ComputePipeline> {
        self.compute_pipeline[2].replace(pipeline)
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    #[allow(dead_code)]
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

        let camera = Camera::new();
        Self {
            surface,
            device,
            queue,
            config,
            compute_pipeline: [None, None, None, None],
            camera_uniform: None,
            seed_uniform: None,
            dim_uniform: None,
            rays_buf: None,
            hit_buf: None,
            materials_buf: None,
            spheres_buf: None,
            frame_texture: None,
            frame_texview: None,
            materials: Vec::new(),
            window,
            camera,
            size,
        }
    }

    // NOTE: Later we had other parameters to control the number of rays per pixel
    // NOTE: For now one ray per pixel
    pub fn num_rays(&self) -> u32 {
        self.size.width * self.size.height
    }

    #[allow(dead_code)]
    fn img_bytes_per_row(width: u32) -> u32 {
        let bytes_per_row = std::mem::size_of::<u32>() * width as usize;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        // NOTE: Humm there is a better way to do this with bits
        let pad = (alignment - (bytes_per_row % alignment)) % alignment;
        let num = bytes_per_row + pad;
        num as u32
    }

    fn create_rec_buf (&mut self ) {
        let buffer = vec![0 as u8; self.num_rays() as usize * std::mem::size_of::<HitRecord>()];

        let hit_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Hit rec buffer"),
                contents: &buffer,
                usage: wgpu::BufferUsages::STORAGE,
            });
        if let Some(buf) = self.hit_buf.as_ref() {
            buf.destroy();
        }
        self.hit_buf = Some(hit_buf);
    }

    fn create_ray_buf (&mut self ) {
        let buffer = vec![0 as u8; self.num_rays() as usize * std::mem::size_of::<Ray>()];

        let ray_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Ray buffer"),
                contents: &buffer,
                usage: wgpu::BufferUsages::STORAGE,
            });
        if let Some(buf) = self.rays_buf.as_ref() {
            buf.destroy();
        }
        self.rays_buf = Some(ray_buf);
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
        if let Some(tex) = self.frame_texture.as_ref() {
            tex.destroy();
            self.frame_texview = None;
        }
        self.frame_texview = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        self.frame_texture = Some(texture);
    }

    fn create_dim_uniform (&mut self)  {

        let uniform_buf =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dimensions uniform"),
                    contents: bytemuck::cast_slice(&[self.size.width, self.size.height]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        if let Some(uniform) = self.dim_uniform.as_ref() {
            uniform.destroy();
        }
        self.dim_uniform= Some(uniform_buf);
    }

    fn create_seed_uniform (&mut self)  {
        let seed: f32 = 3.0;
        let uniform_buf =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Seed uniform"),
                    contents: bytemuck::cast_slice(&[seed]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        if let Some(uniform) = self.seed_uniform.as_ref() {
            uniform.destroy();
        }
        self.seed_uniform= Some(uniform_buf);
    }


    fn create_camera_uniform(&mut self) {
        let camera_uniform_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Camera uniform"),
                    contents: bytemuck::cast_slice(&[self.camera]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        if let Some(uniform) = self.camera_uniform.as_ref() {
            uniform.destroy();
        }
        self.camera_uniform= Some(camera_uniform_buffer);
    }

    pub fn make_world(&mut self) {
        let red_cap = Material {albedo: [0.1, 0.2, 0.5, 1.0]};
        let green_cap = Material {albedo: [0.8, 0.8, 0.0, 1.0]};
        let blue_cap = Material {albedo: [0.5, 0.2, 0.1, 1.0]};
        self.materials.push(red_cap);
        self.materials.push(green_cap);
        self.materials.push(blue_cap);

        #[rustfmt::skip]
        let top_sphere = Sphere::new(
             [0.0, 0.0, 500.0],
             0,
             50.0,
        );
        #[rustfmt::skip]
        let bottom_sphere  =  Sphere::new(
             [0.0, -1050.0, 500.0],
             1,
             1000.0,
        );

        let mut spheres = vec![top_sphere, bottom_sphere];
        #[rustfmt::skip]
        spheres.push (
          Sphere::new(
             [-80.0, -20.0, 300.0],
             2,
             50.0)
        ); 

        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Spheres array"),
                contents: bytemuck::cast_slice(spheres.as_slice()),
                usage: wgpu::BufferUsages::STORAGE,
            });
        if let Some(sphere_buf) = self.spheres_buf.as_ref() {
            sphere_buf.destroy();
        }
        self.spheres_buf = Some(buf);

        // Make material buffer
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Materials"),
                contents: bytemuck::cast_slice(self.materials.as_slice()),
                usage: wgpu::BufferUsages::STORAGE,
            });
        if let Some(matbuf) = self.materials_buf.as_ref() {
            matbuf.destroy();
        }
        self.materials_buf = Some(buf);

        self.create_seed_uniform();
    }


    fn create_pipelines(&mut self) {
        let rays_grp_lay = binding::buf_bind_group_lay(&self.device, Renderer::RAYS_BUF_BIND, false);

        if self.ray_pipeline().is_none() {
            let camera_grp_lay =
                binding::uniform_bind_group_lay(&self.device, Renderer::CAMERA_UNIFORM_BIND);

            let compute_pipeline_layout =
                self.device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &[&camera_grp_lay, &rays_grp_lay],
                        push_constant_ranges: &[],
                    });

            let shader_desc = include_wgsl!("../www/public/shaders/rays.wgsl");
            let shader_mod = self.device.create_shader_module(shader_desc);
            let compute_pipeline =
                self.device
                    .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("Rays pipeline"),
                        layout: Some(&compute_pipeline_layout),
                        module: &shader_mod,
                        entry_point: Some("main"),
                        compilation_options: Default::default(),
                        cache: None,
                    });
            let _ = self.set_ray_pipeline(compute_pipeline);
        }

        let hit_rec_lay =
            binding::buf_bind_group_lay(&self.device, Renderer::HIT_REC_BUF_BIND, false);

        let dim_grp_lay = binding::uniform_bind_group_lay(&self.device, Renderer::DIM_UNIFORM_BIND);


        if self.intersect_pipeline().is_none() {
            let sphere_grp_lay =
                binding::buf_bind_group_lay(&self.device, Renderer::SPHERE_BUF_BIND, true);

            let compute_pipeline_layout =
                self.device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &[
                            &rays_grp_lay,
                            &hit_rec_lay,
                            &sphere_grp_lay,
                            &dim_grp_lay,
                        ],
                        push_constant_ranges: &[],
                    });

            let shader_desc = include_wgsl!("../www/public/shaders/intersect.wgsl");
            let shader_mod = self.device.create_shader_module(shader_desc);
            let compute_pipeline =
                self.device
                    .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("Intersect pipeline"),
                        layout: Some(&compute_pipeline_layout),
                        module: &shader_mod,
                        entry_point: Some("main"),
                        compilation_options: Default::default(),
                        cache: None,
                    });
            let _ = self.set_intersect_pipeline(compute_pipeline);
        }

        if self.shade_pipeline().is_none() {
            let frame_tex_lay = binding::img_texture_bind_group_lay(
                &self.device,
                self.config.format,
                Renderer::IMG_TEX_BIND,
            );

            let material_grp_lay = binding::material_n_seed_group_lay(
                &self.device,
                Renderer::MAT_BUF_BIND,
                Renderer::SEED_UNIFORM_BIND,
                true,
            );

            let sphere_dim_grp_lay = binding::sphere_n_dim_group_lay(
                &self.device,
                Renderer::SPHERE_BUF_BIND,
                Renderer::DIM_UNIFORM_BIND,
                true,
            );


            let compute_pipeline_layout =
                self.device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &[
                            &hit_rec_lay,
                            &sphere_dim_grp_lay,
                            &frame_tex_lay,
                            &material_grp_lay,
                        ],
                        push_constant_ranges: &[],
                    });

            let shader_desc = include_wgsl!("../www/public/shaders/shade.wgsl");
            let shader_mod = self.device.create_shader_module(shader_desc);
            let compute_pipeline =
                self.device
                    .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("Shade pipeline"),
                        layout: Some(&compute_pipeline_layout),
                        module: &shader_mod,
                        entry_point: Some("main"),
                        compilation_options: Default::default(),
                        cache: None,
                    });
            let _ = self.set_shade_pipeline(compute_pipeline);

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
            self.camera.set_focal_length(35.0);
            self.camera.set_resolution(
                new_size.width,
                new_size.height,
                true,
            );
            self.camera.position = [0.0, 400.0, -100.0]; 
            self.camera.look_at = [0.0, 0.0, 500.0];

            self.create_img_texture();
            // NOTE: We could create the buffers, than update the resolution of the camera and dim
            // uniform
            self.create_camera_uniform();
            self.create_dim_uniform();
            self.create_ray_buf();
            self.create_rec_buf();

            self.create_pipelines();
            return true;
        }
        false
    }

    fn set_buffer_binding<'a, 'b>(
        &self,
        compute_pass: &mut wgpu::ComputePass<'a>,
        pipeline: &wgpu::ComputePipeline,
        resource: wgpu::BindingResource<'b>,
        grp_index: u32,
        binding: u32,
    ) {
        let grp = binding::bind_group_from(
            &self.device,
            resource,
            binding,
            &pipeline.get_bind_group_layout(grp_index),
        );
        compute_pass.set_bind_group(grp_index, &grp, &[]);
    }

    pub fn render (&mut self) -> Result<(), wgpu::SurfaceError>{
        // log::warn!("Render") ; 
        let output = self.surface.get_current_texture()?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let width = self.size.width;
        let height = self.size.height;

        let workgrp_x = width.div_ceil(8);
        let workgrp_y = height.div_ceil(8);

        // Rays pass #########################################
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Ray pass"),
            ..Default::default()
        });
        let compute_pipeline = self.ray_pipeline().unwrap();

        compute_pass.set_pipeline(compute_pipeline);

        self.set_buffer_binding(
            &mut compute_pass,
            compute_pipeline,
            self.camera_uniform.as_ref().unwrap().as_entire_binding(),
            0,
            Renderer::CAMERA_UNIFORM_BIND,
        );

        self.set_buffer_binding(
            &mut compute_pass,
            compute_pipeline,
            self.rays_buf.as_ref().unwrap().as_entire_binding(),
            1,
            Renderer::RAYS_BUF_BIND,
        );

        compute_pass.dispatch_workgroups(workgrp_x, workgrp_y, 1);
        std::mem::drop(compute_pass);

        // Intersection pass ##################################

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Intersect pass"),
            ..Default::default()
        });
        let compute_pipeline = self.intersect_pipeline().unwrap();

        compute_pass.set_pipeline(compute_pipeline);

        self.set_buffer_binding(
            &mut compute_pass,
            compute_pipeline,
            self.rays_buf.as_ref().unwrap().as_entire_binding(),
            0,
            Renderer::RAYS_BUF_BIND,
        );

        self.set_buffer_binding(
            &mut compute_pass,
            compute_pipeline,
            self.hit_buf.as_ref().unwrap().as_entire_binding(),
            1,
            Renderer::HIT_REC_BUF_BIND,
        );

        self.set_buffer_binding(
            &mut compute_pass,
            compute_pipeline,
            self.spheres_buf.as_ref().unwrap().as_entire_binding(),
            2,
            Renderer::SPHERE_BUF_BIND,
        );

        self.set_buffer_binding(
            &mut compute_pass,
            compute_pipeline,
            self.dim_uniform.as_ref().unwrap().as_entire_binding(),
            3,
            Renderer::DIM_UNIFORM_BIND,
        );

        compute_pass.dispatch_workgroups(workgrp_x, workgrp_y, 1);
        std::mem::drop(compute_pass);

        // Shading pass ##################################

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Shading pass"),
            ..Default::default()
        });
        let compute_pipeline = self.shade_pipeline().unwrap();

        compute_pass.set_pipeline(compute_pipeline);

        self.set_buffer_binding(
            &mut compute_pass,
            compute_pipeline,
            self.hit_buf.as_ref().unwrap().as_entire_binding(),
            0,
            Renderer::HIT_REC_BUF_BIND,
        );

        // Bind spheres and dim
        {
            let grp = binding::sphere_n_dim_bind_group(
                &self.device,
                self.spheres_buf.as_ref().unwrap().as_entire_binding(),
                self.dim_uniform.as_ref().unwrap().as_entire_binding(),
                Renderer::SPHERE_BUF_BIND,
                Renderer::DIM_UNIFORM_BIND,
                &compute_pipeline.get_bind_group_layout(1),
            );
            compute_pass.set_bind_group(1, &grp, &[]);
        }


        // Bind materials and seed value
        {
            let grp = binding::material_n_seed_bind_group(
                &self.device,
                self.materials_buf.as_ref().unwrap().as_entire_binding(),
                self.seed_uniform.as_ref().unwrap().as_entire_binding(),
                Renderer::MAT_BUF_BIND,
                Renderer::SEED_UNIFORM_BIND,
                &compute_pipeline.get_bind_group_layout(3),
            );
            compute_pass.set_bind_group(3, &grp, &[]);
        }

        // Bind texture
        {
            let frame_tex_lay = compute_pipeline.get_bind_group_layout(2);
            let frame_tex_grp = binding::img_texture_bind_group(
                &self.device,
                self.frame_texview.as_ref().unwrap(),
                Renderer::IMG_TEX_BIND,
                &frame_tex_lay,
            );
            compute_pass.set_bind_group(2, &frame_tex_grp, &[]);
        }

        compute_pass.dispatch_workgroups(workgrp_x, workgrp_y, 1);
        std::mem::drop(compute_pass);

        // Copy to surface texture
        let texture = self.frame_texture.as_ref().unwrap();
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
