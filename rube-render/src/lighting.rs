use crate::tree_bind_group::VoxelTreeBindGroup;
use rube_platform::{Driver, wgpu};

pub struct LightingPipeline {
    prepare_occlusion_pipeline: wgpu::ComputePipeline,
    occlusion_pipeline: wgpu::ComputePipeline,
    lighting_pipeline: wgpu::ComputePipeline,
    prepare_occlusion_bind_group: wgpu::BindGroup,
    occlusion_bind_group: wgpu::BindGroup,
    indirect: wgpu::Buffer,
}

impl LightingPipeline {
    pub fn new(
        driver: &Driver,
        input_texture: &wgpu::TextureView,
        output_texture: &wgpu::TextureView,
        tree: &VoxelTreeBindGroup,
    ) -> Self {
        let indirect = driver.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 12,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        let prepare_occlusion_bind_group_layout =
            driver
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let prepare_occlusion_bind_group =
            driver.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &prepare_occlusion_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 2,
                    resource: indirect.as_entire_binding(),
                }],
            });

        let occlusion_bind_group_layout =
            driver
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::ReadOnly,
                                format: wgpu::TextureFormat::R32Uint,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });
        let occlusion_bind_group = driver.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &occlusion_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(input_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(output_texture),
                },
            ],
        });

        let module = driver.device.create_shader_module(crate::include_wgsl!(
            "shaders/lighting.wgsl"
            "shaders/cast.wgsl",
        ));
        let pipeline_layout =
            driver
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[
                        &tree.bind_group_layout,
                        &prepare_occlusion_bind_group_layout,
                    ],
                    immediate_size: 0,
                });
        let prepare_occlusion_pipeline =
            driver
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("prepare_occlusion_compute_pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some("prepare_occlusion"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });
        let pipeline_layout =
            driver
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&tree.bind_group_layout, &occlusion_bind_group_layout],
                    immediate_size: 0,
                });
        let occlusion_pipeline =
            driver
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("occlusion_compute_pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some("occlusion"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });
        let lighting_pipeline =
            driver
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("lighting_compute_pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some("lighting"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });

        Self {
            prepare_occlusion_pipeline,
            occlusion_pipeline,
            lighting_pipeline,
            prepare_occlusion_bind_group,
            occlusion_bind_group,
            indirect,
        }
    }

    pub fn compute_pass(
        &self,
        driver: &Driver,
        encoder: &mut wgpu::CommandEncoder,
        tree_bind_group: &wgpu::BindGroup,
    ) {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("prepare occlusion pass"),
            ..Default::default()
        });
        cpass.set_pipeline(&self.prepare_occlusion_pipeline);
        cpass.set_bind_group(0, tree_bind_group, &[]);
        cpass.set_bind_group(1, &self.prepare_occlusion_bind_group, &[]);
        cpass.dispatch_workgroups(1, 1, 1);
        drop(cpass);

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("occlusion pass"),
            ..Default::default()
        });
        cpass.set_pipeline(&self.occlusion_pipeline);
        cpass.set_bind_group(0, tree_bind_group, &[]);
        cpass.set_bind_group(1, &self.occlusion_bind_group, &[]);
        cpass.dispatch_workgroups_indirect(&self.indirect, 0);
        drop(cpass);

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("shadow pass"),
            ..Default::default()
        });
        cpass.set_pipeline(&self.lighting_pipeline);
        cpass.set_bind_group(0, tree_bind_group, &[]);
        cpass.set_bind_group(1, &self.occlusion_bind_group, &[]);
        let x = driver.width.div_ceil(8);
        let y = driver.height.div_ceil(8);
        cpass.dispatch_workgroups(x, y, 1);
    }
}
