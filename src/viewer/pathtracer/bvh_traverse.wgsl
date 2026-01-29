// BVH traversal compute shader with progressive path tracing.
//
// Features:
// - PCG random number generator for stochastic sampling
// - Progressive accumulation via read_write storage buffer
// - GGX microfacet specular with metallic workflow
// - Emission support for self-luminous objects
// - Cosine-weighted hemisphere sampling for diffuse bounces
// - Up to MAX_BOUNCES indirect illumination
// - HDR sky environment lighting

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
    object_id: u32,
    v2: vec3<f32>,
    _pad1: u32,
    n0: vec3<f32>,
    _pad2: u32,
    n1: vec3<f32>,
    _pad3: u32,
    n2: vec3<f32>,
    _pad4: u32,
};

// Material matching GpuMaterial layout (144 bytes, vec4-packed).
// Full Autodesk Standard Surface parameters.
struct Material {
    base_color_weight: vec4<f32>,         // rgb=color, a=weight
    specular_color_weight: vec4<f32>,     // rgb=color, a=weight
    transmission_color_weight: vec4<f32>, // rgb=color, a=weight
    subsurface_color_weight: vec4<f32>,   // rgb=color, a=weight
    coat_color_weight: vec4<f32>,         // rgb=color, a=weight
    emission_color_weight: vec4<f32>,     // rgb=color, a=weight (intensity in a)
    opacity: vec4<f32>,                   // rgb=opacity, a=unused
    params1: vec4<f32>,                   // x=diffuse_rough, y=metalness, z=spec_rough, w=spec_IOR
    params2: vec4<f32>,                   // x=anisotropy, y=coat_rough, z=coat_IOR, w=visible
};

struct Camera {
    inv_view: mat4x4<f32>,      // offset 0, 64 bytes
    inv_proj: mat4x4<f32>,      // offset 64, 64 bytes
    position: vec3<f32>,        // offset 128, 12 bytes
    _pad0: u32,                 // offset 140, 4 bytes (vec3 padding)
    frame_count: u32,           // offset 144, 4 bytes
    max_bounces: u32,           // offset 148, 4 bytes
    max_transmission_depth: u32, // offset 152, 4 bytes
    dof_enabled: u32,           // offset 156, 4 bytes
    aperture: f32,              // offset 160, 4 bytes
    focus_distance: f32,        // offset 164, 4 bytes
    _pad1: vec2<u32>,           // offset 168, 8 bytes
    _pad2: vec4<u32>,           // offset 176, 16 bytes
    // Total: 192 bytes
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
@group(0) @binding(4) var<storage, read_write> accum: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read> materials: array<Material>;
@group(0) @binding(6) var env_map: texture_2d<f32>;
@group(0) @binding(7) var env_sampler: sampler;
@group(0) @binding(8) var<uniform> env: EnvParams;
@group(0) @binding(9) var<storage, read> visibility: array<u32>;
@group(0) @binding(10) var<storage, read> env_marginal_cdf: array<f32>;
@group(0) @binding(11) var<storage, read> env_conditional_cdf: array<f32>;

// Environment parameters
struct EnvParams {
    intensity: f32,
    rotation: f32,
    enabled: f32,
    use_importance_sampling: f32,
    env_width: u32,
    env_height: u32,
    _pad: vec2<u32>,
};

const MAX_STACK_DEPTH: u32 = 32u;
const T_MAX: f32 = 1e30;
const EPSILON: f32 = 1e-6;
const PI: f32 = 3.14159265359;

// Sun light parameters (could be uniforms later)
const SUN_DIR: vec3<f32> = vec3<f32>(0.5, 0.8, 0.3);  // normalized in code
const SUN_COLOR: vec3<f32> = vec3<f32>(1.0, 0.98, 0.95);
const SUN_INTENSITY: f32 = 5.0;
const SUN_ANGULAR_RADIUS: f32 = 0.00465; // ~0.53 degrees, real sun size

// ---- PCG random number generator (kept for RR and lobe selection) ----

fn pcg_hash(input: u32) -> u32 {
    var state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rand(state: ptr<function, u32>) -> f32 {
    *state = pcg_hash(*state);
    return f32(*state) / 4294967296.0;
}

// ---- R2 Low-Discrepancy Sequence ----
// Provides better sample distribution than pure random for faster convergence.
// Based on the plastic constant (generalized golden ratio for 2D).

const R2_A1: f32 = 0.7548776662466927;  // 1.0 / plastic constant
const R2_A2: f32 = 0.5698402909980532;  // 1.0 / (plastic^2)

// Generate 2D R2 sample for given index.
fn r2_2d(n: u32) -> vec2<f32> {
    return fract(vec2<f32>(0.5) + f32(n) * vec2<f32>(R2_A1, R2_A2));
}

// Cranley-Patterson rotation: add random offset per pixel for spatial variation.
// This decorrelates samples between neighboring pixels.
fn r2_2d_rotated(n: u32, offset: vec2<f32>) -> vec2<f32> {
    return fract(r2_2d(n) + offset);
}

// Dimensions per frame for R2 sampling:
// - dim 0: AA jitter
// - dim 1: DoF lens
// - dim 2+bounce*2: main lobe (diffuse/spec/trans)
// - dim 3+bounce*2: secondary (coat, sun disc)
const R2_DIMS_PER_FRAME: u32 = 20u;  // enough for 8 bounces

// Get R2 sample for specific dimension within current frame.
// Uses frame_count as base index for progressive refinement.
fn get_r2_sample(frame: u32, dim: u32, offset: vec2<f32>) -> vec2<f32> {
    let idx = frame * R2_DIMS_PER_FRAME + dim;
    return r2_2d_rotated(idx, offset);
}

// ---- Intersection routines ----

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
    stack[0] = 0u;
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
                let tri_id = node.left_or_first + i;
                let hit = intersect_tri(ray, tri_id);
                if hit.hit && hit.t < best.t {
                    // Skip hidden objects/materials
                    let tri = triangles[tri_id];
                    if visibility[tri.object_id] == 0u { continue; }
                    if materials[tri.material_id].params2.w < 0.5 { continue; }
                    best = hit;
                }
            }
        } else {
            // Internal: push children with near-far ordering based on ray direction
            if sp + 2u <= MAX_STACK_DEPTH {
                let left_child = node.left_or_first;
                let right_child = node.left_or_first + 1u;
                
                // Determine split axis from AABB extent (dominant axis)
                let extent = node.aabb_max - node.aabb_min;
                
                // Find dominant axis and check if ray goes in negative direction
                var flip = false;
                if extent.x >= extent.y && extent.x >= extent.z {
                    flip = ray.dir.x < 0.0;  // X dominant
                } else if extent.y >= extent.z {
                    flip = ray.dir.y < 0.0;  // Y dominant
                } else {
                    flip = ray.dir.z < 0.0;  // Z dominant
                }
                
                // Order: near child processed first (pushed last onto LIFO stack)
                let near_child = select(left_child, right_child, flip);
                let far_child = select(right_child, left_child, flip);
                
                stack[sp] = far_child; sp += 1u;
                stack[sp] = near_child; sp += 1u;
            }
        }
    }

    return best;
}

