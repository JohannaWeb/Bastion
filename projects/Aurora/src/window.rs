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
use wgpu::util::DeviceExt;

const BLIT_SHADER: &str = r#"
@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var samp: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var uv = vec2<f32>(f32(vertex_index & 1u), f32((vertex_index >> 1u) & 1u));
    var position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    position.y = -position.y;
    return VertexOutput(position, uv);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var rgba = textureSample(tex, samp, in.uv);
    // Swizzle RGBA to BGRA
    return vec4<f32>(rgba.b, rgba.g, rgba.r, rgba.a);
}
"#;
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
    blit_pipeline: Option<wgpu::RenderPipeline>,
    blit_sampler: Option<wgpu::Sampler>,
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
            blit_pipeline: None,
            blit_sampler: None,
        }
    }

    fn render(&mut self) {
        let surface = self.surface.as_ref().unwrap();
        let _window = self.window.as_ref().unwrap();
        let width = surface.config.width;
        let height = surface.config.height;
        let device_handle = &self.context.devices[surface.dev_id];

        let mut scene = Scene::new();

        // Transform the scene based on scroll
        let transform = Affine::translate((0.0, -self.scroll_y));

        // Paint the layout
        scene.push_layer(
            Fill::NonZero,
            vello::peniko::BlendMode::default(),
            1.0,
            transform,
            &vello::kurbo::Rect::new(0.0, 0.0, width as f64, 10000.0),
        );
        GpuPainter::paint(self.layout.root(), &mut scene);
        scene.pop_layer();

        let surface_texture = surface
            .surface
            .get_current_texture()
            .expect("failed to get surface texture");

        let render_params = vello::RenderParams {
            base_color: Color::WHITE,
            antialiasing_method: vello::AaConfig::Msaa16,
            width,
            height,
        };

        if self.renderers[surface.dev_id].is_none() {
            self.renderers[surface.dev_id] = Some(
                Renderer::new(
                    &device_handle.device,
                    RendererOptions {
                        use_cpu: false,
                        antialiasing_support: vello::AaSupport::all(),
                        num_init_threads: None,
                        pipeline_cache: None,
                    },
                )
                .expect("failed to create vello renderer"),
            );
        }

        // Create intermediate RGBA8Unorm texture for vello to render to
        let intermediate_texture = device_handle.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let intermediate_view = intermediate_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let renderer = self.renderers[surface.dev_id].as_mut().unwrap();
        renderer
            .render_to_texture(
                &device_handle.device,
                &device_handle.queue,
                &scene,
                &intermediate_view,
                &render_params,
            )
            .expect("failed to render to intermediate texture");

        // Blit intermediate RGBA to swapchain BGRA using a shader
        let swapchain_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler and pipeline once
        if self.blit_sampler.is_none() {
            let sampler = device_handle.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("blit_sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            self.blit_sampler = Some(sampler);
        }

        if self.blit_pipeline.is_none() {
            let shader_module = device_handle.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("blit_shader"),
                source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
            });

            let bind_group_layout = device_handle.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("blit_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

            let pipeline_layout = device_handle.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("blit_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let pipeline = device_handle.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("blit_pipeline"),
                layout: Some(&pipeline_layout),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface.config.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                multiview: None,
                cache: None,
            });

            self.blit_pipeline = Some(pipeline);
        }

        // Create bind group each frame (texture is recreated each frame)
        let bind_group_layout = device_handle.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blit_layout_frame"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device_handle.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit_bind_group_frame"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&intermediate_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(self.blit_sampler.as_ref().unwrap()),
                },
            ],
        });

        // Render blit pass
        let mut encoder = device_handle.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("blit_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blit_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &swapchain_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(self.blit_pipeline.as_ref().unwrap());
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..6, 0..1);
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
