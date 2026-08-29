---
name: typescript
description: Correct-by-construction TypeScript standards. Use when creating or editing typescript files.
---
## Decision order

When rules conflict:

1. Preserve correctness, safety, and debuggability.
2. Apply these standards to new code and the full behavior under refactor.
3. Follow compatible project architecture and conventions.
4. Contain, rather than copy, incompatible patterns at the nearest boundary.
5. Leave unrelated old code unchanged unless a broader migration was requested.
6. Record meaningful trade-offs in comments or ADRs.

Before adding a pattern, library, or abstraction, inspect existing error, schema, injection, test, observability, and layout conventions. Preserve telemetry and error-reporting hooks; contain weaker patterns at the nearest boundary.

## Core rules

Make illegal states unrepresentable where practical. Prefer correct-by-construction APIs, refined types, composition over inheritance, functional cores, deep cohesive modules, real test seams, and discoverable code.

## Errors, telemetry, and secrets

Every known failure mode should appear as a custom tagged error in the return type, even without caller recovery. These include domain, parsing, authorization, integration, I/O, persistence, configuration, and workflow failures. Callers handle or return them; outer boundaries translate them to HTTP, CLI, retry, dead-letter, or startup outcomes.

Prefer Effect's native conventions in Effect codebases, then `better-result` when available and appropriate, else a small local tagged union. Expected errors generally extend `Error`, `better-result`'s `TaggedError`, or Effect's `Schema.TaggedErrorClass`.

```ts
type Result<T, E extends Error> =
  | { readonly _tag: "ok"; readonly value: T }
  | { readonly _tag: "err"; readonly error: E };
```

Return `Promise<Result<User, UserLookupError>>`; ordinary lookup or storage failures must not reject. Rejection is throwing. The owning Adapter must translate unclassified third-party rejections into known tagged errors before they leave it. Only defects may throw or reject from application code.

Only defects preventing correct execution may throw or panic: violated invariants, impossible branches, temporary `notYetImplemented` paths, or catastrophic runtime conditions. The absence of a recovery path does not make a known failure a defect. Config failures remain values the root reports safely. Use established shared defect helpers, or the project's result-library panic helper: `casesHandled(unexpectedCase: never)` for exhaustive unions, `shouldNeverHappen`, and `notYetImplemented`. Avoid one-off `assertNever` or `absurd` variants when shared helpers exist. Framework-required throws stay in the owning Adapter or composition-root binding.

Expected errors should use the library's tagged convention; local errors extend `Error`. They should have a stable `_tag` with `as const`, useful message, structured safe context and telemetry, and optional `cause: unknown`.

```ts
class UserStoreUnavailable extends Error {
  readonly _tag = "UserStoreUnavailable" as const;

  constructor(
    readonly operation: "findActiveByEmail",
    readonly provider: "postgres",
    readonly cause?: unknown,
  ) {
    super(`User store unavailable during ${operation}`);
  }
}
```

Keep causes only when safe; never log or serialize an unclassified cause. Use precise boundary unions such as `Result<User, UserNotFound | UserStoreUnavailable>`. Avoid broad `AppError` types except near entrypoints, orchestration, logging, or rendering.

Prefer end-to-end tracing with safe IDs, operations, providers, state/error tags, retry counts, and summaries. Never put secrets in errors, traces, logs, snapshots, or unclassified causes. Wrap secrets at ingress in Effect's `Redacted.Redacted` or shared `Redacted<T>`; unwrap only where needed, usually in the outbound Adapter.

## Parsing and domain modeling

At the first meaningful boundary, parse loose or unknown input into application/domain types. Keep protocol projections only for materially different shape or meaning. `DTO` names a prose role, never a symbol; use `CreateUserRequest` or `UserRecord`. Parse `unknown -> protocol -> application input -> domain`, or directly to application input. Never carry schema-inferred transport shapes through the application.

Name untrusted-input parsers `parseX(input): Result<X, ParseXError>`; typed-piece smart constructors `makeX` or `createX`; predicates `isX(value): value is X`; and rare test or framework assertions `assertX`. Do not call a refined-value function `validateX`.

Use schemas as boundary parsers, not core validators. Prefer, in order: the established library; Effect Schema in Effect codebases; Zod 4 otherwise. Prefer Standard Schema compatibility for generic helpers and handwritten parsers or smart constructors for small types when clearer. Produce refined/domain values and typed errors where practical.

