use code_notes::app::app::App;
use code_notes::app::app_state::AppState;
use code_notes::model::util::load_gltf;
fn main() {
    let app = App::default();
    let app_state = pollster::block_on(AppState::new(app.window.unwrap().clone()));
    match load_gltf("milk-truck", &app_state.app_config.device) {
        Ok(_) => println!("success"),
        Err(e) => println!("{:?}", e),
    }
}
