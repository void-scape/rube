use crate::{camera::Camera, render::driver::Driver};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy)]
struct CameraUniform {
    inv_proj_view: Mat4,
    origin: Vec3,
    _padding: f32,
}

pub struct CameraBindGroup {
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
}

impl CameraBindGroup {
    pub fn new(driver: &Driver) -> Self {
        let uniform = driver.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<CameraUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            driver
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("camera_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let bind_group = driver.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        Self {
            uniform,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn write_buffers(&self, driver: &Driver, camera: &Camera) {
        let uniform = CameraUniform {
            inv_proj_view: camera
                .projection_matrix(driver.width, driver.height)
                .mul_mat4(&camera.view_matrix())
                .inverse(),
            origin: camera.translation,
            _padding: 0.0,
        };
        driver
            .queue
            .write_buffer(&self.uniform, 0, crate::render::byte_slice(&[uniform]));
    }
}
