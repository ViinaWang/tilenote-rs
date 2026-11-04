-- Hyprland releases using the Lua configuration (0.53+).
-- Read your release's configuration documentation before installing.
-- Adjust mainMod and the trigger keys to fit your existing binds; the keys
-- below are just examples and may collide with your current setup.
local mainMod = "SUPER"

hl.bind(mainMod .. " + M",          hl.dsp.exec_cmd("tilenote toggle"))
hl.bind(mainMod .. " + SHIFT + M",  hl.dsp.exec_cmd("tilenote new"))
hl.bind(mainMod .. " + ALT + M",    hl.dsp.exec_cmd("tilenote settings"))
