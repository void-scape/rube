#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    let scale = 30;
    rube_platform::run(
        16 * scale,
        9 * scale,
        5,
        #[cfg(not(target_arch = "wasm32"))]
        rube::create_world_from_tree(std::env::args().nth(1).expect("map path provided")),
        #[cfg(target_arch = "wasm32")]
        rube::create_world_from_tree("http://127.0.0.1:1334/assets/sponza.bin.bz2"),
        rube::handle_input,
        rube::update_and_render,
    );
}
