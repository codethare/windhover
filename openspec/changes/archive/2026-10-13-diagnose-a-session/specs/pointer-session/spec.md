# Spec Delta

## ADDED Requirements

### Requirement: Bindings the layout cannot produce are reported

A session SHALL check its configured bindings against the keymap the compositor
gives it, and SHALL name, on standard error, the ones that keymap cannot produce,
since such a binding silently does nothing. The check SHALL run whenever a keymap
is installed, so a layout switch is covered too, and SHALL stay silent when every
binding can be produced.

#### Scenario: A layout without a key for a binding

- **WHEN** a session is configured with a binding whose keysym the compositor's
  keymap has no key for
- **THEN** one line on standard error names that binding and the option it came
  from, and the session carries on

#### Scenario: The key that ends the session

- **WHEN** the keymap cannot produce the key bound to `quit`
- **THEN** that binding is among those named

#### Scenario: A layout that can produce everything

- **WHEN** every configured binding can be produced by the keymap
- **THEN** nothing is written to standard error

### Requirement: A traced session reports what it saw

With `-t` / `--trace`, a session SHALL print to standard error what it observed:
changes of keyboard focus, each key with the keysym it resolved to, each keymap it
installed, each mode change, and a line for every turn of its event loop that
produced an effect or had no time left to wait. Without the flag it SHALL write
nothing beyond its warnings.

#### Scenario: Tracing a session

- **WHEN** the session runs with `--trace` and the user presses a key
- **THEN** standard error shows the focus, the key and its keysym, and the loop
  turn that acted on it

#### Scenario: Not tracing

- **WHEN** the session runs without the flag
- **THEN** standard error stays empty
