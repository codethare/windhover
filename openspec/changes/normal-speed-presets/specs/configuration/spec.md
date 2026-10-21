# Spec Delta

## ADDED Requirements

### Requirement: Speed preset bindings

The three speed presets SHALL be key options, with the fine preset bound to `[`,
the normal preset to `\` and the coarse preset to `]` by default. Binding,
unbinding and rebinding them SHALL follow the same rules as the other key
options.

#### Scenario: Rebinding a preset

- **WHEN** the configuration binds the fine preset to another key
- **THEN** that key selects the fine preset and `[` no longer does

#### Scenario: Unbinding a preset

- **WHEN** the configuration gives `unbind` as a preset's value
- **THEN** no key selects that preset, and the session keeps running

#### Scenario: The defaults are reachable

- **WHEN** the session starts with no configuration
- **THEN** `[`, `\` and `]` select the fine, normal and coarse presets
