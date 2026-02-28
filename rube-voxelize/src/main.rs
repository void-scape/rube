use glam::IVec3;
use rube_voxel::tree::{Node, VoxelTree, generate_tree};

mod obj;
mod vox;

fn main() -> std::io::Result<()> {
    for entry in std::fs::read_dir("assets")? {
        let exp = 12;
        let mut path = entry?.path();
        if let Some(mut map) = match path.extension().map(|e| e.to_string_lossy()).as_deref() {
            Some("vox") => Some(vox::voxelize(&path)),
            Some("obj") => Some(obj::voxelize(&path, 1024 * 2)),
            _ => None,
        } {
            map.shift_to_positive();
            println!("Treeifying {}...", path.display());
            let start = std::time::Instant::now();
            let mut nodes = vec![Node::default()];
            let mut leaves = Vec::new();
            let mut node_hash = rube_voxel::ahash::HashMap::default();
            let mut saved_bytes = 0;
            let node = generate_tree(
                &map,
                &mut node_hash,
                &mut nodes,
                &mut leaves,
                exp,
                IVec3::ZERO,
                &mut saved_bytes,
            );
            nodes[0] = node;
            let palette = map.palette.to_vec();
            let tree = VoxelTree {
                nodes,
                leaves,
                palette,
                exp,
            };
            let bytes = tree.compress();
            let file_stem = path.file_stem().unwrap().to_string_lossy().to_string();
            path.pop();
            std::fs::write(path.join(format!("{}.bin.bz2", file_stem)), bytes)?;
            println!("  [{:?}]", start.elapsed());

            println!("  Voxels: {}", tree.leaves.len());
            println!(
                "  Node tree: {:.2} MB",
                std::mem::size_of_val(tree.nodes.as_slice()) as f32 / 1024.0 / 1024.0
            );
            println!(
                "  Leaves: {:.2} MB",
                std::mem::size_of_val(tree.leaves.as_slice()) as f32 / 1024.0 / 1024.0
            );
            println!(
                "  Total: {:.2} MB",
                (std::mem::size_of_val(tree.nodes.as_slice())
                    + std::mem::size_of_val(tree.leaves.as_slice())) as f32
                    / 1024.0
                    / 1024.0
            );
            println!("Saved: {:.2} MB", saved_bytes as f32 / 1024.0 / 1024.0);
        }
    }
    Ok(())
}
