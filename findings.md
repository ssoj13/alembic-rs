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
   - Evidence (Rust): `src/ogawa/writer/archive.rs:894`, `src/ogawa/writer/archive.rs:1013`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcCoreOgawa/SpwImpl.cpp:53`, `_ref/alembic/lib/Alembic/AbcCoreOgawa/ApwImpl.cpp:53`
   - Impact: Time sampling tables now match constant-property behavior.

3) **Property header first/last change flags use state-derived counts**
   - Fix: `build_property_info` uses the same `num_samples` for info and comparisons.
   - Evidence (Rust): `src/ogawa/writer/archive.rs:1146`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcCoreOgawa/WriteUtil.cpp:290`
   - Impact: Header flags match when samples repeat.

4) **_ai_AlembicVersion is always set (like C++)**
   - Fix: writer now sets the key unconditionally during archive write.
   - Evidence (Rust): `src/ogawa/writer/archive.rs:587`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcCoreOgawa/AwImpl.cpp:90`
   - Impact: Metadata presence now matches C++ behavior.

## Remaining Parity Gaps

1) **_ai_AlembicVersion build date/time differs when env vars are missing**
   - Current: uses `ALEMBIC_BUILD_DATE/TIME`, falls back to `unknown`.
   - Reference: uses compile-time `__DATE__`/`__TIME__`.
   - Evidence (Rust): `src/ogawa/writer/write_util.rs:75`
   - Reference (C++): `_ref/alembic/lib/Alembic/AbcCoreAbstract/Foundation.cpp:44`
   - Impact: Metadata string mismatch when env vars are unset.
   - Fix options: set env vars in build, or inject fixed build time when parity tests run.

2) **Debug-only warnings during read path** (non-binary)
   - Current: `eprintln!` warnings on child read failure in debug builds.
   - Reference: no equivalent logging.
   - Evidence (Rust): `src/ogawa/abc_impl.rs:276`, `src/ogawa/abc_impl.rs:287`
   - Impact: behavior-only (stdout/stderr), not file format.

## Notes on Murmur3
- Alembic’s `MurmurHash3_x64_128` has no seed parameter; `podSize` only affects endian swapping.
- Rust wrapper exposes an optional seed for generality, but writer uses `seed=None` for parity.

## Next Verification
- Re-run binary comparisons after setting `ALEMBIC_BUILD_DATE/TIME`.
- Confirm no diffs in metadata and tail regions for reference outputs.
