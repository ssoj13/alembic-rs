# План оптимизации координатной системы

## Текущая проблема

```
NDC (-1,+1) ──→ ndc_to_uv() ──→ UV (0,1) ──→ uv_to_ndc() ──→ NDC (-1,+1)
     ↑                              │                              │
  vertex                      textureSample              inv_view_proj
```

**Двойное преобразование = лишние вычисления + путаница**

## Анализ: что где нужно

| Шейдер      | textureSample (UV) | Реконструкция (NDC) |
|-------------|-------------------|---------------------|
| SSAO        | 5+ раз            | 5+ раз              |
| SSAO Blur   | 5 раз             | НЕТ                 |
| Lighting    | 4+ раз            | 1 раз (background)  |

## Решение: передавать ОБА значения

```wgsl
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,   // для textureSample
    @location(1) ndc: vec2<f32>,  // для реконструкции
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0)
    );
    let p = positions[index];
    var out: VsOut;
    out.pos = vec4(p, 0.0, 1.0);
    out.ndc = p;                                    // NDC напрямую
    out.uv = vec2(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5); // UV с Y-flip
    return out;
}
```

## Результат

```
Vertex (3 вызова):     pos → ndc (копия)
                       pos → uv (одно преобразование)

Fragment (миллионы):   in.uv  → textureSample  (готово)
                       in.ndc → inv_view_proj  (готово)
```

**Убираем**:
- ❌ `ndc_to_uv()` в fragment shader
- ❌ `uv_to_ndc()` в fragment shader
- ❌ Любые вычисления координат в fragment shader

**Цена**: +8 байт на вершину (ничтожно, интерполяция бесплатна)

## Изменения в коде

### 1. shaders.rs - общий блок

```wgsl
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) ndc: vec2<f32>,
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    let p = positions[index];
    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.ndc = p;
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return out;
}
```

### 2. SSAO - reconstruct_view_pos

```wgsl
// БЫЛО:
fn reconstruct_view_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc_xy = uv_to_ndc(uv);
    let ndc = vec4<f32>(ndc_xy.x, ndc_xy.y, depth, 1.0);
    ...
}

// СТАЛО:
fn reconstruct_view_pos(ndc_xy: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(ndc_xy, depth, 1.0);
    ...
}

// Вызов: reconstruct_view_pos(in.ndc, depth)
```

### 3. Lighting - sample_background

```wgsl
// БЫЛО:
fn sample_background(uv: vec2<f32>) -> vec4<f32> {
    let ndc_xy = uv_to_ndc(uv);
    let ndc = vec4<f32>(ndc_xy.x, ndc_xy.y, 1.0, 1.0);
    ...
}

// СТАЛО:
fn sample_background(ndc_xy: vec2<f32>) -> vec4<f32> {
    let ndc = vec4<f32>(ndc_xy, 1.0, 1.0);
    ...
}

// Вызов: sample_background(in.ndc)
```

## Чеклист

- [ ] Обновить VsOut: добавить ndc
- [ ] Обновить vs_fullscreen: вычислять оба
- [ ] SSAO: reconstruct_view_pos принимает ndc напрямую
- [ ] SSAO: вызовы с in.ndc
- [ ] Lighting: sample_background принимает ndc напрямую  
- [ ] Lighting: вызов с in.ndc
- [ ] Удалить ndc_to_uv() и uv_to_ndc()
- [ ] Билд и тест

## Бонус: SSAO сэмплирование соседей

Сейчас SSAO сэмплирует соседей по UV offset:
```wgsl
let sample_depth = textureSample(depth_tex, samp, uv + duv);
let sample_pos = reconstruct_view_pos(uv + duv, sample_depth);
```

Проблема: нужно и UV+offset для текстуры, и NDC+offset для реконструкции.

Решение: offset в UV, конвертировать в NDC только для реконструкции:
```wgsl
let sample_uv = in.uv + duv;
let sample_ndc = vec2(sample_uv.x * 2.0 - 1.0, 1.0 - sample_uv.y * 2.0);
let sample_depth = textureSample(depth_tex, samp, sample_uv);
let sample_pos = reconstruct_view_pos(sample_ndc, sample_depth);
```

Или добавить inline helper только для этого случая.
