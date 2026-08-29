import { describe, expect, test } from "bun:test";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { CaffeinateProcessError, registerNoSleep } from "../no-sleep-lifecycle";

type TestHandler = (event: unknown, context: unknown) => unknown | Promise<unknown>;
type ExitListener = (code: number | null, signal: NodeJS.Signals | null) => void;

class FakeCaffeinateProcess {
  readonly signals: Array<"SIGTERM" | "SIGKILL"> = [];
  private readonly exitListeners = new Set<ExitListener>();
  private errorListener: ((error: CaffeinateProcessError) => void) | undefined;
  private exited = false;

  constructor(private readonly exitOnSignal: "SIGTERM" | "SIGKILL" | "never" = "SIGTERM") {}

  hasExited(): boolean {
    return this.exited;
  }

  unref(): void {}

  kill(signal: "SIGTERM" | "SIGKILL") {
    this.signals.push(signal);
    if (signal === this.exitOnSignal) this.emitExit(null, signal);
    return { _tag: "ok" as const, value: true };
  }

  onError(listener: (error: CaffeinateProcessError) => void): void {
    this.errorListener = listener;
  }

  onExit(listener: ExitListener): () => void {
    this.exitListeners.add(listener);
    return () => this.exitListeners.delete(listener);
  }

  emitError(error: Error): void {
    this.errorListener?.(new CaffeinateProcessError("runtime", error));
  }

  emitExit(code: number | null, signal: NodeJS.Signals | null): void {
    if (this.exited) return;
    this.exited = true;
    for (const listener of [...this.exitListeners]) listener(code, signal);
    this.exitListeners.clear();
  }
}

function harness(
  platform: NodeJS.Platform = "darwin",
  makeProcess: () => FakeCaffeinateProcess = () => new FakeCaffeinateProcess(),
) {
  const handlers = new Map<string, TestHandler[]>();
  const notifications: Array<{ message: string; level: string }> = [];
  const spawned: Array<{ args: ReadonlyArray<string>; child: FakeCaffeinateProcess }> = [];
  const piMock = {
    on(name: string, handler: TestHandler) {
      handlers.set(name, [...(handlers.get(name) ?? []), handler]);
    },
  };
  let idle = true;
  const ctx = {
    hasUI: true,
    isIdle: () => idle,
    ui: {
      notify: (message: string, level: string) => notifications.push({ message, level }),
    },
  };

  // SAFETY: Registration only calls on(). The test double captures those handlers and supplies its own lifecycle context when emitting them.
  registerNoSleep(piMock as unknown as ExtensionAPI, {
    platform,
    processId: 9876,
    terminationTimeoutMs: 1,
    spawnCaffeinate: (args) => {
      const child = makeProcess();
      spawned.push({ args, child });
      return { _tag: "ok", value: child };
    },
  });

  const emit = async (name: string): Promise<void> => {
    for (const handler of handlers.get(name) ?? []) await handler({}, ctx);
  };

  return {
    emit,
    notifications,
    setIdle: (value: boolean) => {
      idle = value;
    },
    spawned,
  };
}

