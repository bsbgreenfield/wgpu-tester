use winit::event_loop::{self, EventLoop};
mod app;
use app::{App, AppWindow};

pub async fn run() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(event_loop::ControlFlow::Poll);
    let app_window = AppWindow::default();
    let app = &mut App::new(&app_window).await;
    match event_loop.run_app::<App>(app) {
        Ok(_) => (),
        Err(_) => panic!("failed to initialize app"),
    }
}
