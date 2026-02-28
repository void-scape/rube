use glam::Vec3;
use rube_platform::{
    Driver,
    winit::{event::*, keyboard::*, window::Window},
};
use rube_render::{Camera, Renderer};
use std::{collections::VecDeque, sync::mpsc};

pub struct World {
    sliding_fps: VecDeque<f32>,
    camera: Camera,
    renderer: Renderer,
    map_loader: mpsc::Receiver<Vec<u8>>,
}

pub fn create_world_from_map(path: impl Into<String>) -> impl FnOnce(&Window, Driver) -> World {
    |_, driver| {
        let (sender, receiver) = mpsc::channel();
        let path = path.into();
        #[cfg(not(target_arch = "wasm32"))]
        {
            sender.send(std::fs::read(path).unwrap()).unwrap();
        }
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                let response = reqwest::get(&path).await.unwrap();
                let bytes = response.bytes().await.unwrap();
                sender.send(bytes.to_vec()).unwrap();
            });
        }
        World {
            map_loader: receiver,
            sliding_fps: VecDeque::with_capacity(100),
            renderer: Renderer::new(driver),
            camera: Camera {
                translation: Vec3::new(1.0668889, 1.0348904, 1.0522615),
                yaw: 6.5400095,
                pitch: -0.07500112,
                fov: 90f32.to_radians(),
                znear: 0.01,
                zfar: 1000.0,
                speed: 0.5,
                half_speed: true,
                ..Default::default()
            },
        }
    }
}

#[unsafe(no_mangle)]
pub fn handle_input(
    rube_platform::PlatformInput { world, input, .. }: rube_platform::PlatformInput<World>,
) {
    #[allow(clippy::single_match)]
    match input {
        rube_platform::Input::Window(event) => match event {
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
                if state.is_pressed() {
                    match key {
                        #[cfg(feature = "dev")]
                        KeyCode::Escape => {
                            std::process::exit(0);
                        }
                        KeyCode::KeyP => {
                            println!("{:#?}", world.camera);
                        }
                        _ => {}
                    }
                }
                world.camera.handle_key(key, state);
            }
            _ => {}
        },
        rube_platform::Input::Device(event) => {
            if let DeviceEvent::MouseMotion { delta } = event {
                world.camera.handle_mouse(delta.0 as f32, delta.1 as f32);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub fn update_and_render(
    rube_platform::PlatformUpdate {
        world,
        window,
        delta,
        ..
    }: rube_platform::PlatformUpdate<World>,
) {
    if let Ok(map) = world.map_loader.try_recv() {
        world.renderer.load_map(&map);
    }

    if world.sliding_fps.len() >= 100 {
        world.sliding_fps.pop_front();
    }
    world.sliding_fps.push_back(1.0 / delta);
    window.set_title(&format!(
        "I AM RUBE - {:.2}",
        world.sliding_fps.iter().sum::<f32>() / world.sliding_fps.len() as f32
    ));
    world.camera.update(delta);
    world.renderer.render(&world.camera);
}
