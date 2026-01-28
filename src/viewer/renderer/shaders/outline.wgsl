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

// Sample object ID with bounds checking
fn sample_id(pos: vec2<i32>) -> u32 {
    if pos.x < 0 || pos.y < 0 || 
       pos.x >= i32(params.viewport_size.x) || 
       pos.y >= i32(params.viewport_size.y) {
        return 0u;
    }
    return textureLoad(id_texture, pos, 0).r;
}

// Check if pixel is on hovered object
fn is_hovered(pos: vec2<i32>) -> f32 {
    if sample_id(pos) == params.hovered_id {
        return 1.0;
    }
    return 0.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Early out if no hover
    if params.hovered_id == 0u || params.mode == 0u {
        return vec4<f32>(0.0);
    }
    
    let pixel = vec2<i32>(in.uv * params.viewport_size);
    let center_id = sample_id(pixel);
    
    var result = vec4<f32>(0.0);
    
    // Tint mode: overlay color on hovered object
    let do_tint = (params.mode & 2u) != 0u;
    if do_tint && center_id == params.hovered_id {
        result = params.tint_color;
    }
    
    // Outline mode: antialiased using SDF-like approach
    let do_outline = (params.mode & 1u) != 0u;
    if do_outline {
        let width = params.outline_width;
        
        // Sample a grid of neighbors and compute coverage-based SDF
        // Using a larger kernel with distance weighting for smooth edges
        let search_radius = i32(ceil(width)) + 1;
        
        var weighted_sum = 0.0;
        var weight_total = 0.0;
        
        // Compute weighted average of "inside" samples
        // This approximates a signed distance field
        for (var dy = -search_radius; dy <= search_radius; dy = dy + 1) {
            for (var dx = -search_radius; dx <= search_radius; dx = dx + 1) {
                let offset = vec2<f32>(f32(dx), f32(dy));
                let dist = length(offset);
                
                // Only consider samples within our search radius
                if dist <= f32(search_radius) {
                    let neighbor_pos = pixel + vec2<i32>(dx, dy);
                    let is_inside = is_hovered(neighbor_pos);
                    
                    // Weight by gaussian-like falloff for smoothness
                    let sigma = width * 0.5 + 0.5;
                    let weight = exp(-dist * dist / (2.0 * sigma * sigma));
                    
                    weighted_sum += is_inside * weight;
                    weight_total += weight;
                }
            }
        }
        
        // Normalize to get coverage (0 = outside, 1 = inside)
        let coverage = weighted_sum / max(weight_total, 0.001);
        
        // Outline appears at the edge where coverage transitions
        // Map coverage to outline intensity
        // Edge is where coverage is around 0.5
        // We want outline where coverage is between ~0.1 and ~0.9
        
        // Compute edge intensity - peaks at coverage = 0.5 (the edge)
        let edge_sharpness = 2.0 / width;  // Sharper for thinner outlines
        let edge_center = 0.5;
        let edge_dist = abs(coverage - edge_center);
        
        // Outline alpha based on how close we are to the edge
        // Thicker outline = wider band around the edge
        let outline_band = 0.5 * (1.0 - 1.0 / (width + 1.0));
        let outline_alpha = smoothstep(outline_band, 0.0, edge_dist) * params.outline_color.a;
        
        if outline_alpha > 0.01 {
            result = mix(result, vec4<f32>(params.outline_color.rgb, 1.0), outline_alpha);
        }
    }
    
    return result;
}
