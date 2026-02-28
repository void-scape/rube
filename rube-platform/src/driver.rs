use std::sync::Arc;
use winit::window::Window;

pub struct Driver {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
}

impl Driver {
    pub(crate) async fn new(window: Arc<Window>, width: u32, height: u32) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::FLOAT32_FILTERABLE
                    | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
                    | wgpu::Features::TIMESTAMP_QUERY,
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await
            .unwrap();
        let surface = instance.create_surface(window).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];
        let mut gfx = Self {
            device,
            queue,
            surface,
            surface_format,
            width,
            height,
        };
        // Configure surface for the first time
        gfx.configure_surface(width, height);
        gfx
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.configure_surface(width, height);
    }

    fn configure_surface(&mut self, width: u32, height: u32) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width,
            height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoNoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
        self.width = width;
        self.height = height;
    }
}
