//! SAH-based BVH builder.
//!
//! Constructs a flat BVH array from a list of triangles.
//! Uses Surface Area Heuristic for split decisions and
//! produces a compact node array for GPU upload.

use super::bvh::{Aabb, BvhNode, Triangle};

/// Number of SAH bins for split evaluation.
const NUM_BINS: usize = 12;

/// Cost ratio: traversal vs intersection (typical GPU values).
const TRAVERSAL_COST: f32 = 1.0;
const INTERSECT_COST: f32 = 1.0;

/// Maximum triangles per leaf before forcing a split.
const MAX_LEAF_SIZE: usize = 4;

/// Built BVH result.
pub struct Bvh {
    /// Flat node array (index 0 = root).
    pub nodes: Vec<BvhNode>,
    /// Reordered triangle indices (leaves reference into this).
    pub tri_indices: Vec<usize>,
}

/// SAH bin for evaluating split candidates.
struct Bin {
    bounds: Aabb,
    count: usize,
}

impl Bin {
    fn new() -> Self {
        Self {
            bounds: Aabb::EMPTY,
            count: 0,
        }
    }
}

/// Build BVH from triangles using SAH.
///
/// Returns a flat node array + reordered triangle index list.
/// Triangles are NOT modified — indices map into the original slice.
#[tracing::instrument(skip_all, fields(tri_count = triangles.len()))]
pub fn build_bvh(triangles: &[Triangle]) -> Bvh {
    let n = triangles.len();
    if n == 0 {
        return Bvh {
            nodes: vec![BvhNode {
                aabb_min: [0.0; 3],
                left_or_first: 0,
                aabb_max: [0.0; 3],
                count: 0,
            }],
            tri_indices: vec![],
        };
    }

    // Pre-compute centroids and AABBs
    let centroids: Vec<[f32; 3]> = triangles.iter().map(|t| t.centroid()).collect();
    let aabbs: Vec<Aabb> = triangles.iter().map(|t| t.aabb()).collect();

    // Working index array (will be reordered by partitioning)
    let mut indices: Vec<usize> = (0..n).collect();

    // Pre-allocate nodes (worst case: 2*n - 1 for a full binary tree)
    let mut nodes: Vec<BvhNode> = Vec::with_capacity(2 * n);

    // Root node placeholder
    nodes.push(BvhNode {
        aabb_min: [0.0; 3],
        left_or_first: 0,
        aabb_max: [0.0; 3],
        count: 0,
    });

    // Build recursively using a stack (avoid actual recursion for large scenes)
    struct Task {
        node_idx: usize,
        start: usize,
        end: usize, // exclusive
    }

    let mut stack = vec![Task {
        node_idx: 0,
        start: 0,
        end: n,
    }];

    while let Some(task) = stack.pop() {
        let start = task.start;
        let end = task.end;
        let count = end - start;

        // Compute AABB for this range
        let mut node_aabb = Aabb::EMPTY;
        for &idx in &indices[start..end] {
            node_aabb.grow(&aabbs[idx]);
        }

        // Make leaf if small enough
        if count <= MAX_LEAF_SIZE {
            nodes[task.node_idx] = BvhNode {
                aabb_min: node_aabb.min,
                left_or_first: start as u32,
                aabb_max: node_aabb.max,
                count: count as u32,
            };
            continue;
        }

        // Compute centroid bounds for binning
        let mut centroid_bounds = Aabb::EMPTY;
        for &idx in &indices[start..end] {
            centroid_bounds.grow_point(centroids[idx]);
        }

        // Find best split via SAH binning
        let (best_axis, best_split_pos, best_cost) =
            find_best_split(&indices[start..end], &aabbs, &centroids, &centroid_bounds);

        // Cost of not splitting (leaf cost), normalized by parent area
        let parent_area = node_aabb.area();
        let leaf_cost = count as f32 * INTERSECT_COST * parent_area;

        // If SAH says leaf is cheaper, or degenerate centroid extent, make leaf
        if best_cost >= leaf_cost || best_axis == usize::MAX {
            nodes[task.node_idx] = BvhNode {
                aabb_min: node_aabb.min,
                left_or_first: start as u32,
                aabb_max: node_aabb.max,
                count: count as u32,
            };
            continue;
        }

        // Partition indices around split position
        let mid = partition(&mut indices[start..end], |&idx| {
            centroids[idx][best_axis] < best_split_pos
        }) + start;

        // Fallback: if partition is degenerate, split in middle
        let mid = if mid == start || mid == end {
            (start + end) / 2
        } else {
            mid
        };

        // Allocate child nodes
        let left_idx = nodes.len();
        let right_idx = left_idx + 1;
        nodes.push(BvhNode {
            aabb_min: [0.0; 3],
            left_or_first: 0,
            aabb_max: [0.0; 3],
            count: 0,
        });
        nodes.push(BvhNode {
            aabb_min: [0.0; 3],
            left_or_first: 0,
            aabb_max: [0.0; 3],
            count: 0,
        });

        // Set this node as internal
        nodes[task.node_idx] = BvhNode {
            aabb_min: node_aabb.min,
            left_or_first: left_idx as u32,
            aabb_max: node_aabb.max,
            count: 0,
        };

        // Push children (right first so left is processed first — depth-first)
        stack.push(Task {
            node_idx: right_idx,
            start: mid,
            end,
        });
        stack.push(Task {
            node_idx: left_idx,
            start,
            end: mid,
        });
    }

    Bvh {
        nodes,
        tri_indices: indices,
    }
}

