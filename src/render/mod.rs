use crate::{
    HEIGHT, WIDTH,
    camera::Camera,
    render::{
        camera::CameraBindGroup, driver::Driver, postprocess::PostprocessPipeline,
        raymarch::RayMarchPipeline, tree::VoxelTreeBindGroup,
    },
};
use glazer::winit::{dpi::PhysicalSize, window::Window};
use std::path::Path;

mod camera;
mod driver;
mod postprocess;
mod raymarch;
mod tree;

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

pub struct Renderer {
    driver: Driver,
    marcher: RayMarchPipeline,
    camera: CameraBindGroup,
    tree: VoxelTreeBindGroup,
    postprocess: PostprocessPipeline,
}

impl Renderer {
    pub fn new(window: &'static Window) -> Self {
        let driver = pollster::block_on(Driver::new(window, WIDTH as u32, HEIGHT as u32));
        let postprocess = postprocess::PostprocessPipeline::new(&driver);
        let camera = CameraBindGroup::new(&driver);
        let tree = VoxelTreeBindGroup::new(&driver);
        let marcher = raymarch::RayMarchPipeline::new(
            &driver,
            postprocess.compute_texture_view(),
            &camera,
            &tree,
        );

        Self {
            driver,
            marcher,
            tree,
            camera,
            postprocess,
        }
    }

    #[allow(unused)]
    pub fn load_obj<P: AsRef<Path>>(&mut self, path: P) {
        self.tree
            .write_tree(&self.driver, &tree::VoxelTree::from_obj(path));
    }

    #[allow(unused)]
    pub fn load_vox<P: AsRef<Path>>(&mut self, path: P) {
        self.tree
            .write_tree(&self.driver, &tree::VoxelTree::from_vox(path));
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.driver.resize(size.width, size.height);
    }

    pub fn render(&self, camera: &Camera) {
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
        self.marcher.compute_pass(
            &self.driver,
            &mut encoder,
            &self.camera.bind_group,
            tree_bind_group,
        );
        self.postprocess.render_pass(&mut encoder, &surface_view);
        self.driver.queue.submit([encoder.finish()]);
        surface_texture.present();
    }
}
