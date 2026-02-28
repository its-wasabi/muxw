local kb = Mux.input.keyboard.get({
	name = "",   -- Optional
	seat = "Seat0", -- Optional
	port = "COM1", -- Optional
});

kb:layout("us");
-- or
kb:layout({ "us", "pl" });
-- or
kb:layout("us,pl");

kb:options({ "xkb_option", "xkb_option", "xkb_option" })
-- or
kb:options("xkb_option,xkb_option,xkb_option");

kb:apply();