describe("no-sleep lifecycle", () => {
  test("does nothing outside macOS", async () => {
    const testHarness = harness("linux");

    await testHarness.emit("agent_start");
    await testHarness.emit("agent_settled");

    expect(testHarness.spawned).toHaveLength(0);
  });

  test("runs one caffeinate process until the agent settles", async () => {
    const testHarness = harness();

    await testHarness.emit("agent_start");
    await testHarness.emit("agent_start");
    expect(testHarness.spawned).toHaveLength(1);
    expect(testHarness.spawned[0]?.args).toEqual(["-d", "-i", "-s", "-w", "9876"]);

    await testHarness.emit("agent_end");
    expect(testHarness.spawned[0]?.child.signals).toEqual([]);

    await testHarness.emit("agent_settled");
    expect(testHarness.spawned[0]?.child.signals).toEqual(["SIGTERM"]);
  });

  test("does not stop a replacement run during stale settlement", async () => {
    const testHarness = harness();

    await testHarness.emit("agent_start");
    testHarness.setIdle(false);
    await testHarness.emit("agent_settled");

    expect(testHarness.spawned[0]?.child.signals).toEqual([]);
  });

  test("keeps separate Pi runtimes independent", async () => {
    const first = harness();
    const second = harness();

    await first.emit("agent_start");
    await second.emit("agent_start");
    await first.emit("agent_settled");

    expect(first.spawned[0]?.child.signals).toEqual(["SIGTERM"]);
    expect(second.spawned[0]?.child.signals).toEqual([]);

    await second.emit("agent_settled");
    expect(second.spawned[0]?.child.signals).toEqual(["SIGTERM"]);
  });

  test("restarts once when caffeinate exits while the agent is active", async () => {
    const testHarness = harness();
    await testHarness.emit("agent_start");

    testHarness.spawned[0]?.child.emitExit(0, null);
    expect(testHarness.spawned).toHaveLength(2);
    testHarness.spawned[1]?.child.emitExit(1, null);

    expect(testHarness.spawned).toHaveLength(2);
    expect(testHarness.notifications).toEqual([
      {
        message: "No Sleep lost caffeinate unexpectedly (exit code 0)",
        level: "warning",
      },
      {
        message: "No Sleep lost caffeinate unexpectedly (exit code 1)",
        level: "warning",
      },
    ]);
  });

  test("reports asynchronous child startup errors", async () => {
    const testHarness = harness();
    await testHarness.emit("agent_start");

    testHarness.spawned[0]?.child.emitError(new Error("not executable"));

    expect(testHarness.notifications).toEqual([
      {
        message: "No Sleep caffeinate failed: not executable",
        level: "error",
      },
    ]);
  });

  test("reports spawn errors", async () => {
    const handlers = new Map<string, TestHandler[]>();
    const notifications: string[] = [];
    const piMock = {
      on(name: string, handler: TestHandler) {
        handlers.set(name, [...(handlers.get(name) ?? []), handler]);
      },
    };
    const ctx = { hasUI: true, ui: { notify: (message: string) => notifications.push(message) } };

    // SAFETY: Registration only calls on(). The test double captures handlers for direct lifecycle testing.
    registerNoSleep(piMock as unknown as ExtensionAPI, {
      platform: "darwin",
      processId: 9876,
      terminationTimeoutMs: 1,
      spawnCaffeinate: () => ({
        _tag: "err",
        error: new CaffeinateProcessError("spawn", new Error("not executable")),
      }),
    });

    for (const handler of handlers.get("agent_start") ?? []) await handler({}, ctx);
    expect(notifications).toEqual(["No Sleep could not start caffeinate: not executable"]);
  });

  test("retains a child that does not stop instead of spawning a duplicate", async () => {
    const testHarness = harness("darwin", () => new FakeCaffeinateProcess("never"));

    await testHarness.emit("agent_start");
    await testHarness.emit("agent_settled");
    await testHarness.emit("agent_start");

    expect(testHarness.spawned).toHaveLength(1);
    expect(testHarness.spawned[0]?.child.signals).toEqual(["SIGTERM", "SIGKILL"]);
    expect(testHarness.notifications).toEqual([
      {
        message: "No Sleep caffeinate did not exit after SIGKILL",
        level: "error",
      },
    ]);
  });

  test("escalates shutdown when SIGTERM does not stop caffeinate", async () => {
    const testHarness = harness("darwin", () => new FakeCaffeinateProcess("SIGKILL"));

    await testHarness.emit("agent_start");
    await testHarness.emit("session_shutdown");

    expect(testHarness.spawned[0]?.child.signals).toEqual(["SIGTERM", "SIGKILL"]);
    expect(testHarness.notifications).toEqual([]);
  });
});
