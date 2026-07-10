use mlua::Lua;

pub fn setup_dsl(lua: &Lua) -> Result<(), String> {
	lua.load(format!(
		"{}\n{}\n{}\n{}\n{}",
		include_str!("lua/tables.lua"),
		include_str!("lua/bindings.lua"),
		include_str!("lua/sticks.lua"),
		include_str!("lua/gyro.lua"),
		include_str!("lua/events.lua"),
	))
	.exec()
	.map_err(|e| format!("load lua: {}", e))?;
	Ok(())
}

fn add_config_dir_to_package_path(lua: &Lua, config_dir: &str) {
	let _ = lua.load(&format!(
		"package.path = '{}/?.lua;' .. package.path",
		config_dir.replace('\'', "'\\''")
	)).exec();
}

pub fn init_bare(lua: &Lua) {
	add_config_dir_to_package_path(lua, ".");
}

pub fn load(path: &str, lua: &Lua) -> Result<(), String> {
	let abs = std::path::Path::new(path)
		.canonicalize()
		.map_err(|e| format!("canonicalize {}: {}", path, e))?;
	let dir = abs.parent().unwrap_or(std::path::Path::new("."))
		.to_string_lossy()
		.to_string();
	add_config_dir_to_package_path(lua, &dir);

	let src = std::fs::read_to_string(path).map_err(|e| format!("read config: {}", e))?;
	lua.load(&src).exec().map_err(|e| format!("lua exec: {}", e))?;

	Ok(())
}