// Shadow ray: returns true if any intersection found before max_t.
fn trace_shadow_ray(ray: Ray, max_t: f32) -> bool {
    let inv_dir = 1.0 / ray.dir;

    var stack: array<u32, MAX_STACK_DEPTH>;
    var sp: u32 = 0u;
    stack[0] = 0u;
    sp = 1u;

    while sp > 0u {
        sp -= 1u;
        let node_idx = stack[sp];
        let node = nodes[node_idx];

        if !intersect_aabb(ray, inv_dir, node, max_t) {
            continue;
        }

        if node.count > 0u {
            // Leaf: test triangles
            for (var i = 0u; i < node.count; i++) {
                let s_tri_id = node.left_or_first + i;
                let hit = intersect_tri(ray, s_tri_id);
                if hit.hit && hit.t < max_t && hit.t > EPSILON {
                    let tri = triangles[s_tri_id];
                    if visibility[tri.object_id] == 0u { continue; }
                    if materials[tri.material_id].params2.w < 0.5 { continue; }
                    return true; // occluded
                }
            }
        } else {
            // Internal: push children with near-far ordering
            if sp + 2u <= MAX_STACK_DEPTH {
                let left_child = node.left_or_first;
                let right_child = node.left_or_first + 1u;
                
                let extent = node.aabb_max - node.aabb_min;
                var flip = false;
                if extent.x >= extent.y && extent.x >= extent.z {
                    flip = ray.dir.x < 0.0;
                } else if extent.y >= extent.z {
                    flip = ray.dir.y < 0.0;
                } else {
                    flip = ray.dir.z < 0.0;
                }
                
                let near_child = select(left_child, right_child, flip);
                let far_child = select(right_child, left_child, flip);
                
                stack[sp] = far_child; sp += 1u;
                stack[sp] = near_child; sp += 1u;
            }
        }
    }

    return false; // not occluded
}

// Sample direction on sun disc (for soft shadows).
// Takes pre-generated random values for R2 sequence support.
fn sample_sun_direction(r1: f32, r2: f32) -> vec3<f32> {
    let sun_dir = normalize(SUN_DIR);
    
    // Build tangent frame
    var t: vec3<f32>;
    if abs(sun_dir.y) < 0.999 {
        t = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), sun_dir));
    } else {
        t = normalize(cross(vec3<f32>(1.0, 0.0, 0.0), sun_dir));
    }
    let b = cross(sun_dir, t);
    
    // Sample uniformly on disc
    let r = SUN_ANGULAR_RADIUS * sqrt(r1);
    let theta = 2.0 * PI * r2;
    
    // Perturb direction
    return normalize(sun_dir + r * (cos(theta) * t + sin(theta) * b));
}

