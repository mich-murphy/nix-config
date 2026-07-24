# Reference

## Dependency Categories

When assessing candidate for deepening, classify
dependencies:

### 1. In-process

Pure computation, in-memory state, no I/O. Always
deepenable — merge modules and test directly.

### 2. Local-substitutable

Dependencies that have local test stand-ins (e.g.,
PGLite for Postgres, in-memory filesystem). Deepenable
if test substitute exists. Deepened module tested with
local stand-in running in test suite.

### 3. Remote but owned (Ports & Adapters)

Your own services across network boundary
(microservices, internal APIs). Define port (interface)
at module boundary. Deep module owns logic; transport
is injected. Tests use in-memory adapter. Production
uses real HTTP/gRPC/queue adapter.

Recommendation shape: "Define shared interface (port),
implement HTTP adapter for production and in-memory
adapter for testing, so logic can be tested as one
deep module even though deployed across network
boundary."

### 4. True external (Mock)

Third-party services (Stripe, Twilio, etc.) you don't
control. Mock at boundary. Deepened module takes
external dependency as injected port, tests provide
mock implementation.

## Testing Strategy

Core principle: **replace, don't layer.**

- Old unit tests on shallow modules are waste once
  boundary tests exist — delete them
- Write new tests at deepened module's interface
  boundary
- Tests assert on observable outcomes through public
  interface, not internal state
- Tests should survive internal refactors — they describe
  behavior, not implementation
