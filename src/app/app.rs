use super::app_state::AppState;
use crate::util;
use std::{arch::aarch64::vcvt_high_f32_f64, collections::HashMap, str, sync::Arc};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::*,
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{self, Window},
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CtrUniform {
    trans: [[f32; 4]; 4],
}

impl CtrUniform {
    pub fn new() -> Self {
        Self {
            trans: (util::OPENGL_TO_WGPU_MATRIX).into(),
        }
    }
}

#[derive(Default)]
pub struct App<'a> {
    pub window: Option<Arc<Window>>,
    app_state: Option<AppState<'a>>,
    surface_configured: bool,
}

impl App<'_> {
    fn update_state(&mut self) {
        self.app_state.as_mut().unwrap().update();
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes().with_inner_size(PhysicalSize::new(1500, 1500)),
                    )
                    .unwrap(),
            );
            let app_state = pollster::block_on(AppState::new(window.clone()));
            self.app_state = Some(app_state);
            self.window = Some(window.clone());
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::KeyL),
                        ..
                    },
                ..
            } => {
                if self.window.is_some() {
                    println!("{:?}", self.window.as_ref().unwrap());
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.surface_configured = true;
                self.app_state.as_mut().unwrap().resize(physical_size);
            }
            WindowEvent::RedrawRequested => {
                if !self.surface_configured {
                    return;
                }
                self.window.as_ref().unwrap().request_redraw();
                match self.app_state.as_ref().unwrap().draw() {
                    Ok(_) => {}
                    Err(_) => {
                        event_loop.exit();
                    }
                }
            }
            _ => (),
        }
    }
}
