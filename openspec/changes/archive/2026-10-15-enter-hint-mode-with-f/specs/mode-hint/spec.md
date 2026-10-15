# Spec Delta

## MODIFIED Requirements

### Requirement: The lattice target source

By default hint mode SHALL generate a regular lattice across the target output as
its target source, each target carrying a letter label. The label set SHALL be
prefix-free (no label is a prefix of another), so that reaching a unique match
while typing is enough to position immediately.

#### Scenario: Generating lattice hints

- **WHEN** the user presses `f` in normal mode to enter hint mode
- **THEN** the overlay draws a compact label badge at each target position, with
  labels taken from the configured alphabet, and no two labels where one is a
  prefix of the other

#### Scenario: More targets than the alphabet can label

- **WHEN** the required number of targets exceeds what the current alphabet can
  cover at a fixed label length
- **THEN** the session increases the label length or reduces the lattice density
  while keeping the labels prefix-free, and never produces duplicate labels

#### Scenario: Density is configurable

- **WHEN** the configuration changes the hint lattice spacing
- **THEN** the number of generated targets follows, and the labels stay
  prefix-free
