# Design

## Context

See `proposal.md` for why. What matters here:

- `Session::step()` is the only source of the movement distance; `growth()` and
  the `Ctrl` factor multiply it per press, inside `Session::act`.
- Normal mode decides everything else through `Effect` and returns it to the
  session, so a decision that outlives one press has to be an effect or session
  state the session already owns.
- The status beside the cursor already renders the digit multiplier and the drag
  state, and renders nothing when both are absent.

## Goals / Non-Goals

**Goals:**

- Make one step shorter than one step, from the keyboard, without a modifier.
- Keep the minimum movement small enough that a coarse hint cell can be closed
  in one or two presses.

**Non-Goals:**

- Configurable preset values. The three multipliers are the feature; a
  configuration surface for their numbers would be three more options nobody
  asked for.
- Changing the held-key acceleration or the `Ctrl` factor.

## Decisions

### Absolute presets, not a cycle and not a held modifier

Three actions that set `0.1`, `1` and `4` are the smallest thing that reaches
the fine end and back: one action each, no ladder array, no ordering to get
wrong, and the key that returns to normal is always the same key.

Alternatives: a single cycling key (rejected — the state is invisible until you
press it into the wrong place); a held modifier, as mouseless does (rejected —
the session already has one held modifier and a second would have to be
distinguished from it, and a modifier cannot express "coarse and staying
coarse" while another key repeats).

### The preset travels as `Effect::Speed`

Normal mode owns the decision, the session owns the state, and `Effect` is the
channel they already share for `Drag`, `Scroll` and `Switch`. Returning an effect
keeps `Normal::on_action` testable on its own and keeps the session's `apply`
the one place that mutates session state.

The alternative — mutating the session directly in `act` before the mode
dispatch — would special-case one action outside the mode that emits it.

### The preset is shown in the existing status text

The status element already exists to say what the cursor ring cannot, and a
preset is exactly that. Appending it to the same text means no new element, no
new placement rule, and nothing shown when the preset is normal — the common
case stays unlabelled.

## Risks / Trade-offs

- **A session left slow for a reason it forgot.** The status text is the
  mitigation: `0.1x` stays beside the cursor until it is cleared.
- **Coarse plus a long hold compounds to 16 steps per repeat**, 480 logical
  pixels at 4K. That is what acceleration is for, and the screen clamp still
  bounds it. → Accepted; no new limit.
- **`\` sits far from the others on some layouts.** The three are rebindable
  anyway, and the status text makes a wrong key visible rather than silent.
