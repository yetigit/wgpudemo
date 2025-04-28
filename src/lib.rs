use wasm_bindgen::prelude::*;
use std::{sync::Arc, rc::Rc, cell::RefCell};
// use wgpu::{include_wgsl, util::DeviceExt};

mod renderer;
mod camera;
mod sphere;
mod intersection;

use crate::renderer::Renderer; 

// TODO:
// -Seperate MyState in a mod

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    platform::web::{EventLoopExtWebSys, WindowExtWebSys},
    window::{Window, WindowId},
};

enum AppEvent {
    InitStateDone {
        window: Arc<Window>,
        size: winit::dpi::PhysicalSize<u32>,
    },
}

struct App {
    state: Rc<RefCell<Option<Renderer>>>,
    event_proxy: Arc<EventLoopProxy<AppEvent>>,
    surface_configured: bool,
}
impl App {
    fn new(event_proxy: EventLoopProxy<AppEvent>) -> Self {
        Self {
            state: Rc::new(RefCell::new(None)),
            event_proxy: Arc::new(event_proxy),
            surface_configured: false,
        }
    }
    fn make_world(&mut self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            if let Some(state) = state.as_mut() {
                state.make_world();
            }
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::InitStateDone{window, size} => {
                log::warn!("State initialisation is done");
                // log::warn!("Request for size: {:?}", size);
                // Create world
                self.make_world();
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
            let new_state = Renderer::new(window, wgpu::TextureFormat::Rgba8Unorm).await;

            match state_clone.try_borrow_mut() {
                Ok(mut state_obj) => {
                    *state_obj = Some(new_state);
                    let window_clone = state_obj.as_ref().map(|state| state.window.clone()).unwrap();

                    if let Err(e) = event_proxy_clone.send_event(AppEvent::InitStateDone {
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

    let event_loop = EventLoop::<AppEvent>::with_user_event().build().unwrap();
    let my_app = App::new(event_loop.create_proxy());
    
    event_loop.spawn_app(my_app);
    Ok(())
}
