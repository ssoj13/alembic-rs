"""
alembic_rs - Fast Alembic (.abc) file reader/writer for Python

A pure Rust implementation of the Alembic file format, providing fast
read and write access to 3D geometry caches used in VFX and animation.

Example usage:

    import alembic_rs as abc

    # Read an archive
    archive = abc.IArchive("scene.abc")
    root = archive.getTop()
    
    for child in root:
        print(f"Object: {child.getName()}")
        if child.isPolyMesh():
            # Simple API
            sample = child.getPolyMeshSample(0)
            # Or original Alembic-style API
            mesh = abc.IPolyMesh(child)
            sample = mesh.getSchema().getValue()
            print(f"  Vertices: {len(sample.positions)}")
    
    # Write an archive
    out = abc.OArchive.create("output.abc")
    out.setAppName("MyApp v1.0")
    
    mesh = abc.OPolyMesh("cube")
    mesh.addSample(positions, face_counts, face_indices)
    
    root = abc.OObject("")
    root.addPolyMesh(mesh)
    out.writeArchive(root)
"""

# Import everything from the native Rust extension
from .alembic_rs import *

__version__ = "0.1.0"
__all__ = [
    # =========================================================================
    # Archives
    # =========================================================================
    "IArchive",
    "OArchive",
    
    # =========================================================================
    # Objects
    # =========================================================================
    "IObject",
    "OObject",
    
    # =========================================================================
    # Time Sampling
    # =========================================================================
    "TimeSampling",
    
    # =========================================================================
    # Schema Readers (original Alembic API style)
    # =========================================================================
    # PolyMesh
    "IPolyMesh",
    "IPolyMeshSchema",
    # Xform
    "IXform",
    "IXformSchema",
    # SubD
    "ISubD",
    "ISubDSchema",
    # Curves
    "ICurves",
    "ICurvesSchema",
    # Points
    "IPoints",
    "IPointsSchema",
    # Camera
    "ICamera",
    "ICameraSchema",
    # Light
    "ILight",
    "ILightSchema",
    # NuPatch
    "INuPatch",
    "INuPatchSchema",
    # FaceSet
    "IFaceSet",
    # GeomParam
    "IGeomParam",
    
    # =========================================================================
    # Sample Types (read)
    # =========================================================================
    "PolyMeshSample",
    "XformSample",
    "SubDSample",
    "CurvesSample",
    "PointsSample",
    "CameraSample",
    "LightSample",
    "NuPatchSample",
    "FaceSetSample",
    "GeomParamSample",
    
    # =========================================================================
    # Schema Writers
    # =========================================================================
    "OPolyMesh",
    "OXform",
    "OSubD",
    "OCurves",
    "OPoints",
    "OCamera",
    "OLight",
    "ONuPatch",
    "OFaceSet",
    
    # =========================================================================
    # Materials & Collections
    # =========================================================================
    "OMaterial",
    "OCollections",
    "IMaterial",
    "ICollections",
    "Collection",
    
    # =========================================================================
    # Properties
    # =========================================================================
    "ICompoundProperty",
    "PropertyInfo",
    "OScalarProperty",
    "OArrayProperty",
    "OCompoundProperty",
    
    # =========================================================================
    # Visibility
    # =========================================================================
    "ObjectVisibility",
    "OVisibilityProperty",
]
