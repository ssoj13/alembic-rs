#!/usr/bin/env python3
"""Basic example: Writing an Alembic file.

This example demonstrates how to:
- Create a new Alembic archive
- Set archive metadata
- Create geometry (PolyMesh)
- Write with transforms (Xform)
"""

import alembic_rs
from alembic_rs import OArchive


def create_cube():
    """Create a simple cube mesh."""
    # Cube vertices
    positions = [
        [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5],  # back
        [-0.5, -0.5,  0.5], [0.5, -0.5,  0.5], [0.5, 0.5,  0.5], [-0.5, 0.5,  0.5],  # front
    ]
    
    # 6 faces, 4 vertices each
    face_counts = [4, 4, 4, 4, 4, 4]
    
    # Face vertex indices (counter-clockwise winding)
    face_indices = [
        0, 3, 2, 1,  # back
        4, 5, 6, 7,  # front
        0, 1, 5, 4,  # bottom
        2, 3, 7, 6,  # top
        0, 4, 7, 3,  # left
        1, 2, 6, 5,  # right
    ]
    
    return positions, face_counts, face_indices


def write_simple_mesh(output_path: str) -> None:
    """Write a simple mesh to an Alembic file."""
    
    # Create the archive
    archive = OArchive.create(output_path)
    
    # Set metadata
    archive.setAppName("alembic_rs Python Example")
    archive.setDescription("A simple cube mesh")
    
    # Create a PolyMesh
    positions, face_counts, face_indices = create_cube()
    
    mesh = alembic_rs.Abc.OPolyMesh("cube")
    mesh.addSample(positions, face_counts, face_indices)
    
    # Write directly
    archive.writePolyMesh(mesh)
    archive.close()
    
    print(f"Written: {output_path}")


def write_with_transform(output_path: str) -> None:
    """Write a mesh with transform hierarchy."""
    
    archive = OArchive.create(output_path)
    archive.setAppName("alembic_rs Python Example")
    
    # Create transform
    xform = alembic_rs.Abc.OXform("cube_xform")
    xform.addTranslationSample(0.0, 1.0, 0.0)  # Move up by 1 unit
    
    # Create mesh under transform
    positions, face_counts, face_indices = create_cube()
    mesh = alembic_rs.Abc.OPolyMesh("cube")
    mesh.addSample(positions, face_counts, face_indices)
    
    # Add mesh as child of transform
    xform.addPolyMesh(mesh)
    
    # Write
    archive.writeXform(xform)
    archive.close()
    
    print(f"Written: {output_path}")


def write_with_normals_uvs(output_path: str) -> None:
    """Write a mesh with normals and UVs."""
    
    archive = OArchive.create(output_path)
    archive.setAppName("alembic_rs Python Example")
    
    # Simple quad
    positions = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ]
    
    face_counts = [4]
    face_indices = [0, 1, 2, 3]
    
    # Per-vertex normals (all pointing +Z)
    normals = [
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ]
    
    # UV coordinates
    uvs = [
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ]
    
    mesh = alembic_rs.Abc.OPolyMesh("quad")
    mesh.addSample(positions, face_counts, face_indices, normals=normals, uvs=uvs)
    
    archive.writePolyMesh(mesh)
    archive.close()
    
    print(f"Written: {output_path}")


if __name__ == "__main__":
    import os
    
    # Create output directory
    output_dir = "output"
    os.makedirs(output_dir, exist_ok=True)
    
    # Write examples
    write_simple_mesh(os.path.join(output_dir, "simple_cube.abc"))
    write_with_transform(os.path.join(output_dir, "cube_with_transform.abc"))
    write_with_normals_uvs(os.path.join(output_dir, "quad_with_uvs.abc"))
    
    print("\nAll examples written successfully!")
