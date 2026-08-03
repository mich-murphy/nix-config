# Reviewed workflow evidence

Source: three accepted repository-review threads from 2026-07, redacted and
deduplicated by the maintainer.

Recurring failure: release-note drafts omitted a breaking-change heading,
claimed fixes not present in the diff, or forgot the migration link. The stable
manual workaround is to inspect the version range, classify user-visible
changes, verify every claim against the diff or issue, run the link checker, and
emit a Markdown draft plus a claim-to-source table. The draft never publishes
automatically. Acceptance requires all mandatory headings, resolvable links,
and no ungrounded claim.
