#![allow(clippy::type_complexity)]

use std::num::NonZeroU32;
use std::rc::Rc;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
#[cfg(feature = "dev")]
use winit::event::{ElementState, KeyEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(feature = "dev")]
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};
use winit::{application::ApplicationHandler, event_loop::OwnedDisplayHandle};

// extern crates such that global and thread state exists only within `rube-platform`.
// This allows `rube` to be dynamically linked and recompiled at run time while
// `depending` on these crates.
//
// If `rube` were to statically link with them, the global and thread state is
// duplicated, causing crashes after reloading.
//
// NOTE: This is my hypothesis, I am not exactly sure what causes the crashes.
pub extern crate winit;

pub struct PlatformUpdate<'a, World> {
    // logic
    pub world: &'a mut World,
    pub delta: f32,
    // gfx
    /// Pixel format (`u32`):
    ///
    /// 00000000RRRRRRRRGGGGGGGGBBBBBBBB
    ///
    /// 0: Bit is 0
    /// R: Red channel
    /// G: Green channel
    /// B: Blue channel
    pub pixels: &'a mut [u32],
    pub width: usize,
    pub height: usize,
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
    width: usize,
    height: usize,
    scale: usize,
    create_world: impl FnOnce(&Window, usize, usize) -> World + 'static,
    handle_input: fn(PlatformInput<World>),
    update_and_render: fn(PlatformUpdate<World>),
) where
    World: 'static,
{
    #[allow(unused_mut)]
    let mut app = App::Init {
        dimensions: (width, height, scale),
        create_world: Some(Box::new(create_world)),
        fns: Some(FnPtrs::new(handle_input, update_and_render)),
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

enum App<World> {
    Init {
        dimensions: (usize, usize, usize),
        create_world: Option<Box<dyn FnOnce(&Window, usize, usize) -> World>>,
        fns: Option<FnPtrs>,
    },
    Running {
        dimensions: (usize, usize, usize),
        pixels: Vec<u32>,
        surface: softbuffer::Surface<OwnedDisplayHandle, Rc<Window>>,
        window: Rc<Window>,
        world: World,
        now: Time,
        fns: FnPtrs,
    },
}

impl<World: 'static> ApplicationHandler for App<World> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Self::Init {
            dimensions: (width, height, scale),
            create_world,
            fns,
        } = self
        else {
            return;
        };

        let window = match event_loop.create_window(window_attributes(
            *width as u32,
            *height as u32,
            *scale as u32,
        )) {
            Ok(window) => Rc::new(window),
            Err(err) => {
                println!("[ERROR] failed to create the window: {err}");
                event_loop.exit();
                return;
            }
        };

        let context = softbuffer::Context::new(event_loop.owned_display_handle()).unwrap();
        let mut surface =
            softbuffer::Surface::new(&context, window.clone()).expect("failed creating surface");

        // https://github.com/rust-windowing/softbuffer/issues/106
        let size = window.inner_size();
        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            surface.resize(width, height).unwrap();
        }

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        *self = Self::Running {
            dimensions: (*width, *height, *scale),
            pixels: vec![u32::MAX; *width * *height],
            world: (create_world.take().unwrap())(&window, *width, *height),
            surface,
            window,
            now: Time::now(),
            fns: fns.take().unwrap(),
        };
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Self::Running { window, .. } = self {
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Self::Running {
            dimensions: (width, height, scale),
            pixels,
            surface,
            window,
            world,
            now,
            fns,
        } = self
        else {
            return;
        };

        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                #[cfg(target_arch = "wasm32")]
                {
                    use crate::winit::platform::web::WindowExtWebSys;

                    let device_pixel_ratio = web_sys::window().unwrap().device_pixel_ratio();
                    let physical_width = (size.width as f64 * device_pixel_ratio) as u32;
                    let physical_height = (size.height as f64 * device_pixel_ratio) as u32;

                    let canvas = window.canvas().unwrap();
                    canvas.set_width(physical_width);
                    canvas.set_height(physical_height);
                }
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    surface.resize(width, height).unwrap();
                }
            }
            WindowEvent::RedrawRequested => {
                let _reloaded = fns.reload();
                let delta = {
                    let n = Time::now();
                    let delta = n.elapsed_secs(*now);
                    *now = n;
                    delta
                };
                let mut buffer = surface.buffer_mut().unwrap();
                fns.update_and_render(PlatformUpdate {
                    world,
                    delta,
                    //
                    width: *width,
                    height: *height,
                    pixels,
                    //
                    window,
                    event_loop,
                });
                let bw = buffer.width().get() as usize;
                let bh = buffer.height().get() as usize;
                // SAFETY: `Pixel` can be reinterpreted as `u32`.
                // NOTE: wasm is bgr, but it is so slow on wasm anyway that I don't
                // really care about it right now...
                let fb = unsafe {
                    std::mem::transmute::<&mut [softbuffer::Pixel], &mut [u32]>(buffer.pixels())
                };
                blit_scaled(fb, bw, bh, pixels, *width, *height, *scale);
                window.pre_present_notify();
                buffer.present().unwrap();
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
                fns.recompile();
            }
            _ => {}
        }

        fns.handle_input(PlatformInput {
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
        let Self::Running {
            window, world, fns, ..
        } = self
        else {
            return;
        };
        fns.handle_input(PlatformInput {
            world,
            window,
            input: Input::Device(event),
        });
    }
}

pub fn blit_scaled(
    frame_buffer: &mut [u32],
    fw: usize,
    fh: usize,
    pixels: &[u32],
    pw: usize,
    ph: usize,
    scale: usize,
) {
    let ox = (fw - (pw * scale)) / 2;
    let oy = (fh - (ph * scale)) / 2;
    for vy in 0..ph {
        let row = &pixels[vy * pw..(vy + 1) * pw];
        for dy in 0..scale {
            let fb_y = oy + vy * scale + dy;
            let fb_row = &mut frame_buffer[fb_y * fw..(fb_y + 1) * fw];
            for vx in 0..pw {
                let color = row[vx];
                fb_row[ox + vx * scale..ox + vx * scale + scale].fill(color);
            }
        }
    }
}

fn window_attributes(width: u32, height: u32, scale: u32) -> WindowAttributes {
    let attributes = Window::default_attributes()
        .with_inner_size(PhysicalSize::new(width * scale, height * scale));
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

    pub fn handle_input<Input>(&self, input: Input) {
        self.call_fn(self.handle_input, input);
    }

    pub fn update_and_render<Input>(&self, input: Input) {
        self.call_fn(self.update_and_render, input);
    }

    fn call_fn<T>(&self, sym: *mut core::ffi::c_void, input: T) {
        unsafe {
            let func = core::mem::transmute::<*mut core::ffi::c_void, fn(T)>(sym);
            func(input);
        }
    }
}

fn debug_target() -> Option<&'static str> {
    #[cfg(any(
        not(any(target_os = "macos", target_os = "linux")),
        not(feature = "hot-reload")
    ))]
    return None;
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[allow(unreachable_code)]
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
                    .args(["build", "-p", "rube", "--features", "dev"])
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
