// SPDX-License-Identifier: GPL-3.0-only

use std::process::{Command, Stdio};

use tokio::sync::OnceCell;
use zbus::Connection;
use zbus::zvariant::{Array, OwnedValue};
use zbus_systemd::systemd1::ManagerProxy as SystemdManagerProxy;

const SESSION_TARGET: &str = "cosmic-session.target";

/// Reuse one session-bus connection for all systemd manager calls during this
/// session. `zbus::Connection` is internally reference-counted and supports
/// concurrent method calls.
static SYSTEMD_CONNECTION: OnceCell<Connection> = OnceCell::const_new();

async fn systemd_connection() -> zbus::Result<Connection> {
	let connection = SYSTEMD_CONNECTION
		.get_or_try_init(|| async { Connection::session().await })
		.await?;
	Ok(connection.clone())
}

pub async fn set_systemd_environment(key: &str, value: &str) {
	let connection = match systemd_connection().await {
		Ok(connection) => connection,
		Err(err) => {
			warn!("Failed to connect to systemd user manager: {err}");
			return;
		}
	};
	let manager = match SystemdManagerProxy::new(&connection).await {
		Ok(manager) => manager,
		Err(err) => {
			warn!("Failed to access systemd user manager: {err}");
			return;
		}
	};
	if let Err(err) = manager
		.set_environment(vec![format!("{key}={value}")])
		.await
	{
		warn!("Failed to set systemd environment {key}: {err}");
	}
}

pub async fn start_systemd_target() {
	let connection = match systemd_connection().await {
		Ok(connection) => connection,
		Err(err) => {
			warn!("Failed to connect to systemd user manager: {err}");
			return;
		}
	};
	let manager = match SystemdManagerProxy::new(&connection).await {
		Ok(manager) => manager,
		Err(err) => {
			warn!("Failed to access systemd user manager: {err}");
			return;
		}
	};
	// StartUnit queues the job and returns without waiting for the target to
	// become active, matching the previous `systemctl --no-block` behavior.
	if let Err(err) = manager
		.start_unit(SESSION_TARGET.to_string(), "replace".to_string())
		.await
	{
		warn!("Failed to start {SESSION_TARGET}: {err}");
	}
}

pub fn stop_systemd_target() {
	// This runs from a synchronous scope guard while the session is shutting
	// down, so keep the external no-block command here.
	run_optional_command(
		"systemctl",
		&["--user", "stop", "--no-block", SESSION_TARGET],
	);
}

async fn load_systemd_env() -> zbus::Result<Vec<String>> {
	let connection = systemd_connection().await?;
	let systemd_manager = SystemdManagerProxy::new(&connection).await?;
	systemd_manager.environment().await
}

/// Get the systemd user manager environment variables.
pub async fn get_systemd_env() -> Vec<(String, String)> {
	match load_systemd_env().await {
		Ok(env) => {
			env.iter()
				// split into key value pairs
				.filter_map(|var| var.split_once('='))
				.filter(|&(key, _value)| {
					// Only update the envvar if unset
					std::env::var_os(key).is_none()
                              // Blocklist of envvars that we shouldn't touch (taken from KDE)
                              && (!key.starts_with("XDG_")
                                      || key == "XDG_DATA_DIRS"
                                      || key == "XDG_CONFIG_DIRS")
                              && key != "DISPLAY"
                              && key != "XAUTHORITY"
                              && key != "WAYLAND_DISPLAY"
                              && key != "WAYLAND_SOCKET"
                              && key != "_"
                              && key != "SHELL"
                              && key != "SHLVL"
				})
				.map(|(key, value)| (key.to_owned(), value.to_owned()))
				.collect()
		}
		Err(err) => {
			warn!("Failed to sync systemd environment {}.", err);
			Vec::new()
		}
	}
}

/// Spawn a systemd scope unit with the given name and PIDs.
pub async fn spawn_scope(mut command: String, pids: Vec<u32>) -> Result<(), zbus::Error> {
	let connection = systemd_connection().await?;
	let systemd_manager = SystemdManagerProxy::new(&connection).await?;
	let pids = OwnedValue::try_from(Array::from(pids)).unwrap();
	let properties: Vec<(String, OwnedValue)> = vec![(String::from("PIDs"), pids)];
	if command.starts_with('/') {
		// use the last component of the path as the unit name
		command = command.rsplit('/').next().unwrap().to_string();
	}
	let scope_name = format!("{}.scope", command);
	systemd_manager
		.start_transient_unit(
			scope_name.to_string(),
			String::from("replace"),
			properties,
			Vec::new(),
		)
		.await?;
	Ok(())
}

/// run a command, but log errors instead of returning them or panicking
fn run_optional_command(cmd: &str, args: &[&str]) {
	match Command::new(cmd).args(args).stdin(Stdio::null()).status() {
		Ok(status) => {
			if !status.success() {
				match status.code() {
					Some(code) => warn!("{} {}: exit code {}", cmd, args.join(" "), code),
					None => warn!("{} {}: terminated by signal", cmd, args.join(" ")),
				}
			}
		}
		Err(error) => {
			warn!("unable to start {} {}: {}", cmd, args.join(" "), error);
		}
	}
}
