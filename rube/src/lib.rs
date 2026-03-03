use crate::camera::Camera;
use crate::indirect::IndirectPass;
use crate::march::MarchPass;
use crate::tree::VoxelTree;
use glam::Vec3;
use rube_platform::winit::{event::*, keyboard::*, window::Window};
use std::{collections::VecDeque, sync::mpsc};

mod camera;
mod indirect;
pub mod map;
mod march;
mod ray;
pub mod tree;

pub struct World {
    sliding_fps: VecDeque<f32>,
    tree_loader: mpsc::Receiver<VoxelTree>,
    tree: Option<VoxelTree>,
    camera: Camera,
    march_pass: MarchPass,
    indirect_pass: IndirectPass,
}

pub fn create_world_from_tree(
    path: impl Into<String>,
) -> impl FnOnce(&Window, usize, usize) -> World {
    |_, width, height| {
        let (sender, receiver) = mpsc::channel();
        let path = path.into();
        #[cfg(not(target_arch = "wasm32"))]
        {
            sender
                .send(VoxelTree::decompress(&std::fs::read(path).unwrap()))
                .unwrap();
        }
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                let response = reqwest::get(&path).await.unwrap();
                let bytes = response.bytes().await.unwrap();
                sender.send(VoxelTree::decompress(&bytes)).unwrap();
            });
        }
        World {
            tree: None,
            tree_loader: receiver,
            sliding_fps: VecDeque::with_capacity(100),
            march_pass: MarchPass::new(width, height),
            indirect_pass: IndirectPass::new(width, height),
            camera: Camera {
                translation: Vec3::new(1.1192523, 1.0224879, 1.0697857),
                yaw: 7.3650107,
                pitch: 0.20999885,
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
#[profiling::function]
pub fn handle_input(
    rube_platform::PlatformInput { world, input, .. }: rube_platform::PlatformInput<World>,
) {
    #[allow(clippy::single_match)]
    match input {
        rube_platform::Input::Window(event) => match event {
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
#[profiling::function]
pub fn update_and_render(
    rube_platform::PlatformUpdate {
        world,
        delta,
        //
        pixels,
        width,
        height,
        //
        window,
        ..
    }: rube_platform::PlatformUpdate<World>,
) {
    if let Ok(tree) = world.tree_loader.try_recv() {
        world.tree = Some(tree);
    }

    if world.sliding_fps.len() >= 100 {
        world.sliding_fps.pop_front();
    }
    world.sliding_fps.push_back(1.0 / delta);
    window.set_title(&format!(
        "RUBE - {:.2}",
        world.sliding_fps.iter().sum::<f32>() / world.sliding_fps.len() as f32
    ));
    world.camera.update(delta);
    if let Some(tree) = &world.tree {
        march::march_pass(tree, &world.camera, &mut world.march_pass, width, height);
        indirect::indirect_pass(tree, &world.march_pass, &mut world.indirect_pass, pixels);
    }

    profiling::finish_frame!();
}
