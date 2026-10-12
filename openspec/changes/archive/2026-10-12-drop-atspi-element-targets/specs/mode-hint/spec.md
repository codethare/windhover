# Spec Delta

## REMOVED Requirements

### Requirement: Element targets and degradation

**Reason**: The capability was carried by an opt-in cargo feature that cost 67
crates — more than the rest of the project combined — for target precision that
was never verified: the test machine has no accessibility bus and no accessible
application, so only the fallback path was ever exercised. It was also the origin
of the project's one dependency advisory (quick-xml, through zbus) and the only
part of the tree opening IPC to another process. Hint mode does not depend on it:
the lattice is the default source.

**Migration**: Nothing to do. Hint mode's `lattice target source` requirement is
unchanged, and element targets had no configuration of their own — they were
attempted first and silently replaced by the lattice whenever they had nothing to
offer, which is also what happens now in every case.
