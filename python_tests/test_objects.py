"""Tests for IObject and OObject."""

import pytest
import alembic_rs as abc


class TestIObject:
    """Tests for reading objects."""
    
    def test_root_object(self, test_data_dir):
        """Test root object properties."""
        bmw_path = test_data_dir / "bmw.abc"
        if not bmw_path.exists():
            pytest.skip("bmw.abc not found")
        
        archive = abc.IArchive(str(bmw_path))
        root = archive.getTop()
        
        assert root.getName() == "ABC"
        assert root.getFullName() == "/"
        assert root.getNumChildren() > 0
    
    def test_child_traversal(self, test_data_dir):
        """Test traversing children."""
        bmw_path = test_data_dir / "bmw.abc"
        if not bmw_path.exists():
            pytest.skip("bmw.abc not found")
        
        archive = abc.IArchive(str(bmw_path))
        root = archive.getTop()
        
        for i in range(root.getNumChildren()):
            child = root.getChild(i)
            assert child is not None
            assert len(child.getName()) > 0
    
    def test_child_by_name(self, test_data_dir):
        """Test getting child by name."""
        bmw_path = test_data_dir / "bmw.abc"
        if not bmw_path.exists():
            pytest.skip("bmw.abc not found")
        
        archive = abc.IArchive(str(bmw_path))
        root = archive.getTop()
        
        if root.getNumChildren() > 0:
            first_child = root.getChild(0)
            name = first_child.getName()
            
            same_child = root.getChildByName(name)
            assert same_child is not None
            assert same_child.getName() == name
    
    def test_has_child(self, test_data_dir):
        """Test checking if child exists."""
        bmw_path = test_data_dir / "bmw.abc"
        if not bmw_path.exists():
            pytest.skip("bmw.abc not found")
        
        archive = abc.IArchive(str(bmw_path))
        root = archive.getTop()
        
        if root.getNumChildren() > 0:
            first_child = root.getChild(0)
            assert root.hasChild(first_child.getName())
        
        assert not root.hasChild("__nonexistent_child__")


class TestOObject:
    """Tests for writing objects."""
    
    def test_create_object(self):
        """Test creating an object."""
        obj = abc.OObject("test_object")
        assert obj is not None
    
    def test_add_child(self, temp_abc_file):
        """Test adding child objects."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        child1 = abc.OObject("child1")
        child2 = abc.OObject("child2")
        
        root.addChild(child1)
        root.addChild(child2)
        
        archive.writeArchive(root)
        del archive
        
        # Read back and verify
        reader = abc.IArchive(temp_abc_file)
        top = reader.getTop()
        assert top.getNumChildren() == 2
        assert top.hasChild("child1")
        assert top.hasChild("child2")
    
    def test_nested_hierarchy(self, temp_abc_file):
        """Test nested object hierarchy."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        parent = abc.OObject("parent")
        child = abc.OObject("child")
        grandchild = abc.OObject("grandchild")
        
        child.addChild(grandchild)
        parent.addChild(child)
        root.addChild(parent)
        
        archive.writeArchive(root)
        del archive
        
        # Read back and verify
        reader = abc.IArchive(temp_abc_file)
        top = reader.getTop()
        
        parent_obj = top.getChildByName("parent")
        assert parent_obj is not None
        
        child_obj = parent_obj.getChildByName("child")
        assert child_obj is not None
        
        grandchild_obj = child_obj.getChildByName("grandchild")
        assert grandchild_obj is not None
