use std::sync::{Arc, Mutex};

use mlua::{Lua, Nil};

use crate::log_msg;
use crate::mapping::Mapper;
use crate::style;

fn reset_to_defaults(lua: &Lua, mapper: &Mutex<Mapper>) -> mlua::Result<()> {
	mapper.lock().unwrap().release_all();

	crate::config::setup_dsl(lua)
		.map_err(|e| mlua::Error::runtime(e))?;

	let g = lua.globals();
	g.set("log_level", 0)?;
	g.set("instant_press_time", 40)?;
	g.set("hold_press_time", 400)?;
	g.set("double_press_window", 200)?;
	g.set("trigger_threshold", 3000)?;
	g.set("left_stick_inner_deadzone", 0.15)?;
	g.set("left_stick_outer_deadzone", 1.0)?;
	g.set("right_stick_inner_deadzone", 0.15)?;
	g.set("right_stick_outer_deadzone", 1.0)?;
	g.set("left_ring_position", 0.8)?;
	g.set("right_ring_position", 0.8)?;
	g.set("update", Nil)?;

	for name in &["_gyro_raw", "_accel_raw", "_gravity"] {
		if let Ok(t) = g.get::<mlua::Table>(*name) {
			t.set("x", 0)?; t.set("y", 0)?; t.set("z", 0)?;
		}
	}
	if let Ok(t) = g.get::<mlua::Table>("_orientation") {
		t.set("w", 1)?; t.set("x", 0)?; t.set("y", 0)?; t.set("z", 0)?;
	}

	Ok(())
}

pub fn register_api(lua: &Lua, mapper: &Arc<Mutex<Mapper>>) {
	{
		let m = mapper.clone();
		lua.globals()
			.set(
				"_is_held",
				lua.clone()
					.create_function(move |_, btn: String| {
						Ok(m.lock().unwrap().is_held(&btn))
					})
					.unwrap(),
			)
			.unwrap();
	}

	{
		let m = mapper.clone();
		lua.globals()
			.set(
				"_held_buttons",
				lua.clone()
					.create_function(move |_, ()| Ok(m.lock().unwrap().held_buttons()))
					.unwrap(),
			)
			.unwrap();
	}

	{
		let m = mapper.clone();
		lua.globals()
			.set(
				"_press_key",
				lua.clone()
					.create_function(move |_, key: String| {
						m.lock().unwrap().press_key(&key);
						Ok(())
					})
					.unwrap(),
			)
			.unwrap();
	}

	{
		let m = mapper.clone();
		lua.globals()
			.set(
				"_release_key",
				lua.clone()
					.create_function(move |_, key: String| {
						m.lock().unwrap().release_key(&key);
						Ok(())
					})
					.unwrap(),
			)
			.unwrap();
	}

	lua.globals()
		.set(
			"_now",
			lua.clone()
				.create_function(move |_, ()| {
					Ok(std::time::SystemTime::now()
						.duration_since(std::time::UNIX_EPOCH)
						.unwrap_or_default()
						.as_secs_f64())
				})
				.unwrap(),
		)
		.unwrap();

	lua.globals()
		.set(
			"log",
			lua.clone()
				.create_function(|_, (level, msg): (u8, String)| -> mlua::Result<()> {
					log_msg(level, &msg);
					Ok(())
				})
				.unwrap(),
		)
		.unwrap();

	lua.globals()
		.set(
			"_info",
			lua.clone()
				.create_function(|_, msg: String| -> mlua::Result<()> {
					println!("{}", style::info(&msg));
					Ok(())
				})
				.unwrap(),
		)
		.unwrap();

	lua.globals()
		.set(
			"_progress",
			lua.clone()
				.create_function(|_, msg: String| -> mlua::Result<()> {
					println!("{}", style::progress(&msg));
					Ok(())
				})
				.unwrap(),
		)
		.unwrap();

	{
		let m = mapper.clone();
		let reset_fn = lua
			.create_function(move |lua: &Lua, ()| -> mlua::Result<()> {
				reset_to_defaults(lua, &m)
			})
			.unwrap();
		lua.globals().set("reset", reset_fn).unwrap();
	}

	{
		let m = mapper.clone();
		let reload_fn = lua
			.create_function(move |lua: &Lua, ()| -> mlua::Result<()> {
				reset_to_defaults(lua, &m)?;
				if let Some(path) = crate::config::get_config_path() {
					crate::config::load(&path, lua)
						.map_err(|e| mlua::Error::runtime(e))?;
				}
				Ok(())
			})
			.unwrap();
		lua.globals().set("reload", reload_fn).unwrap();
	}
}
