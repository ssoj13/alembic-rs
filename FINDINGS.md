# GPU Path Tracer Optimization Analysis

**Date:** 2026-01-28  
**Purpose:** Identify performance bottlenecks and optimization opportunities for static geometry

---

## 1. Current Architecture Summary

### 1.1 BVH Construction (CPU, `build.rs`)
- **Algorithm:** SAH (Surface Area Heuristic) with binned partitioning
- **Bins:** 12 (standard choice)
- **Max leaf size:** 4 triangles
- **Node size:** 32 bytes (cache-line friendly)
- **Build:** Iterative stack-based (avoids recursion stack overflow)

### 1.2 GPU Traversal (`bvh_traverse.wgsl`)
- **Traversal:** Stack-based, MAX_STACK_DEPTH = 32
- **Workgroup:** 8×8 = 64 threads
- **Intersection:** Moller-Trumbore (standard)
- **AABB test:** Slab method

### 1.3 Data Structures
| Structure | Size | Notes |
|-----------|------|-------|
| `BvhNode` | 32 bytes | AABB + left_or_first + count |
| `GpuTriangle` | 112 bytes | 3 vertices + 3 normals + IDs + padding |
| `GpuMaterial` | 144 bytes | Full Standard Surface params |

### 1.4 Path Tracing Features
- Progressive accumulation (frame averaging)
- GGX specular + Lambertian diffuse
- Coat layer (clearcoat)
- Transmission/refraction with TIR
- NEE for sun light only
- HDR environment map sampling
- PCG random number generator

---

## 2. Performance Bottlenecks

### 2.1 🔴 HIGH IMPACT: BVH Traversal Inefficiency

**Problem:** Current traversal pushes both children without checking which is closer.

```wgsl
// Current code (suboptimal):
if sp + 2u <= MAX_STACK_DEPTH {
    stack[sp] = node.left_or_first + 1u;  // right
    sp += 1u;
    stack[sp] = node.left_or_first;       // left  
    sp += 1u;
}
```

**Fix:** Order children by ray direction sign. Visit near child first, far child only if ray could still hit it.

```wgsl
// Better: order by ray direction
let axis = /* dominant axis of AABB extent */;
let dir_sign = ray.dir[axis] > 0.0;
let near_child = select(node.left_or_first + 1u, node.left_or_first, dir_sign);
let far_child = select(node.left_or_first, node.left_or_first + 1u, dir_sign);
// Push far first (so near is popped first)
stack[sp] = far_child; sp += 1u;
stack[sp] = near_child; sp += 1u;
```

**Expected improvement:** 10-30% traversal speedup

### 2.2 🔴 HIGH IMPACT: Triangle Data Size

**Problem:** 112 bytes per triangle is large. Memory bandwidth is often the bottleneck.

**Current layout:**
```rust
pub struct GpuTriangle {
    v0: [f32; 3], material_id: u32,  // 16 bytes
    v1: [f32; 3], object_id: u32,    // 16 bytes
    v2: [f32; 3], _pad1: u32,        // 16 bytes
    n0: [f32; 3], _pad2: u32,        // 16 bytes
    n1: [f32; 3], _pad3: u32,        // 16 bytes
    n2: [f32; 3], _pad4: u32,        // 16 bytes
}  // Total: 96 bytes (actually 112 with padding?)
```

**Optimizations:**
1. **Quantized positions:** Store positions as fixed-point relative to leaf AABB
2. **Compressed normals:** Octahedral encoding (2 floats → 2 bytes each)
3. **Deferred normals:** Store indices, compute normals only on hit

**Compressed layout (48 bytes):**
```rust
pub struct GpuTriangleCompact {
    v0: [f32; 3], material_id: u16, object_id: u16,  // 16 bytes
    e1: [f32; 3], n0_oct: u32,                        // 16 bytes (e1 = v1-v0)
    e2: [f32; 3], n1n2_oct: u32,                      // 16 bytes (e2 = v2-v0)
}  // Total: 48 bytes
```

**Expected improvement:** 2x memory bandwidth → 20-50% traversal speedup

### 2.3 🟡 MEDIUM IMPACT: No Ray Compaction

