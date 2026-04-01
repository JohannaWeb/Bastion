use crate::layout::LayoutTree;
use crate::gpu_paint::GpuPainter;
use std::sync::Arc;
use vello::{
    kurbo::Affine,
    peniko::{Color, Fill},
    util::{RenderContext, RenderSurface},
    wgpu,
    Renderer, RendererOptions, Scene,
};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::Window,
};

pub fn open(layout: &LayoutTree) {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = AuroraApp::new(layout);
    
    event_loop.run_app(&mut app).expect("failed to run event loop");
}

struct AuroraApp<'a> {
    layout: &'a LayoutTree,
    context: RenderContext,
    renderers: Vec<Option<Renderer>>,
    surface: Option<RenderSurface<'static>>,
    window: Option<Arc<Window>>,
    scroll_y: f64,
}

impl<'a> AuroraApp<'a> {
    fn new(layout: &'a LayoutTree) -> Self {
        Self {
            layout,
            context: RenderContext::new(),
            renderers: Vec::new(),
            surface: None,
            window: None,
            scroll_y: 0.0,
        }
    }

    fn render(&mut self) {
        let surface = self.surface.as_ref().unwrap();
        let _window = self.window.as_ref().unwrap();
        let width = surface.config.width;
        let height = surface.config.height;
        let device_handle = &self.context.devices[surface.dev_id];

        let surface_texture = surface
            .surface
            .get_current_texture()
            .expect("failed to get surface texture");

        // Clear the surface to white
        let swapchain_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device_handle.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("clear_encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &swapchain_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        device_handle.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }
}

impl<'a> winit::application::ApplicationHandler for AuroraApp<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attr = Window::default_attributes()
            .with_title("Aurora Browser (GPU Accelerated)")
            .with_inner_size(winit::dpi::LogicalSize::new(1200.0, 900.0));
        
        let window = Arc::new(event_loop.create_window(window_attr).expect("failed to create window"));
        self.window = Some(window.clone());

        // Create surface
        let surface = pollster::block_on(self.context.create_surface(
            window.clone(),
            1200,
            900,
            vello::wgpu::PresentMode::Fifo,
        ))
            .expect("failed to create surface");
        self.surface = Some(surface);
        
        self.renderers.resize_with(self.context.devices.len(), || None);
        
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = self.surface.as_mut() {
                    self.context.resize_surface(surface, size.width, size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if self.surface.is_some() {
                    self.render();
                }
            }
            WindowEvent::KeyboardInput { 
                event: KeyEvent { 
                    logical_key, 
                    state: ElementState::Pressed, 
                    .. 
                }, 
                .. 
            } => {
                match logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Named(NamedKey::ArrowDown) => {
                        self.scroll_y += 20.0;
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        self.scroll_y = (self.scroll_y - 20.0).max(0.0);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
