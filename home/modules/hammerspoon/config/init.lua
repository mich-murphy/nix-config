-- automatic config reloading
function reloadConfig(files)
	doReload = false
	for _, file in pairs(files) do
		if file:sub(-4) == ".lua" then
			doReload = true
		end
	end
	if doReload then
		hs.reload()
	end
end
configWatcher = hs.pathwatcher.new(os.getenv("HOME") .. "/.hammerspoon/", reloadConfig):start()

-- exit karabiner elements when dock attached
function usbDeviceCallback(data)
	if data["productID"] == 4136 then
		if data["eventType"] == "added" then
			hs.appfinder.appFromName("Karabiner-Elements"):kill9()
      hs.application.open("DisplayLink Manager",5)
		elseif data["eventType"] == "removed" then
      hs.application.open("Karabiner-Elements",5, true):hide()
			hs.appfinder.appFromName("DisplayLink Manager"):kill9()
		end
	end
end
usbWatcher = hs.usb.watcher.new(usbDeviceCallback):start()
