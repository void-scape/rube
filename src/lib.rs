#![allow(clippy::too_many_arguments)]

use crate::{camera::Camera, render::Renderer};
use glazer::winit::{event::*, keyboard::*};
use std::collections::VecDeque;

mod camera;
mod render;
mod voxel;

pub const WIDTH: usize = 2560;
pub const HEIGHT: usize = 1440;

pub struct World {
    sliding_fps: VecDeque<f32>,
    renderer: Renderer,
    camera: Camera,
}

pub fn create_world(window: &'static glazer::winit::window::Window) -> World {
    let mut renderer = Renderer::new(window);
    renderer.load_vox("assets/sponza.vox");
    World {
        sliding_fps: VecDeque::with_capacity(100),
        renderer,
        camera: camera::Camera {
            translation: glam::Vec3::new(2.0, 2.0, 2.0),
            yaw: 3.9350083,
            pitch: -0.6800003,
            fov: 90f32.to_radians(),
            znear: 0.01,
            zfar: 1000.0,
            speed: 0.5,
            ..Default::default()
        },
    }
}

#[unsafe(no_mangle)]
pub fn handle_input(glazer::PlatformInput { world, input, .. }: glazer::PlatformInput<World>) {
    match input {
        glazer::Input::Window(event) => match event {
            WindowEvent::Resized(size) => {
                world.renderer.resize(size);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => {
                #[cfg(feature = "dev")]
                if key == KeyCode::Escape {
                    std::process::exit(0);
                }
                world.camera.handle_key(key, state);
            }
            _ => {}
        },
        glazer::Input::Device(event) => {
            if let DeviceEvent::MouseMotion { delta } = event {
                world.camera.handle_mouse(delta.0 as f32, delta.1 as f32);
            }
        }
    }
}

pub fn update_and_render(
    glazer::PlatformUpdate {
        world,
        window,
        delta,
        ..
    }: glazer::PlatformUpdate<World>,
) {
    if world.sliding_fps.len() >= 100 {
        world.sliding_fps.pop_front();
    }
    world.sliding_fps.push_back(1.0 / delta);
    window.set_title(&format!(
        "RUBE - {:.2}",
        world.sliding_fps.iter().sum::<f32>() / world.sliding_fps.len() as f32
    ));
    world.camera.update(delta);
    world.renderer.render(&world.camera);
}
