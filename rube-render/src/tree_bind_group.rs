// Sparse-64 voxel tree implementation adapted from:
// https://dubiousconst282.github.io/2024/10/03/voxel-ray-tracing/

use rube_platform::{Driver, wgpu};
use rube_voxel::tree::VoxelTree;
use wgpu::util::DeviceExt;

pub struct VoxelTreeBindGroup {
    pub bind_group: Option<wgpu::BindGroup>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub active: Option<wgpu::Buffer>,
}

impl VoxelTreeBindGroup {
    pub fn new(driver: &Driver) -> Self {
        let bind_group_layout =
            driver
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tree_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        Self {
            bind_group: None,
            bind_group_layout,
            active: None,
        }
    }

    // TODO: Check the size of the current buffers, these do not need to be reallocated
    // if they are big enough!
    pub fn write_tree(&mut self, driver: &Driver, tree: &VoxelTree) {
        self.bind_group.take();
        self.active.take();
        let (bg, active) = Self::create_bind_group(driver, tree, &self.bind_group_layout);
        self.bind_group = Some(bg);
        self.active = Some(active);
    }

    fn create_bind_group(
        driver: &Driver,
        tree: &VoxelTree,
        layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::BindGroup, wgpu::Buffer) {
        let cell_size = driver
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: crate::byte_slice(&[f32::from_bits(127u32.wrapping_sub(tree.exp) << 23)]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let nodes = driver
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: crate::byte_slice(tree.nodes.as_slice()),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let leaves = driver
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: crate::byte_slice(tree.leaves.as_slice()),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let voxel_state = driver.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 4 * tree.leaves.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let palette = driver
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: crate::byte_slice(tree.palette.as_slice()),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let active = driver.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (4 + 16 * driver.width * driver.height) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = driver.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tree_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cell_size.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: nodes.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: leaves.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: voxel_state.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: palette.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: active.as_entire_binding(),
                },
            ],
        });

        (bind_group, active)
    }
}
