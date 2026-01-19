# Plan 2 - Writer Parity + Modularization

## Goal
Full parity of Ogawa writer/reader with C++ AbcCoreOgawa, then modularize writer for maintainability.

## Steps
1) Inventory and parity verification (reader/writer vs _ref): **DONE**
2) Apply parity fixes (string terminators, maxSamples for constant, header flags, _ai_AlembicVersion): **DONE**
3) Update docs/report (findings.md, AGENTS.md, DIAGRAMS.md): **DONE**
4) Modularize writer (split `archive.rs` into focused modules, keep API stable): **PENDING**
5) Recheck copy2 procedure vs C++ behavior; update findings: **PENDING**
6) Final binary verification with `ALEMBIC_BUILD_DATE/TIME`: **PENDING**

## Notes
- Remaining parity gap: `_ai_AlembicVersion` string uses env vars; set them to match reference build.
- Keep function naming aligned with C++ where possible.
