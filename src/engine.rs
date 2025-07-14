use crate::graphics::{
    camera::{Camera, CameraController},
    renderer::Renderer,
};
use cgmath::prelude::*;
use winit::{
    event::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode, WindowEvent},
    window::Window,
};

pub struct Engine {
    renderer: Renderer,
    camera: Camera,
    camera_controller: CameraController,
    static_cube_pos: cgmath::Vector3<f32>,
    is_w_pressed: bool,
    is_a_pressed: bool,
    is_s_pressed: bool,
    is_d_pressed: bool,
    mouse_delta: (f32, f32),
}

impl Engine {
    pub async fn new(window: &Window) -> Self {
        let renderer = Renderer::new(window).await;
        let camera = Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: renderer.aspect(),
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let camera_controller = CameraController::new(0.02, 0.001);

        Self {
            renderer,
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
        self.renderer.resize(new_size);
        self.camera.aspect = self.renderer.aspect();
    }

    pub fn device_input(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.mouse_delta.0 += delta.0 as f32;
            self.mouse_delta.1 += delta.1 as f32;
        }
    }

    pub fn input(&mut self, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            input:
                KeyboardInput {
                    state,
                    virtual_keycode: Some(keycode),
                    ..
                },
            ..
        } = event
        {
            let is_pressed = *state == ElementState::Pressed;
            match keycode {
                VirtualKeyCode::W => self.is_w_pressed = is_pressed,
                VirtualKeyCode::A => self.is_a_pressed = is_pressed,
                VirtualKeyCode::S => self.is_s_pressed = is_pressed,
                VirtualKeyCode::D => self.is_d_pressed = is_pressed,
                _ => {}
            }
        }
    }

    pub fn update(&mut self) {
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

        self.renderer.update_view_proj(&self.camera);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.renderer.render(&self.static_cube_pos)
    }
} 