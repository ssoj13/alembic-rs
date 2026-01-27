//! Path tracer module — WGSL compute shader based.
//!
//! Provides BVH acceleration structure, GPU buffer serialization,
//! compute pipeline management, and path tracing for the viewer.
//!
//! ## Architecture
//! ```text
//! Scene triangles → BVH build (CPU, SAH) → GPU buffers → Compute shader → Output texture
//! ```

pub mod bvh;
pub mod build;
pub mod gpu_data;
pub mod compute;
pub mod scene_convert;

pub use compute::PathTraceCompute;
pub use compute::PtCameraUniform;
pub use gpu_data::GpuSceneData;
