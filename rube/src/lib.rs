use crate::indirect::IndirectPass;
use crate::march::MarchPass;
use crate::tree::VoxelTree;
use crate::{bench::Benchmarker, camera::Camera};
use glam::Vec3;
use rube_platform::winit::{event::*, keyboard::*, window::Window};

mod bench;
mod camera;
mod indirect;
pub mod map;
mod march;
mod ray;
pub mod tree;

pub struct World {
    // sliding_fps: VecDeque<f32>,
    // tree_loader: mpsc::Receiver<VoxelTree>,
    tree: VoxelTree,
    camera: Camera,
    march_pass: MarchPass,
    indirect_pass: IndirectPass,
    bencher: Benchmarker,
}

pub fn create_world_from_tree(
    path: impl Into<String>,
) -> impl FnOnce(&Window, usize, usize) -> World {
    |_, width, height| {
        let path = path.into();
        World {
            // sliding_fps: VecDeque::with_capacity(100),
            tree: VoxelTree::decompress(&std::fs::read(path).unwrap()),
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
                disabled: true,
                ..Default::default()
            },
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
                            // println!("{:#?}", world.camera);
                            println!(
                                "Keyframe{{translation:Vec3::new({},{},{}),rotations:({},{}),duration: 1.0}},",
                                world.camera.translation.x,
                                world.camera.translation.y,
                                world.camera.translation.z,
                                world.camera.pitch,
                                world.camera.yaw,
                            );
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

    // world.camera.update(delta);
    bench::update(&mut world.bencher, &mut world.camera, delta);
    march::march_pass(
        &world.tree,
        &world.camera,
        &mut world.march_pass,
        width,
        height,
    );
    indirect::indirect_pass(
        &world.tree,
        &world.march_pass,
        &mut world.indirect_pass,
        pixels,
    );
    profiling::finish_frame!();
}
