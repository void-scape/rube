use crate::{Driver, Input, PlatformInput, PlatformUpdate};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
#[cfg(feature = "dev")]
use winit::event::{ElementState, KeyEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(feature = "dev")]
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

pub fn run<World>(
    width: u32,
    height: u32,
    create_world: impl FnOnce(&Window, Driver) -> World + 'static,
    handle_input: fn(PlatformInput<World>),
    update_and_render: fn(PlatformUpdate<World>),
) where
    World: 'static,
{
    #[allow(unused_mut)]
    let mut app = App {
        init_dimensions: (width, height),
        window: None,
        create_world: Some(Box::new(create_world)),
        world: None,
        now: Time::now(),
        fns: FnPtrs::new(handle_input, update_and_render),
        #[cfg(target_arch = "wasm32")]
        rx: None,
    };

    let event_loop = EventLoop::new().unwrap();
    #[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
    event_loop.run_app(&mut app).unwrap();
    #[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
    {
        console_error_panic_hook::set_once();
        winit::platform::web::EventLoopExtWebSys::spawn_app(event_loop, app);
    }
}

struct App<World> {
    init_dimensions: (u32, u32),
    window: Option<Arc<Window>>,
    create_world: Option<Box<dyn FnOnce(&Window, Driver) -> World>>,
    world: Option<World>,
    now: Time,
    fns: FnPtrs,
    #[cfg(target_arch = "wasm32")]
    rx: Option<std::sync::mpsc::Receiver<World>>,
}

impl<World: 'static> ApplicationHandler for App<World> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let (width, height) = self.init_dimensions;
        let window = match event_loop.create_window(window_attributes(width, height)) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                println!("[ERROR] failed to create the window: {err}");
                event_loop.exit();
                return;
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let driver = pollster::block_on(Driver::new(window.clone(), width, height));
            self.world = Some((self.create_world.take().unwrap())(&window, driver));
        }

        #[cfg(target_arch = "wasm32")]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            self.rx = Some(rx);
            let create_world = self.create_world.take().unwrap();
            let window = window.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let driver = Driver::new(window.clone(), width, height).await;
                let world = create_world(&window, driver);
                let _ = tx.send(world);
            });
        }

        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        #[cfg(target_arch = "wasm32")]
        if let Some(rx) = &self.rx {
            if let Ok(world) = rx.try_recv() {
                self.world = Some(world);
                self.rx = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }

        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            #[cfg(target_arch = "wasm32")]
            WindowEvent::Resized(size) => {
                use crate::winit::platform::web::WindowExtWebSys;

                let device_pixel_ratio = web_sys::window().unwrap().device_pixel_ratio();
                let physical_width = (size.width as f64 * device_pixel_ratio) as u32;
                let physical_height = (size.height as f64 * device_pixel_ratio) as u32;

                if let Some(window) = &self.window {
                    let canvas = window.canvas().unwrap();
                    canvas.set_width(physical_width);
                    canvas.set_height(physical_height);
                }
            }
            WindowEvent::RedrawRequested => {
                let (Some(window), Some(world)) = (&self.window, &mut self.world) else {
                    return;
                };

                let _reloaded = self.fns.reload();

                let delta = {
                    let now = Time::now();
                    let delta = now.elapsed_secs(self.now);
                    self.now = now;
                    delta
                };

                // window.pre_present_notify();
                self.fns.update_and_render(PlatformUpdate {
                    world,
                    delta,
                    //
                    window,
                    event_loop,
                });
                window.request_redraw();
            }
            #[cfg(feature = "dev")]
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::F5),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.fns.recompile();
            }
            _ => {}
        }

        let (Some(window), Some(world)) = (&self.window, &mut self.world) else {
            return;
        };
        self.fns.handle_input(PlatformInput {
            world,
            window,
            input: Input::Window(event),
        });
    }

    fn device_event(
        &mut self,
        _: &ActiveEventLoop,
        _: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let (Some(window), Some(world)) = (&self.window, &mut self.world) else {
            return;
        };
        self.fns.handle_input(PlatformInput {
            world,
            window,
            input: Input::Device(event),
        });
    }
}

fn window_attributes(width: u32, height: u32) -> WindowAttributes {
    let attributes = Window::default_attributes().with_inner_size(PhysicalSize::new(width, height));
    #[cfg(target_arch = "wasm32")]
    let attributes = winit::platform::web::WindowAttributesExtWebSys::with_append(attributes, true);
    attributes
}

#[derive(Clone, Copy)]
struct Time(inner::Time);

