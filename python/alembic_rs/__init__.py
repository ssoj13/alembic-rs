"""
alembic_rs - Fast Alembic (.abc) file reader/writer for Python

A pure Rust implementation of the Alembic file format, providing fast
read and write access to 3D geometry caches used in VFX and animation.

Example usage:

    import alembic_rs as abc

    # Read an archive
    archive = abc.IArchive("scene.abc")
    root = archive.getTop()
    
    for i in range(root.getNumChildren()):
        child = root.getChild(i)
        print(f"Object: {child.getName()}")
    
    # Write an archive
    out = abc.OArchive.create("output.abc")
    out.setAppName("MyApp v1.0")
    
    mesh = abc.OPolyMesh("cube")
    sample = abc.OPolyMeshSample()
    sample.setPositions([...])
    sample.setFaceCounts([...])
    sample.setFaceIndices([...])
    mesh.addSample(sample)
    
    root = abc.OObject("")
    root.addChild(mesh.build())
    out.writeArchive(root)
"""

# Import everything from the native Rust extension
from .alembic_rs import *

__version__ = "0.1.0"
__all__ = [
    # Archives
    "IArchive",
    "OArchive",
    # Objects
    "IObject",
    "OObject",
    # Time Sampling
    "TimeSampling",
    # Geometry - PolyMesh
    "IPolyMesh",
    "IPolyMeshSample",
    "OPolyMesh",
    "OPolyMeshSample",
    # Geometry - Xform
    "IXform",
    "IXformSample",
    "OXform",
    "OXformSample",
    # Geometry - Curves
    "OCurves",
    "OCurvesSample",
    # Geometry - Points
    "OPoints",
    "OPointsSample",
    # Geometry - SubD
    "OSubD",
    "OSubDSample",
    # Geometry - Camera
    "OCamera",
    # Geometry - NuPatch
    "ONuPatch",
    "ONuPatchSample",
    # Geometry - Light
    "OLight",
    # Geometry - FaceSet
    "OFaceSet",
    "OFaceSetSample",
    # Materials
    "OMaterial",
    "OMaterialSample",
    # Collections
    "OCollections",
    # Properties
    "OScalarProperty",
    "OArrayProperty",
    "OCompoundProperty",
    # Visibility
    "ObjectVisibility",
    "OVisibilityProperty",
    # GeomParam
    "IGeomParam",
    # FaceSet
    "IFaceSet",
]
