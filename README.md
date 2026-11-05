# zinc-session

The session manager for the Zinc desktop environment.

## Startup policy

By default, `zinc-session` eagerly starts the core desktop services needed at
login: the panel, notification daemon, settings daemon, background, greeter /
locker, OSD (including its polkit agent), OSK, files applet, and idle service.

The application library, launcher, and workspace overview are **not preloaded
by default**. They are started on demand by their desktop entries, global
actions, and panel buttons. This reduces idle memory and login work at the cost
of a cold-start delay on the first open.

To restore eager preloading of those three launcher applications, set the
following environment variable in the session environment before starting Zinc:

```sh
ZINC_PRELOAD_LAUNCHERS=1
```

Only the exact value `1` (surrounding whitespace is ignored) enables eager
preloading. Unset or any other value keeps the on-demand policy. The selected
policy is logged by `cosmic-session` at startup.

All independent components are still started only after the notifications
daemon and panel have completed their required notification-socket handshake.
