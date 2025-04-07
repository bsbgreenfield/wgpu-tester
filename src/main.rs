use code_notes::run;
mod model;
use model::util::load_gltf;
fn main() {
    match load_gltf("milk-truck") {
        Ok(_) => println!("success"),
        Err(e) => println!("{:?}", e),
    }
}
