use super::app_state::AppState;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::*,
    event_loop::ActiveEventLoop,
    keyboard::{Key, KeyCode, PhysicalKey},
    window::{self, Window},
};

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
    fn process_keypress(
        &mut self,
        state: ElementState,
        keycode: KeyCode,
        event_loop: &ActiveEventLoop,
    ) {
        if let Some(app_state) = self.app_state.as_mut() {
            let is_pressed = state == ElementState::Pressed;
            match keycode {
                KeyCode::KeyD => {
                    app_state.input_controller.key_d_down = is_pressed;
                }
                KeyCode::KeyA => {
                    app_state.input_controller.key_a_down = is_pressed;
                }
                KeyCode::KeyW => {
                    app_state.input_controller.key_w_down = is_pressed;
                }
                KeyCode::KeyS => {
                    app_state.input_controller.key_s_down = is_pressed;
                }
                KeyCode::KeyQ => {
                    app_state.input_controller.key_q_down = is_pressed;
                }
                KeyCode::KeyE => {
                    app_state.input_controller.key_e_down = is_pressed;
                }
                KeyCode::Escape => {
                    event_loop.exit();
                }
                _ => {}
            }
        }
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
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => {
                self.process_keypress(state, keycode, event_loop);
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
                self.update_state();
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
