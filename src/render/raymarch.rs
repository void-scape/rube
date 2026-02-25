use crate::render::{camera::CameraBindGroup, driver::Driver, tree::VoxelTreeBindGroup};

pub struct RayMarchPipeline {
    marcher: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
}

impl RayMarchPipeline {
    pub fn new(
        driver: &Driver,
        compute_texture: &wgpu::TextureView,
        camera: &CameraBindGroup,
        tree: &VoxelTreeBindGroup,
    ) -> Self {
        let bind_group_layout =
            driver
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("ray_marcher_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba32Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    }],
                });
        let bind_group = driver.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ray_marcher_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(compute_texture),
            }],
        });

        let module = driver
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/raymarch.wgsl"));
        let pipeline_layout =
            driver
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[
                        &bind_group_layout,
                        &camera.bind_group_layout,
                        &tree.bind_group_layout,
                    ],
                    immediate_size: 0,
                });
        let marcher = driver
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ray_marcher_compute_pipeline"),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some("raymarch"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        Self {
            marcher,
            bind_group,
        }
    }

    pub fn compute_pass(
        &self,
        driver: &Driver,
        encoder: &mut wgpu::CommandEncoder,
        camera_bind_group: &wgpu::BindGroup,
        tree_bind_group: &wgpu::BindGroup,
    ) {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.marcher);
        cpass.set_bind_group(0, &self.bind_group, &[]);
        cpass.set_bind_group(1, camera_bind_group, &[]);
        cpass.set_bind_group(2, tree_bind_group, &[]);
        let x = driver.width.div_ceil(8);
        let y = driver.height.div_ceil(8);
        cpass.dispatch_workgroups(x, y, 1);
    }
}
