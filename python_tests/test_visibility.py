"""Tests for visibility support."""

import pytest
import alembic_rs as abc


class TestVisibility:
    """Tests for object visibility."""
    
    def test_visibility_constants(self):
        """Test visibility constant values."""
        assert abc.ObjectVisibility.deferred() == -1
        assert abc.ObjectVisibility.hidden() == 0
        assert abc.ObjectVisibility.visible() == 1
    
    def test_write_visibility(self, temp_abc_file):
        """Write visibility property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("visible_obj")
        
        vis_prop = obj.addVisibilityProperty()
        vis_prop.setVisible()  # Frame 0: visible
        vis_prop.setHidden()   # Frame 1: hidden
        vis_prop.setDeferred() # Frame 2: deferred
        
        root.addChild(obj)
        archive.writeArchive(root)
    
    def test_read_visibility(self, temp_abc_file):
        """Read visibility property."""
        # First write
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_vis")
        
        vis_prop = obj.addVisibilityProperty()
        vis_prop.setVisible()
        vis_prop.setHidden()
        
        root.addChild(obj)
        archive.writeArchive(root)
        del archive
        
        # Then read
        reader = abc.IArchive(temp_abc_file)
        top = reader.getTop()
        child = top.getChildByName("test_vis")
        
        assert child is not None
        # Check visibility at sample 0
        vis = child.getVisibility(0)
        assert vis == abc.ObjectVisibility.visible()
        
        # Check visibility at sample 1
        vis = child.getVisibility(1)
        assert vis == abc.ObjectVisibility.hidden()
    
    def test_is_visible_helper(self, temp_abc_file):
        """Test isVisible helper method."""
        # Write
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_vis")
        
        vis_prop = obj.addVisibilityProperty()
        vis_prop.setVisible()
        vis_prop.setHidden()
        
        root.addChild(obj)
        archive.writeArchive(root)
        del archive
        
        # Read
        reader = abc.IArchive(temp_abc_file)
        top = reader.getTop()
        child = top.getChildByName("test_vis")
        
        assert child.isVisible(0) == True
        assert child.isVisible(1) == False
        assert child.isHidden(1) == True