// ---- Sampling ----

// Cosine-weighted hemisphere sample around +Y.
fn cosine_hemisphere(r1: f32, r2: f32) -> vec3<f32> {
    let phi = 2.0 * PI * r1;
    let cos_theta = sqrt(r2);
    let sin_theta = sqrt(1.0 - r2);
    return vec3<f32>(cos(phi) * sin_theta, cos_theta, sin(phi) * sin_theta);
}

// GGX importance sampling: returns half-vector in local space.
fn sample_ggx(r1: f32, r2: f32, alpha: f32) -> vec3<f32> {
    let a2 = alpha * alpha;
    let phi = 2.0 * PI * r1;
    let cos_theta = sqrt((1.0 - r2) / (1.0 + (a2 - 1.0) * r2));
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
    return vec3<f32>(cos(phi) * sin_theta, cos_theta, sin(phi) * sin_theta);
}

// GGX normal distribution function.
fn ggx_d(ndoth: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let d = ndoth * ndoth * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d + EPSILON);
}

// Smith G1 for GGX.
fn smith_g1(ndotv: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let denom = ndotv + sqrt(a2 + (1.0 - a2) * ndotv * ndotv);
    return 2.0 * ndotv / (denom + EPSILON);
}

// Schlick Fresnel approximation.
fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    let t = 1.0 - cos_theta;
    let t2 = t * t;
    return f0 + (1.0 - f0) * (t2 * t2 * t);
}

// ---- MIS (Multiple Importance Sampling) ----

// Power heuristic for MIS (beta=2 is common choice)
fn mis_power_heuristic(pdf_a: f32, pdf_b: f32) -> f32 {
    let a2 = pdf_a * pdf_a;
    let b2 = pdf_b * pdf_b;
    return a2 / (a2 + b2 + EPSILON);
}

// PDF for cosine-weighted hemisphere sampling
fn pdf_cosine_hemisphere(cos_theta: f32) -> f32 {
    return cos_theta / PI;
}

// PDF for GGX importance sampling (half-vector distribution)
fn pdf_ggx(ndoth: f32, hdotv: f32, alpha: f32) -> f32 {
    let d = ggx_d(ndoth, alpha);
    // Convert from half-vector PDF to reflection direction PDF
    return d * ndoth / (4.0 * hdotv + EPSILON);
}

// PDF for uniform sampling on sun disc
fn pdf_sun_disc() -> f32 {
    // Solid angle of sun disc ≈ PI * (angular_radius)^2 for small angles
    let solid_angle = PI * SUN_ANGULAR_RADIUS * SUN_ANGULAR_RADIUS;
    return 1.0 / solid_angle;
}

// Build orthonormal basis from normal (n = up direction).
fn onb_from_normal(n: vec3<f32>) -> mat3x3<f32> {
    var t: vec3<f32>;
    if abs(n.y) < 0.999 {
        t = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), n));
    } else {
        t = normalize(cross(vec3<f32>(1.0, 0.0, 0.0), n));
    }
    let b = cross(n, t);
    return mat3x3<f32>(t, n, b); // columns: tangent, normal, bitangent
}

// ---- Camera ----

// Generate camera ray with sub-pixel jitter for anti-aliasing.
// Sample point on unit disk (for DoF lens sampling)
fn sample_disk(r1: f32, r2: f32) -> vec2<f32> {
    let theta = 2.0 * PI * r1;
    let r = sqrt(r2);
    return vec2<f32>(r * cos(theta), r * sin(theta));
}

fn gen_ray(x: f32, y: f32, dims: vec2<f32>, jx: f32, jy: f32, rng: ptr<function, u32>) -> Ray {
    let ndc = vec2<f32>(
        (x + jx) / dims.x * 2.0 - 1.0,
        1.0 - (y + jy) / dims.y * 2.0,
    );

    let near = camera.inv_proj * vec4<f32>(ndc, -1.0, 1.0);
    let far  = camera.inv_proj * vec4<f32>(ndc,  1.0, 1.0);
    let near3 = near.xyz / near.w;
    let far3  = far.xyz / far.w;

    let origin_view = near3;
    let dir_view = normalize(far3 - near3);
    
    var ray: Ray;
    
    if camera.dof_enabled != 0u && camera.aperture > 0.0 {
        // Thin lens DoF model
        // Find point on focal plane along the ray
        let t_focus = camera.focus_distance / max(abs(dir_view.z), 0.001);
        let focus_point_view = origin_view + dir_view * t_focus;
        
        // Sample point on lens (aperture disk)
        let lens_sample = sample_disk(rand(rng), rand(rng)) * camera.aperture;
        let lens_origin_view = origin_view + vec3<f32>(lens_sample, 0.0);
        
        // New ray from lens point through focus point
        let new_dir_view = normalize(focus_point_view - lens_origin_view);
        
        // Transform to world space
        ray.origin = (camera.inv_view * vec4<f32>(lens_origin_view, 1.0)).xyz;
        ray.dir = normalize((camera.inv_view * vec4<f32>(new_dir_view, 0.0)).xyz);
    } else {
        // Pinhole camera (no DoF)
        ray.origin = (camera.inv_view * vec4<f32>(origin_view, 1.0)).xyz;
        ray.dir = normalize((camera.inv_view * vec4<f32>(dir_view, 0.0)).xyz);
    }
    
    return ray;
}

