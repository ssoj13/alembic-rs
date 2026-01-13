"""Tests for properties: scalar, array, compound."""

import pytest
import alembic_rs as abc


class TestScalarProperties:
    """Tests for scalar properties."""
    
    def test_write_int_property(self, temp_abc_file):
        """Write an integer scalar property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        prop = obj.addScalarProperty("int_prop", "int")
        prop.addSampleInt(42)
        prop.addSampleInt(100)
        
        root.addChild(obj)
        archive.writeArchive(root)
    
    def test_write_float_property(self, temp_abc_file):
        """Write a float scalar property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        prop = obj.addScalarProperty("float_prop", "float")
        prop.addSampleFloat(3.14)
        prop.addSampleFloat(2.71)
        
        root.addChild(obj)
        archive.writeArchive(root)
    
    def test_write_double_property(self, temp_abc_file):
        """Write a double scalar property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        prop = obj.addScalarProperty("double_prop", "double")
        prop.addSampleDouble(3.14159265358979)
        
        root.addChild(obj)
        archive.writeArchive(root)
    
    def test_write_bool_property(self, temp_abc_file):
        """Write a boolean scalar property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        prop = obj.addScalarProperty("bool_prop", "bool")
        prop.addSampleBool(True)
        prop.addSampleBool(False)
        
        root.addChild(obj)
        archive.writeArchive(root)
    
    def test_write_string_property(self, temp_abc_file):
        """Write a string scalar property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        prop = obj.addScalarProperty("string_prop", "string")
        prop.addSampleString("Hello, World!")
        prop.addSampleString("Another string")
        
        root.addChild(obj)
        archive.writeArchive(root)
    
    def test_write_vec3f_property(self, temp_abc_file):
        """Write a vec3f scalar property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        prop = obj.addScalarProperty("vec3f_prop", "vec3f")
        prop.addSampleVec3f(1.0, 2.0, 3.0)
        
        root.addChild(obj)
        archive.writeArchive(root)


class TestArrayProperties:
    """Tests for array properties."""
    
    def test_write_int_array(self, temp_abc_file):
        """Write an integer array property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        prop = obj.addArrayProperty("int_array", "int")
        prop.addSampleInts([1, 2, 3, 4, 5])
        prop.addSampleInts([10, 20, 30])
        
        root.addChild(obj)
        archive.writeArchive(root)
    
    def test_write_float_array(self, temp_abc_file):
        """Write a float array property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        prop = obj.addArrayProperty("float_array", "float")
        prop.addSampleFloats([1.0, 2.0, 3.0, 4.0])
        
        root.addChild(obj)
        archive.writeArchive(root)
    
    def test_write_double_array(self, temp_abc_file):
        """Write a double array property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        prop = obj.addArrayProperty("double_array", "double")
        prop.addSampleDoubles([1.0, 2.0, 3.0])
        
        root.addChild(obj)
        archive.writeArchive(root)
    
    def test_write_string_array(self, temp_abc_file):
        """Write a string array property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        prop = obj.addArrayProperty("string_array", "string")
        prop.addSampleStrings(["Hello", "World", "Test"])
        
        root.addChild(obj)
        archive.writeArchive(root)
    
    def test_write_vec3f_array(self, temp_abc_file):
        """Write a vec3f array property."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        prop = obj.addArrayProperty("vec3f_array", "vec3f")
        # 3 vectors (9 floats total)
        prop.addSampleVec3fs([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0])
        
        root.addChild(obj)
        archive.writeArchive(root)


class TestCompoundProperties:
    """Tests for compound properties."""
    
    def test_compound_with_children(self, temp_abc_file):
        """Write a compound property with child properties."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        compound = obj.addCompoundProperty("compound_prop")
        
        scalar = compound.addScalar("value", "float")
        scalar.addSampleFloat(42.0)
        
        array = compound.addArray("data", "int")
        array.addSampleInts([1, 2, 3])
        
        root.addChild(obj)
        archive.writeArchive(root)
    
    def test_nested_compounds(self, temp_abc_file):
        """Write nested compound properties."""
        archive = abc.OArchive.create(temp_abc_file)
        
        root = abc.OObject("")
        obj = abc.OObject("test_obj")
        
        outer = obj.addCompoundProperty("outer")
        inner = outer.addCompound("inner")
        
        prop = inner.addScalar("value", "int")
        prop.addSampleInt(123)
        
        root.addChild(obj)
        archive.writeArchive(root)