**Problem:** Terminated rays waste compute. Each bounce, some rays exit the scene.

**Solution:** Between bounces:
1. Compact active rays to front of buffer
2. Only dispatch work for active rays
3. Use atomic counter for compaction

**Implementation complexity:** Requires wavefront architecture (multiple dispatch calls).

### 2.4 🟡 MEDIUM IMPACT: Megakernel vs Wavefront

**Problem:** Single monolithic shader with divergent control flow.

**Wavefront architecture:**
1. **Generate:** Create primary rays
2. **Extend:** Traverse BVH, find intersections
3. **Shade:** Evaluate materials, generate shadow/bounce rays
4. **Connect:** Shadow ray visibility tests

**Benefits:**
- Material-coherent shading (sort rays by material)
- Separate optimization per kernel
- Better occupancy

**Drawback:** More complex, multiple dispatch calls, intermediate buffers.

### 2.5 🟡 MEDIUM IMPACT: Stackless Traversal

**Problem:** Stack uses register space. 32 × u32 = 128 bytes of registers per thread.

**Alternative: Restart trail**
- Encode path through tree as bits
- No stack needed
- Slightly more traversal but less register pressure

**Alternative: Short stack + restart**
- Small stack (4-8 entries)
- On overflow, restart from root with better heuristic

### 2.6 🟢 LOWER IMPACT: Better Sampling

**Current:** PCG random, cosine hemisphere, GGX importance sampling.

**Improvements:**
1. **Blue noise:** Sobol/Halton sequences for faster convergence
2. **Env map importance sampling:** Build CDF, sample bright regions
3. **MIS:** Multiple importance sampling for NEE

---

## 3. Optimization Roadmap

### Phase 1: Quick Wins (No Architecture Change)

| Optimization | Effort | Impact | Status |
|--------------|--------|--------|--------|
| Child ordering by ray direction | Low | High | ✅ Done |
| Early termination in AABB test | Low | Medium | 🔲 TODO |
| Increase SAH bins to 32 | Low | Low | 🔲 TODO |
| Tune workgroup size (16×16?) | Low | Low | 🔲 TODO |

### Phase 2: Data Structure Optimization

| Optimization | Effort | Impact | Status |
|--------------|--------|--------|--------|
| Triangle compression (48 bytes) | Medium | High | 🔲 TODO |
| Octahedral normal encoding | Medium | Medium | 🔲 TODO |
| Separate vertex/normal buffers | Low | Medium | 🔲 TODO |

### Phase 3: Advanced Algorithms

| Optimization | Effort | Impact | Status |
|--------------|--------|--------|--------|
| Wavefront path tracing | High | High | 🔲 TODO |
| Ray compaction | Medium | High | 🔲 TODO |
| Stackless traversal | Medium | Medium | 🔲 TODO |
| LBVH GPU construction | High | High (dynamic) | 🔲 TODO |

### Phase 4: Sampling Quality

| Optimization | Effort | Impact | Status |
|--------------|--------|--------|--------|
| Blue noise (Sobol) | Medium | Medium | ✅ R2 Done |
| Env importance sampling | Medium | Medium | ✅ Done |
| MIS for NEE | Low | Low | ✅ Done |

---

## 4. Immediate Action Items

### 4.1 Child Ordering Fix (Highest Priority)

Modify `trace_ray()` in `bvh_traverse.wgsl`:

```wgsl
// Store axis hint in BVH node (reuse a padding bit or store in build)
// For now, use longest axis of AABB

fn trace_ray(ray: Ray) -> HitInfo {
    // ... existing code ...
    
    } else {
        // Internal: push children with ordering
        if sp + 2u <= MAX_STACK_DEPTH {
            let left_child = node.left_or_first;
            let right_child = node.left_or_first + 1u;
            
            // Determine axis (could be stored in node)
            let extent = node.aabb_max - node.aabb_min;
            var axis: u32 = 0u;
            if extent.y > extent.x && extent.y > extent.z { axis = 1u; }
            if extent.z > extent.x && extent.z > extent.y { axis = 2u; }
            
            // Order by ray direction
            let dir_positive = ray.dir[axis] > 0.0;
            let first = select(right_child, left_child, dir_positive);
            let second = select(left_child, right_child, dir_positive);
            
            // Push far child first (so near is processed first)
            stack[sp] = second; sp += 1u;
            stack[sp] = first; sp += 1u;
        }
    }
```

