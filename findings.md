# Binary Parity Investigation Findings

## ROOT CAUSE FOUND!

The root object has properties that we DON'T copy:

```
Object: ABC [_ai_AlembicVersion=...; _ai_Application=Maya 2022; ...]
  Properties (3):
    [0] .childBnds (interpretation=box)   <- NOT COPIED!
    [1] statistics ()                     <- NOT COPIED!
    [2] 1.samples ()                      <- NOT COPIED!
```

Our `convert_object()` only copies schema-specific properties (Xform, PolyMesh, etc),
but ignores arbitrary properties on the root object!

### What these properties are:
1. **`.childBnds`** - Child bounding boxes (Box3d) for the entire hierarchy
2. **`statistics`** - Archive statistics (sizes, counts, etc.)
3. **`1.samples`** - Sample index data for time sampling 1

These are created by Maya/Alembic automatically and contain important metadata.

## Current Status

- **Match**: ~77% (due to missing root properties)
- **Original size**: 5921 bytes
- **Our output size**: 5843 bytes (-78 bytes - missing the 3 root properties!)

## Solution Options

### Option 1: Copy arbitrary properties
Add generic property copying to `convert_object()`:
```rust
// After schema-specific copying, also copy remaining properties
for i in 0..props.getNumProperties() {
    if let Some(header) = props.getPropertyHeader(i) {
        if !out.properties.iter().any(|p| p.name == header.name) {
            // Property not yet copied - copy it generically
            copy_arbitrary_property(props, i, &mut out);
        }
    }
}
```

### Option 2: Add specific handling
Add explicit handling for `.childBnds`, `statistics`, `1.samples`:
```rust
// In write_archive, add root-level properties
if has_child_bounds { ... }
if has_statistics { ... }
```

### Option 3: Full property tree copy
Implement deep property tree copying that preserves everything.

## Recommended: Option 1 or 3

For true binary parity, we need to copy ALL properties, not just schema-specific ones.

## Full heart.abc Structure

```
Time samplings: 2
  TS[0]: Uniform { time_per_cycle: 1.0, start_time: 0.0 }
  TS[1]: Uniform { time_per_cycle: 0.0417, start_time: 0.0417 }

Object: ABC (root)
  Properties (3):
    [0] .childBnds (box)
    [1] statistics 
    [2] 1.samples
    
  Object: heart [Xform]
    Properties (1):
      [0] .xform
      
    Object: heartShape [PolyMesh]  
      Properties (1):
        [0] .geom
```

## Previous Session Note

The 99.34% match mentioned in the session summary was likely:
1. From a different test that didn't include root properties
2. Or from a special test file that was simpler
3. Or comparing against our own output (self-consistency)

Current ~77% is accurate for copying Maya-created files with full metadata.

## Next Steps

1. Implement arbitrary property copying in convert_object()
2. Test with heart.abc
3. Verify improvement in binary parity
4. Then focus on remaining structural differences (wrapper groups, etc.)