/// SAH binned split search across all 3 axes.
/// Returns (best_axis, split_position, cost). axis=usize::MAX if no valid split.
fn find_best_split(
    indices: &[usize],
    aabbs: &[Aabb],
    centroids: &[[f32; 3]],
    centroid_bounds: &Aabb,
) -> (usize, f32, f32) {
    let mut best_axis = usize::MAX;
    let mut best_pos = 0.0f32;
    let mut best_cost = f32::INFINITY;

    for axis in 0..3 {
        let extent = centroid_bounds.max[axis] - centroid_bounds.min[axis];
        if extent < 1e-8 {
            continue; // degenerate axis
        }

        // Initialize bins
        let mut bins = Vec::with_capacity(NUM_BINS);
        for _ in 0..NUM_BINS {
            bins.push(Bin::new());
        }

        let inv_extent = NUM_BINS as f32 / extent;

        // Assign primitives to bins
        for &idx in indices {
            let bin_id = ((centroids[idx][axis] - centroid_bounds.min[axis]) * inv_extent) as usize;
            let bin_id = bin_id.min(NUM_BINS - 1);
            bins[bin_id].bounds.grow(&aabbs[idx]);
            bins[bin_id].count += 1;
        }

        // Sweep from left: compute prefix areas and counts
        let mut left_area = [0.0f32; NUM_BINS - 1];
        let mut left_count = [0usize; NUM_BINS - 1];
        let mut sweep = Aabb::EMPTY;
        let mut sweep_count = 0;
        for i in 0..NUM_BINS - 1 {
            sweep.grow(&bins[i].bounds);
            sweep_count += bins[i].count;
            left_area[i] = sweep.area();
            left_count[i] = sweep_count;
        }

        // Sweep from right and evaluate SAH cost
        sweep = Aabb::EMPTY;
        sweep_count = 0;
        for i in (1..NUM_BINS).rev() {
            sweep.grow(&bins[i].bounds);
            sweep_count += bins[i].count;
            let cost = TRAVERSAL_COST
                + INTERSECT_COST
                    * (left_count[i - 1] as f32 * left_area[i - 1]
                        + sweep_count as f32 * sweep.area());

            if cost < best_cost {
                best_cost = cost;
                best_axis = axis;
                best_pos = centroid_bounds.min[axis] + (i as f32 / NUM_BINS as f32) * extent;
            }
        }
    }

    (best_axis, best_pos, best_cost)
}

/// Partition slice in-place. Returns count of elements where predicate is true.
fn partition<T, F>(slice: &mut [T], pred: F) -> usize
where
    F: Fn(&T) -> bool,
{
    let mut left = 0;
    let mut right = slice.len();
    while left < right {
        if pred(&slice[left]) {
            left += 1;
        } else {
            right -= 1;
            slice.swap(left, right);
        }
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tri(cx: f32, cy: f32, cz: f32) -> Triangle {
        Triangle {
            v0: [cx - 0.5, cy - 0.5, cz],
            v1: [cx + 0.5, cy - 0.5, cz],
            v2: [cx, cy + 0.5, cz],
            n0: [0.0, 0.0, 1.0],
            n1: [0.0, 0.0, 1.0],
            n2: [0.0, 0.0, 1.0],
            material_id: 0,
        }
    }

    #[test]
    fn test_empty_bvh() {
        let bvh = build_bvh(&[]);
        assert_eq!(bvh.nodes.len(), 1);
        assert_eq!(bvh.tri_indices.len(), 0);
    }

    #[test]
    fn test_single_triangle() {
        let tris = vec![make_tri(0.0, 0.0, 0.0)];
        let bvh = build_bvh(&tris);
        assert_eq!(bvh.nodes.len(), 1); // just a leaf
        assert_eq!(bvh.nodes[0].count, 1);
        assert_eq!(bvh.tri_indices.len(), 1);
    }

    #[test]
    fn test_many_triangles_builds_tree() {
        // 100 triangles spread along X axis → should split into a tree
        let tris: Vec<Triangle> = (0..100)
            .map(|i| make_tri(i as f32 * 2.0, 0.0, 0.0))
            .collect();
        let bvh = build_bvh(&tris);

        // Must have internal nodes (more than 1 node)
        assert!(bvh.nodes.len() > 1, "BVH should have internal nodes");

        // All triangle indices must be present
        let mut sorted = bvh.tri_indices.clone();
        sorted.sort();
        assert_eq!(sorted, (0..100).collect::<Vec<_>>());

        // Root AABB must encompass all triangles
        let root = &bvh.nodes[0];
        assert!(root.aabb_min[0] < 0.0);
        assert!(root.aabb_max[0] > 198.0);
    }

    #[test]
    fn test_leaf_count_correct() {
        // 3 triangles → should be a single leaf
        let tris = vec![
            make_tri(0.0, 0.0, 0.0),
            make_tri(1.0, 0.0, 0.0),
            make_tri(2.0, 0.0, 0.0),
        ];
        let bvh = build_bvh(&tris);
        assert_eq!(bvh.nodes[0].count, 3); // leaf with 3 triangles
    }
}
