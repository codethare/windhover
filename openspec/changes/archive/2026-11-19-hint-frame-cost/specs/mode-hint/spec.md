# Spec Delta

## MODIFIED Requirements

### Requirement: Consistent hint cells

Hint mode's cells SHALL be tied to their target and SHALL not make the visual
content at the target completely unreadable. The lattice SHALL be drawn as
hairline grid lines rather than as a fill over every cell, and a cell SHALL be
filled, translucently, only while the labels are being narrowed, so that the
thing being aimed at stays visible and the frame does not cost as much as filling
the screen. The cell and its label SHALL use the same font, corner radii and
colour rules as grid mode's key hints.

#### Scenario: The badge does not cover the target centre

- **WHEN** a cell is drawn for a target
- **THEN** the area it covers does not render the visual content at the target's
  centre completely unreadable, because an unfiltered lattice is lines rather
  than a fill and a filtered cell's fill is translucent

#### Scenario: Legible on any background

- **WHEN** a cell is drawn over a light or a dark background
- **THEN** the label text and the cell's hairline edge keep enough contrast
  against it, with a stroke or a backing panel if that is what it takes

#### Scenario: A typed character fills what is left

- **WHEN** the user types a character that narrows the labels
- **THEN** the cells whose labels still match are filled translucently and the
  rest are left as lines

#### Scenario: A redraw does not redraw the labels

- **WHEN** a frame is drawn after the first one with the same theme and scale
- **THEN** the labels are taken from the frame before rather than rasterized
  again
