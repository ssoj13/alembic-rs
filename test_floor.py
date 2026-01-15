#!/usr/bin/env python3
"""Debug floor.abc mesh data."""
import alembic_rs as abc

archive = abc.IArchive("data/Abc/floor.abc")
root = archive.getTop()

def find_mesh(obj, depth=0):
    """Find PolyMesh in hierarchy."""
    name = obj.getName()
    full = obj.getFullName()
    print(f"{'  '*depth}[{obj.matchesSchema('AbcGeom_PolyMesh_v1') and 'MESH' or 'obj'}] {name} ({full})")
    
    if obj.matchesSchema("AbcGeom_PolyMesh_v1"):
        mesh = abc.IPolyMesh(obj)
        sample = mesh.getSample(0)
        
        print(f"\n=== Mesh Data ===")
        print(f"Positions: {len(sample.positions)}")
        print(f"Face counts: {len(sample.faceCounts)} = {sample.faceCounts}")
        print(f"Face indices: {len(sample.faceIndices)}")
        
        # Check indices validity
        max_idx = max(sample.faceIndices)
        min_idx = min(sample.faceIndices)
        print(f"Index range: {min_idx} to {max_idx} (valid: 0-{len(sample.positions)-1})")
        
        # Print first few positions
        print(f"\nFirst 10 positions:")
        for i, p in enumerate(sample.positions[:10]):
            print(f"  [{i}] ({p[0]:.4f}, {p[1]:.4f}, {p[2]:.4f})")
        
        # Print first few indices 
        print(f"\nFace indices (all {len(sample.faceIndices)}):")
        print(sample.faceIndices)
        
    for i in range(obj.getNumChildren()):
        find_mesh(obj.getChild(i), depth+1)

find_mesh(root)
