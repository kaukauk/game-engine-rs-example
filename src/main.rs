use cgmath::prelude::*;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        proj * view
    }
}

struct CameraController {
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    fn new(speed: f32, sensitivity: f32) -> Self {
        Self { speed, sensitivity }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

const CUBE_VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5, 0.5], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.5, -0.5, 0.5], color: [0.0, 1.0, 0.0] },
    Vertex { position: [0.5, 0.5, 0.5], color: [0.0, 0.0, 1.0] },
    Vertex { position: [-0.5, 0.5, 0.5], color: [1.0, 1.0, 0.0] },
    Vertex { position: [-0.5, -0.5, -0.5], color: [1.0, 0.0, 1.0] },
    Vertex { position: [0.5, -0.5, -0.5], color: [0.0, 1.0, 1.0] },
    Vertex { position: [0.5, 0.5, -0.5], color: [1.0, 1.0, 1.0] },
    Vertex { position: [-0.5, 0.5, -0.5], color: [0.5, 0.5, 0.5] },
];

const CUBE_INDICES: &[u16] = &[
    0, 1, 2, 0, 2, 3, // Front
    1, 5, 6, 1, 6, 2, // Right
    5, 4, 7, 5, 7, 6, // Back
    4, 0, 3, 4, 3, 7, // Left
    3, 2, 6, 3, 6, 7, // Top
    4, 5, 1, 4, 1, 0, // Bottom
];

const PLANE_VERTICES: &[Vertex] = &[
    Vertex {
        position: [-10.0, -0.5, -10.0],
        color: [0.3, 0.4, 0.3],
    },
    Vertex {
        position: [10.0, -0.5, -10.0],
        color: [0.3, 0.4, 0.3],
    },
    Vertex {
        position: [10.0, -0.5, 10.0],
        color: [0.3, 0.4, 0.3],
    },
    Vertex {
        position: [-10.0, -0.5, 10.0],
        color: [0.3, 0.4, 0.3],
    },
];

const PLANE_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

impl Uniforms {
    fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            model: cgmath::Matrix4::identity().into(),
        }
    }
}

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    cube_vertex_buffer: wgpu::Buffer,
    cube_index_buffer: wgpu::Buffer,
    plane_vertex_buffer: wgpu::Buffer,
    plane_index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    camera: Camera,
    camera_controller: CameraController,
    static_cube_pos: cgmath::Vector3<f32>,
    is_w_pressed: bool,
    is_a_pressed: bool,
    is_s_pressed: bool,
    is_d_pressed: bool,
    mouse_delta: (f32, f32),
}

impl State {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| {
                matches!(
                    *f,
                    wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb
                )
            })
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let cube_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Vertex Buffer"),
            contents: bytemuck::cast_slice(CUBE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let cube_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Index Buffer"),
            contents: bytemuck::cast_slice(CUBE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let plane_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Plane Vertex Buffer"),
            contents: bytemuck::cast_slice(PLANE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let plane_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Plane Index Buffer"),
            contents: bytemuck::cast_slice(PLANE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let camera = Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let camera_controller = CameraController::new(0.2, 0.1);

        let mut uniforms = Uniforms::new();
        uniforms.view_proj =
            (OPENGL_TO_WGPU_MATRIX * camera.build_view_projection_matrix()).into();

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("bind_group_layout"),
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            cube_vertex_buffer,
            cube_index_buffer,
            plane_vertex_buffer,
            plane_index_buffer,
            uniform_buffer,
            uniform_bind_group,
            camera,
            camera_controller,
            static_cube_pos: cgmath::Vector3::new(3.0, 0.0, -3.0),
            is_w_pressed: false,
            is_a_pressed: false,
            is_s_pressed: false,
            is_d_pressed: false,
            mouse_delta: (0.0, 0.0),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.camera.aspect = self.config.width as f32 / self.config.height as f32;
        }
    }

    fn input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::W => self.is_w_pressed = is_pressed,
                    VirtualKeyCode::A => self.is_a_pressed = is_pressed,
                    VirtualKeyCode::S => self.is_s_pressed = is_pressed,
                    VirtualKeyCode::D => self.is_d_pressed = is_pressed,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        let forward = (self.camera.target - self.camera.eye).normalize();
        let right = forward.cross(self.camera.up);

        if self.is_w_pressed {
            self.camera.eye += forward * self.camera_controller.speed;
        }
        if self.is_s_pressed {
            self.camera.eye -= forward * self.camera_controller.speed;
        }
        if self.is_d_pressed {
            self.camera.eye += right * self.camera_controller.speed;
        }
        if self.is_a_pressed {
            self.camera.eye -= right * self.camera_controller.speed;
        }

        let (yaw, pitch) = (
            self.mouse_delta.0 * self.camera_controller.sensitivity,
            self.mouse_delta.1 * self.camera_controller.sensitivity,
        );
        self.mouse_delta = (0.0, 0.0);

        let new_forward = cgmath::Matrix3::from_axis_angle(self.camera.up, cgmath::Rad(-yaw))
            * cgmath::Matrix3::from_axis_angle(right, cgmath::Rad(-pitch))
            * forward;

        self.camera.target = self.camera.eye + new_forward;

        let mut uniforms = Uniforms::new();
        uniforms.view_proj =
            (OPENGL_TO_WGPU_MATRIX * self.camera.build_view_projection_matrix()).into();
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            // Draw the plane
            let model = cgmath::Matrix4::from_translation(cgmath::Vector3::new(0.0, 0.0, 0.0));
            self.update_model(model);
            render_pass.set_vertex_buffer(0, self.plane_vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.plane_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..PLANE_INDICES.len() as u32, 0, 0..1);

            // Draw the static cube
            let model = cgmath::Matrix4::from_translation(self.static_cube_pos);
            self.update_model(model);
            render_pass.set_vertex_buffer(0, self.cube_vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.cube_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..CUBE_INDICES.len() as u32, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    fn update_model(&self, model: cgmath::Matrix4<f32>) {
        let mut uniforms = Uniforms::new();
        uniforms.model = model.into();
        self.queue.write_buffer(
            &self.uniform_buffer,
            64,
            bytemuck::cast_slice(&[uniforms.model]),
        );
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("First-Person Player")
        .build(&event_loop)
        .unwrap();
    let mut state = State::new(&window).await;
    window.set_cursor_grab(winit::window::CursorGrabMode::Confined).unwrap();
    window.set_cursor_visible(false);


    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                state.mouse_delta.0 += delta.0 as f32;
                state.mouse_delta.1 += delta.1 as f32;
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                state.input(event);
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("Render error: {:?}", e),
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
} 