"""Tests for geometry types: PolyMesh, Xform, etc."""

import pytest
import alembic_rs as abc


class TestPolyMesh:
    """Tests for PolyMesh geometry."""
    
    def test_write_triangle(self, temp_abc_file):
        """Write a simple triangle mesh."""
        archive = abc.OArchive.create(temp_abc_file)
        
        mesh = abc.OPolyMesh("triangle")
        
        # Triangle vertices
        positions = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0]
        face_counts = [3]  # One triangle
        face_indices = [0, 1, 2]
        
        sample = abc.OPolyMeshSample()
        sample.setPositions(positions)
        sample.setFaceCounts(face_counts)
        sample.setFaceIndices(face_indices)
        
        mesh.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(mesh.build())
        
        archive.writeArchive(root)
    
    def test_write_cube(self, temp_abc_file):
        """Write a cube mesh."""
        archive = abc.OArchive.create(temp_abc_file)
        
        mesh = abc.OPolyMesh("cube")
        
        # Cube vertices
        positions = [
            -1, -1, -1,  1, -1, -1,  1,  1, -1, -1,  1, -1,  # Back face
            -1, -1,  1,  1, -1,  1,  1,  1,  1, -1,  1,  1   # Front face
        ]
        
        # 6 quad faces
        face_counts = [4, 4, 4, 4, 4, 4]
        face_indices = [
            0, 1, 2, 3,  # Back
            4, 5, 6, 7,  # Front
            0, 4, 5, 1,  # Bottom
            3, 2, 6, 7,  # Top
            0, 3, 7, 4,  # Left
            1, 5, 6, 2   # Right
        ]
        
        sample = abc.OPolyMeshSample()
        sample.setPositions(positions)
        sample.setFaceCounts(face_counts)
        sample.setFaceIndices(face_indices)
        
        mesh.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(mesh.build())
        
        archive.writeArchive(root)
    
    def test_read_polymesh(self, test_data_dir):
        """Read a PolyMesh from file."""
        bmw_path = test_data_dir / "bmw.abc"
        if not bmw_path.exists():
            pytest.skip("bmw.abc not found")
        
        archive = abc.IArchive(str(bmw_path))
        # BMw typically has meshes at /bmw/Body etc.
        # Just test that we can open and traverse
        assert archive.valid()
    
    def test_polymesh_with_uvs(self, temp_abc_file):
        """Write a mesh with UV coordinates."""
        archive = abc.OArchive.create(temp_abc_file)
        
        mesh = abc.OPolyMesh("mesh_with_uvs")
        
        positions = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0]
        face_counts = [3]
        face_indices = [0, 1, 2]
        uvs = [0.0, 0.0, 1.0, 0.0, 0.5, 1.0]
        
        sample = abc.OPolyMeshSample()
        sample.setPositions(positions)
        sample.setFaceCounts(face_counts)
        sample.setFaceIndices(face_indices)
        sample.setUVs(uvs)
        
        mesh.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(mesh.build())
        
        archive.writeArchive(root)
    
    def test_polymesh_with_normals(self, temp_abc_file):
        """Write a mesh with normals."""
        archive = abc.OArchive.create(temp_abc_file)
        
        mesh = abc.OPolyMesh("mesh_with_normals")
        
        positions = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0]
        face_counts = [3]
        face_indices = [0, 1, 2]
        normals = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]
        
        sample = abc.OPolyMeshSample()
        sample.setPositions(positions)
        sample.setFaceCounts(face_counts)
        sample.setFaceIndices(face_indices)
        sample.setNormals(normals)
        
        mesh.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(mesh.build())
        
        archive.writeArchive(root)


class TestXform:
    """Tests for Xform (transform) nodes."""
    
    def test_write_identity_xform(self, temp_abc_file):
        """Write an identity transform."""
        archive = abc.OArchive.create(temp_abc_file)
        
        xform = abc.OXform("xform_node")
        
        sample = abc.OXformSample()
        sample.setTranslation(0.0, 0.0, 0.0)
        sample.setScale(1.0, 1.0, 1.0)
        
        xform.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(xform.build())
        
        archive.writeArchive(root)
    
    def test_write_translated_xform(self, temp_abc_file):
        """Write a transform with translation."""
        archive = abc.OArchive.create(temp_abc_file)
        
        xform = abc.OXform("translated")
        
        sample = abc.OXformSample()
        sample.setTranslation(10.0, 5.0, -3.0)
        
        xform.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(xform.build())
        
        archive.writeArchive(root)
    
    def test_write_scaled_xform(self, temp_abc_file):
        """Write a transform with scale."""
        archive = abc.OArchive.create(temp_abc_file)
        
        xform = abc.OXform("scaled")
        
        sample = abc.OXformSample()
        sample.setScale(2.0, 2.0, 2.0)
        
        xform.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(xform.build())
        
        archive.writeArchive(root)
    
    def test_write_rotated_xform(self, temp_abc_file):
        """Write a transform with rotation."""
        archive = abc.OArchive.create(temp_abc_file)
        
        xform = abc.OXform("rotated")
        
        sample = abc.OXformSample()
        # Rotate 45 degrees around Y axis
        sample.setRotationFromEuler(0.0, 45.0, 0.0)
        
        xform.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(xform.build())
        
        archive.writeArchive(root)
    
    def test_write_full_xform(self, temp_abc_file):
        """Write a transform with full TRS."""
        archive = abc.OArchive.create(temp_abc_file)
        
        xform = abc.OXform("full_xform")
        
        sample = abc.OXformSample()
        sample.setTranslation(1.0, 2.0, 3.0)
        sample.setRotationFromEuler(0.0, 90.0, 0.0)
        sample.setScale(0.5, 0.5, 0.5)
        
        xform.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(xform.build())
        
        archive.writeArchive(root)
    
    def test_xform_with_matrix(self, temp_abc_file):
        """Write a transform from a 4x4 matrix."""
        archive = abc.OArchive.create(temp_abc_file)
        
        xform = abc.OXform("matrix_xform")
        
        # Identity matrix as flat array
        matrix = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            5.0, 0.0, 0.0, 1.0  # Translation in last row
        ]
        
        sample = abc.OXformSample()
        sample.setMatrix(matrix)
        
        xform.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(xform.build())
        
        archive.writeArchive(root)


class TestCurves:
    """Tests for Curves geometry."""
    
    def test_write_simple_curve(self, temp_abc_file):
        """Write a simple linear curve."""
        archive = abc.OArchive.create(temp_abc_file)
        
        curves = abc.OCurves("curve")
        
        positions = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 1.0, 0.0, 3.0, 0.0, 0.0]
        num_verts = [4]  # One curve with 4 vertices
        
        sample = abc.OCurvesSample()
        sample.setPositions(positions)
        sample.setNumVerts(num_verts)
        
        curves.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(curves.build())
        
        archive.writeArchive(root)


class TestPoints:
    """Tests for Points geometry."""
    
    def test_write_point_cloud(self, temp_abc_file):
        """Write a simple point cloud."""
        archive = abc.OArchive.create(temp_abc_file)
        
        points = abc.OPoints("points")
        
        positions = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
        ids = [0, 1, 2, 3]
        
        sample = abc.OPointsSample()
        sample.setPositions(positions)
        sample.setIds(ids)
        
        points.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(points.build())
        
        archive.writeArchive(root)
