use crate::{
    camera_bind_group::CameraBindGroup, lighting::LightingPipeline,
    postprocess::PostprocessPipeline, raymarch::RayMarchPipeline,
    tree_bind_group::VoxelTreeBindGroup,
};
use rube_platform::Driver;
use rube_platform::winit::dpi::PhysicalSize;

pub use camera::Camera;
mod camera;
mod camera_bind_group;
mod lighting;
mod postprocess;
mod raymarch;
mod tree_bind_group;

/// Cast a slice to bytes.
pub fn byte_slice<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), std::mem::size_of_val(slice)) }
}

/// Cast a slice to mutable bytes.
pub fn byte_slice_mut<T>(slice: &mut [T]) -> &mut [u8] {
    unsafe {
        std::slice::from_raw_parts_mut(slice.as_mut_ptr().cast(), std::mem::size_of_val(slice))
    }
}

#[macro_export]
macro_rules! concat_files {
    ($($path:expr),* $(,)?) => {
        concat!($(include_str!($path)),*)
    };
}

#[macro_export]
macro_rules! include_wgsl {
    ($first_path:literal $(,)? $($path:literal),* $(,)?) => {
        rube_platform::wgpu::ShaderModuleDescriptor {
            label: Some($first_path),
            source: rube_platform::wgpu::ShaderSource::Wgsl($crate::concat_files!($first_path, $($path),*).into()),
        }
    };
}

pub struct Renderer {
    driver: Driver,
    marcher: RayMarchPipeline,
    lighting: LightingPipeline,
    postprocess: PostprocessPipeline,
    camera: CameraBindGroup,
    tree: VoxelTreeBindGroup,
}

impl Renderer {
    pub fn new(driver: Driver) -> Self {
        let postprocess = postprocess::PostprocessPipeline::new(&driver);
        let camera = CameraBindGroup::new(&driver);
        let tree = VoxelTreeBindGroup::new(&driver);
        let marcher = RayMarchPipeline::new(&driver, &camera, &tree);
        let lighting = LightingPipeline::new(
            &driver,
            marcher.compute_texture_view(),
            postprocess.compute_texture_view(),
            &tree,
        );

        Self {
            driver,
            marcher,
            lighting,
            postprocess,
            camera,
            tree,
        }
    }

    pub fn load_map(&mut self, map_bytes: &[u8]) {
        let tree = rube_voxel::tree::VoxelTree::decompress(map_bytes);
        self.tree.write_tree(&self.driver, &tree);
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.driver.resize(size.width, size.height);
    }

    pub fn render(&mut self, camera: &Camera) {
        let Some(tree_bind_group) = &self.tree.bind_group else {
            return;
        };

        // write buffers
        self.camera.write_buffers(&self.driver, camera);

        // render/compute passes
        let surface_texture = self.driver.surface.get_current_texture().unwrap();
        let surface_view = surface_texture.texture.create_view(&Default::default());
        let mut encoder = self
            .driver
            .device
            .create_command_encoder(&Default::default());
        if let Some(buffer) = &self.tree.active {
            // clear just the count
            self.driver.queue.write_buffer(buffer, 0, &[0; 4]);
        }
        self.marcher.compute_pass(
            &self.driver,
            &mut encoder,
            &self.camera.bind_group,
            tree_bind_group,
        );
        self.lighting
            .compute_pass(&self.driver, &mut encoder, tree_bind_group);
        self.postprocess.render_pass(&mut encoder, &surface_view);
        self.driver.queue.submit([encoder.finish()]);
        surface_texture.present();
    }
}
