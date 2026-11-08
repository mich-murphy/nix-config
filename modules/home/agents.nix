{ config, ... }:
let
  agentConfig = "${config.nixConfig.checkout}/config/agents";
  skills = config.nixConfig.skills;
  liveLink = path: config.lib.file.mkOutOfStoreSymlink path;
in
{
  home.file = {
    ".agents/skills".source = liveLink skills;
    ".claude/CLAUDE.md".source = liveLink "${agentConfig}/AGENTS.md";
    ".codex/AGENTS.md".source = liveLink "${agentConfig}/AGENTS.md";
    ".pi/agent/AGENTS.md".source = liveLink "${agentConfig}/AGENTS.md";
    # Per-skill links keep ~/.claude/skills a real directory so Claude
    # Code's managed-skills sync cannot write into the skills repo.
    ".claude/skills/bro".source = liveLink "${skills}/bro";
    ".claude/skills/create-verification".source = liveLink "${skills}/create-verification";
    ".claude/skills/show-me".source = liveLink "${skills}/show-me";
  };
}
