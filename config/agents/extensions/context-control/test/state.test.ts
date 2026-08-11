import { describe, expect, test } from "bun:test";
import { readStateFromBranch } from "../state";

describe("readStateFromBranch", () => {
  test("restores the latest valid context selection on the active branch", () => {
    const entries = [
      {
        type: "custom",
        customType: "context-control-state",
        data: {
          disabledContextPaths: ["/global/AGENTS.md"],
          hiddenSkillPaths: [],
        },
      },
      {
        type: "custom",
        customType: "other-extension",
        data: { ignored: true },
      },
      {
        type: "custom",
        customType: "context-control-state",
        data: {
          disabledContextPaths: ["/project/AGENTS.md"],
          hiddenSkillPaths: ["/skills/deploy/SKILL.md"],
        },
      },
    ];

    expect(readStateFromBranch(entries)).toEqual({
      disabledContextPaths: ["/project/AGENTS.md"],
      hiddenSkillPaths: ["/skills/deploy/SKILL.md"],
    });
  });
});
