# Нерешённые проблемы alembic-rs

**Дата:** 2026-01-09
**Статус:** После Bug Hunt сессии

---

## Проблемы, которые были ЗАДОКУМЕНТИРОВАНЫ, но НЕ ИСПРАВЛЕНЫ

Эти проблемы требуют архитектурных изменений или являются фундаментальными ограничениями:

### 1. #20: Mmap Safety - ТОЛЬКО ДОКУМЕНТАЦИЯ

**Файл:** `src/ogawa/reader.rs`

**Проблема:** Memory-mapped файлы могут вызвать undefined behavior если внешний процесс модифицирует файл во время чтения (SIGBUS, corrupt data).

**Что сделано:** Добавлена документация о рисках и workaround (use_mmap=false).

**Возможное решение:**
- Добавить `advisory lock` при открытии файла
- Реализовать copy-on-read для критических данных
- Добавить runtime-проверки целостности (checksums)

---

### 2. #35: O(n) Metadata Search - ТОЛЬКО ДОКУМЕНТАЦИЯ

**Файл:** `src/core/metadata.rs`

**Проблема:** Линейный поиск O(n) по metadata entries.

**Что сделано:** Документировано как "приемлемо для малых n".

**Почему это ОК:** n обычно 2-6 записей, HashMap overhead доминирует.

**Возможное решение (если нужно):**
- При n > 8 автоматически конвертировать в HashMap
- Кэшировать частые lookups

---

### 3. #36: Race Condition in Cache Insert - ТОЛЬКО ДОКУМЕНТАЦИЯ

**Файл:** `src/core/cache.rs`

**Проблема:** Cache может временно превысить max_size из-за race condition между check и insert.

**Что сделано:** Документировано как "intentional heuristic".

**Возможное решение:**
- Использовать RwLock для атомарного check-and-insert
- Использовать concurrent hashmap (dashmap)
- Мягкий лимит с периодической cleanup

---

### 4. #37: parent() Not Implemented - АРХИТЕКТУРНОЕ ОГРАНИЧЕНИЕ

**Файл:** `src/ogawa/abc_impl.rs`, `src/core/traits.rs`

**Проблема:** `parent()` всегда возвращает None из-за Rust ownership constraints.

**Что сделано:** Документировано ограничение, добавлен workaround через `full_name()`.

**Почему это сложно:**
```rust
// Возврат &dyn ObjectReader требует self-referential struct
// или Arc<Mutex<>> overhead на каждом объекте
fn parent(&self) -> Option<&dyn ObjectReader>  // lifetime проблемы!
```

**Возможные решения:**
1. **Arena allocator:** Все объекты в arena, возвращать индексы
2. **Arc<Object>:** Wrap all objects in Arc
3. **Path-based API:** Вместо parent() - `archive.parent_of(path)`
4. **Unsafe:** Self-referential struct с pin

---

## Проблемы, которые НЕ БЫЛИ ЗАТРОНУТЫ

### #38-40: Minor Python Issues

**#38: Unused #[allow(non_snake_case)]**
- PyO3 требует camelCase для Python API compatibility
- ВЫВОД: Это не баг, это необходимость

**#39: Clone-heavy Iterator Design**

**Файл:** `src/python/object.rs` (предположительно)

**Проблема:** Итераторы клонируют данные вместо возврата references.

**Решение:**
- Использовать `&self` где возможно
- Для Python это часто неизбежно (GIL, reference counting)

**#40: Arc Cloning on Every Traversal**

**Проблема:** При обходе иерархии каждый child создаёт новый Arc clone.

**Решение:**
- Использовать weak references где возможно
- Кэшировать children при первом доступе
- Принять как cost of Python bindings safety

---

### #41-45: Documentation/Style Issues

**#41:** Dead code attribute на `inner` field
- Intentional - поле нужно для lifetime

**#42:** Inconsistent naming (camelCase vs snake_case)
- Python API требует camelCase
- Rust API использует snake_case
- Это правильно

**#43:** Missing `hasChildren()` convenience method
- Легко добавить: `fn has_children(&self) -> bool { self.num_children() > 0 }`

**#44-45:** Minor doc improvements
- Low priority

---

## Code Duplication (Phase 3 из плана)

Это не баги, а technical debt:

### Основные места дублирования:

1. **`.geom` Property Access Pattern** (~300 строк)
   - Решение: `fn get_geom_compound(&self) -> Result<ICompoundProperty>`

2. **arb_geom_params / user_properties** (~360 строк)
   - Решение: `trait GeomSchemaExt` с default implementations

3. **self_bounds Reading** (~105 строк)
   - Решение: `fn read_self_bounds(geom, index) -> Option<BBox3d>`

4. **positions Reading** (~100 строк)
   - Решение: общая утилита

5. **compute_bounds()** (~75 строк)
   - Решение: generic implementation

6. **collect_bounds_recursive** (~300 строк)
   - Два почти идентичных варианта

**Общая экономия:** ~1500-2000 строк

---

## Incomplete O* Output Structs (Phase 4)

**Файлы:** `src/ogawa/writer.rs`, `src/python/write.rs`

**Проблема:** OPolyMesh, OXform и др. - минимальные заглушки.

**Что работает:**
- Базовое создание архива
- Запись иерархии
- Базовые property types

**Что НЕ работает или неполно:**
- Animated properties (time sampling output)
- Arbitrary geometry parameters
- User properties
- Face sets output
- UV sets output
- Subdivision schemes parameters

---

## Рекомендации по приоритетам

### Высокий приоритет (если проект активно используется):
1. Code deduplication - уменьшит bug surface
2. O* structs completion - для полной read/write roundtrip

### Средний приоритет:
3. Python iterator optimization (#39-40)
4. Mmap advisory locking (#20)

### Низкий приоритет:
5. parent() implementation - workaround достаточен
6. Documentation issues (#41-45)

---

## Что было ДЕЙСТВИТЕЛЬНО исправлено в этой сессии

Для справки, эти проблемы были реально исправлены (код изменён):

- #1 MIN_ALEMBIC_VERSION: 9999 → 10709
- #2 TimeSampling: _times parameter теперь используется
- #3 Compression: ошибки propagate корректно
- #4 bytemuck: try_cast_slice вместо cast_slice
- #5 addVisibilityProperty: теперь действительно добавляет
- #6 std::mem::replace: исправлен паттерн
- #7-9: Различные исправления парсинга
- #10-13: Float comparison, static mut, face_set, zlib
- #14-19: Hash, _num_samples, unreachable
- #21: LRU eviction implemented
- #22: Dynamic buffer size
- #23-24: Bounds и visibility improvements  
- #25: ILight - все 16 camera params
- #26-32: Interface consistency (topology_variance, has_self_bounds)
- #33-34: Lock messages, float comparison
