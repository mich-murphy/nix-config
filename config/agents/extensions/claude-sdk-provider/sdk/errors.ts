/** Error produced when a deferred Pi tool request is malformed or names an unavailable tool. */
export class InvalidDeferredCallError extends Error {
  /** Stable error discriminator. */
  readonly _tag = "InvalidDeferredCallError" as const;

  /**
   * Create an invalid deferred-call error.
   *
   * @param requestedName - The requested inner Pi tool name, or an empty string when absent.
   * @param reason - A safe explanation suitable for the SDK permission response.
   */
  constructor(
    readonly requestedName: string,
    reason: string,
  ) {
    super(reason);
    this.name = "InvalidDeferredCallError";
  }
}

/** Error produced when an SDK message does not match the protocol shape used by this provider. */
export class SdkProtocolError extends Error {
  /** Stable error discriminator. */
  readonly _tag = "SdkProtocolError" as const;

  /**
   * Create an SDK protocol error.
   *
   * @param messageType - The safe SDK message or event type being parsed.
   * @param detail - A description that does not include prompt or credential data.
   */
  constructor(
    readonly messageType: string,
    detail: string,
  ) {
    super(`Malformed Claude Agent SDK ${messageType}: ${detail}`);
    this.name = "SdkProtocolError";
  }
}

/** Error returned by a terminal SDK result. */
export class SdkResultError extends Error {
  /** Stable error discriminator. */
  readonly _tag = "SdkResultError" as const;

  /**
   * Create an SDK result error.
   *
   * @param terminalReason - The SDK terminal reason when one was supplied.
   * @param detail - The SDK's safe error summary.
   */
  constructor(
    readonly terminalReason: string | undefined,
    detail: string,
  ) {
    super(detail);
    this.name = "SdkResultError";
  }
}

/** Error produced when the model exceeds the invalid deferred-call retry limit. */
export class InvalidDeferredCallLimitError extends Error {
  /** Stable error discriminator. */
  readonly _tag = "InvalidDeferredCallLimitError" as const;

  /**
   * Create an invalid-call limit error.
   *
   * @param attempts - Number of invalid calls observed during the turn.
   * @param lastError - The final invalid call.
   */
  constructor(
    readonly attempts: number,
    readonly lastError: InvalidDeferredCallError,
  ) {
    super(`Claude exceeded the invalid Pi tool-call limit after ${attempts} attempts: ${lastError.message}`);
    this.name = "InvalidDeferredCallLimitError";
  }
}

function safeCauseSummary(cause: unknown): string {
  const summary = cause instanceof Error ? cause.message : String(cause);
  return summary.slice(0, 1_000);
}

/** Error produced when the SDK query rejects or ends without a terminal result. */
export class SdkQueryError extends Error {
  /** Stable error discriminator. */
  readonly _tag = "SdkQueryError" as const;

  /**
   * Create an SDK query error.
   *
   * @param operation - Query phase that failed.
   * @param cause - Original SDK rejection or failure value.
   */
  constructor(
    readonly operation: "start" | "iterate" | "terminal-result",
    override readonly cause: unknown,
  ) {
    super(`Claude Agent SDK query failed during ${operation}: ${safeCauseSummary(cause)}`, { cause });
    this.name = "SdkQueryError";
  }
}

/** Expected failures that can terminate one provider turn. */
export type SdkRunError =
  | InvalidDeferredCallLimitError
  | SdkProtocolError
  | SdkQueryError
  | SdkResultError;
