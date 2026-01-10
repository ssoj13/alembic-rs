# Alembic-rs Bug Hunt Report and Action Plan

## Executive Summary

During the bug hunt, we identified critical issues in the Ogawa writer implementation that prevent files from opening correctly in Blender. The main problems stem from format incompatibilities with the official Alembic specification.

## Issues Found

### 1. Archive Structure Issues
- **Root Group Structure**: Incorrect ordering of children in the root group
- **Expected order**:
  - Child 0: Version data (4 bytes)
  - Child 1: File version data (4 bytes) 
  - Child 2: Root object (group)
  - Child 3: Archive metadata (data)
  - Child 4: Time samplings (data)
  - Child 5: Indexed metadata (data)

### 2. Header Format Issues
- Version field encoding may not match official format
- Finalization process might not follow exact sequence required

### 3. Object Header Serialization Issues
- Hash calculation for object headers may not match official implementation
- Format of object headers differs from expected format
- Child object names and metadata serialization may not follow official format

### 4. Property Header Format Issues
- Property info bitmask construction may not match official format
- Variable-length field encoding with size hints differs from official implementation
- Time sampling indices encoding may not match expectations

### 5. Data Block Format Issues
- Keyed data format (16-byte MD5 digest prefix) may not be consistent
- Array dimensions storage format may differ
- String data encoding might not match official expectations

### 6. Schema-Specific Issues
- Property naming conventions may not follow strict Alembic requirements
- Missing required metadata entries for certain schemas
- Internal structure of geometric data may not match expected formats

### 7. Endianness Issues
- Potential inconsistencies in multi-byte value handling compared to official implementation

## Key Differences from Official Implementation

1. **Initialization Order**: Official implementation writes version data first, then library version, then creates the root group, etc.

2. **Metadata Handling**: Official implementation uses a `MetaDataMap` to efficiently store and reference metadata strings.

3. **Hash Calculations**: Official implementation uses `SpookyHash` for certain hash calculations, while the Rust version uses MD5 digests.

4. **Finalization Sequence**: The exact order of writing time samplings, metadata, and finalizing the archive differs.

## Recommended Actions

### Immediate Fixes (Priority 1)
1. **Align Header Format**: Ensure the header exactly matches the official format with proper byte ordering
2. **Fix Root Structure**: Follow the exact child ordering expected by the official implementation
3. **Correct Property Headers**: Fix the property info bitmask construction to match official implementation

### Medium Priority Fixes (Priority 2)
4. **Verify Hash Algorithms**: Ensure hash computations match the official implementation where required
5. **Validate Schema Compliance**: Ensure all schema-specific requirements are met
6. **Test with Minimal Cases**: Create minimal test cases to verify basic functionality

### Long-term Improvements (Priority 3)
7. **Add Binary Format Verification**: Implement tools to compare binary output with official implementation
8. **Improve Test Coverage**: Add more comprehensive round-trip tests
9. **Enhance Compatibility Testing**: Regular testing with various DCC applications (Blender, Maya, etc.)

## Implementation Plan

### Phase 1: Critical Fixes (Week 1)
- [ ] Fix header format and initialization sequence
- [ ] Correct root group structure and child ordering
- [ ] Validate property header format

### Phase 2: Schema Compliance (Week 2)
- [ ] Fix object header serialization
- [ ] Ensure schema-specific requirements are met
- [ ] Test with basic geometric primitives

### Phase 3: Validation and Testing (Week 3)
- [ ] Create comprehensive test suite
- [ ] Verify compatibility with Blender and other DCC tools
- [ ] Perform binary comparison with official implementation output

## Files/Line Numbers Requiring Attention

### src/ogawa/writer.rs
- Lines 100-150: Header writing and initialization
- Lines 200-250: Root group structure creation
- Lines 400-500: Object header serialization
- Lines 500-600: Property header serialization
- Lines 600-700: Archive finalization

### src/abc/mod.rs
- Lines 100-200: Archive reading functionality (for verification)
- Lines 300-400: Object and property handling

### src/geom/* (all schema writers)
- All schema-specific writers need verification for format compliance

## Success Criteria

- [ ] Files created by the Rust implementation open correctly in Blender
- [ ] Round-trip tests pass (write → read → verify)
- [ ] Binary format matches official implementation for equivalent content
- [ ] All existing functionality remains intact
- [ ] Performance is maintained or improved

## Risk Assessment

- **High Risk**: Changes to core format may break existing functionality
- **Mitigation**: Comprehensive testing with existing test suite
- **Medium Risk**: Schema-specific changes may affect geometric data handling
- **Mitigation**: Gradual implementation with verification at each step