# Spec Delta

## ADDED Requirements

### Requirement: Losing the keyboard ends the session

A session SHALL treat the loss of the keyboard as the end of what keys were
doing: the repeat of a held key SHALL stop, and a drag in progress SHALL release
its button rather than leaving a window dragging with it held down. A session
whose keyboard does not return within a short grace period SHALL end itself,
removing its overlay and writing one message naming the reason, because no key
can reach a session without the keyboard and nothing else would dismiss it.

#### Scenario: A key is held when the keyboard goes

- **WHEN** the seat takes the keyboard away while a direction key is held
- **THEN** the pointer stops moving, without waiting for a release that can
  no longer arrive

#### Scenario: A drag is in progress when the keyboard goes

- **WHEN** the seat takes the keyboard away during a drag
- **THEN** the left button is released, leaving no window dragging behind

#### Scenario: The keyboard does not come back

- **WHEN** the keyboard stays away past the grace period
- **THEN** the session ends, and one message on standard error says the
  compositor took the keyboard away

#### Scenario: The keyboard comes straight back

- **WHEN** the keyboard is taken and returned within the grace period, as the
  startup handshake does
- **THEN** the session carries on, and the keys that follow still act
