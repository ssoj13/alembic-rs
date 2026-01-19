# Binary Parity Findings (Re-verified)

## Status
Re-verified against `_ref` AbcCoreOgawa. Writer/reader parity improved; remaining deltas are isolated and documented below.

## Fixed Since Last Pass

1) **String/Wstring payloads always include terminators (including empty)**
   - Fix: `encode_sample_for_pod` now appends terminators even for empty inputs.
   - Evidence (Rust): `src/ogawa/writer/write_util.rs:86`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcCoreAbstract/ArraySample.cpp:53`
   - Impact: Digest/payload parity for string/wstring samples.

2) **Max samples for constant properties now match C++**
   - Fix: `max_samples` uses 1 when `first/last` are 0 and samples > 0.
   - Evidence (Rust): `src/ogawa/writer/archive/properties.rs:131`, `src/ogawa/writer/archive/properties.rs:247`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcCoreOgawa/SpwImpl.cpp:53`, `_ref/alembic/lib/Alembic/AbcCoreOgawa/ApwImpl.cpp:53`
   - Impact: Time sampling tables now match constant-property behavior.

3) **Property header first/last change flags use state-derived counts**
   - Fix: `build_property_info` uses the same `num_samples` for info and comparisons.
   - Evidence (Rust): `src/ogawa/writer/archive/properties.rs:344`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcCoreOgawa/WriteUtil.cpp:290`
   - Impact: Header flags match when samples repeat.

4) **_ai_AlembicVersion is always set (like C++)**
   - Fix: writer now sets the key unconditionally during archive write.
   - Evidence (Rust): `src/ogawa/writer/archive/mod.rs:183`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcCoreOgawa/AwImpl.cpp:90`
   - Impact: Metadata presence now matches C++ behavior.

5) **copy2 property merge now prefers source on name collision**
   - Fix: scalar/array properties are replaced by the source property when names collide; compounds still merge.
   - Evidence (Rust): `src/bin/alembic/main.rs:876`
   - Impact: Avoids silently keeping schema-built properties when the source authoring differs.

6) **Build metadata can be pinned for parity**
   - Fix: `build.rs` now respects `ALEMBIC_BUILD_DATE/TIME` when set.
   - Evidence (Rust): `build.rs:1`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcCoreAbstract/Foundation.cpp:44`
   - Impact: `_ai_AlembicVersion` now matches reference when env is provided.

7) **PolyMesh property creation order matches OGeomBase**
   - Fix: `.selfBnds` is created before `P`, matching base-schema init order.
   - Evidence (Rust): `src/ogawa/writer/schema/polymesh.rs:86`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcGeom/OGeomBase.h:214`
   - Impact: property header order and metadata index ordering match C++.

8) **copy2 preserves writer-side data_write_order**
   - Fix: merge keeps existing `data_write_order` when replacing properties.
   - Evidence (Rust): `src/bin/alembic/main.rs:880`
   - Impact: data block ordering for schema-built properties matches C++ set() order.

9) **Array homogeneity flag matches C++**
   - Fix: arrays with extent > 1 are marked non-homogenous on first sample.
   - Evidence (Rust): `src/ogawa/writer/archive/properties.rs:304`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcCoreOgawa/ApwImpl.cpp:126`
   - Impact: property header info bit (0x400) matches reference.

10) **OGeomBase self bounds ordering added to Points/Curves**
   - Fix: `.selfBnds` is created before `P` and written after data by `data_write_order`.
   - Evidence (Rust): `src/ogawa/writer/schema/points.rs:45`, `src/ogawa/writer/schema/curves.rs:63`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcGeom/OGeomBase.h:214`
   - Impact: property header order and data write order align with C++.

11) **Data write order aligned for SubD/NuPatch**
   - Fix: assigned `data_write_order` to match C++ `set()` ordering (positions/topology first, self bounds later).
   - Evidence (Rust): `src/ogawa/writer/schema/subd.rs:95`, `src/ogawa/writer/schema/nupatch.rs:96`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcGeom/OSubD.cpp:136`, `_ref/alembic/lib/Alembic/AbcGeom/ONuPatch.cpp:135`
   - Impact: data block ordering for schema-written files matches reference.

## Remaining Parity Gaps

1) **Debug-only warnings during read path** (non-binary)
   - Current: `eprintln!` warnings on child read failure in debug builds.
   - Reference: no equivalent logging.
   - Evidence (Rust): `src/ogawa/abc_impl.rs:276`, `src/ogawa/abc_impl.rs:287`
   - Impact: behavior-only (stdout/stderr), not file format.

2) **Binary parity still fails on larger meshes**
   - Evidence: `data/Abc/heart.abc` (50 bytes differ), `data/Abc/chess3.abc` (large diff).
   - Impact: copy2 output not byte-identical on these assets; likely remaining ordering or schema mapping mismatch.

3) **HDF5 Alembic files not supported**
   - Evidence: `_ref/alembic/prman/Tests/testdata/{cube,xforms}.abc` fail Ogawa magic check.
   - Impact: parity checks against HDF5 assets are blocked (requires HDF5 reader/writer).

## Notes on Murmur3
- Alembic’s `MurmurHash3_x64_128` has no seed parameter; `podSize` only affects endian swapping.
- Rust wrapper exposes an optional seed for generality, but writer uses `seed=None` for parity.

## Next Verification
- Re-run binary comparisons for more reference assets (not only `cpp_triangle.abc`).
- Confirm parity in writer output when additional schemas are present (curves, points, subd, nurbs).
