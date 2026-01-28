// Hover highlight post-process shader
// Reads object ID texture and applies outline and/or tint effects
// Mode: 0=none, 1=outline, 2=tint, 3=both

struct HoverParams {
    hovered_id: u32,           // ID of hovered object (0 = none)
    mode: u32,                 // 0=none, 1=outline, 2=tint, 3=both
    outline_width: f32,        // Outline thickness in pixels
    _pad0: f32,
    outline_color: vec4<f32>,  // Outline color (orange by default)
    tint_color: vec4<f32>,     // Tint overlay color
    viewport_size: vec2<f32>,  // Viewport dimensions
    _pad1: vec2<f32>,
}

@group(0) @binding(0) var id_texture: texture_2d<u32>;
@group(0) @binding(1) var<uniform> params: HoverParams;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle (3 vertices cover entire screen)
@vertex
fn vs_main(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    var out: VertexOutput;
    // Triangle vertices: (-1,-1), (3,-1), (-1,3)
    let x = f32(i32(vertex_idx & 1u) * 4 - 1);
    let y = f32(i32(vertex_idx >> 1u) * 4 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);  // No Y flip - matches blit.wgsl
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Early out if no hover
    if params.hovered_id == 0u || params.mode == 0u {
        return vec4<f32>(0.0);
    }
    
    let pixel = vec2<i32>(in.uv * params.viewport_size);
    let center_id = textureLoad(id_texture, pixel, 0).r;
    
    var result = vec4<f32>(0.0);
    
    // Tint mode: overlay color on hovered object
    let do_tint = (params.mode & 2u) != 0u;
    if do_tint && center_id == params.hovered_id {
        result = params.tint_color;
    }
    
    // Outline mode: detect edges around hovered object
    let do_outline = (params.mode & 1u) != 0u;
    if do_outline {
        let width = i32(params.outline_width);
        var is_outline = false;
        
        // Check if this pixel is at the edge of the hovered object
        // Either: we're on the hovered object and neighbor is not
        // Or: we're not on it but neighbor is (outer glow)
        let on_hovered = center_id == params.hovered_id;
        
        for (var dy = -width; dy <= width; dy = dy + 1) {
            for (var dx = -width; dx <= width; dx = dx + 1) {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let neighbor_pos = pixel + vec2<i32>(dx, dy);
                // Bounds check
                if neighbor_pos.x < 0 || neighbor_pos.y < 0 || 
                   neighbor_pos.x >= i32(params.viewport_size.x) || 
                   neighbor_pos.y >= i32(params.viewport_size.y) {
                    continue;
                }
                let neighbor_id = textureLoad(id_texture, neighbor_pos, 0).r;
                let neighbor_on_hovered = neighbor_id == params.hovered_id;
                
                // Edge: transition between hovered and non-hovered
                if on_hovered != neighbor_on_hovered {
                    is_outline = true;
                    break;
                }
            }
            if is_outline {
                break;
            }
        }
        
        if is_outline {
            // Blend outline over tint
            result = mix(result, params.outline_color, params.outline_color.a);
        }
    }
    
    return result;
}
