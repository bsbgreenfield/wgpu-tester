pub mod app;
pub mod model;
pub mod scene;
mod transforms;
use winit::event_loop::{self, EventLoop};
pub fn run() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(event_loop::ControlFlow::Poll);
    let mut app = app::app::App::default();

    event_loop.run_app(&mut app).unwrap();
}
