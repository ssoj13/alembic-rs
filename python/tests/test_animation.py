"""Tests for animated data and time sampling."""

import pytest
import alembic_rs as abc


class TestTimeSampling:
    """Tests for time sampling."""
    
    def test_uniform_time_sampling(self, temp_abc_file):
        """Test uniform (fps-based) time sampling."""
        archive = abc.OArchive.create(temp_abc_file)
        
        ts_idx = archive.addUniformTimeSampling(24.0)  # 24 fps
        assert ts_idx == 1  # 0 is always identity
        
        root = abc.OObject("")
        archive.writeArchive(root)
        del archive
        
        # Read back
        reader = abc.IArchive(temp_abc_file)
        ts = reader.getTimeSampling(1)
        assert ts is not None
        assert ts.isUniform()
    
    def test_acyclic_time_sampling(self, temp_abc_file):
        """Test acyclic (irregular) time sampling."""
        archive = abc.OArchive.create(temp_abc_file)
        
        times = [0.0, 0.5, 1.0, 2.0, 5.0]
        ts_idx = archive.addAcyclicTimeSampling(times)
        
        root = abc.OObject("")
        archive.writeArchive(root)
        del archive
        
        # Read back
        reader = abc.IArchive(temp_abc_file)
        ts = reader.getTimeSampling(ts_idx)
        assert ts is not None
        assert ts.isAcyclic()
        assert ts.getNumStoredTimes() == 5
    
    def test_cyclic_time_sampling(self, temp_abc_file):
        """Test cyclic time sampling."""
        archive = abc.OArchive.create(temp_abc_file)
        
        time_per_cycle = 1.0
        times = [0.0, 0.25, 0.5, 0.75]  # 4 samples per second
        ts_idx = archive.addCyclicTimeSampling(time_per_cycle, times)
        
        root = abc.OObject("")
        archive.writeArchive(root)
        del archive
        
        # Read back
        reader = abc.IArchive(temp_abc_file)
        ts = reader.getTimeSampling(ts_idx)
        assert ts is not None
        assert ts.isCyclic()


class TestAnimatedGeometry:
    """Tests for animated geometry."""
    
    def test_animated_polymesh(self, temp_abc_file):
        """Test writing animated mesh."""
        archive = abc.OArchive.create(temp_abc_file)
        
        ts_idx = archive.addUniformTimeSampling(24.0)
        
        mesh = abc.OPolyMesh("animated_mesh")
        mesh.setTimeSamplingIndex(ts_idx)
        
        # Frame 0
        sample0 = abc.OPolyMeshSample()
        sample0.setPositions([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0])
        sample0.setFaceCounts([3])
        sample0.setFaceIndices([0, 1, 2])
        mesh.addSample(sample0)
        
        # Frame 1 - moved
        sample1 = abc.OPolyMeshSample()
        sample1.setPositions([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 2.0, 0.0])
        sample1.setFaceCounts([3])
        sample1.setFaceIndices([0, 1, 2])
        mesh.addSample(sample1)
        
        root = abc.OObject("")
        root.addChild(mesh.build())
        archive.writeArchive(root)
    
    def test_animated_xform(self, temp_abc_file):
        """Test writing animated transform."""
        archive = abc.OArchive.create(temp_abc_file)
        
        ts_idx = archive.addUniformTimeSampling(24.0)
        
        xform = abc.OXform("animated_xform")
        xform.setTimeSamplingIndex(ts_idx)
        
        # Frame 0: origin
        sample0 = abc.OXformSample()
        sample0.setTranslation(0.0, 0.0, 0.0)
        xform.addSample(sample0)
        
        # Frame 1: moved
        sample1 = abc.OXformSample()
        sample1.setTranslation(10.0, 0.0, 0.0)
        xform.addSample(sample1)
        
        # Frame 2: moved more
        sample2 = abc.OXformSample()
        sample2.setTranslation(20.0, 0.0, 0.0)
        xform.addSample(sample2)
        
        root = abc.OObject("")
        root.addChild(xform.build())
        archive.writeArchive(root)
    
    def test_animated_property(self, temp_abc_file):
        """Test writing animated custom property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        ts_idx = archive.addUniformTimeSampling(30.0)  # 30 fps
        
        root = abc.OObject("")
        obj = abc.OObject("animated_obj")
        
        prop = obj.addScalarProperty("animated_value", "float")
        prop.setTimeSamplingIndex(ts_idx)
        
        # Write 10 frames
        for i in range(10):
            prop.addSampleFloat(float(i) * 0.1)
        
        root.addChild(obj)
        archive.writeArchive(root)


class TestReadAnimatedData:
    """Tests for reading animated data."""
    
    def test_read_animated_mesh(self, temp_abc_file):
        """Test reading animated mesh samples."""
        # Write
        archive = abc.OArchive.create(temp_abc_file)
        ts_idx = archive.addUniformTimeSampling(24.0)
        
        mesh = abc.OPolyMesh("mesh")
        mesh.setTimeSamplingIndex(ts_idx)
        
        for i in range(5):
            sample = abc.OPolyMeshSample()
            y = float(i) * 0.5
            sample.setPositions([0.0, y, 0.0, 1.0, y, 0.0, 0.5, y + 1.0, 0.0])
            sample.setFaceCounts([3])
            sample.setFaceIndices([0, 1, 2])
            mesh.addSample(sample)
        
        root = abc.OObject("")
        root.addChild(mesh.build())
        archive.writeArchive(root)
        del archive
        
        # Read
        reader = abc.IArchive(temp_abc_file)
        top = reader.getTop()
        mesh_obj = top.getChildByName("mesh")
        
        assert mesh_obj is not None
        # Get the IPolyMesh schema
        polymesh = abc.IPolyMesh(mesh_obj)
        assert polymesh.getNumSamples() == 5
