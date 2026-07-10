use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use mlua::Lua;

use crate::log_msg;
use crate::mapping::Mapper;
use crate::types::GyroCmd;

pub fn register_api(lua: &Lua, mapper: &Arc<Mutex<Mapper>>, gyro_tx: &Sender<GyroCmd>) {
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

	{
		let gt = gyro_tx.clone();
		lua.globals()
			.set(
				"gyro_enable",
				lua.clone()
					.create_function(move |_, ()| {
						gt.send(GyroCmd::Enable).ok();
						Ok(())
					})
					.unwrap(),
			)
			.unwrap();
	}

	{
		let gt = gyro_tx.clone();
		lua.globals()
			.set(
				"gyro_disable",
				lua.clone()
					.create_function(move |_, ()| {
						gt.send(GyroCmd::Disable).ok();
						Ok(())
					})
					.unwrap(),
			)
			.unwrap();
	}

	{
		let gt = gyro_tx.clone();
		lua.globals()
			.set(
				"gyro_toggle",
				lua.clone()
					.create_function(move |_, ()| {
						gt.send(GyroCmd::Toggle).ok();
						Ok(())
					})
					.unwrap(),
			)
			.unwrap();
	}

	{
		let gt = gyro_tx.clone();
		lua.globals()
			.set(
				"gyro_hold",
				lua.clone()
					.create_function(move |lua, ()| {
						let btn: String = lua.globals().get("_current_btn").unwrap_or_default();
						gt.send(GyroCmd::Hold(btn)).ok();
						Ok(())
					})
					.unwrap(),
			)
			.unwrap();
	}

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

	{
		let m = mapper.clone();
		let reset_fn = lua
			.create_function(move |_, ()| -> mlua::Result<()> {
				m.lock().unwrap().release_all();
				Ok(())
			})
			.unwrap();
		lua.globals().set("reset", reset_fn).unwrap();
	}
}