Use branded/refined IDs, strings, numbers, and units when they prevent realistic misuse or invalid construction. Construct them through parsers or smart constructors; do not pass raw primitives where a domain type exists. Push optionality outward; branch or parse before required-value calls. Avoid `Partial<T>` for application/domain input unless partiality is the domain concept. Define operation-specific inputs.

Represent meaningful lifecycles with tagged unions or equivalent value classes:

```ts
type Invoice =
  | { readonly _tag: "Draft"; readonly id: InvoiceId }
  | { readonly _tag: "Sent"; readonly id: InvoiceId; readonly sentAt: Instant };
```

Avoid booleans plus optional state, such as `isSent: boolean` with `sentAt?: Date`, and boolean parameters that control behavior; prefer `{ emailVerification: "skip" }`. Booleans are fine for predicates such as `isExpired`.

## Modules and boundaries

Domain, Application Service, and Adapter Modules name responsibilities, not required directories, suffixes, or constructs. They may be functions, objects, classes, files, or packages. Do not add needless layers: a simple boundary may call an Application Service without inventing a Domain Module.

The normal flow is:

```txt
external input -> inbound Adapter -> Application Service -> Domain Module
                                           +-> application-owned port
                                                 -> outbound Adapter -> external system
```

An inbound Adapter may call a Domain Module directly only for a pure operation with no authorization, application policy, persistence, external calls, or effect sequencing. The root builds and injects concrete Adapters. Dependencies point inward: Domain Modules know neither services nor Adapters; services know application-owned ports, not technologies; Adapters implement ports and translate at edges.

Classify code by why it changes:

- Business meaning, invariant, calculation, or legal transition: Domain Module.
- Application policy, authorization enforcement, or effect order: Application Service.
- Framework, protocol, database, runtime, serialization, process, or provider mechanics: Adapter.
- Construction, configuration, resource acquisition, or wiring: composition root.

Split abstractions with multiple change reasons, not for the taxonomy. Trace each caller-visible operation through every effect; refactor its full behavior and modified dependencies. Put meanings and transitions in Domain Modules, policy and order in services, translation in Adapters, and wiring at the root. At each public seam, verify domain results, application outcomes, and boundary records/responses. Use project layout and vocabulary; contain out-of-scope mixed code at an Adapter seam.

In password reset, `EmailAddress` and `ResetToken` are Domain Modules, `PasswordReset` the Application Service, an HTTP route the inbound Adapter, Postgres and email implementations outbound Adapters, and bootstrap the wiring.

A deep module hides substantial behavior, invariants, policy, sequencing, or translation behind a cohesive interface. Avoid forwarders, table mirrors, API renames, or exposed steps. Deleting a useful module spreads complexity; deleting pass-through waste removes it. Low burden does not mean few functions.

### Domain Modules

A Domain Module is a pure, type-centered abstract data type for one type or related family. Use one for a meaningful distinction, invariant, calculation, decision, or lifecycle; keep a primitive or local function when it prevents no misuse and centralizes no rule.

Keep applicable types, parsers, smart constructors, combinators, predicates, legal transitions, projections, and formatting together. Constructors return refined values and precise failures. Remain deterministic, without I/O, frameworks, persistence, ambient time, randomness, or global mutation.

It may decide permissions over parsed values. It should not authenticate, gather authorization context, enforce authorization during an operation, order effects, access storage or networks, or expose transport or persistence types. Callers use its operations, not copied checks or branding casts.

A compact functional API is often enough:

```ts
/** A parsed, normalized email address. */
export type EmailAddress = Brand<string, "EmailAddress">;
/** Parse untrusted input. */
export function parse(input: string): Result<EmailAddress, InvalidEmailAddress>;
/** Render an address. */
export function toString(email: EmailAddress): string;
```

Plain functions, immutable value classes, and cohesive static-style classes are acceptable. Classes require smart construction, unconstructable invalid instances, immutable values, cohesive methods, no hidden dependencies or I/O, and no inheritance.

### Application Service Modules

An Application Service owns one cohesive operation or capability. Use one to coordinate authorization, domain decisions, persistence, external calls, transactions, messages, time, IDs, telemetry, or multiple entrypoints. Call a Domain Module directly only without application policy or effect orchestration.

