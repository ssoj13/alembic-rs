"""Tests for IArchive and OArchive."""

import pytest
import alembic_rs as abc


class TestIArchive:
    """Tests for reading archives."""
    
    def test_open_nonexistent(self):
        """Opening a nonexistent file should raise IOError."""
        with pytest.raises(IOError):
            abc.IArchive("/nonexistent/path.abc")
    
    def test_open_valid_file(self, test_data_dir):
        """Open a valid archive."""
        bmw_path = test_data_dir / "bmw.abc"
        if bmw_path.exists():
            archive = abc.IArchive(str(bmw_path))
            assert archive.valid()
            assert "bmw" in archive.getName()
    
    def test_get_top(self, test_data_dir):
        """Get the top (root) object."""
        bmw_path = test_data_dir / "bmw.abc"
        if bmw_path.exists():
            archive = abc.IArchive(str(bmw_path))
            top = archive.getTop()
            assert top is not None
            assert top.getFullName() == "/"
    
    def test_time_samplings(self, test_data_dir):
        """Test time sampling access."""
        bmw_path = test_data_dir / "bmw.abc"
        if bmw_path.exists():
            archive = abc.IArchive(str(bmw_path))
            num_ts = archive.getNumTimeSamplings()
            assert num_ts >= 1
            
            ts = archive.getTimeSampling(0)
            assert ts is not None
    
    def test_archive_version(self, test_data_dir):
        """Test archive version access."""
        bmw_path = test_data_dir / "bmw.abc"
        if bmw_path.exists():
            archive = abc.IArchive(str(bmw_path))
            version = archive.getArchiveVersion()
            assert version > 0
            
            version_str = archive.getArchiveVersionString()
            assert "." in version_str


class TestOArchive:
    """Tests for writing archives."""
    
    def test_create(self, temp_abc_file):
        """Create an empty archive."""
        archive = abc.OArchive.create(temp_abc_file)
        assert archive.getName() == temp_abc_file
        
        root = abc.OObject("")
        archive.writeArchive(root)
    
    def test_roundtrip_metadata(self, temp_abc_file):
        """Test archive metadata roundtrip."""
        # Write
        archive = abc.OArchive.create(temp_abc_file)
        archive.setAppName("Test App v1.0")
        archive.setDateWritten("2025-01-09")
        archive.setDescription("Test description")
        archive.setDccFps(24.0)
        
        root = abc.OObject("")
        archive.writeArchive(root)
        del archive
        
        # Read back
        reader = abc.IArchive(temp_abc_file)
        assert reader.getAppName() == "Test App v1.0"
        assert reader.getDateWritten() == "2025-01-09"
        assert reader.getUserDescription() == "Test description"
        assert reader.getDccFps() == 24.0
    
    def test_time_sampling_uniform(self, temp_abc_file):
        """Test adding uniform time sampling."""
        archive = abc.OArchive.create(temp_abc_file)
        
        ts_idx = archive.addUniformTimeSampling(24.0)  # 24 fps
        assert ts_idx >= 0
        
        root = abc.OObject("")
        archive.writeArchive(root)
    
    def test_time_sampling_acyclic(self, temp_abc_file):
        """Test adding acyclic time sampling."""
        archive = abc.OArchive.create(temp_abc_file)
        
        times = [0.0, 0.5, 1.0, 2.0, 5.0]
        ts_idx = archive.addAcyclicTimeSampling(times)
        assert ts_idx >= 0
        
        root = abc.OObject("")
        archive.writeArchive(root)
    
    def test_compression_hint(self, temp_abc_file):
        """Test setting compression hint."""
        archive = abc.OArchive.create(temp_abc_file)
        archive.setCompressionHint(5)
        
        root = abc.OObject("")
        archive.writeArchive(root)
    
    def test_dedup_enabled(self, temp_abc_file):
        """Test enabling/disabling deduplication."""
        archive = abc.OArchive.create(temp_abc_file)
        archive.setDedupEnabled(True)
        archive.setDedupEnabled(False)
        
        root = abc.OObject("")
        archive.writeArchive(root)
