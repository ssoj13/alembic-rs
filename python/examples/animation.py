#!/usr/bin/env python3
"""Animation example: Working with animated Alembic data.

This example demonstrates how to:
- Create time sampling for animation
- Write animated transforms
- Write animated geometry
- Read animated data at different times
"""

import math
import alembic_rs
from alembic_rs import IArchive, OArchive


def write_animated_transform(output_path: str, fps: float = 24.0, num_frames: int = 48) -> None:
    """Write an animated spinning transform."""
    
    archive = OArchive.create(output_path)
    archive.setAppName("alembic_rs Animation Example")
    archive.setDccFps(fps)
    
    # Add uniform time sampling (24 fps)
    ts_index = archive.addUniformTimeSampling(fps, start_time=0.0)
    
    # Create spinning transform
    xform = alembic_rs.Abc.OXform("spinner")
    
    # Add rotation samples for 2 seconds (48 frames at 24fps)
    for frame in range(num_frames):
        angle = (frame / num_frames) * 2 * math.pi  # Full rotation
        
        # Create rotation matrix around Y axis
        cos_a = math.cos(angle)
        sin_a = math.sin(angle)
        
        matrix = [
            [cos_a, 0.0, sin_a, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [-sin_a, 0.0, cos_a, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
        
        xform.addMatrixSample(matrix, inherits=True)
    
    # Add a cube under the transform
    mesh = alembic_rs.Abc.OPolyMesh("cube")
    positions = [
        [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5],
        [-0.5, -0.5,  0.5], [0.5, -0.5,  0.5], [0.5, 0.5,  0.5], [-0.5, 0.5,  0.5],
    ]
    face_counts = [4, 4, 4, 4, 4, 4]
    face_indices = [
        0, 3, 2, 1, 4, 5, 6, 7, 0, 1, 5, 4,
        2, 3, 7, 6, 0, 4, 7, 3, 1, 2, 6, 5,
    ]
    mesh.addSample(positions, face_counts, face_indices)
    
    xform.addPolyMesh(mesh)
    archive.writeXform(xform)
    archive.close()
    
    print(f"Written animated transform: {output_path}")
    print(f"  {num_frames} frames at {fps} fps")


def write_animated_mesh(output_path: str, fps: float = 24.0, num_frames: int = 24) -> None:
    """Write a deforming mesh (bouncing ball)."""
    
    archive = OArchive.create(output_path)
    archive.setAppName("alembic_rs Animation Example")
    
    # Add time sampling
    ts_index = archive.addUniformTimeSampling(fps, start_time=0.0)
    
    # Create deforming sphere
    mesh = alembic_rs.Abc.OPolyMesh("ball")
    
    # Generate sphere topology (simplified)
    def make_sphere(radius: float, squash: float = 1.0, y_offset: float = 0.0):
        """Create a simple UV sphere."""
        positions = []
        segments = 8
        rings = 6
        
        for ring in range(rings + 1):
            phi = math.pi * ring / rings
            for seg in range(segments):
                theta = 2 * math.pi * seg / segments
                
                x = radius * math.sin(phi) * math.cos(theta)
                y = radius * math.cos(phi) * squash + y_offset
                z = radius * math.sin(phi) * math.sin(theta)
                
                positions.append([x, y, z])
        
        return positions
    
    # Topology (shared across all frames)
    segments = 8
    rings = 6
    face_counts = []
    face_indices = []
    
    for ring in range(rings):
        for seg in range(segments):
            next_seg = (seg + 1) % segments
            
            i0 = ring * segments + seg
            i1 = ring * segments + next_seg
            i2 = (ring + 1) * segments + next_seg
            i3 = (ring + 1) * segments + seg
            
            face_counts.append(4)
            face_indices.extend([i0, i1, i2, i3])
    
    # Animate bouncing ball
    for frame in range(num_frames):
        t = frame / num_frames
        
        # Bounce animation
        bounce = abs(math.sin(t * 2 * math.pi))
        y_pos = bounce * 2.0  # Height
        squash = 1.0 - 0.3 * (1.0 - bounce)  # Squash at ground
        
        positions = make_sphere(0.5, squash, y_pos + 0.5)
        mesh.addSample(positions, face_counts, face_indices)
    
    archive.writePolyMesh(mesh)
    archive.close()
    
    print(f"Written animated mesh: {output_path}")


def read_animated_data(path: str) -> None:
    """Read and print animated data from an Alembic file."""
    
    archive = IArchive(path)
    
    print(f"\nReading: {archive.getName()}")
    print(f"Time samplings: {archive.getNumTimeSamplings()}")
    
    # Get time sampling info
    for i in range(archive.getNumTimeSamplings()):
        ts = archive.getTimeSampling(i)
        if ts:
            print(f"  TimeSampling {i}: {ts}")
    
    # Find animated objects
    top = archive.getTop()
    find_animated_objects(top)


def find_animated_objects(obj, path: str = "") -> None:
    """Find and report animated objects."""
    current_path = f"{path}/{obj.getName()}"
    
    num_samples = obj.getNumSamples()
    if num_samples > 1:
        print(f"\n{current_path}: {num_samples} samples")
        
        if obj.isXform():
            # Print first and last xform
            sample0 = obj.getXformSample(0)
            sample_last = obj.getXformSample(num_samples - 1)
            
            t0 = sample0.getTranslation()
            t1 = sample_last.getTranslation()
            
            print(f"  Frame 0 translation: [{t0[0]:.2f}, {t0[1]:.2f}, {t0[2]:.2f}]")
            print(f"  Frame {num_samples-1} translation: [{t1[0]:.2f}, {t1[1]:.2f}, {t1[2]:.2f}]")
        
        elif obj.isPolyMesh():
            sample0 = obj.getPolyMeshSample(0)
            sample_last = obj.getPolyMeshSample(num_samples - 1)
            
            print(f"  Vertices: {len(sample0.positions)}")
            print(f"  First vertex frame 0: {sample0.positions[0]}")
            print(f"  First vertex frame {num_samples-1}: {sample_last.positions[0]}")
    
    # Recurse
    for i in range(obj.getNumChildren()):
        find_animated_objects(obj.getChild(i), current_path)


if __name__ == "__main__":
    import os
    
    output_dir = "output"
    os.makedirs(output_dir, exist_ok=True)
    
    # Write animated examples
    anim_xform_path = os.path.join(output_dir, "animated_transform.abc")
    anim_mesh_path = os.path.join(output_dir, "animated_mesh.abc")
    
    write_animated_transform(anim_xform_path)
    write_animated_mesh(anim_mesh_path)
    
    # Read back
    read_animated_data(anim_xform_path)
    read_animated_data(anim_mesh_path)
