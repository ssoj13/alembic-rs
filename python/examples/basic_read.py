#!/usr/bin/env python3
"""Basic example: Reading an Alembic file.

This example demonstrates how to:
- Open an Alembic archive
- Navigate the object hierarchy
- Read geometry data (positions, faces)
- Access metadata
"""

from alembic_rs import IArchive


def read_archive(path: str) -> None:
    """Read and print information from an Alembic file."""
    
    # Open the archive
    archive = IArchive(path)
    
    # Print archive info
    print(f"Archive: {archive.getName()}")
    print(f"Version: {archive.getArchiveVersionString()}")
    
    # Print metadata if available
    if archive.getAppName():
        print(f"Application: {archive.getAppName()}")
    if archive.getDateWritten():
        print(f"Date written: {archive.getDateWritten()}")
    if archive.getUserDescription():
        print(f"Description: {archive.getUserDescription()}")
    
    # Get the top-level object
    top = archive.getTop()
    print(f"\nRoot object: {top.getFullName()}")
    print(f"Number of children: {top.getNumChildren()}")
    
    # Recursively print hierarchy
    print("\nObject hierarchy:")
    print_hierarchy(top, indent=0)


def print_hierarchy(obj, indent: int = 0) -> None:
    """Recursively print object hierarchy."""
    prefix = "  " * indent
    
    # Get object type
    obj_type = "Object"
    if obj.isPolyMesh():
        obj_type = "PolyMesh"
    elif obj.isXform():
        obj_type = "Xform"
    elif obj.isCamera():
        obj_type = "Camera"
    elif obj.isCurves():
        obj_type = "Curves"
    elif obj.isPoints():
        obj_type = "Points"
    elif obj.isSubD():
        obj_type = "SubD"
    elif obj.isLight():
        obj_type = "Light"
    
    print(f"{prefix}- {obj.getName()} [{obj_type}]")
    
    # Print mesh info
    if obj.isPolyMesh():
        print_mesh_info(obj, indent + 1)
    elif obj.isXform():
        print_xform_info(obj, indent + 1)
    
    # Recurse into children
    for i in range(obj.getNumChildren()):
        child = obj.getChild(i)
        print_hierarchy(child, indent + 1)


def print_mesh_info(obj, indent: int) -> None:
    """Print PolyMesh information."""
    prefix = "  " * indent
    
    num_samples = obj.getNumSamples()
    print(f"{prefix}Samples: {num_samples}")
    
    # Read first sample
    try:
        sample = obj.getPolyMeshSample(0)
        print(f"{prefix}Vertices: {sample.getNumVertices()}")
        print(f"{prefix}Faces: {sample.getNumFaces()}")
        
        # Print first few positions
        positions = sample.positions
        if len(positions) > 0:
            print(f"{prefix}First position: {positions[0]}")
        
        # Check for optional data
        if sample.normals:
            print(f"{prefix}Has normals: {len(sample.normals)} vectors")
        if sample.uvs:
            print(f"{prefix}Has UVs: {len(sample.uvs)} coords")
        if sample.selfBounds:
            bounds = sample.selfBounds
            print(f"{prefix}Bounds: min={bounds[0]}, max={bounds[1]}")
    except Exception as e:
        print(f"{prefix}Error reading sample: {e}")


def print_xform_info(obj, indent: int) -> None:
    """Print Xform information."""
    prefix = "  " * indent
    
    try:
        sample = obj.getXformSample(0)
        trans = sample.getTranslation()
        scale = sample.getScale()
        print(f"{prefix}Translation: [{trans[0]:.2f}, {trans[1]:.2f}, {trans[2]:.2f}]")
        print(f"{prefix}Scale: [{scale[0]:.2f}, {scale[1]:.2f}, {scale[2]:.2f}]")
    except Exception as e:
        print(f"{prefix}Error reading xform: {e}")


if __name__ == "__main__":
    import sys
    
    if len(sys.argv) < 2:
        print("Usage: python basic_read.py <path_to_abc_file>")
        print("\nExample: python basic_read.py cube.abc")
        sys.exit(1)
    
    read_archive(sys.argv[1])
