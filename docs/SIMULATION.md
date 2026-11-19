# Deck simulator: order of operations

The simulator (`spire_core::analysis::sim`) plays seeded fights against a
passive target. Many relics and powers change the same numbers, so the order
they apply in changes the result. This page records the order the simulator
uses, where each step comes from, and what is still an assumption.

Each fact is marked with its source:

- **[Data]**: Spire Codex game data, which is extracted from the game files
  (card, relic, power and orb text and numbers).
- **[Codex]**: Spire Codex's [combat mechanics](https://spire-codex.com/mechanics/combat-mechanics)
  and [orb mechanics](https://spire-codex.com/mechanics/orb-mechanics) pages,
  written from the game's own classes (`VulnerablePower`, `StrengthPower` and so on).
- **[Wiki]**: the community wiki's StS2 pages:
  [Vulnerable](https://slaythespire.wiki.gg/wiki/Slay_the_Spire_2:Vulnerable),
  [Weak](https://slaythespire.wiki.gg/wiki/Slay_the_Spire_2:Weak) and
  [Gambling Chip](https://slaythespire.wiki.gg/wiki/Slay_the_Spire_2:Gambling_Chip).
- **[Assumed]**: no StS2 source was found. The choice is the simplest reading
  of the card text, and is listed under "Open questions".

## Damage from an attack

| Step | Rule | Source |
|---|---|---|
| 1 | Start from the printed damage, with the upgrade applied. | Data |
| 2 | Add flat bonuses: Strength; "deals N additional damage" relics and powers (Strike Dummy, Accuracy, Miniature Cannon); Vigor; and Aim (Shoot cards only). Vigor and Aim are spent by the attack but count on every hit. | Codex (Strength is additive and comes first); Player (Vigor and Aim apply to every hit) |
| 3 | Multiply: first-attack bonuses (Lethality), then Vulnerable on the target. | Codex (multipliers come after flat bonuses) |
| 4 | Vulnerable is the game's percentage, 50%. Paper Phrog replaces it ("75% rather than 50%"). Cruelty adds to it: 50 + 25 = 75%. | Data (power text); Player (Cruelty adds) |
| 5 | Round down once, per hit. | Codex (each hit goes through the whole pipeline); Wiki (one rounding at the end); still unconfirmed in game |
| 6 | Multi-hit attacks repeat steps 1–5 for each hit; Thumbscrew adds one hit to attacks that already hit more than once. | Codex; Data |

Example: a Strike (6) with Strike Dummy (+1) into a Vulnerable enemy deals
floor((6 + 1) × 1.5) = 10. Without the step order it would read 6 × 1.5 + 1 = 10,
or 11 if rounded late. The test is `vulnerable_multiplies_after_flat_bonuses_and_rounds_per_hit`.

**Damage that isn't from an attack** is never multiplied by Vulnerable, and
doesn't get Strength. This covers orbs, Poison, and trigger damage (Juggernaut,
Mercury Hourglass, Lost Wisp). The Vulnerable text says "more damage from
Attacks". *[Data]*

## Block from a card

Card Block, plus Dexterity, plus "additional Block from Defend cards"
(Fasten), plus Stretch. Stretch is spent by the card but adds to each of its
Block gains. *[Player]* There's no Frail in the simulator, since
there are no enemies to apply it. *[Codex]* for the Dexterity step. Block
from relics, powers and orbs doesn't get Dexterity. *[Codex: "card block"]*

## A fight

### Combat start (turn 1 only)

1. Shuffle the deck, then put Innate cards on top. *[Codex]*
2. Set energy: the character's base, plus any carried energy (Plasma, Ice Cream).
3. Fire combat-start relics, in the order the run gained them. Energy, Block,
   Strength, Vigor, Aim, orbs and Vulnerable apply now. Extra draws ("draw 2
   additional cards") are added to the opening draw. *[Wiki: Gambling Chip
   resolves after start-of-combat relics and the opening draw, and after the
   draw relics]*
4. Fire the turn-1 timings (see the next section).
5. Draw 5 plus any extra draws, up to a hand of 10. *[Codex: 5 per turn, 10 max]*

### Each turn

1. Block from the previous turn is gone (block clears at the start of turn). *[Codex]*
2. Plating drops by 1 (not on turn 1). *[Data: Plating text]*
3. Energy resets to base, plus energy carried from last turn (Plasma passive,
   Ice Cream).
4. "Every N plays in a single turn" counters restart.
5. Start-of-turn timings fire, relics first, then powers in the order they
   were played. That covers "at the start of your turn", "turn N", "first N
   turns", "turn N onward" and "every N turns". Their draws join the hand draw.
6. Draw the hand. Each draw fires "whenever you draw" triggers, and a reshuffle
   fires "whenever you shuffle". *[Codex: the discard pile is shuffled in when
   the draw pile runs out]*

### Playing a card

1. Pay energy and Stars. Corruption-style "Skills cost 0" makes Skills free.
2. Resolve the card. An echo (Echo Form for the first card each turn, Throwing
   Axe for the first card of combat) resolves it twice. Resolving means:
   channel orbs (when slots are full, the leftmost orb is evoked first;
   *[Codex]*), then Arrow bookkeeping, then the card's damage, Block and
   other effects.
3. Fire "whenever you play…" and "every N plays" triggers.
4. **A power's own triggers start with the next card.** Afterimage doesn't
   give Block for itself. *[Player]* The test is `powers_trigger_on_later_cards_not_themselves`.
5. A turn stops after 50 cards. Free cards that draw each other can loop
   for ever (Capture Spirit's Souls with Invoke); the game would allow it,
   the simulator doesn't. "Next turn, … gain N Energy" (Invoke) arrives next
   turn.
6. Exhaust (Exhaust keyword, or Corruption) fires "whenever a card is
   exhausted". Otherwise the card is discarded *after* it resolves, so its own
   draw can't shuffle it back in.

### End of turn

1. "At the end of your turn" triggers fire (relics, then powers).
2. Plating adds its Block.
3. Orb passives, leftmost first. Glass passive then loses 1, and its evoke
   follows at twice the passive. *[Codex]*
4. Poison ticks (at the start of the enemy's turn), then drops by 1. Accelerant
   adds ticks. *[Data: Poison text]*
5. Vulnerable on the enemy drops by 1. *[Codex: debuffs tick down at end of turn]*
6. Hand: Retain cards (or the whole hand with Runic Pyramid or Well-Laid Plans)
   stay. Ethereal cards are exhausted, which fires exhaust triggers. The rest
   are discarded. *[Codex]*

## Interactions the order settles

These are the cases where a different order gives a different answer, and
what the simulator does:

- **Strength or flat bonus vs Vulnerable:** the bonus is added first, then
  multiplied. 6 + 2 Strength into Vulnerable deals 12, not 11.
- **Vigor on a multi-hit attack:** Vigor is added to every hit, before
  multipliers, and spent by that attack. The same goes for Aim on shots, and
  for Stretch on a card's Block. *[Player]*
- **Combat-start draw relics vs the opening hand:** they add to it, so Bag of
  Preparation and Ring of the Snake mean a 7-card first hand.
- **Combat-start energy (Lantern):** it applies after the energy reset, so it
  isn't wiped.
- **A power and its own trigger:** no self-trigger (see above).
- **Echoed powers:** Echo Form on a power applies it twice (two copies of its
  triggers).
- **Combat-start Vulnerable (Bag of Marbles) with 1 stack:** it lasts for the
  player's first turn only.
- **Plating:** it gives Block at the end of turn N, then drops at the start of
  turn N + 1, so 4 Plating gives 4, 3, 2, 1.
- **Trigger chains:** a trigger that causes another trigger (a draw that fires
  an on-draw that draws…) stops at three levels deep.

## Settled and open questions

**Confirmed by a player:**
- Cruelty adds to Vulnerable (75%).
- Vigor, Aim (on shots) and Stretch (on Block) apply to every hit or Block gain of the card that spends them.
- Vigor, Aim and Stretch last until first used, across turns: unspent, they carry into the next turn. The simulator never resets them at a turn's end.
- A card that makes setup is worth what the deck can make of it: Aim at the deck's Shoot cards' mean Aim multiplier, Vigor if the deck has an attack, Stretch if it has a Block card, and nothing otherwise. Setup already waiting counts toward the card that spends it. The simulator reports each store per turn (`DeckSim::stores`: made, used, and cards waiting on an Arrow or on Stars) for `analysis::balance`.
- A shot Arrow returns after 3 attack hits, each hit of a multi-hit attack counting (`arrow_return_hits` in the playbook's `simulation`).
- "Whenever you play a card" powers don't count themselves.
- No card reads a current value at the moment a same-timing effect fires, so the order of same-timing relics and powers doesn't change results. The simulator fires relics first (in pickup order), then powers in play order.

**Still open:**
- Rounding: the wiki says once at the end; Codex says each hit goes through the whole pipeline. The simulator rounds down per hit. The two only differ when fractions would add up across hits.

## What isn't read yet

`audit -- --mechanics` lists every power and relic, and whether it is read:
77 of 133 powers and 84 of 315 relics. Most relics that aren't read work
outside combat (pickup, gold, rest sites, shops). The combat effects left
out are:

- conditions ("if you end your turn without Block", "the first time…")
- random cards or random orbs added to the hand
- enemy deaths
- HP loss
- block retention (Barricade, Sturdy Clamp)
- "this turn" buffs
- amounts scaled "for each" something

The Relics tab and each power's card page say what is and isn't simulated.
