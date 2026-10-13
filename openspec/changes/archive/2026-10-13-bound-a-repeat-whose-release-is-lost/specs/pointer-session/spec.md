# Spec Delta

## ADDED Requirements

### Requirement: A repeat whose release never arrives is bounded

A key the session believes is held SHALL stop repeating once its hold outlasts a
fixed limit, since a compositor can swallow the release without saying so. At
that point the session SHALL release a drag's button and end itself, writing one
message naming the reason, because a compositor that lost the release may also
have stopped delivering keys, leaving no key that could end the session.

#### Scenario: A hold that outlasts the limit

- **WHEN** a direction key stays down past the limit without its release ever
  arriving
- **THEN** the repeat stops, any drag in progress has its button released, and
  the session ends, naming the reason on standard error

#### Scenario: A hold within the limit

- **WHEN** a key is held for less than the limit and then released
- **THEN** the repeat behaves as it always did, and the release stops it
