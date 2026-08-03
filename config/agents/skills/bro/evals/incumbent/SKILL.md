---
name: bro
description: Restate the immediately preceding assistant response in plain, concise human language without jargon while preserving important details and useful formatting. Use when the user says "bro", asks for a simpler explanation, or wants a technical answer made easier to read.
---

# Restate the Previous Response

Rewrite the immediately preceding assistant response for the same audience and
purpose.

## Route Model and Effort

Route this mechanical transformation to an efficient model at low effort.
Escalate to a balanced model at medium effort only when the source is ambiguous
or contains conflicting, consequential details. Verify fidelity against the
source; extra analysis is not an improvement.

1. Simplify the language, not the information. Use plain, natural words and
   briefly explain any technical term that must remain. Remove repetition, but
   keep material facts, caveats, decisions, reasons, commands, code, links,
   file paths, warnings, and next steps.
2. Preserve the meaning. Do not add claims, change the recommendation, weaken a
   warning, or omit a detail merely because it is technical.
3. Preserve useful information design from the original response. Keep
   headings, lists, tables, code fences, inline code, links, visible warnings,
   and emphasis when they still help. Add or change semantic Markdown when it
   makes the rewrite clearer:
   - use short headings when the response has distinct sections;
   - use numbered lists for sequences and bullet lists for unordered items;
   - keep or use a table for comparisons, mappings, or repeated fields unless a
     different layout is clearly easier to understand; and
   - use bold text sparingly to emphasize decisions or important labels.
4. Match the amount of formatting to the information. Do not flatten structured
   content into one paragraph, and do not turn a simple answer into a decorated
   document.
5. Return only the rewritten response. Do not mention this skill or explain the
   rewrite.
