# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: Consistent hint badges`
  TO: `### Requirement: Consistent hint cells`

## MODIFIED Requirements

### Requirement: The lattice target source

By default hint mode SHALL generate a regular lattice across the target output as
its target source: the output is partitioned into a grid of cells, and each cell
carries one target at its centre and one label from the configured alphabet. The
partition SHALL cover the whole output, and the label set SHALL be prefix-free
(no label is a prefix of another), so that reaching a unique match while typing
is enough to position immediately. The label length SHALL be the shortest the
alphabet can name the chosen number of cells in: two characters for the default
twenty-character alphabet.

#### Scenario: Generating lattice hints

- **WHEN** the user presses `f` in normal mode to enter hint mode
- **THEN** the overlay draws a translucent cell over each target region with a
  label centred in it, the labels are taken from the configured alphabet, every
  label is two characters long under the default alphabet, and no two labels
  where one is a prefix of the other are produced

#### Scenario: The cells tile the output

- **WHEN** the lattice is generated for an output
- **THEN** the cells cover the output without gaps or overlap, and each cell's
  label sits at that cell's centre

#### Scenario: More targets than the alphabet can label

- **WHEN** the configured lattice spacing asks for more cells than the alphabet
  can name in two characters
- **THEN** the session raises the label length to three characters while keeping
  the labels prefix-free and never producing duplicate labels

#### Scenario: More targets than the alphabet can name at all

- **WHEN** the configured lattice spacing asks for more cells than the alphabet
  can name in three characters
- **THEN** the session refuses to start and says which option is too dense,
  rather than dropping targets or repeating labels

#### Scenario: Density is configurable

- **WHEN** the configuration sets the hint lattice spacing
- **THEN** the number of cells follows that spacing, and the labels stay
  prefix-free at whatever length the cell count needs

### Requirement: Consistent hint cells

Hint mode's cells SHALL be tied to their target and SHALL not make the visual
content at the target's centre completely unreadable: the cell SHALL be a
translucent fill with a hairline edge rather than an opaque panel, so what is
underneath stays visible. The cell and its label SHALL use the same font, corner
radii and colour rules as grid mode's key hints.

#### Scenario: The badge does not cover the target centre

- **WHEN** a cell is drawn for a target
- **THEN** the area it covers does not render the visual content at the target's
  centre completely unreadable, because the fill is translucent and what is under
  it stays visible

#### Scenario: Legible on any background

- **WHEN** a cell is drawn over a light or a dark background
- **THEN** the label text and the cell's hairline edge keep enough contrast
  against it, with a stroke or a backing panel if that is what it takes
