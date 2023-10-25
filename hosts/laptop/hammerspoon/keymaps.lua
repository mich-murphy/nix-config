-- Remap control to control or escape
local ctrl_table = {
	send_escape = true,
	last_mods = {},
}

local control_key_timer = hs.timer.delayed.new(0.15, function()
	ctrl_table["send_escape"] = false
end)

local last_mods = {}

local control_handler = function(event)
	local new_mods = event:getFlags()
	if last_mods["ctrl"] == new_mods["ctrl"] then
		return false
	end
	if not last_mods["ctrl"] then
		last_mods = new_mods
		ctrl_table["send_escape"] = true
		control_key_timer:start()
	else
		last_mods = new_mods
		control_key_timer:stop()
		if ctrl_table["send_escape"] then
			return true,
				{
					hs.eventtap.event.newKeyEvent({}, "escape", true),
					hs.eventtap.event.newKeyEvent({}, "escape", false),
				}
		end
	end
	return false
end
local control_tap = hs.eventtap.new({ 12 }, control_handler)

control_tap:start()
