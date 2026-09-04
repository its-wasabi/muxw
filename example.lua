Mux.bind(Mux.window.spawn, function(ctx)
	if ctx.window.class == "Kitty" then
		ctx.window.opacity = 0.8;
		ctx.window.blur = true;
	end
end)
