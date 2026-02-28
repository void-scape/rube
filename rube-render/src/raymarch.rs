use crate::{camera_bind_group::CameraBindGroup, tree_bind_group::VoxelTreeBindGroup};
use rube_platform::{Driver, wgpu};

pub struct RayMarchPipeline {
    marcher: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    compute_texture_view: wgpu::TextureView,
}

impl RayMarchPipeline {
    pub fn new(driver: &Driver, camera: &CameraBindGroup, tree: &VoxelTreeBindGroup) -> Self {
        let compute_texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: driver.width,
                height: driver.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            label: None,
            view_formats: &[],
        };
        let compute_texture = driver.device.create_texture(&compute_texture_desc);
        let compute_texture_view = compute_texture.create_view(&Default::default());

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
                            format: wgpu::TextureFormat::R32Uint,
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
                resource: wgpu::BindingResource::TextureView(&compute_texture_view),
            }],
        });

        let module = driver.device.create_shader_module(crate::include_wgsl!(
            "shaders/raymarch.wgsl"
            "shaders/cast.wgsl",
        ));
        let pipeline_layout =
            driver
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[
                        &tree.bind_group_layout,
                        &camera.bind_group_layout,
                        &bind_group_layout,
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
            compute_texture_view,
        }
    }

    pub fn compute_texture_view(&self) -> &wgpu::TextureView {
        &self.compute_texture_view
    }

    pub fn compute_pass(
        &self,
        driver: &Driver,
        encoder: &mut wgpu::CommandEncoder,
        camera_bind_group: &wgpu::BindGroup,
        tree_bind_group: &wgpu::BindGroup,
    ) {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("raymarch compute pass"),
            ..Default::default()
        });
        cpass.set_pipeline(&self.marcher);
        cpass.set_bind_group(0, tree_bind_group, &[]);
        cpass.set_bind_group(1, camera_bind_group, &[]);
        cpass.set_bind_group(2, &self.bind_group, &[]);
        let x = driver.width.div_ceil(8);
        let y = driver.height.div_ceil(8);
        cpass.dispatch_workgroups(x, y, 1);
    }
}