**Note:** WGSL doesn't allow indexing vec3 with runtime variable. Need workaround:

```wgsl
fn get_axis(v: vec3<f32>, axis: u32) -> f32 {
    switch axis {
        case 0u: { return v.x; }
        case 1u: { return v.y; }
        default: { return v.z; }
    }
}
```

### 4.2 Store Split Axis in BVH Node

Modify `BvhNode` to include split axis:

```rust
pub struct BvhNode {
    pub aabb_min: [f32; 3],
    pub left_or_first: u32,
    pub aabb_max: [f32; 3],
    pub count_and_axis: u32,  // low 30 bits = count, high 2 bits = axis
}
```

This avoids recomputing axis in shader.

---

## 5. Benchmarking Notes

Need to measure before/after for each optimization:
- **Metric:** Rays per second (width × height × samples / time)
- **Test scenes:** 
  - Simple (floor + few objects)
  - Complex (BMW, chess)
  - Dense (millions of triangles)

---

## 6. References

- [Efficient BVH Traversal on GPUs](https://research.nvidia.com/publication/understanding-efficiency-ray-traversal-gpus)
- [Wavefront Path Tracing](https://research.nvidia.com/publication/megakernels-considered-harmful-wavefront-path-tracing-gpus)
- [Octahedral Normal Encoding](https://jcgt.org/published/0003/02/01/)
- [LBVH Construction](https://research.nvidia.com/publication/fast-bvh-construction-gpus)

---

## 7. Advanced Sampling Techniques (Modern Renderers)

Современные рендеры используют продвинутые техники сэмплинга для радикального уменьшения шума.

### 7.1 🔥 ReSTIR (Reservoir-based Spatiotemporal Importance Resampling)

**Прорыв NVIDIA 2020 года.** Используется в Cyberpunk 2077, Alan Wake 2, Path of Exile 2.

**Как работает:**
- Каждый пиксель хранит "резервуар" (reservoir) с несколькими световыми сэмплами
- Weighted Importance Sampling (WIS) выбирает лучшие сэмплы
- **Spatial reuse:** Соседние пиксели делятся сэмплами (multiplier ~25x)
- **Temporal reuse:** Сэмплы переиспользуются между кадрами (multiplier ~10x)
- **Результат:** 100-1000x меньше шума для сцен с множеством источников света

**Варианты:**
| Variant | Description | Complexity | Impact |
|---------|-------------|------------|--------|
| ReSTIR DI | Direct Illumination | Medium | Very High |
| ReSTIR GI | Global Illumination | High | Extreme |
| ReSTIR PT | Full Path Tracing | Very High | Extreme |

**Реализация ReSTIR DI (базовая):**
```wgsl
struct Reservoir {
    sample: LightSample,  // selected sample
    w_sum: f32,           // sum of weights
    M: u32,               // number of candidates seen
    W: f32,               // final weight
};

fn update_reservoir(r: ptr<function, Reservoir>, sample: LightSample, w: f32, rng: ptr<function, u32>) {
    (*r).w_sum += w;
    (*r).M += 1u;
    if rand(rng) < w / (*r).w_sum {
        (*r).sample = sample;
    }
}
```

### 7.2 Blue Noise / Low-Discrepancy Sequences

**Проблема:** PCG/white noise создаёт кластеры сэмплов, что замедляет сходимость.

**Решения:**

| Sequence | Quality | Speed | Notes |
|----------|---------|-------|-------|
| Sobol | Excellent | Medium | Needs Owen scrambling |
| Halton | Good | Fast | Simple but correlation issues |
| R2 | Very Good | Very Fast | Simple formula, recommended |
| Blue Noise Textures | Excellent | Very Fast | Precomputed, spatially varied |

**R2 Sequence (простейшая реализация):**
```wgsl
const PLASTIC: f32 = 1.32471795724; // plastic constant
const A1: f32 = 1.0 / PLASTIC;
const A2: f32 = 1.0 / (PLASTIC * PLASTIC);

fn r2_sample(n: u32) -> vec2<f32> {
    return fract(vec2<f32>(0.5) + vec2<f32>(f32(n)) * vec2<f32>(A1, A2));
}
```

**Blue Noise Texture:**
- Предвычисленная 128×128 текстура с blue noise
- Каждый пиксель читает по своим координатам + frame offset
- Даёт spatially-varied низкодискрепантный шум

### 7.3 Path Guiding

**Идея:** Запоминать, откуда приходит свет, и направлять сэмплинг в важные направления.

**Реализации:**
- **SD-Tree (NVIDIA PPG):** Пространственное дерево + directional tree per voxel
- **VMM (Von Mises-Fisher Mixture Model):** Направленные гауссианы
- **Neural (NVIDIA NRCG):** Нейросеть предсказывает распределение

**Сложность:** Требует много памяти и вычислений. Подходит для offline рендеринга.

### 7.4 Environment Map Importance Sampling

**Текущее состояние:** Семплим env map uniform → много шума на ярких пикселях.

**Улучшение:**
1. Build 2D CDF (cumulative distribution function) из luminance
2. Sample CDF для выбора направления
3. PDF = luminance / total_luminance

```wgsl
// Precompute: row CDFs + marginal CDF
// At runtime:
fn sample_env_importance(r1: f32, r2: f32) -> vec3<f32> {
    // Binary search in marginal CDF → v
    // Binary search in row CDF[v] → u
    // Return direction from (u, v)
}
```

### 7.5 Multiple Importance Sampling (MIS)

**Текущее состояние:** NEE для солнца, но без MIS.

**Улучшение:** Комбинировать BSDF sampling и light sampling с balance heuristic:

```wgsl
fn mis_weight(pdf_a: f32, pdf_b: f32) -> f32 {
    return pdf_a / (pdf_a + pdf_b);  // balance heuristic
}

// Or power heuristic (beta=2):
fn mis_power(pdf_a: f32, pdf_b: f32) -> f32 {
    let a2 = pdf_a * pdf_a;
    let b2 = pdf_b * pdf_b;
    return a2 / (a2 + b2);
}
```

### 7.6 Adaptive Sampling

**Идея:** Семплить больше там, где высокая variance.

**Реализация:**
1. Compute per-pixel variance (running estimate)
2. Allocate more samples to high-variance pixels
3. Stop sampling converged pixels early

---

## 8. Recommended Implementation Order for Advanced Sampling

| Priority | Technique | Effort | Impact | Dependencies |
|----------|-----------|--------|--------|-------------|
| 1 | R2/Sobol sequences | Low | Medium | ✅ Done |
| 2 | Env importance sampling | Medium | High | ✅ Done |
| 3 | MIS for NEE | Low | Medium | ✅ Done |
| 4 | Blue noise textures | Low | Medium | Precomputed texture |
| 5 | ReSTIR DI | High | Very High | Screen-space buffers |
| 6 | ReSTIR GI | Very High | Extreme | ReSTIR DI, radiance cache |
| 7 | Path guiding | Very High | High | SD-tree or neural net |

---

## 9. Session Log

| Time | Action | Result |
|------|--------|--------|
| 2026-01-28 | Initial analysis | Identified 6 major bottlenecks |
| 2026-01-28 | Code review | BVH traversal lacks child ordering |
| 2026-01-28 | Created optimization roadmap | 4 phases, prioritized |
| 2026-01-28 | Added advanced sampling section | ReSTIR, Blue Noise, Path Guiding, Env IS |
| 2026-01-28 | User question | Requested info on modern renderer sampling |
| 2026-01-28 | Implemented R2 sequence | AA jitter now uses low-discrepancy R2 |
| 2026-01-28 | Implemented BVH child ordering | Near-far ordering by ray direction |
| 2026-01-28 | Added env CDF generation | CPU-side importance sampling CDFs ready |
| 2026-01-28 | Implemented MIS for NEE | Power heuristic for sun light sampling |
| 2026-01-28 | GPU env importance sampling | Full integration: CDF bindings, binary search, NEE |

