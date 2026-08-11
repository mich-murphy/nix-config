export interface ContextControlState {
  disabledContextPaths: string[];
  hiddenSkillPaths: string[];
}

export const EMPTY_STATE: ContextControlState = {
  disabledContextPaths: [],
  hiddenSkillPaths: [],
};

export const STATE_TYPE = "context-control-state";

export function readStateFromBranch(entries: Iterable<unknown>): ContextControlState {
  let current: ContextControlState | undefined;

  for (const entry of entries) {
    if (!isRecord(entry) || entry.type !== "custom" || entry.customType !== STATE_TYPE) {
      continue;
    }
    if (!isContextControlState(entry.data)) continue;
    current = {
      disabledContextPaths: [...entry.data.disabledContextPaths],
      hiddenSkillPaths: [...entry.data.hiddenSkillPaths],
    };
  }

  return current ?? { ...EMPTY_STATE };
}

function isContextControlState(value: unknown): value is ContextControlState {
  if (!isRecord(value)) return false;
  return (
    isStringArray(value.disabledContextPaths) &&
    isStringArray(value.hiddenSkillPaths)
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}
