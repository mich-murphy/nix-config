local prefix = "ctrl+a"
local function pbind(key, action, args)
  rex.bind(prefix .. ">" .. key, action, args)
end

pbind("ctrl+a", "pane.send_key", { key = "ctrl+a" })

pbind("shift+\\", "pane.split", { direction = "right" })
pbind("-", "pane.split", { direction = "down" })
pbind("x", "pane.close")
pbind("z", "pane.zoom")
pbind("[", "pane.copy_mode.enter")
pbind("tab", "pane.focus_next")
pbind("shift+tab", "pane.focus_previous")

local directions = { h = "left", j = "down", k = "up", l = "right" }
for key, direction in pairs(directions) do
  pbind(key, "pane.focus", { direction = direction, wrap = false })
  pbind("shift+" .. key, "pane.move", { direction = direction })
end

pbind("c", "client.tab.new")
pbind("n", "client.tab.next")
pbind("p", "client.tab.previous")
pbind(",", "client.tab.rename")
pbind("shift+x", "client.tab.close")
for i = 1, 9 do
  pbind(tostring(i), "client.tab.goto", { index = i })
end

pbind("shift+9", "session.previous")
pbind("shift+0", "session.next")
pbind("shift+n", "session.new")
pbind("shift+w", "session.rename")
pbind("shift+d", "session.close")
pbind("w", "session.switch")
pbind("g", "pane.go_to")

pbind("b", "client.sidebar.toggle")
pbind("shift+/", "client.palette.toggle")
pbind("s", "client.settings")
pbind("shift+r", "client.config.reload")

rex.mode("resize", { exclusive = true })
pbind("r", "client.mode.enter", { name = "resize" })
for key, direction in pairs(directions) do
  rex.bind("resize/" .. key, "pane.resize", { direction = direction })
  rex.bind("resize/shift+" .. key, "pane.resize", { direction = direction, amount = 15 })
end
rex.bind("resize/e", "pane.balance")
rex.bind("resize/=", "pane.balance")
rex.bind("resize/enter", "client.mode.exit")

local programs_with_own_splits = { nvim = true, vim = true, herdr = true, tmux = true }

local function navigate(ctx, direction)
  if ctx.block_id then
    local p = rex.block.call("com.superlogical.terminal", "process", { block_id = ctx.block_id })
    local fg = p and p.foreground and p.foreground.name
    if fg and programs_with_own_splits[fg] then
      return false
    end
  end
  rex.session.focus_direction{ direction = direction, wrap = false }
  return true
end

rex.action{
  name = "pane_navigate",
  title = "Navigate Pane (Neovim-aware)",
  args = { direction = "string" },
  run = function(ctx, args)
    return { moved = navigate(ctx, args.direction) }
  end,
}

for key, direction in pairs(directions) do
  rex.bind("ctrl+" .. key, function(ctx)
    return navigate(ctx, direction)
  end)
end

local fish_clear_screen_key = "f12"
rex.bind("ctrl+shift+l", "pane.send_key", { key = fish_clear_screen_key })
