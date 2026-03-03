use crate::bench::Benchmarker;
use crate::indirect::IndirectPass;
use crate::march::MarchPass;
use crate::scene::Scene;
use rube_platform::winit::{event::*, keyboard::*, window::Window};
use std::path::Path;

mod bench;
mod camera;
pub mod indirect;
pub mod map;
pub mod march;
mod ray;
pub mod scene;
pub mod tree;

pub struct World {
    // sliding_fps: VecDeque<f32>,
    scene: Scene,
    march_pass: MarchPass,
    indirect_pass: IndirectPass,
    #[allow(unused)]
    bencher: Benchmarker,
}

pub fn create_world_from_tree(
    path: impl AsRef<Path>,
) -> impl FnOnce(&Window, usize, usize) -> World {
    |_, width, height| {
        World {
            // sliding_fps: VecDeque::with_capacity(100),
            scene: Scene::from_tree(path),
            march_pass: MarchPass::new(width, height),
            indirect_pass: IndirectPass::new(width, height),
            bencher: bench::bench1(),
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
                            println!("{:#?}", world.scene.camera);
                            // println!(
                            //     "Keyframe{{translation:Vec3::new({},{},{}),rotations:({},{}),duration: 1.0}},",
                            //     world.scene.camera.translation.x,
                            //     world.scene.camera.translation.y,
                            //     world.scene.camera.translation.z,
                            //     world.scene.camera.pitch,
                            //     world.scene.camera.yaw,
                            // );
                        }
                        _ => {}
                    }
                }
                world.scene.camera.handle_key(key, state);
            }
            _ => {}
        },
        rube_platform::Input::Device(event) => {
            if let DeviceEvent::MouseMotion { delta } = event {
                world
                    .scene
                    .camera
                    .handle_mouse(delta.0 as f32, delta.1 as f32);
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
        // window,
        ..
    }: rube_platform::PlatformUpdate<World>,
) {
    // if world.sliding_fps.len() >= 100 {
    //     world.sliding_fps.pop_front();
    // }
    // world.sliding_fps.push_back(1.0 / delta);
    // window.set_title(&format!(
    //     "RUBE - {:.2}",
    //     world.sliding_fps.iter().sum::<f32>() / world.sliding_fps.len() as f32
    // ));

    // world.scene.camera.update(delta);
    bench::update(&mut world.bencher, &mut world.scene.camera, delta);
    march::march_pass(&world.scene, &mut world.march_pass, width, height);
    indirect::indirect_pass(
        &world.scene,
        &world.march_pass,
        &mut world.indirect_pass,
        pixels,
    );
    profiling::finish_frame!();
}
