# Writer Parity Fix Plan (Draft)

## Goals
- Achieve binary parity with Alembic AbcCoreOgawa writer for identical input.
- Preserve full property trees (root + arbitrary) during copy.

## Phase 1: Hash/Key Parity
1) Implement Alembic-compatible sample digest generation:
   - Use MurmurHash3_x64_128 with POD-size seed.
   - For string/wstring, encode null-terminated sequences before hashing.
2) Replace `ArraySampleContentKey::from_data` with API that accepts POD type and
   encoded data so the digest matches AbcCoreAbstract::ArraySample::Key.
3) Update `write_keyed_data` to use the Alembic digest for key bytes.

## Phase 2: Property Header Parity
1) Initialize `first_changed_index` and `last_changed_index` to 0 for scalar/array properties.
2) Track `isHomogenous` based on `dims.numPoints()` changes, not extent.
3) Implement `set_from_previous`-like behavior or at least allow explicitly
   marking repeat samples to preserve first/last change indices when needed.
4) Use stable sorting for `data_write_order` ties.

## Phase 3: Metadata Parity
1) Allow 254 indexed metadata entries (+ empty) before forcing inline metadata.
2) Align `_ai_AlembicVersion` with build-time string or copy from source metadata.

## Phase 4: Copy Parity
1) Implement full property tree copying:
   - Compound properties: recurse.
   - Scalar/array properties: copy raw bytes + dimensions.
   - Preserve metadata and time sampling indices (with remap).
2) Ensure root object properties (`.childBnds`, `statistics`, `N.samples`) are preserved.

## Phase 5: Validation
1) Add binary parity tests against sample archives (`heart.abc`, `gears.abc`, etc.).
2) Compare metadata tables and object/property hashes for exact match.
