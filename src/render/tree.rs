// Sparse-64 voxel tree implementation adapted from:
// https://dubiousconst282.github.io/2024/10/03/voxel-ray-tracing/

use crate::{
    render::driver::Driver,
    voxel::map::{Brick, VoxelMap},
};
use glam::UVec3;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use wgpu::util::DeviceExt;

pub struct VoxelTreeBindGroup {
    pub bind_group: Option<wgpu::BindGroup>,
    pub bind_group_layout: wgpu::BindGroupLayout,
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                    ],
                });

        Self {
            bind_group: None,
            bind_group_layout,
        }
    }

    // TODO: Check the size of the current buffers, these do not need to be reallocated
    // if they are big enough!
    pub fn write_tree(&mut self, driver: &Driver, tree: &VoxelTree) {
        self.bind_group.take();
        self.bind_group = Some(Self::create_bind_group(
            driver,
            tree,
            &self.bind_group_layout,
        ));
    }

    fn create_bind_group(
        driver: &Driver,
        tree: &VoxelTree,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        let nodes = driver
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: crate::render::byte_slice(tree.nodes.as_slice()),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
        let leaves = driver
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: crate::render::byte_slice(tree.leaves.as_slice()),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
        driver.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tree_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: nodes.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: leaves.as_entire_binding(),
                },
            ],
        })
    }
}

pub struct VoxelTree {
    nodes: Vec<Node>,
    leaves: Vec<u8>,
}

impl VoxelTree {
    pub fn from_obj<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        let stem = path.file_stem().unwrap();
        let full_stem = if let Some(parent) = path.parent() {
            PathBuf::from(parent).join(stem)
        } else {
            PathBuf::from(stem)
        };
        let compressed = full_stem.with_extension("bin.bz2");
        let uncompressed = full_stem.with_extension("bin");

        let (nodes, leaves) = if let Ok(true) = std::fs::exists(&compressed) {
            println!("Loading saved tree from {}", compressed.to_string_lossy());
            std::process::Command::new("bunzip2")
                .arg("-k")
                .arg(&compressed)
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            let file = std::fs::read(&uncompressed).unwrap();
            std::fs::remove_file(&uncompressed).unwrap();
            let node_bytes = u32::from_le_bytes([file[0], file[1], file[2], file[3]]) as usize;
            let nodes = unsafe {
                std::slice::from_raw_parts(
                    file[4..].as_ptr().cast(),
                    node_bytes / std::mem::size_of::<Node>(),
                )
            }
            .to_vec();
            let leaves = file[node_bytes + 4..].to_vec();
            (nodes, leaves)
        } else {
            let map = crate::voxel::voxelize::obj_to_voxels(path, 1024 * 2 * 2);
            println!("Building tree for {}...", path.to_string_lossy());
            let start = std::time::Instant::now();
            let mut nodes = vec![Node::default()];
            let mut leaves = Vec::new();
            let node = generate_tree(&map, &mut nodes, &mut leaves, 12, UVec3::ZERO);
            nodes[0] = node;
            println!("  [{:?}]", start.elapsed());

            let mut file = std::fs::File::create(&uncompressed).unwrap();
            let node_bytes = crate::render::byte_slice(nodes.as_slice());
            let node_size = node_bytes.len() as u32;
            file.write_all(&node_size.to_le_bytes()).unwrap();
            file.write_all(node_bytes).unwrap();
            file.write_all(leaves.as_slice()).unwrap();
            drop(file);

            // bzip2 beast.bin
            std::process::Command::new("bzip2")
                .arg(&uncompressed)
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            println!("Saving voxel tree to {}...", compressed.to_string_lossy());

            (nodes, leaves)
        };
        println!("Voxels: {}", leaves.len());
        println!(
            "Node tree: {:.2} MB",
            std::mem::size_of_val(nodes.as_slice()) as f32 / 1024.0 / 1024.0
        );
        println!(
            "Leaves: {:.2} MB",
            std::mem::size_of_val(leaves.as_slice()) as f32 / 1024.0 / 1024.0
        );
        println!(
            "Total: {:.2} MB",
            (std::mem::size_of_val(nodes.as_slice()) + std::mem::size_of_val(leaves.as_slice()))
                as f32
                / 1024.0
                / 1024.0
        );

        Self { nodes, leaves }
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Node {
    // [     31    |    1    ]
    // [ child_ptr | is_leaf ]
    // is_leaf   // Indicates if this node is a leaf containing plain voxels.
    // child_ptr // Absolute offset to array of existing child nodes/voxels.
    child_ptr_is_leaf: u32,
    // [   32  |   32  ]
    // [ maskh | maskl ]
    // Indicates which children/voxels are present in array.
    maskl: u32,
    maskh: u32,
    _pad: u32,
}

fn generate_tree(
    map: &VoxelMap,
    nodes: &mut Vec<Node>,
    leaves: &mut Vec<u8>,
    mut scale: u32,
    pos: UVec3,
) -> Node {
    debug_assert!(
        scale.is_multiple_of(2),
        "the tree is descended in increments of 2"
    );

    // Create leaf
    if scale == 2 {
        debug_assert_eq!((pos.x | pos.y | pos.z) % 4, 0);
        match map.brick(pos) {
            Some(brick) => {
                let mut node = Node::default();
                // Repack voxels into 4x4x4 tile
                // Cells are indexed by `x + z*4 + y*16`
                let mut temp = [0u8; 64];
                for i in (0..64).step_by(4) {
                    let offset = Brick::voxel_index(
                        UVec3::new(
                            pos.x,
                            pos.y + ((i as u32 >> 4) & 3),
                            pos.z + ((i as u32 >> 2) & 3),
                        ) & 7,
                    );
                    temp[i..i + 4].copy_from_slice(&brick.data[offset..offset + 4]);
                }
                let mut mask = 0u64;
                let leaf_index = leaves.len() as u32;
                // generate bitmask of `temp[i] != 0`.
                for (i, &data) in temp.iter().enumerate() {
                    if data != 0 {
                        mask |= 1 << i;
                        leaves.push(data);
                    }
                }
                node.maskl = (mask & 0xFFFFFFFF) as u32;
                node.maskh = (mask >> 32) as u32;
                node.child_ptr_is_leaf = (leaf_index << 1) | 1;
                node
            }
            None => Node::default(),
        }
    } else {
        // Descend
        scale -= 2;
        let mut children = Vec::with_capacity(64);
        let mut node = Node::default();
        let mut mask = 0u64;
        for i in 0..64 {
            let child_pos = UVec3::new(i & 3, (i >> 4) & 3, (i >> 2) & 3);
            let child = generate_tree(map, nodes, leaves, scale, pos + (child_pos << scale));
            // Node contains voxel/children data
            if child.maskl != 0 || child.maskh != 0 {
                mask |= 1 << i;
                children.push(child);
            }
        }
        node.maskl = (mask & 0xFFFFFFFF) as u32;
        node.maskh = (mask >> 32) as u32;
        node.child_ptr_is_leaf = (nodes.len() as u32) << 1;
        nodes.extend(children);
        node
    }
}
