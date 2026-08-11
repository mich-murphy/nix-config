---
name: wtf
description: Restates the immediately preceding assistant response in plain, concise language when the user says "wtf", says the answer did not land, or asks for a simpler explanation. Does not answer new questions, modify code, or simplify an implementation.
---

# Re-pitch the Previous Response

Return only the rewritten response.

1. Add only the prerequisite context needed to understand the answer.
2. Use plain, natural language. Briefly explain technical terms that must remain.
3. Preserve all material facts, recommendations, reasons, warnings, commands,
   code, paths, links, ordering constraints, and next steps.
4. Preserve useful structure, but keep a simple answer simple.
5. Make the response shorter where possible without losing important
   information.

Do not answer a new question, take an action, or mention this skill.
