use std::sync::{Arc, Mutex};

use mlua::Lua;

use crate::log_msg;
use crate::mapping::Mapper;
use crate::style;

fn reset_to_defaults(lua: &Lua, mapper: &Mutex<Mapper>) -> mlua::Result<()> {
	mapper.lock().unwrap().release_all();
	lua.globals()
		.get::<mlua::Function>("_reset_internals")?
		.call::<()>(())?;
	crate::config::setup_dsl(lua).map_err(|e| mlua::Error::runtime(e))?;
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
