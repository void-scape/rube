fn main() {
    glazer::run(
        rube::WIDTH,
        rube::HEIGHT,
        rube::create_world,
        rube::handle_input,
        rube::update_and_render,
        glazer::debug_target(),
    );
}