A service uses application or domain types and precise error unions, defines minimal ports, receives config, clocks, randomness, and other capabilities explicitly, and owns effect policy and order. It is independent of HTTP, CLI, queues, ORMs, vendor SDKs, and runtime types. It should not parse envelopes, render responses, execute SQL, translate vendor records, or duplicate domain invariants. Prefer constructor injection or Effect services, tags, and layers. Avoid per-call dependency bags.

There is no method limit. Split unrelated capabilities, change reasons, or dependency sets. Avoid vague `Manager`, `Processor`, `Helper`, or generic `UserService` unless established.

### Adapter Modules and ports

An Adapter owns boundary translation and technology mechanics across framework, protocol, serialization, process, persistence, runtime, or provider boundaries. Inbound Adapters parse requests/events/commands, call a service or eligible pure Domain Module, and project results. Outbound Adapters implement ports, translating external values/failures into application/domain types and typed errors.

Adapters own boundary schemas/projections, framework lifecycle, external-error classification, safe diagnostics, and technology mechanics. They may retry short technical failures only when safely repeatable and transparent without changing port meaning. They do not decide business eligibility, authorization policy, legal transitions, or operation order. Keep raw external types and framework-required throws in the owning Adapter or root.

A port is an application-owned contract, not an Adapter. Define it beside its consuming service, in application language, with the smallest meaningful capability and application or domain types. Structural typing lets a wider cohesive Adapter satisfy it:

```ts
type UsersForPasswordReset = {
  findActiveByEmail(email: EmailAddress): Promise<Result<ActiveUser, UserLookupError>>;
};
class PasswordReset {
  constructor(private readonly users: UsersForPasswordReset) {}
}
// PostgresUsers may also expose findById and updateProfile.
```

Avoid one-method Adapter sprawl, but allow a cohesive one-method Adapter that hides real translation or mechanics. Never add pass-through Adapters.

Before creating an Adapter or Application Service, audit existing ones and reuse Domain Modules and services. Use an existing Adapter through a narrow dependency type; extend only if the method fits its cohesive capability and change reason. Create one only if reuse or extension causes bad coupling or an accidental interface.

After the audit, create an ADR only for a lasting boundary, shared pattern, provider strategy, or deliberate exception, not routine feature Adapters or services. Name what was checked, why reuse or extension failed, and why the new boundary or pattern is cohesive.

Persistence modules are outbound Adapters or their internals. Avoid repository-per-table by default. A repository-like Adapter may represent cohesive domain persistence; it exposes meaningful domain operations and parsed domain types with typed errors, never raw rows or ORM errors. Parse rows and ORM models before application or domain code; keep SQL and ORM details in that Adapter or its internals.

### Composition and entrypoints

The composition root acquires resources, constructs Adapters, injects services, and owns wiring and framework bindings, never domain rules, application policy, or reusable translation.

Domain Modules form the functional core; in the imperative shell, services own policy and sequencing, while only Adapters own technology-specific translation and I/O.

Entrypoint Adapters should be thin protocol translators: parse, refine through Domain Modules, call a service for policy or effects, and render. Direct Domain calls must meet the rule above. Do not repeat business rules in controllers, resolvers, commands, or handlers.

Inbound Adapters verify credentials into a parsed `Principal`, `Session`, or `CommandActor`. Domain Modules may decide permissions over parsed values. Services gather context and enforce authorization during operations. Adapters map missing or invalid credentials and denials to protocol outcomes; they do not define permission policy.

## Workflows, transactions, and retries

Use calls or database transactions for simple, single-boundary operations. Use a saga or durable workflow when progress or retries must survive process loss or redelivery, or work needs long delays, compensation, resumability, timers, human approval, cross-service coordination, or multiple transaction boundaries. A short retry alone does not justify one.

Adapter retries must be short, transparent, safely repeatable, and preserve port meaning. Application-policy retries belong in Application Services. Durable retries belong in workflows and must survive crashes, delays, and redelivery. Never hold database transactions across network calls or long operations.

Every retryable, externally visible mutation or transition needs an explicit idempotency strategy: a key, unique constraint, deduplication record, guarded transition, or transactional outbox or inbox. Never assume a side effect is "probably safe."

## Testing

Add an end-to-end test whenever behavior can be exercised through its real public entrypoint in the normal test environment without unreliable third parties or unreasonable setup, runtime, or cost. Add lower-level tests for important extra cases. Prefer end-to-end tests, integration through real seams, focused or property tests for pure Domain Modules, then behavioral units.

