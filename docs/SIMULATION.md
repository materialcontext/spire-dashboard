# Deck simulator: order of operations

`spire_core::analysis::sim` plays seeded fights against a passive target, 8 turns each (`simulation.turns`: about a boss fight; elites and hallway fights end sooner). The survival check reads turns past the last as the last one. Order changes results; this page fixes it.

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
4. "Every N plays in a turn" counters and "the first time each turn" flags reset; this turn's rules clear (cost changes, draw locks, play limits, kept hand), and last turn's "next turn, Attacks deal double" applies.
5. Start-of-turn timings: relics, then powers in play order ("start of your turn", "turn N", "first N turns", "turn N onward", "every N turns"); their draws join the hand draw.
6. Effects queued by cards for this turn arrive ("Next turn, draw 2", "At the start of the next 2 turns, Channel…"); cards returning to the hand come back from the discard pile.
7. Draw; each draw fires on-draw triggers (and grows or cheapens cards that change on draw), a reshuffle fires on-shuffle. *[Codex]*

**Playing a card**
1. Pay energy and Stars. The cost is the printed one, changed by this card's own changes (growth in cost, cheaper per draw), this turn's rules (cheaper per X, next card of a type free, a capped or free hand, everything costs 1 more), "Skills cost 0", and the first-N-free passive. A card that "can only be played if…" waits until it can.
2. Resolve, twice if echoed (Echo Form: first card a turn; Throwing Axe: first of combat), plus Replay and enchantment replays: channel orbs (full slots evoke the leftmost first *[Codex]*), Arrow bookkeeping, then its choices (discard, exhaust, fetch, autoplay, evoke, orb passives), damage, Block and effects. Conditions gate the card or a bonus; counts scale its damage, hits, Block or draw.
3. "Whenever you play" and "every N plays" triggers.
4. A power's triggers start with the **next** card (no self-trigger). *[Player]* Test: `powers_trigger_on_later_cards_not_themselves`.
5. Cap: 50 cards a turn (free draw loops end). "Next turn, gain N Energy" arrives next turn.
6. Exhaust (keyword or Corruption) fires exhaust triggers; otherwise discard **after** resolving, so the card's own draw can't reshuffle it.

**End of turn**
1. End-of-turn triggers: relics, then powers.
2. Plating adds Block.
3. Orb passives, leftmost first; Glass passive then −1, evoke at twice the passive. *[Codex]*
4. Poison ticks, then −1; Accelerant adds ticks. *[Data]*
5. Enemy Vulnerable and Weak −1. *[Codex]*
6. Held cards' end-of-turn effects (Burn, Decay, Regret: lose HP).
7. Retain stays (all with Runic Pyramid or Well-Laid Plans, or a card that keeps the hand this turn); Ethereal exhausts; the rest discards. *[Codex]*

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
- Forge: the first Forge in a fight adds the Forge card (`GameRules::forge_card`, Sovereign Blade) to the hand, or the discard pile when the hand is full; every Forge adds its amount to that card's damage, wherever it is. It then cycles like any card. Sword Sage's Replay plays it again; Conqueror doubles it for the turn and Parry gives Block per play. Seeking Edge (hits all) isn't read. *[Codex glossary]*
- Made cards (`mechanics::card_text`): "Add N <card> into your Hand / Draw Pile / Discard Pile" (Shivs, Souls, Sadness, Joy, Wounds, Burns, Sweeping Gaze…) makes that card there; a full hand sends it to the discard pile, the draw pile takes it at a random place. Powers do it on their timing (Infinite Blades each turn). "Add a copy of this card" copies it. A random card ("add a random Attack", Colorless, a class Power, an Ethereal card) is drawn from an even spread of up to 12 cards of that pool, Common to Rare.
- The Exhaust Pile is kept: exhaust triggers (Feel No Pain, Dark Embrace), "N additional damage for each card (or Soul) in your Exhaust Pile", "if you have 3 or more cards in your Exhaust Pile", "if you Exhausted a card this turn", "when this card is Exhausted", "costs 1 less for each card Exhausted this combat", and "if this is in your Exhaust Pile, play it" (Bombardment, Howl from Beyond).
- "Exhaust your Hand / all non-Attacks / all Status cards / all Sadness … for each card Exhausted": the card's own numbers (or the sentence's effect) once per card taken. "Discard your Hand … for each card discarded" likewise.
- "This turn, your next Skill (Attack) is played an extra time" plays the next one twice.
- A card draws only when its text says "draw": the data's card count also marks "Add 3 Shivs" and "3 or more cards". A base damage given only as the calculation's base (Ashen Strike, Soul Storm) counts as its damage.
- Each turn's Block keeps its spread (`TurnOutput::block_steps`, every 5%) for the survival check.

