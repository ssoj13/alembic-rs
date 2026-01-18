# Binary Parity Investigation Findings (Verified)

## Status

Полная перепроверка writer/copy против референса AbcCoreOgawa выполнена. Ниже — полный список подтверждённых расхождений. Исправления применены, верификация будет после завершения текущего этапа.

## Критичные расхождения

1) **Digest семплов не совпадает с референсом**
   - Текущее: MurmurHash3 x64_128 без seed (POD size), `ArraySampleContentKey::from_data`.
   - Референс: `MurmurHash3_x64_128(data, len, PODNumBytes)`.
   - Evidence:
     - `src/core/cache.rs:18`
     - `_ref/alembic/lib/Alembic/AbcCoreAbstract/ArraySample.cpp:73`
   - Влияние: ключи data, property hashes, object hashes, dedup — все отличаются.

2) **Строковые/Wstring данные пишутся без нулевых терминаторов**
   - Текущее: `as_bytes()` без `\0`.
   - Референс: строки сериализуются с `\0` между элементами.
   - Evidence:
     - `src/ogawa/writer.rs:3669`
     - `_ref/alembic/lib/Alembic/AbcCoreAbstract/ArraySample.cpp:87`
   - Влияние: payload и digest отличаются.

## Высокие расхождения

3) **`isHomogenous` вычисляется неверно**
   - Текущее: `false` для массивов с extent > 1.
   - Референс: сбрасывается только при изменении `dims.numPoints()` между семплами.
   - Evidence:
     - `src/ogawa/writer.rs:1455`
     - `_ref/alembic/lib/Alembic/AbcCoreOgawa/ApwImpl.cpp:198`

4) **Инициализация `first_changed_index/last_changed_index`**
   - Текущее: `first=1, last=0` для scalar/array.
   - Референс: `first=0, last=0`.
   - Evidence:
     - `src/ogawa/writer.rs:1802`
     - `_ref/alembic/lib/Alembic/AbcCoreOgawa/Foundation.h:71`

5) **`first_changed_index/last_changed_index` вычисляются только по количеству семплов**
   - Текущее: при `n>1` всегда `first=1, last=n-1`, даже если семплы повторяются.
   - Референс: индексы обновляются только при смене key, повторения не двигают `lastChangedIndex`.
   - Evidence:
     - `src/ogawa/writer.rs:1874`
     - `_ref/alembic/lib/Alembic/AbcCoreOgawa/SpwImpl.cpp:163`
     - `_ref/alembic/lib/Alembic/AbcCoreOgawa/ApwImpl.cpp:167`

6) **Copy-процедуры не копируют произвольные/array/compound свойства**
   - Текущее: схемы копируются, arbitrary свойства теряются (включая root).
   - Evidence:
     - `src/bin/alembic/main.rs:866`
     - `tests/write_tests.rs:498`
     - `tests/copy_heart_test.rs:94`

## Средние расхождения

7) **indexed metadata capacity off-by-one**
   - Текущее: inline при `len >= 254`.
   - Референс: разрешено 254 entries + empty.
   - Evidence:
     - `src/ogawa/writer.rs:358`
     - `_ref/alembic/lib/Alembic/AbcCoreOgawa/MetaDataMap.cpp:41`

8) **Нестабильная сортировка при `data_write_order` tie**
   - Текущее: `sort_by_key` (unstable), может менять порядок.
   - Evidence:
     - `src/ogawa/writer.rs:922`

9) **`isScalarLike` для array по умолчанию неверен**
   - Текущее: array создаётся с `is_scalar_like = false`, флаг никогда не становится true.
   - Референс: `PropertyHeaderAndFriends` стартует `isScalarLike = true`, сбрасывается при `dims.numPoints() != 1`.
   - Evidence:
     - `src/ogawa/writer.rs:1820`
     - `_ref/alembic/lib/Alembic/AbcCoreOgawa/Foundation.h:79`
     - `_ref/alembic/lib/Alembic/AbcCoreOgawa/ApwImpl.cpp:192`

10) **`_ai_AlembicVersion` захардкожен**
   - Текущее: фиксированная строка.
   - Референс: `GetLibraryVersion()` (build date/time).
   - Evidence:
     - `src/ogawa/writer.rs:678`
     - `_ref/alembic/lib/Alembic/AbcCoreAbstract/Foundation.cpp:65`

## Корневая причина для heart.abc

Root object содержит свойства `.childBnds`, `statistics`, `1.samples`, которые не копируются.
Это даёт ~77% совпадения и размер меньше на ~78 байт.

## Рекомендация по исправлениям (кратко)

- Привести digest/encoding к референсу (seed + string/wstring encoding).
- Выравнять property header flags (isHomogenous, first/last changed).
- Сделать полноценный copy дерева свойств (compound/scalar/array).
- Исправить indexed metadata capacity и стабильность сортировки.
- `_ai_AlembicVersion` брать из build-time или из исходного метаданных.
