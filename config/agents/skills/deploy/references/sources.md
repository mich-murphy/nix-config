# Evidence and Provenance

This package incorporates the relevant synthesis directly. Its primary sources
are:

- Gene Kim et al.,
  [*The DevOps Handbook* excerpt](https://itrevolution.com/wp-content/uploads/2022/06/DOHB2_Excerpt.pdf),
  for flow, feedback, continual learning, reliability, and security.
- DORA on [continuous delivery](https://dora.dev/capabilities/continuous-delivery/),
  [small batches](https://dora.dev/capabilities/working-in-small-batches/),
  [monitoring and observability](https://dora.dev/capabilities/monitoring-and-observability/),
  and [pervasive security](https://dora.dev/capabilities/pervasive-security/).
- Google SRE's
  [canarying guidance](https://sre.google/workbook/canarying-releases/), for
  progressive exposure, health signals, and recovery decisions.

DORA evidence is observational and context-dependent. It supports local trials
of delivery capabilities, not a universal causal claim that a particular tool
or deployment frequency improves outcomes. The agent deployment contract is a
low-certainty synthesis.

Measure deployment success and rework, change fail rate, recovery behavior,
user and reliability outcomes, human burden, and escaped incidents. Do not use
deployment count as an individual or agent productivity target.