**Open:** rounding per hit (Codex) versus once at the end (Wiki); they differ only when fractions add up across hits.

## How cards are picked

Each play takes the card worth most per energy for the turn's lean (damage or Block). What lasts counts for the turns left in the fight, so setup is played early and skipped late:

- Strength, Dexterity and Focus are worth what they add each turn (about two hits, a Block card, every orb) times the turns they hold.
- A power's triggers count every firing still to come: Demon Form's Strength each turn counts once per later turn, each for the turns after it, so on turn 1 it outranks any Strike and on the last turn it's barely worth playing. Triggers on events (cards played, draws) count a firing or a few a turn.

## Choices

The simulator has no player, so choices follow a policy:

- **Discard and exhaust** a chosen card: Sly cards first (a discarded Sly card plays for free), then an unplayable card, then the card worth least to play under either lean. Random exhausts pick at random.
- **Fetch** from a pile (tutors, "put a card from your Discard Pile into your Hand", Headbutt's top-of-draw) and **autoplay** ("play the top card", "play 2 random cards from your Hand", Knife Trap, Cho Chorus): the card worth most to play under either lean, among those the text allows.
- **Evoke** on demand takes the rightmost orbs (or all, or the leftmost, as the text says); orb passives on demand likewise.

## Character systems

- **Osty**: his HP is the sum of Summons this fight. He takes no damage (there is no enemy) and dies only to his own cards (Bone Shards, Sacrifice). His attacks deal his damage only, without your Strength. *[community guides]*
- **Doom** sits on the target. Time's Up deals it; "if you applied Doom this turn" reads it. There is no enemy HP, so no kill.
- **Melodies**: playing a Wah, Cho or Ki card gains that Melody for the rest of the fight. That fires "whenever you gain a Melody" and "when you gain every Melody", and feeds the Melody conditions and counts. *[community guides; the data doesn't say]*
- **HP loss** (cards, held curses and Burns, powers) is counted. It fires on-HP-loss triggers (Rupture, Inferno, Centennial Puzzle), feeds Spite and Tear Asunder, and goes to the survival check as `TurnOutput::self_damage`.
- **Enemy Weak** is applied and kept. Each turn reports how often the enemy ends it Weak, times Weak's percentage (`GameRules::weak_pct`, from the power data; Paper Krane's 40%), as `TurnOutput::mitigation`. The survival check takes that share off incoming damage.
- **Enchantments** on deck cards (`DeckCard.enchantment` and its amount) change the card's play: Sharp, Instinct, Tezcatara's Ember, Corrupted, Nimble, Adroit, Inky, Vigorous, Swift, Sown, Glam, Spiral, Momentum, Goopy, Soul's Power, Steady, Royally Approved and Imbued.

## Not read yet

`audit -- --mechanics`: 101 of 133 powers and 113 of 315 relics are read; most unread relics act outside combat. `audit -- --coverage` prints what the simulator reads from every card and relic. Still open: kills and execute effects (no enemy HP), Thorns, Intangible and Buffer (no enemy attacks), upgrades and transforms in combat, Block kept across turns (Barricade, Blur, Sturdy Clamp), copies of a chosen card, draw-until and fill-the-hand, Confused, and encounter-kind relics. The Relics tab and power pages say what is simulated.

[research/](research/README.md) goes card by card and relic by relic: what each reads, what it misses, and how cards and relics interact.