// ---- Shading ----

// Interpolate smooth normal at hit point.
fn hit_normal(hit: HitInfo) -> vec3<f32> {
    let tri = triangles[hit.tri_idx];
    let w = 1.0 - hit.u - hit.v;
    return normalize(w * tri.n0 + hit.u * tri.n1 + hit.v * tri.n2);
}

// Hit position.
fn hit_pos(ray: Ray, hit: HitInfo) -> vec3<f32> {
    return ray.origin + ray.dir * hit.t;
}

// Get material for hit triangle.
fn hit_material(hit: HitInfo) -> Material {
    let tri = triangles[hit.tri_idx];
    return materials[tri.material_id];
}

// Convert direction to equirectangular UV coordinates.
fn dir_to_equirect_uv(dir: vec3<f32>, rotation: f32) -> vec2<f32> {
    // Spherical coordinates: theta (azimuth), phi (elevation)
    let theta = atan2(dir.z, dir.x);  // -PI to PI
    let phi = asin(clamp(dir.y, -1.0, 1.0));  // -PI/2 to PI/2
    
    // Map to UV: u = theta/(2*PI) + 0.5, v = phi/PI + 0.5
    var u = theta / (2.0 * PI) + 0.5 + rotation / (2.0 * PI);
    let v = 0.5 - phi / PI;  // flip v so top is up
    
    // Wrap u
    u = u - floor(u);
    
    return vec2<f32>(u, v);
}

// Convert equirectangular UV back to direction.
fn equirect_uv_to_dir(uv: vec2<f32>) -> vec3<f32> {
    let theta = (uv.x - 0.5) * 2.0 * PI;  // -PI to PI
    let phi = (0.5 - uv.y) * PI;          // -PI/2 to PI/2
    let cos_phi = cos(phi);
    return vec3<f32>(cos_phi * cos(theta), sin(phi), cos_phi * sin(theta));
}

