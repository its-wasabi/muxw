local function config_kb(kb)
	kb:layout("us");
	kb:options("caps-esc:swap");
end

-- Keyboard configuration methods
local kb = Mux.input.keyboard.get({
	name = "",   -- Optional
	seat = "Seat0", -- Optional
	port = "COM1", -- Optional
});
kb:layout("us");
kb:layout({ "us", "pl" });
kb:layout("us,pl");
kb:options({ "xkb_option", "xkb_option", "xkb_option" })
kb:options("xkb_option,xkb_option,xkb_option");
local event_id = Mux.event.add(Mux.event.keyboard.added, function(ctx)
	print("Event triggered")

	ctx.keyboard:layout({ "us", "pl", "jp" });
	ctx.keyboard:options({ "caps-esc:swap" });

	config_kb(ctx.keyboard);

	if ctx.timestamp > 200 then
		print("Compositor config expiriencing some lag");
	end
end);

-- Delete some event listener
Mux.event.del(event_id);



--[[
Mux.event.keyboard
Mux.event.keyboard.added
Mux.event.keyboard.removed
Mux.event.mouse
Mux.event.mouse.added
Mux.event.mouse.removed
Mux.event.monitor
Mux.event.monitor.added
Mux.event.monitor.remove
Mux.event.window
Mux.event.window.focus
Mux.event.window.focus.gain
Mux.event.window.focus.lose
Mux.event.workspace
Mux.event.workspace.focus
Mux.event.workspace.focus.gain
Mux.event.workspace.focus.lose
]] --

-- TODO: Think about more optimal structure of actions so they feel more natural and integrate
-- seamlessly with their nature about allowing for setting them for any object or specific one
--
--[[
Mux.action.focus.left
Mux.action.focus.right
Mux.action.focus.up
Mux.action.focus.down
Mux.action.focus.window.left
Mux.action.focus.window.right
Mux.action.focus.window.up
Mux.action.focus.window.down
Mux.action.focus.workspace.left
Mux.action.focus.workspace.right
Mux.action.focus.workspace.up
Mux.action.focus.workspace.down
Mux.action.move.window.left
Mux.action.move.window.right
Mux.action.move.window.up
Mux.action.move.window.down
Mux.action.move.workspace.left
Mux.action.move.workspace.right
Mux.action.move.workspace.up
Mux.action.move.workspace.down
]] --
