import { spawn } from "node:child_process";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import {
  type CaffeinateProcess,
  CaffeinateProcessError,
  type CaffeinateProcessResult,
  type NoSleepDependencies,
  registerNoSleep,
} from "./no-sleep-lifecycle";

const CAFFEINATE_PATH = "/usr/bin/caffeinate";
const TERMINATION_TIMEOUT_MS = 1_000;

function spawnCaffeinate(
  args: ReadonlyArray<string>,
): CaffeinateProcessResult<CaffeinateProcess> {
  try {
    const child = spawn(CAFFEINATE_PATH, [...args], { stdio: "ignore" });
    return {
      _tag: "ok",
      value: {
        hasExited: () => child.exitCode !== null || child.signalCode !== null,
        unref: () => child.unref(),
        kill: (signal) => {
          try {
            return { _tag: "ok", value: child.kill(signal) };
          } catch (cause) {
            return {
              _tag: "err",
              error: new CaffeinateProcessError(signal, cause),
            };
          }
        },
        onError: (listener) => child.once(
          "error",
          (cause) => listener(new CaffeinateProcessError("runtime", cause)),
        ),
        onExit: (listener) => {
          child.once("exit", listener);
          return () => child.off("exit", listener);
        },
      },
    };
  } catch (cause) {
    return {
      _tag: "err",
      error: new CaffeinateProcessError("spawn", cause),
    };
  }
}

function dependencies(): NoSleepDependencies {
  return {
    platform: process.platform,
    processId: process.pid,
    terminationTimeoutMs: TERMINATION_TIMEOUT_MS,
    spawnCaffeinate,
  };
}

/** Register macOS sleep prevention for the periods when Pi is doing agent work. */
export default function noSleep(pi: ExtensionAPI): void {
  registerNoSleep(pi, dependencies());
}