#[cfg(target_arch = "wasm32")]
mod inner {
    pub type Time = f64;
    impl super::Time {
        pub fn now() -> Self {
            Self(web_sys::window().unwrap().performance().unwrap().now())
        }
        pub fn elapsed_secs(self, earlier: Self) -> f32 {
            ((self.0 - earlier.0).abs() / 1_000.0) as f32
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod inner {
    pub type Time = std::time::SystemTime;
    impl super::Time {
        pub fn now() -> Self {
            Self(std::time::SystemTime::now())
        }
        pub fn elapsed_secs(self, earlier: Self) -> f32 {
            self.0
                .duration_since(earlier.0)
                .unwrap_or_default()
                .as_secs_f32()
        }
    }
}

pub struct FnPtrs {
    #[allow(unused)]
    reloading: hot_reloading::HotReloading,
    handle_input: *mut core::ffi::c_void,
    update_and_render: *mut core::ffi::c_void,
}

impl FnPtrs {
    pub fn new<World>(
        handle_input: fn(PlatformInput<World>),
        update_and_render: fn(PlatformUpdate<World>),
    ) -> Self {
        Self {
            reloading: hot_reloading::HotReloading::from_path(debug_target()),
            handle_input: handle_input as *mut core::ffi::c_void,
            update_and_render: update_and_render as *mut core::ffi::c_void,
        }
    }

    pub fn handle_input<World>(&self, input: PlatformInput<World>) {
        unsafe {
            let handle_input = core::mem::transmute::<
                *mut core::ffi::c_void,
                fn(PlatformInput<World>),
            >(self.handle_input);
            handle_input(input);
        }
    }

    pub fn update_and_render<Input>(&self, input: Input) {
        unsafe {
            let update_and_render =
                core::mem::transmute::<*mut core::ffi::c_void, fn(Input)>(self.update_and_render);
            update_and_render(input);
        }
    }
}

fn debug_target() -> Option<&'static str> {
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return None;
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        #[cfg(target_os = "linux")]
        let extension = "so";
        #[cfg(target_os = "macos")]
        let extension = "dylib";
        let path = format!("target/debug/librube.{}", extension);
        println!("[DEV] Watching {path} ...");
        match std::fs::exists(&path) {
            Ok(_) => Some(String::leak(path)),
            Err(err) => panic!("failed to load {path}: {err}"),
        }
    }
}

#[cfg(not(feature = "hot-reload"))]
mod hot_reloading {
    use super::*;
    pub struct HotReloading;
    impl HotReloading {
        pub fn from_path(_: Option<&str>) -> Self {
            Self
        }
    }
    impl FnPtrs {
        #[cfg(feature = "dev")]
        pub fn recompile(&self) {}
        pub fn reload(&mut self) -> bool {
            false
        }
    }
}

#[cfg(feature = "hot-reload")]
mod hot_reloading {
    use super::*;
    use std::ffi::CString;
    extern crate std;
    pub struct HotReloading {
        dylib: *mut core::ffi::c_void,
        path: Option<String>,
        loaded: std::time::SystemTime,
    }
    impl HotReloading {
        pub fn from_path(path: Option<&str>) -> Self {
            Self {
                dylib: core::ptr::null_mut(),
                path: path.map(|inner| inner.to_string()),
                loaded: std::time::SystemTime::now(),
            }
        }
    }
    impl FnPtrs {
        #[cfg(feature = "dev")]
        pub fn recompile(&self) {
            println!("[DEV] Recompiling...");
            std::thread::spawn(|| {
                if let Err(e) = std::process::Command::new("cargo")
                    .env("RUSTFLAGS", "-C prefer-dynamic")
                    .args(["build", "-p", "rube"])
                    .spawn()
                    .expect("cargo is available")
                    .wait()
                {
                    println!("[ERROR] Failed to recompile: {e}");
                }
            });
        }

        pub fn reload(&mut self) -> bool {
            let Some(path) = self.reloading.path.as_deref() else {
                return false;
            };
            let Some(modified) = std::fs::metadata(path).ok().and_then(|meta| {
                meta.modified().ok().and_then(|modified| {
                    modified
                        .duration_since(self.reloading.loaded)
                        .is_ok_and(|dur| !dur.is_zero())
                        .then_some(modified)
                })
            }) else {
                return false;
            };

            if !self.reloading.dylib.is_null() {
                // NOTE: This does nothing on macos.
                debug_assert_eq!(unsafe { libc::dlclose(self.reloading.dylib) }, 0);
            }
            self.reloading.loaded = modified;

            println!("[DEV] Loading functions from {path}");
            let mut copy = std::path::PathBuf::from(path);
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap();
            copy.pop();
            copy.push(format!("{}", time.as_millis()));
            // NOTE: need to copy path on macos to prevent dylib caching
            std::fs::copy(path, &copy).expect("failed to copy dynamic library");
            let filename = CString::new(copy.to_str().unwrap()).unwrap();

            let dylib =
                unsafe { libc::dlopen(filename.as_ptr(), libc::RTLD_LOCAL | libc::RTLD_LAZY) };
            if !dylib.is_null() {
                let symbol = unsafe { libc::dlsym(dylib, c"update_and_render".as_ptr().cast()) };
                if !symbol.is_null() {
                    self.update_and_render = symbol;

                    let symbol = unsafe { libc::dlsym(dylib, c"handle_input".as_ptr().cast()) };
                    if !symbol.is_null() {
                        self.handle_input = symbol;
                    } else {
                        err("failed to load symbol handle_input");
                    }
                } else {
                    err("failed to load symbol update_and_render");
                }
            } else {
                err(&format!("failed to open {path}"));
            }

            fn err(msg: &str) {
                let str = unsafe { core::ffi::CStr::from_ptr(libc::dlerror()) };
                println!("ERROR: {}: {}", msg, str.to_str().unwrap());
            }

            true
        }
    }
}
