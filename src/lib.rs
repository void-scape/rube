#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    rube_platform::run(
        2560,
        1440,
        #[cfg(not(target_arch = "wasm32"))]
        rube::create_world_from_map("assets/sponza.bin.bz2"),
        #[cfg(target_arch = "wasm32")]
        rube::create_world_from_map("http://127.0.0.1:1334/assets/sponza.bin.bz2"),
        rube::handle_input,
        rube::update_and_render,
    );
}