Never use `vi.mock` or `jest.mock` for modules. Use injected contracts, Effect services or layers, a local database, simple in-memory Adapters, or fake external Adapters. Assert outcomes, persisted state, messages, responses, or fake-captured records. Avoid spies unless interaction is the only observable behavior. When SQL, schemas, or transactions matter, prefer a local database over a hand-built fake.

Use `fast-check` when properties beat examples, especially for parsers, refinements, state machines, round trips, normalization, idempotence, and lawful combinators. Use arbitraries for test data. Prefer exporting reusable arbitraries in adjacent test-support files near their Domain Module. Tests should not bypass parsers, smart constructors, or invariants.

## TypeScript safety and style

Use these strict options where practical:

- `strict: true`
- `noUncheckedIndexedAccess: true`
- `exactOptionalPropertyTypes: true`
- `noImplicitOverride: true`
- `noFallthroughCasesInSwitch: true`

Prefer readonly values and `ReadonlyArray`. Mutation is acceptable in localized imperative-shell code, performance-sensitive internals, builders, or Adapters when hidden by a precise interface.

Avoid `any`, `!`, and `as Type`; allow `as const`. Permit rare casts or `any` only in highly generic helpers, branding internals, interop boundaries, or combinators with invariants TypeScript cannot express. Every other cast needs a nearby Rust-like `// SAFETY:` comment:

```ts
// SAFETY: Parsing checked the brand; only this parser constructs EmailAddress.
return normalized as EmailAddress;
```

Rare `any` requires a targeted oxlint disable and justification, for example:

```ts
// oxlint-disable-next-line no-explicit-any -- SAFETY: Variadic parameters require any here.
type Fn = (...args: any[]) => unknown;
```

Never use `!`; branch, parse, or refine.

Prefer direct imports from the owning file; avoid `index.ts` barrels by default. Namespace imports often preserve Domain Module shape; use named imports for classes and focused helpers. Use `import type` and `export type`. Export only caller-facing symbols, never internals just for tests. Avoid TypeScript `namespace` without a compelling interop need.

Use precise names such as `email-address.ts`, not vague `utils.ts`, `helpers.ts`, `common.ts`, or `misc.ts`. Tiny ubiquitous helpers may share one explicit module only without a precise owner: defect helpers, `Redacted`, tag or broad type utilities, and local `Result` helpers. Keep only project-justified helpers; domain and application policy stay with their owner.

Do not impose file-size limits. Prefer cohesion and discovery. Split files for unrelated change reasons or when callers must understand unrelated concepts.

## Comments and JSDoc

Comments explain invariants, trade-offs, non-obvious rules, and safety claims, not obvious code. Every exported symbol and each public method or property of an exported class requires JSDoc. Document internals only when warranted. Put docs on originals; re-exports need no duplicate. Do not use `@inheritDoc`, `@inherit`, or similar tags; document each member.

Use standard JSDoc with summaries and useful `@template`, `@param`, and `@returns`:

```ts
/**
 * Map a result's success.
 *
 * @template T - Input type.
 * @template U - Output type.
 * @template E - Error type.
 * @param result - Input result.
 * @param fn - Mapper.
 * @returns Mapped success or original error.
 */
export function map<T, U, E extends Error>(result: Result<T, E>, fn: (value: T) => U): Result<U, E>;
```

Use `@throws` only for unrecoverable defects, temporary `notYetImplemented` paths, or framework-required behavior in its Adapter or composition-root binding. Never document expected typed errors as throws. For complex exported object types, document fields when useful:

```ts
/** Input required to create a user. */
export type CreateUserInput = {
  /** The actor creating the user. */
  readonly actor: AdminUser;
  /** The parsed email address for the new user. */
  readonly email: EmailAddress;
};
```

## Configuration and resources

At the root, parse startup environment/configuration into typed config with appropriate branded or redacted values. Return known config failures as tagged values. Report a useful, safe startup message and terminate; invalid config is not a defect. Never read `process.env` throughout the application.

Avoid top-level effects outside true entrypoint or bootstrap files. Modules should not start servers, open connections, read environment, register handlers, or perform I/O at import time. Bootstrap, imperative-shell code, or Effect layers explicitly own resource creation and cleanup.

Avoid mutable singletons and global state; constants and pure lookup tables are fine. Isolate framework-required singletons at their boundary. Inject `Clock` and `Random` into dependency-bearing modules; pure domain functions may accept explicit time or random values.
