use wasm_bindgen::prelude::*;
use std::{iter, sync::Arc, rc::Rc, cell::RefCell};
use wgpu::{include_wgsl, util::DeviceExt};

mod camera;

use camera::Camera;

// TODO:
// -Seperate MyState in a mod

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    platform::web::{EventLoopExtWebSys, WindowExtWebSys},
    window::{Window, WindowId},
};

struct MyState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    render_pipeline: wgpu::RenderPipeline,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    camera: Camera,
    compute_pipeline: Option<wgpu::ComputePipeline>,
    rt_img_buffer: Option<wgpu::Buffer>,
    camera_uniform_buffer: Option<wgpu::Buffer>,
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

impl MyState {
    const CAMERA_UNIFORM_BIND:u32 = 0;
    const IMG_BUFFER_BIND:u32 = 1;
    fn window(&self) -> &Window {
        &self.window
    }
    async fn new(window: Window) -> Self {
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

        // NOTE: This format is available on chrome web
        let surface_format = wgpu::TextureFormat::Rgba8Unorm;

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

        let shader_code = fetch_shader("public/shaders/simple.wgsl").await.unwrap();

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
            rt_img_buffer: None,
            camera_uniform_buffer: None,
        }
    }

    fn img_bytes_per_row(width: u32) -> u32 {
        let bytes_per_row = std::mem::size_of::<u32>() * width as usize;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let pad = (alignment - (bytes_per_row % alignment)) % alignment;
        let num = bytes_per_row + pad;
        num as u32
    }

    fn create_rt_buffer(&mut self) {
        let width = self.size.width;
        let height = self.size.height;
        let output_size = MyState::img_bytes_per_row(width) as usize * height as usize;

        // Black screen
        let buffer = vec![0 as u8; output_size];
        let rt_img_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Rt buffer"),
                contents: buffer.as_slice(),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            });
        if let Some(img_buffer) = &self.rt_img_buffer {
            img_buffer.destroy();
        }
        self.rt_img_buffer = Some(rt_img_buffer);
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
        if let Some(cam_uniform_buf) = &self.camera_uniform_buffer {
            cam_uniform_buf.destroy();
        }
        self.camera_uniform_buffer = Some(camera_uniform_buffer);
    }

    // TODO: Make constants for the binding index
    fn create_rt_bindgrp(&self, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main bind group"),
            // Get it from our compute pipeline
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: MyState::CAMERA_UNIFORM_BIND,
                    resource: self
                        .camera_uniform_buffer
                        .as_ref()
                        .unwrap()
                        .as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: MyState::IMG_BUFFER_BIND,
                    resource: self.rt_img_buffer.as_ref().unwrap().as_entire_binding(),
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
                            binding: MyState::CAMERA_UNIFORM_BIND,
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
                            binding: MyState::IMG_BUFFER_BIND,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
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

    fn on_resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) -> bool {
        
        log::warn!("w={}, h={}", new_size.width, new_size.height);

        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            log::warn!("Building or updating 🛠 buffers");
            self.camera.set_resolution(
                new_size.width,
                new_size.height,
                true,
            );
            self.create_rt_buffer();
            self.create_camera_uniform();
            self.create_rt_pipeline();
            return true;
        }
        false
    }

    fn render (&mut self) -> Result<(), wgpu::SurfaceError>{
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

        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: self.rt_img_buffer.as_ref().unwrap(),
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(MyState::img_bytes_per_row(width)),
                    rows_per_image: Some(height),
                },
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

enum MyEvent {
    InitStateDone {
        window: Arc<Window>,
        size: winit::dpi::PhysicalSize<u32>,
    },
}

struct MyApp {
    state: Rc<RefCell<Option<MyState>>>,
    event_proxy: Arc<EventLoopProxy<MyEvent>>,
    surface_configured: bool,
}
impl MyApp {
    fn new(event_proxy: EventLoopProxy<MyEvent>) -> Self {
        Self {
            state: Rc::new(RefCell::new(None)),
            event_proxy: Arc::new(event_proxy),
            surface_configured: false,
        }
    }
}

impl ApplicationHandler<MyEvent> for MyApp {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: MyEvent) {
        match event {
            MyEvent::InitStateDone{window, size} => {
                log::warn!("State initialisation is done");
                // log::warn!("Request for size: {:?}", size);
                // Ask for resize
                self.surface_configured = false;
                let _ = window.as_ref().request_inner_size(size);
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes().with_title("gpudemo");
        let window = event_loop.create_window(window_attributes).unwrap();

        let web_window = web_sys::window().expect("No web window");
        web_window
            .document()
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()?))
                    .ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
        let web_width = web_window.inner_width().unwrap().as_f64().unwrap() as u32;
        let web_height = web_window.inner_height().unwrap().as_f64().unwrap() as u32;
        let web_size = winit::dpi::PhysicalSize::new(web_width, web_height);
        // let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(web_size));

        let state_clone = self.state.clone();
        let event_proxy_clone = self.event_proxy.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let new_state = MyState::new(window).await;

            match state_clone.try_borrow_mut() {
                Ok(mut state_obj) => {
                    *state_obj = Some(new_state);
                    let window_clone = state_obj.as_ref().map(|state| state.window.clone()).unwrap();

                    if let Err(e) = event_proxy_clone.send_event(MyEvent::InitStateDone {
                        window: window_clone,
                        size: web_size,
                    }) {
                        log::warn!("Failed to send user event: {}", e);
                    }
                }
                Err(_) => log::warn!("Could not borrow for initialisation"),
            };
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key:
                            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape),
                        ..
                    },
                ..
            } => {
                if let Ok(state) = self.state.try_borrow() {
                    if let Some(state) = state.as_ref() {
                        if window_id == state.window().id() {
                            event_loop.exit();
                        }
                    }
                }
            }

            WindowEvent::Resized(physical_size) => {
                log::warn!("Event: resize");
                if let Ok(mut state) = self.state.try_borrow_mut() {
                    if let Some(state) = state.as_mut() {
                        if window_id == state.window().id() {
                            self.surface_configured = state.on_resize(physical_size);
                        }
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                if let Ok(mut state) = self.state.try_borrow_mut() {
                    if let Some(state) = state.as_mut() {
                        if window_id == state.window().id() {
                            state.window().request_redraw();
                            if self.surface_configured{
                                match state.render() {
                                    Ok(_) => {}
                                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                        // NOTE: Hmm
                                        log::warn!("Lost");
                                        self.surface_configured = state.on_resize(state.size);
                                    }

                                    Err(wgpu::SurfaceError::OutOfMemory) => {
                                        log::error!("Out of memory");
                                        event_loop.exit();
                                    }

                                    Err(wgpu::SurfaceError::Timeout) => {
                                        log::warn!("Surface timeout");
                                    }
                                    Err(wgpu::SurfaceError::Other) => {
                                        log::warn!("Unknown error");
                                    }
                                };
                           }
                        }
                    }
                }
            }
            _ => {}
        };

    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Ok(state) = self.state.try_borrow() {
            if let Some(state) = state.as_ref() {
                state.window().request_redraw();
            }
        }
    }
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");

    let event_loop = EventLoop::<MyEvent>::with_user_event().build().unwrap();
    let my_app = MyApp::new(event_loop.create_proxy());
    
    event_loop.spawn_app(my_app);
    Ok(())
}
