# Deck simulator: order of operations

`spire_core::analysis::sim` plays seeded fights against a passive target. Order changes results; this page fixes it.

Sources: **[Data]** Spire Codex game data · **[Codex]** Codex [combat](https://spire-codex.com/mechanics/combat-mechanics) and [orb](https://spire-codex.com/mechanics/orb-mechanics) mechanics, written from the game's classes · **[Wiki]** StS2 wiki: [Vulnerable](https://slaythespire.wiki.gg/wiki/Slay_the_Spire_2:Vulnerable), [Weak](https://slaythespire.wiki.gg/wiki/Slay_the_Spire_2:Weak), [Gambling Chip](https://slaythespire.wiki.gg/wiki/Slay_the_Spire_2:Gambling_Chip) · **[Player]** confirmed in play by the project owner.

## Attack damage

| Step | Rule | Source |
|---|---|---|
| 1 | Printed damage, upgrade applied | Data |
| 2 | Add flat bonuses: Strength, "N additional damage" (Strike Dummy, Accuracy, Miniature Cannon), Vigor, Aim (Shoot only). Vigor and Aim are spent by the attack but count on every hit | Codex; Player |
| 3 | Multiply: first-attack bonuses (Lethality), then Vulnerable | Codex |
| 4 | Vulnerable 50%; Paper Phrog makes it 75%; Cruelty adds 25 | Data; Player |
| 5 | Round down once per hit | Codex; Wiki disagrees (open) |
| 6 | Multi-hit repeats 1–5 per hit; Thumbscrew adds a hit to multi-hit attacks | Codex; Data |

Strike (6) + Strike Dummy (+1) into Vulnerable: floor(7 × 1.5) = 10. Test: `vulnerable_multiplies_after_flat_bonuses_and_rounds_per_hit`.

Non-attack damage (orbs, Poison, Juggernaut, Mercury Hourglass, Lost Wisp) gets neither Vulnerable nor Strength. *[Data]*

## Card Block

Card Block + Dexterity + Defend bonus (Fasten) + Stretch (spent by the card, added to each Block gain). *[Codex; Player]* Relic, power and orb Block gets no Dexterity. No Frail: no enemies apply it.

## A fight

**Combat start (turn 1)**
1. Shuffle; Innate cards on top. *[Codex]*
2. Energy: base plus carried (Plasma, Ice Cream).
3. Combat-start relics in pickup order: energy, Block, Strength, Vigor, Aim, orbs, Vulnerable; extra draws join the opening draw. *[Wiki]*
4. Turn-1 timings.
5. Draw 5 plus extras, hand max 10. *[Codex]*

**Each turn**
1. Block clears. *[Codex]*
2. Plating −1 (not turn 1). *[Data]*
3. Energy resets to base plus carried.
4. "Every N plays in a turn" counters reset.
5. Start-of-turn timings: relics, then powers in play order ("start of your turn", "turn N", "first N turns", "turn N onward", "every N turns"); their draws join the hand draw.
6. Draw; each draw fires on-draw triggers, a reshuffle fires on-shuffle. *[Codex]*

**Playing a card**
1. Pay energy and Stars; "Skills cost 0" makes Skills free.
2. Resolve, twice if echoed (Echo Form: first card a turn; Throwing Axe: first of combat): channel orbs (full slots evoke the leftmost first *[Codex]*), Arrow bookkeeping, then damage, Block, effects.
3. "Whenever you play" and "every N plays" triggers.
4. A power's triggers start with the **next** card (no self-trigger). *[Player]* Test: `powers_trigger_on_later_cards_not_themselves`.
5. Cap: 50 cards a turn (free draw loops end). "Next turn, gain N Energy" arrives next turn.
6. Exhaust (keyword or Corruption) fires exhaust triggers; otherwise discard **after** resolving, so the card's own draw can't reshuffle it.

**End of turn**
1. End-of-turn triggers: relics, then powers.
2. Plating adds Block.
3. Orb passives, leftmost first; Glass passive then −1, evoke at twice the passive. *[Codex]*
4. Poison ticks, then −1; Accelerant adds ticks. *[Data]*
5. Enemy Vulnerable −1. *[Codex]*
6. Retain stays (all with Runic Pyramid or Well-Laid Plans); Ethereal exhausts; the rest discards. *[Codex]*

## Settled interactions

- Flat bonus before Vulnerable: 6 + 2 Strength into Vulnerable = 12, not 11.
- Vigor, Aim, Stretch: on every hit or Block gain of the card that spends them; unspent, they carry across turns. *[Player]*
- Combat-start draw relics add to the opening hand (Bag of Preparation + Ring of the Snake: 7 cards).
- Combat-start energy (Lantern) applies after the reset.
- Echo Form on a power applies it twice.
- Combat-start Vulnerable with 1 stack lasts the first player turn.
- Plating 4 gives 4, 3, 2, 1.
- Trigger chains stop three levels deep.
- Same-timing relics and powers: no card reads a value mid-timing, so their order can't change results.
- Cards whose need nothing supplies (`simulation.requirements`: Sadness, Shivs, Souls, Sovereign Blade, orbs, Lightning, Arrows, Stars) are dead draws. *[Player]*
- Setup is worth what the deck can spend: Aim at the deck's Shoot cards' mean multiplier, Vigor with any attack, Stretch with any Block card. Stores per turn feed `analysis::balance`.
- A shot Arrow returns after 3 attack hits (`simulation.arrow_return_hits`). *[Player]*
- Forge: the first Forge in a fight adds the Forge card (`GameRules::forge_card`, Sovereign Blade) to the hand, or the discard pile when the hand is full; every Forge adds its amount to that card's damage, wherever it is. It then cycles like any card. Sword Sage's Replay plays it again; Seeking Edge, Conqueror and Parry aren't read yet. *[Codex glossary]*
- Made cards (`mechanics::card_text`): "Add N <card> into your Hand / Draw Pile / Discard Pile" (Shivs, Souls, Sadness, Joy, Wounds, Burns, Sweeping Gaze…) makes that card there; a full hand sends it to the discard pile, the draw pile takes it at a random place. Powers do it on their timing (Infinite Blades each turn). "Add a copy of this card" copies it. Random cards aren't made.
- The Exhaust Pile is kept: exhaust triggers (Feel No Pain, Dark Embrace), "N additional damage for each card (or Soul) in your Exhaust Pile", "if you have 3 or more cards in your Exhaust Pile", "if you Exhausted a card this turn", "when this card is Exhausted", "costs 1 less for each card Exhausted this combat", and "if this is in your Exhaust Pile, play it" (Bombardment, Howl from Beyond).
- "Exhaust your Hand / all non-Attacks / all Status cards / all Sadness … for each card Exhausted": the card's own numbers (or the sentence's effect) once per card taken. "Discard your Hand … for each card discarded" likewise.
- "This turn, your next Skill (Attack) is played an extra time" plays the next one twice.
- A card draws only when its text says "draw": the data's card count also marks "Add 3 Shivs" and "3 or more cards". A base damage given only as the calculation's base (Ashen Strike, Soul Storm) counts as its damage.
- Each turn's Block keeps its spread (`TurnOutput::block_steps`, every 5%) for the survival check.

**Open:** rounding per hit (Codex) versus once at the end (Wiki); they differ only when fractions add up across hits.

## Not read yet

`audit -- --mechanics`: 77 of 133 powers and 84 of 315 relics are read; most unread relics act outside combat. Unread combat effects: conditions ("end your turn without Block", "the first time"), random cards or orbs, choices ("put a card from your Discard Pile into your Hand", Headbutt, Hologram, Dredge), enemy deaths, HP loss, Block retention (Barricade, Sturdy Clamp), "this turn" buffs, most "for each" scaling. The Relics tab and power pages say what is simulated.

[research/](research/README.md) goes card by card and relic by relic: what each reads, what it misses, and how cards and relics interact.
