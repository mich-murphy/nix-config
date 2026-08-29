import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";

type ExitListener = (code: number | null, signal: NodeJS.Signals | null) => void;

type Result<T, E extends Error> =
  | { readonly _tag: "ok"; readonly value: T }
  | { readonly _tag: "err"; readonly error: E };

/** A classified failure from the Node child-process adapter. */
export class CaffeinateProcessError extends Error {
  /** Stable error discriminator. */
  readonly _tag = "CaffeinateProcessError" as const;

  /** Process operation that failed. */
  readonly operation: "spawn" | "runtime" | "SIGTERM" | "SIGKILL";

  /** Unclassified error received from Node. */
  override readonly cause: unknown;

  /**
   * Create a classified caffeinate process failure.
   *
   * @param operation - Process operation that failed.
   * @param cause - Unclassified error received from Node.
   */
  constructor(
    operation: "spawn" | "runtime" | "SIGTERM" | "SIGKILL",
    cause: unknown,
  ) {
    super(`caffeinate process failed during ${operation}`);
    this.operation = operation;
    this.cause = cause;
  }
}

/** Result returned by caffeinate process adapters. */
export type CaffeinateProcessResult<T> = Result<T, CaffeinateProcessError>;

/** Process operations required by the no-sleep runtime. */
export type CaffeinateProcess = {
  /** Return whether the child has exited. */
  hasExited(): boolean;
  /** Allow Pi to exit independently of the child-process handle. */
  unref(): void;
  /** Send a termination signal to the child. */
  kill(signal: "SIGTERM" | "SIGKILL"): CaffeinateProcessResult<boolean>;
  /** Subscribe once to a classified child-process error. */
  onError(listener: (error: CaffeinateProcessError) => void): void;
  /** Subscribe once to child exit and return a listener-removal function. */
  onExit(listener: ExitListener): () => void;
};

/** Runtime and process dependencies used by the no-sleep lifecycle. */
export type NoSleepDependencies = {
  /** Current operating-system platform. */
  readonly platform: NodeJS.Platform;
  /** PID that caffeinate watches for crash-safe cleanup. */
  readonly processId: number;
  /** Time allowed for each graceful or forced termination attempt. */
  readonly terminationTimeoutMs: number;
  /** Start caffeinate with the supplied command arguments. */
  spawnCaffeinate(args: ReadonlyArray<string>): CaffeinateProcessResult<CaffeinateProcess>;
};

class CaffeinateStopTimeout extends Error {
  readonly _tag = "CaffeinateStopTimeout" as const;

  constructor() {
    super("caffeinate did not exit after SIGKILL");
  }
}

type StopResult = Result<void, CaffeinateProcessError | CaffeinateStopTimeout>;

const ok: StopResult = { _tag: "ok", value: undefined };

function notify(
  ctx: ExtensionContext,
  message: string,
  level: "warning" | "error",
): void {
  if (ctx.hasUI) ctx.ui.notify(message, level);
}

function waitForExit(child: CaffeinateProcess, timeoutMs: number): Promise<boolean> {
  if (child.hasExited()) return Promise.resolve(true);

  return new Promise((resolve) => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const removeExitListener = child.onExit(() => {
      if (timer !== undefined) clearTimeout(timer);
      resolve(true);
    });
    timer = setTimeout(() => {
      removeExitListener();
      resolve(child.hasExited());
    }, timeoutMs);
  });
}

async function stopProcess(
  child: CaffeinateProcess,
  timeoutMs: number,
): Promise<StopResult> {
  if (child.hasExited()) return ok;

  const terminate = child.kill("SIGTERM");
  if (terminate._tag === "err" && !child.hasExited()) return terminate;

  if (await waitForExit(child, timeoutMs)) return ok;

  const kill = child.kill("SIGKILL");
  if (kill._tag === "err" && !child.hasExited()) return kill;

  if (await waitForExit(child, timeoutMs)) return ok;
  return { _tag: "err", error: new CaffeinateStopTimeout() };
}

/**
 * Register the no-sleep lifecycle using an injected caffeinate process adapter.
 *
 * @param pi - Pi extension API used to subscribe to lifecycle events.
 * @param dependencies - Platform and process operations owned by this extension runtime.
 */
export function registerNoSleep(
  pi: ExtensionAPI,
  dependencies: NoSleepDependencies,
): void {
  let agentActive = false;
  let caffeinate: CaffeinateProcess | undefined;
  let unexpectedRestartUsed = false;
  const stopping = new Set<CaffeinateProcess>();

  function start(ctx: ExtensionContext): void {
    if (
      dependencies.platform !== "darwin"
      || caffeinate
      || stopping.size > 0
    ) return;

    const spawned = dependencies.spawnCaffeinate([
      "-d",
      "-i",
      "-s",
      "-w",
      String(dependencies.processId),
    ]);
    if (spawned._tag === "err") {
      const detail = spawned.error.cause instanceof Error ? `: ${spawned.error.cause.message}` : "";
      notify(ctx, `No Sleep could not start caffeinate${detail}`, "error");
      return;
    }

    const child = spawned.value;
    caffeinate = child;
    child.unref();
    child.onError((error) => {
      if (caffeinate !== child) return;
      caffeinate = undefined;
      const detail = error.cause instanceof Error ? `: ${error.cause.message}` : "";
      notify(ctx, `No Sleep caffeinate failed${detail}`, "error");
    });
    child.onExit((code, signal) => {
      if (stopping.delete(child)) {
        if (agentActive && !caffeinate && stopping.size === 0) start(ctx);
        return;
      }
      if (caffeinate !== child) return;

      caffeinate = undefined;
      if (!agentActive) return;

      const outcome = signal === null ? `exit code ${code ?? "unknown"}` : `signal ${signal}`;
      notify(ctx, `No Sleep lost caffeinate unexpectedly (${outcome})`, "warning");
      if (!unexpectedRestartUsed) {
        unexpectedRestartUsed = true;
        start(ctx);
      }
    });
  }

  async function stop(ctx: ExtensionContext): Promise<void> {
    const child = caffeinate;
    caffeinate = undefined;
    if (!child) return;

    stopping.add(child);
    const result = await stopProcess(child, dependencies.terminationTimeoutMs);
    if (result._tag === "ok") {
      stopping.delete(child);
      return;
    }

    const detail = result.error instanceof CaffeinateProcessError
      && result.error.cause instanceof Error
      ? `: ${result.error.cause.message}`
      : "";
    notify(ctx, `No Sleep ${result.error.message}${detail}`, "error");
  }

  pi.on("agent_start", (_event, ctx) => {
    agentActive = true;
    unexpectedRestartUsed = false;
    start(ctx);
  });

  pi.on("agent_settled", async (_event, ctx) => {
    if (!ctx.isIdle()) return;
    agentActive = false;
    unexpectedRestartUsed = false;
    await stop(ctx);
  });

  pi.on("session_shutdown", async (_event, ctx) => {
    agentActive = false;
    unexpectedRestartUsed = false;
    await stop(ctx);
  });
}
