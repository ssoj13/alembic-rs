# Bug Hunt:

## Prereqs: 
  - Answer in Russian in chat, write English code and .md files.
  - MANDATORY: Use filesystem MCP to work with files, memory MCP to remember, log things and create relations and github MCP or "gh" tool if needed. 
  - Use sub-agents and work in parallel.

## Do not assume and do not guess:
  - Перестань гадать пожалуйста, do not assume - проверяй ВСЁ, без пропусков, без частных выборок - ПОДРЯД, ВСЁ, ДО ЕДИНОЙ СТРОКИ - КОД, ЛОГИКУ, ВСЁ ОСТАЛЬНОЕ.
  - Не надо наугад писать код и тесты, надо просмотреть документацию последней версии и код и написать ЗНАЯ.
  - Делай всё ВНИМАТЕЛЬНО, ПЕРЕПРОВЕРЯЯ, ты пишешь под продакшн!
  - Убедись что все фиксы не точечные, а дедуплицированные, using singular code paths, reusing functions and not creating a lot of new ones.


## Workflow:
  - Check the app, try to spot some illogical places, errors, mistakes, unused and dead code and such things.
  - Check interface compatibility, all FIXME, TODO, all unfinished code - try to understand what to do with it, offer suggestions.
  - Find unused code and try to figure out why it was created. I think you haven't finished the big refactoring and lost pieces by the way.
  - Check possibilities for code deduplication and single source of ground truth about entities, items and logic in app.
  - Unify the architecture, dataflows, codepaths, deduplicate everything, simplify but keeping the logic and functionality! Do not remove features!
  - Avoid of creation of special function with long stupid names in favor of arguments: just add the optional argument to existing function if needed.
  - Do not guess, you have to be sure and produce production-grade decisions and problem solutions. Consult context7 MCP use fetch MCP to search internet.
  - Create a comprehensive dataflow for human and for yourself to help you understand the logic.
  - Before deleting "dead code" make sure it'a actually dead and not some unfinished feature we will need in the future.
  - Do not try to simplify things or take shortcuts or remove functionality, we need just the best practices: fast, compact and elegant, powerful code.
  - Do not also create multitude of a new functions without dire need: reuse existing functions and names, add arguments, some with default values (where needed). Reusability and Deduplication are The King and The King!
  - If you feel task is complex - ask questions, then just split it into sub-tasks, create a plan and follow it updating that plan on each step (setting checkboxes on what's done).
  - Don't be lazy and do not assume things, do not guess code. You need to be SURE, since you're writing a production code. Do not simplify things unless it will significantly improve the code logic.
  - Discard any compatibility issues, we don't need it.
  - Create comprehensive report so you could "survive" after context compactification, re-read it and continue without losing details. Offer pro-grade solutions.
  - Report should contain references to all places (problematic or requiring attention or explanatory) as a file/line number (pick the best format).
  - Search and look around very thoroughful, do not assume or skip anything, check literally every item, create comprehensive TODO lists and follow them precisely. This is very important for the final result. If you're not sure - CHECK.
  - If you found some specific problem: take a look around and try to check similar places for similar problems - chances if it was wrong in one place, it's wrong in some other places.


## Main TASK:
  - Это Rust-порт C++ пакетов для VFX pipelines. Референсный код тут: _ref/OpenImageIO, _ref/OpenColorIO. Задача этого порта - собрать оба пакета в один на чистом Rust, быть лучше чем C++ оригинал. Удобнее, быстрее, модульнее, расширяемее.
  - Нужно проверить соответствие этого порта с оригиналами, реализованные и нереализованные разделы и фичи. Нужно внимательно прошерстить всё и отыскать все несовпадения. Нужно составить подробные списки всего что нужно улучшить и дописать, нам нужна полная Parity с оригиналами, или быть лучше их.
  - Мы можем использовать другую логику или API если это приведёт к улучшениям и не изменит качество результата. Parity report - Это очень важно.
  - Особенно нужно проверить логику рендера, весь пайплайн и шейдера, автоматические нормали и вообще все фичи в src/viewer.


## Outputs:
  - At the end create a professional comprehensive report and update plan and write it to planN.md where N is the next available number, and wait for approval! 
  - Also create or fix existing AGENTS.md with dataflow and codepath ascii diagrams while doing the research. Create mermaid diagrams in DIAGRAMS.md as well.