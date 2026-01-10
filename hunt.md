# Bug Hunt:

## Prereqs: 
  - Answer in Russian in chat, write English code and .md files.
  - MANDATORY: Use filesystem MCP to work with files, memory MCP to remember, log things and create relations and github MCP or "gh" tool if needed. 
  - Use sub-agents and work in parallel.

## Workflow:
  - Файлы записанные этим крейтом не открываются в Blender. Попробуй выяснить почему. Где-то ошибки в записи.

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


## Outputs:
  - At the end create a professional comprehensive report and update plan and write it to planN.md where N is the next available number, and wait for approval! 
  - Also create or fix existing AGENTS.md with dataflow and codepath diagrams.