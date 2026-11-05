// SPDX-License-Identifier: GPL-3.0-only

//! Startup policy derived from the session environment.
//!
//! The launcher applications are intentionally not preloaded by default. Their
//! existing desktop entries, global actions, and panel buttons launch them on
//! demand. Users who prefer the previous eager behavior can opt back in with
//! `ZINC_PRELOAD_LAUNCHERS=1`.

pub(crate) const PRELOAD_LAUNCHERS_ENV: &str = "ZINC_PRELOAD_LAUNCHERS";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct StartupPolicy {
	preload_launchers: bool,
}

impl StartupPolicy {
	pub(crate) fn from_env_value(value: Option<&str>) -> Self {
		Self {
			preload_launchers: value.is_some_and(|value| value.trim() == "1"),
		}
	}

	pub(crate) fn preload_launchers(self) -> bool {
		self.preload_launchers
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn launchers_are_lazy_when_unset() {
		assert!(!StartupPolicy::from_env_value(None).preload_launchers());
	}

	#[test]
	fn one_enables_eager_launcher_preload() {
		assert!(StartupPolicy::from_env_value(Some("1")).preload_launchers());
		assert!(StartupPolicy::from_env_value(Some(" 1 ")).preload_launchers());
	}

	#[test]
	fn other_values_keep_launchers_lazy() {
		for value in ["", "0", "true", "yes", "false", " 0 ", "1x"] {
			assert!(
				!StartupPolicy::from_env_value(Some(value)).preload_launchers(),
				"unexpected eager preload for {value:?}"
			);
		}
	}
}