// Binary search in CDF array. Returns index where cdf[index-1] < xi <= cdf[index].
fn binary_search_cdf(cdf_offset: u32, size: u32, xi: f32) -> u32 {
    var lo = 0u;
    var hi = size;
    while lo < hi {
        let mid = (lo + hi) / 2u;
        if env_conditional_cdf[cdf_offset + mid] < xi {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }
    return min(lo, size - 1u);
}

// Binary search in marginal CDF.
fn binary_search_marginal(size: u32, xi: f32) -> u32 {
    var lo = 0u;
    var hi = size;
    while lo < hi {
        let mid = (lo + hi) / 2u;
        if env_marginal_cdf[mid] < xi {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }
    return min(lo, size - 1u);
}

// Sample environment map direction using importance sampling.
// Returns (direction, pdf).
fn sample_env_direction(r1: f32, r2: f32) -> vec4<f32> {
    let w = env.env_width;
    let h = env.env_height;
    
    // Sample row from marginal CDF
    let y = binary_search_marginal(h, r2);
    
    // Sample column from conditional CDF for that row
    let row_offset = y * w;
    let x = binary_search_cdf(row_offset, w, r1);
    
    // Convert pixel coords to UV (center of pixel)
    let u = (f32(x) + 0.5) / f32(w);
    let v = (f32(y) + 0.5) / f32(h);
    
    // Convert UV to direction (applying rotation)
    let uv_rotated = vec2<f32>(u - env.rotation / (2.0 * PI), v);
    let dir = equirect_uv_to_dir(uv_rotated);
    
    // Calculate PDF
    // PDF = (luminance / total_luminance) * (width * height) / (2 * PI^2 * sin_theta)
    // But we've normalized CDFs, so we use the CDF values directly
    let sin_theta = max(sin(PI * v), EPSILON);
    
    // Get marginal PDF (probability of selecting row y)
    var marginal_pdf: f32;
    if y == 0u {
        marginal_pdf = env_marginal_cdf[0];
    } else {
        marginal_pdf = env_marginal_cdf[y] - env_marginal_cdf[y - 1u];
    }
    
    // Get conditional PDF (probability of selecting column x given row y)
    var conditional_pdf: f32;
    if x == 0u {
        conditional_pdf = env_conditional_cdf[row_offset];
    } else {
        conditional_pdf = env_conditional_cdf[row_offset + x] - env_conditional_cdf[row_offset + x - 1u];
    }
    
    // Combined PDF with Jacobian for equirectangular mapping
    let pdf = max(marginal_pdf * conditional_pdf * f32(w) * f32(h) / (2.0 * PI * PI * sin_theta), EPSILON);
    
    return vec4<f32>(dir, pdf);
}

// Calculate PDF for a given direction under environment importance sampling.
fn env_pdf(dir: vec3<f32>) -> f32 {
    let uv = dir_to_equirect_uv(dir, env.rotation);
    let w = env.env_width;
    let h = env.env_height;
    
    // Find pixel coordinates
    let x = min(u32(uv.x * f32(w)), w - 1u);
    let y = min(u32(uv.y * f32(h)), h - 1u);
    
    // Get marginal PDF
    var marginal_pdf: f32;
    if y == 0u {
        marginal_pdf = env_marginal_cdf[0];
    } else {
        marginal_pdf = env_marginal_cdf[y] - env_marginal_cdf[y - 1u];
    }
    
    // Get conditional PDF
    let row_offset = y * w;
    var conditional_pdf: f32;
    if x == 0u {
        conditional_pdf = env_conditional_cdf[row_offset];
    } else {
        conditional_pdf = env_conditional_cdf[row_offset + x] - env_conditional_cdf[row_offset + x - 1u];
    }
    
    // Jacobian
    let sin_theta = max(sin(PI * uv.y), EPSILON);
    
    return max(marginal_pdf * conditional_pdf * f32(w) * f32(h) / (2.0 * PI * PI * sin_theta), EPSILON);
}

// HDR sky environment: use loaded HDR map or fallback to gradient.
fn sky_color(dir: vec3<f32>) -> vec3<f32> {
    if env.enabled > 0.5 {
        // Sample HDR environment map
        let uv = dir_to_equirect_uv(dir, env.rotation);
        let color = textureSampleLevel(env_map, env_sampler, uv, 0.0).rgb;
        return color * env.intensity;
    } else {
        // Fallback: procedural gradient + sun
        let t = dir.y * 0.5 + 0.5;
        let sky = mix(vec3<f32>(0.7, 0.75, 0.8), vec3<f32>(0.4, 0.6, 1.0), t);
        let sun_dir = normalize(vec3<f32>(0.5, 0.8, 0.3));
        let sun_dot = max(dot(dir, sun_dir), 0.0);
        let sun = pow(sun_dot, 256.0) * vec3<f32>(10.0, 9.0, 7.0);
        let sun_glow = pow(sun_dot, 8.0) * vec3<f32>(0.3, 0.25, 0.15);
        return sky + sun + sun_glow;
    }
}

// ---- Path tracing kernel ----

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(output);
    let w = dims.x;
    let h = dims.y;
    let px = gid.xy;

    if px.x >= w || px.y >= h {
        return;
    }

    let pixel_idx = px.y * w + px.x;

    // Seed RNG from pixel position (used for RR and lobe selection only)
    var rng = pcg_hash(pixel_idx * 1973u + camera.frame_count * 6133u + 1u);
    
    // Per-pixel offset for Cranley-Patterson rotation (decorrelates R2 between pixels)
    let pixel_offset = vec2<f32>(rand(&rng), rand(&rng));
    let frame = camera.frame_count;
    
    // R2 low-discrepancy sequence for all primary samples (better convergence)
    // Dimension 0: AA jitter
    let jitter = get_r2_sample(frame, 0u, pixel_offset);
    let jx = jitter.x;
    let jy = jitter.y;
    var ray = gen_ray(f32(px.x), f32(px.y), vec2<f32>(f32(w), f32(h)), jx, jy, &rng);

    // Path trace with multiple bounces
    var throughput = vec3<f32>(1.0);
    var radiance = vec3<f32>(0.0);
    var transmission_depth = 0u;

    for (var bounce = 0u; bounce <= camera.max_bounces; bounce++) {
        let hit = trace_ray(ray);

        if !hit.hit {
            // Miss: accumulate sky contribution
            radiance += throughput * sky_color(ray.dir);
            break;
        }

        let n = hit_normal(hit);
        let p = hit_pos(ray, hit);
        let mat = hit_material(hit);

        // Ensure normal faces the ray
        var normal = n;
        if dot(normal, ray.dir) > 0.0 {
            normal = -normal;
        }

        // Unpack material fields (Standard Surface)
        let base_color = mat.base_color_weight.rgb;
        let base_weight = mat.base_color_weight.a;
        let spec_color = mat.specular_color_weight.rgb;
        let spec_weight = mat.specular_color_weight.a;
        let transmission_color = mat.transmission_color_weight.rgb;
        let transmission_weight = mat.transmission_color_weight.a;
        let coat_color = mat.coat_color_weight.rgb;
        let coat_weight = mat.coat_color_weight.a;
        let emission = mat.emission_color_weight.rgb * mat.emission_color_weight.a;
        let metallic = mat.params1.y;
        let roughness = mat.params1.z;
        let ior = mat.params1.w;
        let coat_roughness = mat.params2.y;
        let coat_ior = mat.params2.z;

        // Add emission
        radiance += throughput * emission;

        // Russian roulette after first bounce
        if bounce > 0u {
            let p_continue = max(max(throughput.x, throughput.y), throughput.z);
            if rand(&rng) > p_continue {
                break;
            }
            throughput /= p_continue;
        }

        let v_dir = -ray.dir; // view direction (toward camera)
        let ndotv = max(dot(normal, v_dir), EPSILON);
        let basis = onb_from_normal(normal);

        // ============================================================
        // Next Event Estimation (NEE) - Direct sun light sampling with MIS
        // ============================================================
        // Only for diffuse/non-transmission surfaces, and when HDR not loaded
        if transmission_weight < 0.5 && env.enabled < 0.5 {
            // Use R2 sequence for sun disc sampling (dim 3 + bounce*2 = secondary sample)
            let sun_sample = get_r2_sample(frame, 3u + bounce * 2u, pixel_offset);
            let sun_dir_sample = sample_sun_direction(sun_sample.x, sun_sample.y);
            let ndotl_sun = dot(normal, sun_dir_sample);
            
            if ndotl_sun > 0.0 {
                // Shadow ray
                var shadow_ray: Ray;
                shadow_ray.origin = p + normal * 0.001;
                shadow_ray.dir = sun_dir_sample;
                
                if !trace_shadow_ray(shadow_ray, T_MAX) {
                    // Evaluate diffuse BRDF for sun light
                    let diffuse_color = base_color * base_weight * (1.0 - metallic);
                    let f0_dielectric = vec3<f32>(pow((ior - 1.0) / (ior + 1.0), 2.0));
                    let f0_nee = mix(f0_dielectric, base_color, metallic);
                    let f_sun = fresnel_schlick(ndotl_sun, f0_nee);
                    
                    // Lambert diffuse contribution
                    let diffuse_contrib = diffuse_color * (1.0 - f_sun) * ndotl_sun / PI;
                    
                    // GGX specular contribution
                    let alpha = roughness * roughness;
                    let h_sun = normalize(v_dir + sun_dir_sample);
                    let ndoth_sun = max(dot(normal, h_sun), EPSILON);
                    let hdotv_sun = max(dot(h_sun, v_dir), EPSILON);
                    let d_sun = ggx_d(ndoth_sun, alpha);
                    let g_sun = smith_g1(ndotv, alpha) * smith_g1(ndotl_sun, alpha);
                    let f_spec_sun = fresnel_schlick(hdotv_sun, f0_nee);
                    let spec_contrib = spec_weight * f_spec_sun * d_sun * g_sun / (4.0 * ndotv * ndotl_sun + EPSILON);
                    
                    // MIS weight: combine light sampling PDF with BSDF PDF
                    let pdf_light = pdf_sun_disc();
                    // BSDF PDF is weighted average of diffuse and specular PDFs
                    let pdf_diffuse = pdf_cosine_hemisphere(ndotl_sun);
                    let pdf_specular = pdf_ggx(ndoth_sun, hdotv_sun, alpha);
                    // Weight by lobe probabilities (simplified: assume 50/50 for non-metals)
                    let spec_prob = mix(0.5, 1.0, metallic);
                    let pdf_bsdf = mix(pdf_diffuse, pdf_specular, spec_prob);
                    
                    let mis_weight = mis_power_heuristic(pdf_light, pdf_bsdf);
                    
                    let sun_contrib = (diffuse_contrib + spec_contrib) * SUN_COLOR * SUN_INTENSITY * mis_weight;
                    radiance += throughput * sun_contrib;
                }
            }
        }
        
        // ============================================================
        // NEE for HDR environment map with importance sampling
        // ============================================================
        if transmission_weight < 0.5 && env.enabled > 0.5 && env.use_importance_sampling > 0.5 {
            // Sample environment using CDF importance sampling
            let env_sample = get_r2_sample(frame, 3u + bounce * 2u, pixel_offset);
            let env_result = sample_env_direction(env_sample.x, env_sample.y);
            let env_dir = env_result.xyz;
            let env_pdf_light = env_result.w;
            let ndotl_env = dot(normal, env_dir);
            
            if ndotl_env > 0.0 {
                // Shadow ray to check visibility
                var shadow_ray: Ray;
                shadow_ray.origin = p + normal * 0.001;
                shadow_ray.dir = env_dir;
                
                if !trace_shadow_ray(shadow_ray, T_MAX) {
                    // Get environment radiance
                    let env_radiance = sky_color(env_dir);
                    
                    // Evaluate diffuse BRDF
                    let diffuse_color = base_color * base_weight * (1.0 - metallic);
                    let f0_dielectric = vec3<f32>(pow((ior - 1.0) / (ior + 1.0), 2.0));
                    let f0_env = mix(f0_dielectric, base_color, metallic);
                    let f_env = fresnel_schlick(ndotl_env, f0_env);
                    
                    // Lambert diffuse
                    let diffuse_contrib_env = diffuse_color * (1.0 - f_env) * ndotl_env / PI;
                    
                    // GGX specular
                    let alpha = roughness * roughness;
                    let h_env = normalize(v_dir + env_dir);
                    let ndoth_env = max(dot(normal, h_env), EPSILON);
                    let hdotv_env = max(dot(h_env, v_dir), EPSILON);
                    let d_env = ggx_d(ndoth_env, alpha);
                    let g_env = smith_g1(ndotv, alpha) * smith_g1(ndotl_env, alpha);
                    let f_spec_env = fresnel_schlick(hdotv_env, f0_env);
                    let spec_contrib_env = spec_weight * f_spec_env * d_env * g_env / (4.0 * ndotv * ndotl_env + EPSILON);
                    
                    // MIS: combine env sampling PDF with BSDF PDF
                    let pdf_diffuse_env = pdf_cosine_hemisphere(ndotl_env);
                    let pdf_specular_env = pdf_ggx(ndoth_env, hdotv_env, alpha);
                    let spec_prob_env = mix(0.5, 1.0, metallic);
                    let pdf_bsdf_env = mix(pdf_diffuse_env, pdf_specular_env, spec_prob_env);
                    
                    let mis_weight_env = mis_power_heuristic(env_pdf_light, pdf_bsdf_env);
                    
                    let env_contrib = (diffuse_contrib_env + spec_contrib_env) * env_radiance * mis_weight_env / max(env_pdf_light, EPSILON);
                    radiance += throughput * env_contrib;
                }
            }
        }

        // ============================================================
        // Layer 1: Coat (clearcoat on top)
        // ============================================================
        if coat_weight > 0.001 {
            let coat_f0 = vec3<f32>(pow((coat_ior - 1.0) / (coat_ior + 1.0), 2.0));
            let coat_fresnel = fresnel_schlick(ndotv, coat_f0);
            let coat_reflect_prob = coat_weight * (coat_fresnel.x + coat_fresnel.y + coat_fresnel.z) / 3.0;

            if rand(&rng) < coat_reflect_prob {
                // Sample coat GGX using R2 sequence
                let coat_alpha = coat_roughness * coat_roughness;
                let coat_sample = get_r2_sample(frame, 3u + bounce * 2u, pixel_offset);
                let h_local = sample_ggx(coat_sample.x, coat_sample.y, coat_alpha);
                let h_world = normalize(basis * h_local);
                let hdotv = max(dot(h_world, v_dir), EPSILON);
                let reflect_dir = reflect(-v_dir, h_world);
                let ndotl = dot(normal, reflect_dir);

                if ndotl > 0.0 {
                    let ndoth = max(dot(normal, h_world), EPSILON);
                    let f = fresnel_schlick(hdotv, coat_f0);
                    let g = smith_g1(ndotv, coat_alpha) * smith_g1(ndotl, coat_alpha);
                    let weight = f * g * hdotv / (ndotv * ndoth + EPSILON);
                    throughput *= coat_color * weight / coat_reflect_prob;

                    ray.origin = p + normal * 0.001;
                    ray.dir = normalize(reflect_dir);
                    continue;
                }
            }
            // If we didn't reflect off coat, attenuate by (1 - coat_fresnel)
            throughput *= 1.0 - coat_weight * coat_fresnel;
        }

        // ============================================================
        // Layer 2: Specular reflection / Transmission / Diffuse
        // ============================================================
        
        // Metallic workflow: F0 for dielectrics vs metals
        let f0_dielectric = vec3<f32>(pow((ior - 1.0) / (ior + 1.0), 2.0));
        let f0 = mix(f0_dielectric * spec_color, base_color, metallic);
        let alpha = roughness * roughness;

        // Compute Fresnel for lobe selection
        let fresnel_estimate = fresnel_schlick(ndotv, f0);
        let fresnel_avg = (fresnel_estimate.x + fresnel_estimate.y + fresnel_estimate.z) / 3.0;

        // Probability weights for each lobe
        let w_spec = spec_weight * fresnel_avg;
        let w_trans = transmission_weight * (1.0 - fresnel_avg);
        let w_diff = base_weight * (1.0 - metallic) * (1.0 - fresnel_avg);
        let w_total = w_spec + w_trans + w_diff + EPSILON;

        let p_spec = w_spec / w_total;
        let p_trans = w_trans / w_total;
        // p_diff = 1 - p_spec - p_trans

        let lobe_rand = rand(&rng);

        if lobe_rand < p_spec {
            // ---- Specular (GGX) reflection using R2 sequence ----
            let spec_sample = get_r2_sample(frame, 2u + bounce * 2u, pixel_offset);
            let h_local = sample_ggx(spec_sample.x, spec_sample.y, alpha);
            let h_world = normalize(basis * h_local);
            let hdotv = max(dot(h_world, v_dir), EPSILON);
            let reflect_dir = reflect(-v_dir, h_world);
            let ndotl = dot(normal, reflect_dir);

            if ndotl <= 0.0 {
                break;
            }

            let ndoth = max(dot(normal, h_world), EPSILON);
            let f = fresnel_schlick(hdotv, f0);
            let g = smith_g1(ndotv, alpha) * smith_g1(ndotl, alpha);
            let weight = f * g * hdotv / (ndotv * ndoth + EPSILON);

            throughput *= weight / p_spec;
            ray.origin = p + normal * 0.001;
            ray.dir = normalize(reflect_dir);

        } else if lobe_rand < p_spec + p_trans {
            // ---- Transmission (refraction) ----
            // Check transmission depth limit
            if transmission_depth >= camera.max_transmission_depth {
                // Exceeded transmission depth - treat as opaque
                break;
            }
            transmission_depth += 1u;
            
            let eta = select(ior, 1.0 / ior, dot(n, ray.dir) < 0.0);
            
            // Sample microfacet for rough refraction using R2 sequence
            let trans_sample = get_r2_sample(frame, 2u + bounce * 2u, pixel_offset);
            let h_local = sample_ggx(trans_sample.x, trans_sample.y, alpha);
            let h_world = normalize(basis * h_local);
            
            // Compute refracted direction
            let cos_i = dot(v_dir, h_world);
            let sin2_t = eta * eta * (1.0 - cos_i * cos_i);
            
            if sin2_t > 1.0 {
                // Total internal reflection
                let reflect_dir = reflect(-v_dir, h_world);
                throughput *= transmission_color;
                ray.origin = p + normal * 0.001;
                ray.dir = normalize(reflect_dir);
            } else {
                let cos_t = sqrt(1.0 - sin2_t);
                let refract_dir = -eta * v_dir + (eta * cos_i - cos_t) * h_world;
                
                // Fresnel term for transmission
                let f = fresnel_schlick(abs(cos_i), f0);
                let trans_weight = (1.0 - f) * transmission_color;
                
                throughput *= trans_weight / p_trans;
                ray.origin = p - normal * 0.001; // offset into surface
                ray.dir = normalize(refract_dir);
            }

        } else {
            // ---- Diffuse (Lambert) using R2 sequence ----
            let diff_sample = get_r2_sample(frame, 2u + bounce * 2u, pixel_offset);
            let local_dir = cosine_hemisphere(diff_sample.x, diff_sample.y);
            let world_dir = basis * local_dir;

            let diffuse_color = base_color * base_weight * (1.0 - metallic);
            let f_diffuse = fresnel_schlick(max(dot(normal, normalize(world_dir)), 0.0), f0);
            let diff_weight = diffuse_color * (1.0 - f_diffuse);
            
            let p_diff = 1.0 - p_spec - p_trans;
            throughput *= diff_weight / max(p_diff, EPSILON);

            ray.origin = p + normal * 0.001;
            ray.dir = normalize(world_dir);
        }
    }

    // Progressive accumulation: blend new sample with previous frames
    // On first frame (fc=1), ignore prev to clear any NaN/Inf from old frames
    let fc = f32(camera.frame_count);
    var new_color: vec4<f32>;
    if fc <= 1.0 {
        // First frame: just use new radiance, ignore prev (clears NaN/Inf)
        new_color = vec4<f32>(radiance, 1.0);
    } else {
        let prev = accum[pixel_idx];
        new_color = vec4<f32>(
            (prev.rgb * (fc - 1.0) + radiance) / fc,
            1.0,
        );
    }
    
    // Sanitize output - clamp to prevent NaN/Inf propagation
    new_color = clamp(new_color, vec4<f32>(0.0), vec4<f32>(100.0));
    
    accum[pixel_idx] = new_color;
    textureStore(output, vec2<i32>(px), new_color);
}
