use winit::window::Window;

pub extern crate wgpu;
pub extern crate winit;
pub use driver::Driver;

mod app;
mod driver;

pub struct PlatformUpdate<'a, World> {
    // logic
    pub world: &'a mut World,
    pub delta: f32,
    // window
    pub event_loop: &'a winit::event_loop::ActiveEventLoop,
    pub window: &'a winit::window::Window,
}

pub struct PlatformInput<'a, World> {
    pub world: &'a mut World,
    pub window: &'a winit::window::Window,
    pub input: Input,
}

pub enum Input {
    Window(winit::event::WindowEvent),
    Device(winit::event::DeviceEvent),
}

pub fn run<World>(
    width: u32,
    height: u32,
    create_world: impl FnOnce(&Window, Driver) -> World + 'static,
    handle_input: fn(PlatformInput<World>),
    update_and_render: fn(PlatformUpdate<World>),
) where
    World: 'static,
{
    app::run(width, height, create_world, handle_input, update_and_render);
}
