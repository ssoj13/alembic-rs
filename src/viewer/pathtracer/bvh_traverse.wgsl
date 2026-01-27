// BVH traversal compute shader for path tracing.
//
// Renders a depth/normal buffer by tracing primary rays through a BVH.
// This validates BVH correctness before full path tracing.

struct BVHNode {
    aabb_min: vec3<f32>,
    left_or_first: u32,
    aabb_max: vec3<f32>,
    count: u32,
};

struct Triangle {
    v0: vec3<f32>,
    material_id: u32,
    v1: vec3<f32>,
    _pad0: u32,
    v2: vec3<f32>,
    _pad1: u32,
    n0: vec3<f32>,
    _pad2: u32,
    n1: vec3<f32>,
    _pad3: u32,
    n2: vec3<f32>,
    _pad4: u32,
};

struct Camera {
    inv_view: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    position: vec3<f32>,
    frame_count: u32,
};

struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
};

struct HitInfo {
    t: f32,
    u: f32,
    v: f32,
    tri_idx: u32,
    hit: bool,
};

@group(0) @binding(0) var<storage, read> nodes: array<BVHNode>;
@group(0) @binding(1) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(2) var<uniform> camera: Camera;
@group(0) @binding(3) var output: texture_storage_2d<rgba32float, write>;

const MAX_STACK_DEPTH: u32 = 32u;
const T_MAX: f32 = 1e30;
const EPSILON: f32 = 1e-6;

// Moller-Trumbore ray-triangle intersection.
fn intersect_tri(ray: Ray, tri_idx: u32) -> HitInfo {
    var hit: HitInfo;
    hit.hit = false;
    hit.t = T_MAX;

    let tri = triangles[tri_idx];
    let e1 = tri.v1 - tri.v0;
    let e2 = tri.v2 - tri.v0;
    let h = cross(ray.dir, e2);
    let a = dot(e1, h);

    if abs(a) < EPSILON {
        return hit;
    }

    let f = 1.0 / a;
    let s = ray.origin - tri.v0;
    let u = f * dot(s, h);
    if u < 0.0 || u > 1.0 {
        return hit;
    }

    let q = cross(s, e1);
    let v = f * dot(ray.dir, q);
    if v < 0.0 || u + v > 1.0 {
        return hit;
    }

    let t = f * dot(e2, q);
    if t > EPSILON {
        hit.t = t;
        hit.u = u;
        hit.v = v;
        hit.tri_idx = tri_idx;
        hit.hit = true;
    }
    return hit;
}

// Ray-AABB slab test.
fn intersect_aabb(ray: Ray, inv_dir: vec3<f32>, node: BVHNode, t_best: f32) -> bool {
    let t1 = (node.aabb_min - ray.origin) * inv_dir;
    let t2 = (node.aabb_max - ray.origin) * inv_dir;
    let tmin = max(max(min(t1.x, t2.x), min(t1.y, t2.y)), min(t1.z, t2.z));
    let tmax = min(min(max(t1.x, t2.x), max(t1.y, t2.y)), max(t1.z, t2.z));
    return tmax >= max(tmin, 0.0) && tmin < t_best;
}

// Stack-based BVH traversal.
fn trace_ray(ray: Ray) -> HitInfo {
    var best: HitInfo;
    best.hit = false;
    best.t = T_MAX;

    let inv_dir = 1.0 / ray.dir;

    var stack: array<u32, MAX_STACK_DEPTH>;
    var sp: u32 = 0u;
    stack[0] = 0u; // root
    sp = 1u;

    while sp > 0u {
        sp -= 1u;
        let node_idx = stack[sp];
        let node = nodes[node_idx];

        if !intersect_aabb(ray, inv_dir, node, best.t) {
            continue;
        }

        if node.count > 0u {
            // Leaf: test triangles
            for (var i = 0u; i < node.count; i++) {
                let hit = intersect_tri(ray, node.left_or_first + i);
                if hit.hit && hit.t < best.t {
                    best = hit;
                }
            }
        } else {
            // Internal: push children (right first for front-to-back order)
            if sp + 2u <= MAX_STACK_DEPTH {
                stack[sp] = node.left_or_first + 1u;
                sp += 1u;
                stack[sp] = node.left_or_first;
                sp += 1u;
            }
        }
    }

    return best;
}

// Generate camera ray for pixel (x, y).
fn gen_ray(x: f32, y: f32, dims: vec2<f32>) -> Ray {
    // NDC [-1, 1]
    let ndc = vec2<f32>(
        (x + 0.5) / dims.x * 2.0 - 1.0,
        1.0 - (y + 0.5) / dims.y * 2.0,
    );

    // Unproject near/far points
    let near = camera.inv_proj * vec4<f32>(ndc, -1.0, 1.0);
    let far  = camera.inv_proj * vec4<f32>(ndc,  1.0, 1.0);
    let near3 = near.xyz / near.w;
    let far3  = far.xyz / far.w;

    // Transform to world space
    let origin = (camera.inv_view * vec4<f32>(near3, 1.0)).xyz;
    let target = (camera.inv_view * vec4<f32>(far3, 1.0)).xyz;
    let dir = normalize(target - origin);

    var ray: Ray;
    ray.origin = origin;
    ray.dir = dir;
    return ray;
}

// Interpolate smooth normal at hit point.
fn hit_normal(hit: HitInfo) -> vec3<f32> {
    let tri = triangles[hit.tri_idx];
    let w = 1.0 - hit.u - hit.v;
    return normalize(w * tri.n0 + hit.u * tri.n1 + hit.v * tri.n2);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<f32>(textureDimensions(output));
    let px = vec2<f32>(f32(gid.x), f32(gid.y));

    if px.x >= dims.x || px.y >= dims.y {
        return;
    }

    let ray = gen_ray(px.x, px.y, dims);
    let hit = trace_ray(ray);

    var color: vec4<f32>;
    if hit.hit {
        // Simple N·L shading for validation
        let n = hit_normal(hit);
        let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
        let ndotl = max(dot(n, light_dir), 0.0);
        let ambient = 0.1;
        let shade = ambient + ndotl * 0.9;
        color = vec4<f32>(shade, shade, shade, 1.0);
    } else {
        // Sky gradient
        let t = ray.dir.y * 0.5 + 0.5;
        color = vec4<f32>(mix(vec3<f32>(0.8, 0.85, 0.9), vec3<f32>(0.3, 0.5, 0.8), t), 1.0);
    }

    textureStore(output, vec2<i32>(gid.xy), color);
}
