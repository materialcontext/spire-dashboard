# Card dossier

Every card in the Slay the Spire 2 beta (Spire Codex data, v0.113.1): what it does, what the deck simulator reads from it, and what it would take to read the rest. Written for the simulator overhaul; [INTERACTIONS.md](INTERACTIONS.md) builds on it, and [RELICS.md](RELICS.md) does the same for relics.

Card text and numbers come from the Codex card data (`cards.json`, with `description_raw`, vars and upgrades). Keyword and power rules come from the Codex keyword, glossary and power definitions. Where the data is silent, a note says so; two facts come from community guides instead, and are marked where used (how Melodies are gained, and that Strength does not apply to Osty). The simulator reading is generated from the code at the commit that adds this file.

## How to read an entry

```
**Name** · cost · type · rarity · keywords · coverage
Card text.
*Upgraded:* upgraded text, or what the upgrade changes.
*Makes:* cards it creates.
*Simulator:* what `play_of` and the mechanics reader take from it.
Note on how it works and what is missing. `GAP_CODES`
```

Coverage: ✅ read (everything that matters to the fight output is modelled), ◐ partly read, ○ not read (the card does nothing in the simulator), ⚠️ misread (the simulator credits something the card doesn't do, or drops what it does do in a way that changes today's numbers). Gap codes are listed in [Gaps](#gaps), grouped by the simulator change that would close them.

## Summary

| Group | Cards | ✅ | ◐ | ○ | ⚠️ |
|---|---:|---:|---:|---:|---:|
| Ironclad | 90 | 37 | 31 | 16 | 6 |
| Silent | 91 | 37 | 36 | 13 | 5 |
| Defect | 91 | 40 | 25 | 18 | 8 |
| Necrobinder | 91 | 38 | 21 | 13 | 19 |
| Regent | 91 | 35 | 28 | 16 | 12 |
| The Fool | 90 | 26 | 26 | 23 | 15 |
| Colorless | 66 | 23 | 16 | 23 | 4 |
| Tokens (made in combat) | 39 | 19 | 2 | 14 | 4 |
| Event and Ancient cards | 32 | 10 | 5 | 11 | 6 |
| Statuses | 13 | 4 | 0 | 6 | 3 |
| Curses | 18 | 9 | 0 | 5 | 4 |
| Quest cards | 4 | 4 | 0 | 0 | 0 |
| **All** | **716** | **282** | **190** | **158** | **86** |

Most cards that are ✅ are plain damage, Block, draw and energy. The misreads cluster in a few causes, each fixable in one place:

- `TRIGGER_FALLBACK` (21 misread): A power trigger the reader doesn't know ("Whenever you apply…", "Whenever you create…") loses its timing and is read as a one-time effect on play. Fix: a sentence that opens with "whenever" or "every" and has no known timing should be reported unread, not played. Then add the timings one by one (below).
- `OSTY_DOUBLE` (10 misread): Osty attacks carry both `damage` and `ostydamage`; the simulator adds both, and the player copy also takes Strength and Vulnerable. Use only `ostydamage` when the text says "Osty deals".
- `ENERGY_VAR` (6 misread): An `energy` var that is a cost or threshold ("costs 2 less", "costs 2 or more", "if X is 4 or more") is read as energy gained. Only a "Gain N Energy" sentence should give energy.
- `VAR_POWER` (5 misread): Weak and Vulnerable stored under a generic `power` var ("Apply 1 Weak. Apply 1 Vulnerable.") are dropped. Map `power` to every debuff the text names.
- `DRAW_NEXT` (5 misread): "Next turn, draw N" draws now (or, without a `cards` var, not at all). Move it to `pending_draw` for next turn.
- `DEAD_PLAYABLE` (3 misread): Statuses and curses with a cost (Slimed, Debris, Toxic, Spore Mind) are treated as unplayable. They can be played to clear them, and their exhaust feeds exhaust payoffs.
- `DOUBLE_CHANNEL` (3 misread): A power's channel list (from the data) fires on play, and its trigger fires again. Powers should channel through their triggers only.
- `DEX_TEMP` (1 misread): "N Dexterity this turn" is kept all fight.

## Rules the cards rely on

From the Codex glossary, keyword and power definitions unless marked otherwise.

**Turn.** You draw 5 cards at the start of each turn. Energy pays for cards (3 per turn for every character). Block prevents damage until your next turn. When the Draw Pile is empty, the Discard Pile is shuffled into it.

**Keywords.**

- Exhaust: removed until the end of combat.
- Ethereal: exhausted if still in the hand at end of turn.
- Retain: not discarded at end of turn.
- Innate: starts each combat in the hand.
- Sly: if discarded from the hand before the end of your turn, it is played for free.
- Unplayable: can't be played.
- Eternal: can't be removed or transformed from the deck.
- Replay: plays the card an additional time.
- Fatal: triggers when the card kills a non-minion enemy.
- Transform: the card becomes a random card of any rarity, unless the text names one.
- Stun: the enemy doesn't act on its next turn.

**Buffs and debuffs.**

- Strength: + attack damage per hit.
- Dexterity: + Block from cards.
- Focus: + orb effects (not Plasma).
- Vigor: the next Attack deals that much more.
- Vulnerable: +50% attack damage taken, for N turns.
- Weak: −25% attack damage dealt.
- Frail: −25% Block from cards.
- Poison: lose that much HP at the start of the creature's turn, then 1 less.
- Doom: at the end of the enemy turn, the creature dies if its HP is at or below its Doom.
- Plating: Block at end of turn, −1 each turn.
- Thorns: damage back when hit by an attack.
- Intangible: all damage and HP loss reduced to 1 this turn.
- Buffer: prevents the next HP loss.
- Artifact: negates debuffs.

**Character systems.**

- *Defect, orbs (3 slots):*
  - Channelling fills the first empty slot.
  - With no slot free, the first orb is evoked to make room.
  - Evoke consumes the rightmost orb, unless the card says leftmost.
  - The orbs, with passive and then evoke values:

    | Orb | Passive | Evoke |
    |---|---|---|
    | Lightning | 3 to a random enemy at end of turn | 8 |
    | Frost | 2 Block at end of turn | 5 Block |
    | Dark | gains 6 each end of turn | its stored damage, to the lowest-HP enemy |
    | Glass | 4 to all enemies at end of turn, then loses 1 | 8 to all |
    | Plasma | 1 energy at start of turn | 2 energy; ignores Focus |
- *Necrobinder, Osty:*
  - Summon N summons Osty, or raises his Max HP for this combat if he is already up.
  - Bound Phylactery (the starting relic) summons 1 each turn.
  - Osty attacks ("Osty deals") are his, not yours. Community guides say Strength doesn't apply to them; the Codex data doesn't say.
  - Souls are 0-cost Exhaust Skills that draw 2.
- *Regent, Stars and Forge:*
  - Stars are a second resource that some cards cost (shown as "+ N Stars").
  - The first Forge each combat adds Sovereign Blade (2 energy, 10 damage, Retain) to the hand. Each Forge adds its amount to the Blade's damage.
- *The Fool, Arrows (1 slot):*
  - Cards that "Shoot an Arrow" spend an Arrow, which sticks in the enemy (Impaled).
  - An Impaled enemy's Arrow is Dislodged (returned) after you hit it 3 more times, or by a card that Dislodges.
  - Aim adds damage to your next Arrow; Song Bow (the starting relic) starts each combat with 6 Aim.
  - Stretch adds Block to your next Block gain from a card.
- *The Fool, Melodies:*
  - Wah, Cho and Ki Melodies are what Melody payoffs count ("each Melody you have", "when you gain every Melody").
  - The data doesn't say how a Melody is gained. Community guides say playing a Wah, Cho or Ki card gains that Melody.
- *The Fool, other cards:*
  - Sadness is a Retain, Unplayable Status that Fool cards make; Growth, Punch of Joy and Ecstasy turn it into value.
  - Joy is a 0-cost Exhaust Skill: +1 energy, draw 1.
  - Mythic Form makes random Tarot cards (22 tokens).

**What the simulator models.** It plays seeded fights against a passive target:

- It shuffles, draws 5, and spends energy greedily under two policies: one leaning to damage, one to Block.
- It carries Strength, Dexterity, Focus, orbs, Poison, Plating, Vulnerable, Aim, Stretch, Vigor, Stars, Arrows, Forge and the Exhaust Pile between turns.
- Powers and relics run through the mechanics reader's triggers.
- It has no enemy (no attacks, no HP, no kills), no player HP, no potions, no choices, and no random cards. Survival is a separate check (`survival.rs`) against the enemy's turn patterns.

## Gaps

Each code names one thing the simulator lacks. The count is how many cards need it; ⚠️ counts cards where it causes a misread today.

### Reader bugs

The card is read, but read wrong. These change today's numbers, so they come first. Most are local fixes in `play_of`, `mechanics::read` or the tag reader.

| Code | Cards | ⚠️ | What it needs | Cards |
|---|---:|---:|---|---|
| `TRIGGER_FALLBACK` | 21 | 21 | A power trigger the reader doesn't know ("Whenever you apply…", "Whenever you create…") loses its timing and is read as a one-time effect on play. Fix: a sentence that opens with "whenever" or "every" and has no known timing should be reported unread, not played. Then add the timings one by one (below). | All Is Well, Arsenal, Automation, Black Hole, Caltrops, Devour Life, Envenom, Inferno, Motif, Orbit, Pagestorm, Parry and 9 more |
| `OSTY_DOUBLE` | 10 | 10 | Osty attacks carry both `damage` and `ostydamage`; the simulator adds both, and the player copy also takes Strength and Vulnerable. Use only `ostydamage` when the text says "Osty deals". | Bone Shards, Fetch, Flatten, High Five, Poke, Rattle, Right Hand Hand, Sic 'Em, Snap, Sweeping Gaze |
| `ENERGY_VAR` | 6 | 6 | An `energy` var that is a cost or threshold ("costs 2 less", "costs 2 or more", "if X is 4 or more") is read as energy gained. Only a "Gain N Energy" sentence should give energy. | Banshee's Cry, Expect a Fight, Heavenly Drill, Melancholy, Right Hand Hand, Transfigure |
| `VAR_POWER` | 7 | 5 | Weak and Vulnerable stored under a generic `power` var ("Apply 1 Weak. Apply 1 Vulnerable.") are dropped. Map `power` to every debuff the text names. | Expose, Hobbling Shot, Nail, Putrefy, Shockwave, The Empress, Uppercut |
| `DRAW_NEXT` | 6 | 5 | "Next turn, draw N" draws now (or, without a `cards` var, not at all). Move it to `pending_draw` for next turn. | Glow, Guiding Star, Plot, Predator, Relax, The Star |
| `DEAD_PLAYABLE` | 4 | 3 | Statuses and curses with a cost (Slimed, Debris, Toxic, Spore Mind) are treated as unplayable. They can be played to clear them, and their exhaust feeds exhaust payoffs. | Debris, Slimed, Spore Mind, Toxic |
| `DOUBLE_CHANNEL` | 3 | 3 | A power's channel list (from the data) fires on play, and its trigger fires again. Powers should channel through their triggers only. | Consuming Shadow, Spinner, Storm |
| `FOCUS_TEMP` | 4 | 1 | "N Focus this turn" (or −N) is kept all fight, or dropped on an enemy-targeted card. | Focused Strike, Hotfix, Hyperbeam, Synchronize |
| `STR_SELF_ON_ATTACK` | 2 | 1 | A self-buff on a card that targets enemies is skipped (`on_self` is false). Read the subject of each sentence instead of the card's target. | Fight Me!, Resonance |
| `STR_TEMP` | 2 | 1 | "Gain N Strength this turn" is kept all fight (or dropped when the card targets an enemy). Needs a this-turn Strength pool. | Feeding Frenzy, Setup Strike |
| `AIM_MULT` | 1 | 1 | "Aim affects this card 2 times" has no `aimmultiplier` var; read the number from the text. | Heavy Shot |
| `DELAYED_CHANNEL` | 1 | 1 | "At the start of the next 2 turns, Channel…" channels now. | Lightning Rod |
| `DEX_TEMP` | 1 | 1 | "N Dexterity this turn" is kept all fight. | Anticipate |
| `EVENT_VARIANT` | 1 | 1 | An event card whose effect depends on the event choice; both variants' vars are applied. | Mad Science |
| `SHOOT_FLAG` | 1 | 1 | "Whenever you Shoot an Arrow" makes the card a shooter. Match "Shoot an Arrow to" only. | Whistling Arrows |
| `STARS_VAR` | 1 | 1 | "Gain 9 Stars" reads as 0: the Stars check looks for the base value in the text and misses it. | Royal Gamble |
| `VAR_HITS` | 1 | 1 | "Gain 5 Block twice" reads one gain. Block needs a repeat count like `hits`. | Cho Limbs |
| `VAR_REPEAT` | 1 | 1 | `repeat` ("3 Poison 3 times") isn't multiplied in. | Bouncing Flask |
| `X_DOUBLE` | 1 | 1 | "Double X if it's 4 or more." | Heavenly Drill |
| `X_STARS` | 1 | 1 | X paid in Stars (`is_x_star_cost`) isn't read: hits should equal the Stars spent. | Stardust |

### Conditions and counts on state the simulator already tracks

Vulnerable, Poison, stuck Arrows, orbs, the piles, draws, plays and Stars are already tracked per fight. These cards only need a condition or a multiplier read from that state.

| Code | Cards | ⚠️ | What it needs | Cards |
|---|---:|---:|---|---|
| `IMPALE_COND` | 8 | 6 | Effect only if the enemy is Impaled (`stuck > 0`). | Arrow Stab, Bow Smack, Ki Hop, Nail, Recall, Uproot, Wah Hammer, Wrest |
| `PLAY_COND` | 3 | 2 | "Can only be played if…" / "If the enemy has Poison…": gate the card on tracked state. | Bubble Bubble, Clash, Grand Finale |
| `COUNTER_DRAWN` | 3 | 1 | Per card drawn this turn or combat. | Automation, Death March, Murder |
| `COUNTER_ENERGY` | 2 | 1 | Per energy spent. | Helix Drill, Orbit |
| `DISLODGE_COND` | 1 | 1 | Effect only if a stuck Arrow is dislodged. | Epic Retrieval |
| `VULN_COND` | 1 | 1 | Effect only if the enemy is Vulnerable (`vulnerable > 0`). | Dismantle |
| `COUNT_ORBS` | 4 |  | Per orb, or per unique orb, in the row. | Barrage, Compile Driver, Coolant, Synchronize |
| `DECK_COUNT` | 4 |  | Per deck card matching a name or cost (Strikes, Star-cost cards, Wah/Cho/Ki cards, Osty Attacks). | Crescent Spear, Perfected Strike, Squeeze, Wah Symphony |
| `DRAW_COND` | 3 |  | Effect only if the drawn card or hand meets a condition. | Escape Plan, Impatience, Restlessness |
| `VULN_STACKS` | 3 |  | Scales with (or doubles) the enemy's Vulnerable. | Bully, Dominate, Molten Fist |
| `COUNTER_MADE` | 2 |  | Per card (or Sadness) created this combat. | Anguish, Supermassive |
| `COUNTER_PLAYS` | 2 |  | Per card played this turn or combat. | Gold Axe, Pale Blue Dot |
| `COUNT_HAND` | 2 |  | Per (other) card or Skill in hand. | Flechettes, Precise Cut |
| `ARROW_COND` | 1 |  | Effect only with an Arrow ready (`arrows > 0`). | Cho Firm |
| `BLOCK_DMG` | 1 |  | Damage equal to current Block. | Body Slam |
| `BLOCK_FROM_DMG` | 1 |  | Block equal to damage dealt. | Fisticuffs |
| `BLOCK_FROM_POISON` | 1 |  | Block equal to enemy Poison. | Mirage |
| `COND_DOOM` | 1 |  | Effect only if Doom was applied this turn. | Death's Door |
| `COUNTER_ATTACKS` | 1 |  | Per Attack played this turn. | Finisher |
| `COUNTER_CHANNEL` | 1 |  | Per Lightning channelled this combat. | Voltaic |
| `COUNTER_ETHEREAL` | 1 |  | Per Ethereal card played this combat. | Pull from Below |
| `COUNTER_HITS` | 1 |  | Per hit on the enemy this turn. | Beat into Shape |
| `COUNTER_SKILLS` | 1 |  | Per Skill played this turn. | Lunar Blast |
| `COUNTER_STARS` | 1 |  | Per Star gained this turn. | Radiate |
| `COUNT_DEBUFFS` | 1 |  | Per unique enemy debuff. | Rend |
| `COUNT_DISCARD` | 1 |  | Per card in the Discard Pile. | Stack |
| `COUNT_DRAW_PILE` | 1 |  | Per card in the Draw Pile. | Mind Blast |
| `COUNT_SOURCE` | 1 |  | Per card a named card exhausted. | The Shot |
| `DEBUFF_AMP` | 1 |  | Vulnerable and Weak doubled for N turns. | Debilitate |
| `FLOOR` | 1 |  | Scales with floors travelled (run state). | The Tower |
| `GOLD` | 1 |  | Scales with Gold (run state). | My Shinies |
| `ORB_COND` | 1 |  | Effect only with a Frost orb. | Hailstorm |
| `POISON_NOW` | 1 |  | "Poison triggers immediately": one extra tick now. | Outbreak |
| `STRETCH_FROM_DMG` | 1 |  | Stretch equal to damage dealt. | Blossom Shot |
| `STRETCH_REGAIN` | 1 |  | Regain spent Stretch. | Cho Ki |
| `WEAK_DMG` | 1 |  | Weak enemies take more damage. Needs enemy Weak tracked like Vulnerable. | Tracking |

### This-turn and next-turn effects

Effects that last until end of turn, or arrive at the start of the next one. They need a small per-turn modifier record that `end_turn` clears, and a next-turn queue like `next_energy`.

| Code | Cards | ⚠️ | What it needs | Cards |
|---|---:|---:|---|---|
| `COST_DOWN` | 6 | 3 | Costs 1 less per X this turn. | Banshee's Cry, Expect a Fight, Flatten, Pinpoint, Rocket Punch, Stomp |
| `ON_APPLY` | 3 | 2 | Trigger: whenever you apply a debuff (Vulnerable, Doom, any). `apply` already counts debuffs. | Shroud, Sleight of Flesh, Vicious |
| `ON_PLAY_NAMED` | 3 | 2 | Trigger: whenever you play a named card (Soul, Sovereign Blade). `OnPlay(Named)` exists; the reader needs the phrase. | Devour Life, Haunt, Parry |
| `ON_MAKE` | 2 | 2 | Trigger: whenever you create a card. `make` is the single entry point. | Arsenal, Pillar of Creation |
| `ON_PLAY_TEMP` | 4 | 1 | "Whenever you play a card this turn, …": a trigger that expires at end of turn. | Monologue, Oblivion, Rage, Strangle |
| `PLAY_LIMIT` | 3 | 1 | Only N more cards this turn. | Encore, Normality, Sloth |
| `DELAYED_DMG` | 2 | 1 | Damage after N turns. | God Byrd, The Bomb |
| `ON_STARS` | 2 | 1 | Trigger: whenever you gain or spend Stars. `gain`/`spend(Store::Stars)` are the hooks. | Black Hole, Child of the Stars |
| `ON_STATUS` | 2 | 1 | Trigger: whenever you create a Status card. | Smokestack, Trash to Treasure |
| `ON_DRAW_ETHEREAL` | 1 | 1 | Trigger: whenever you draw an Ethereal card. | Pagestorm |
| `ON_EVOKE` | 1 | 1 | Trigger: whenever an orb is evoked. | Thunder |
| `ON_MAKE_NAMED` | 1 | 1 | Trigger: whenever you create a named card (Sadness). | All Is Well |
| `ON_OSTY_HIT` | 1 | 1 | Trigger: whenever Osty hits this turn. | Sic 'Em |
| `ON_PLAY_ETHEREAL` | 1 | 1 | Trigger: whenever you play an Ethereal card. Needs a keyword filter. | Spirit of Ash |
| `ON_UNBLOCKED` | 1 | 1 | Trigger per unblocked Attack hit. Against a passive target every hit is unblocked; count hits. | Envenom |
| `RETAIN_TEMP` | 5 |  | Retain your hand this turn. | Convergence, Equilibrium, Expertise, Ki Rest, Salvo |
| `BLOCK_NEXT` | 4 |  | Block at the start of the next turn(s). | Dodge and Roll, Glitterstream, Prolong, Toric Toughness |
| `COST_NEXT` | 4 |  | The next card of a type costs 0. | Pounce, Synthesis, Unrelenting, Veilpiercer |
| `BLOCK_MULT` | 3 |  | Double Block gained (this turn, or the first gain) or current Block. | Entrench, Shadowmeld, Unmovable |
| `BLOCK_KEEP` | 2 |  | Block carries into the next turn. The simulator logs Block per turn; it needs a carried pool. | Barricade, Blur |
| `DRAW_LOCK` | 2 |  | No more draws this turn. | Battle Trance, Bullet Time |
| `STARS_NEXT` | 2 |  | Stars next turn. | Convergence, Hidden Cache |
| `BLOCK_LOCK` | 1 |  | No Block from cards for 2 turns. | Panic Button |
| `COST_CAP` | 1 |  | Hand costs capped at 1. | Enlightenment |
| `COST_UP_TEMP` | 1 |  | Every card costs 1 more this turn. | Borrowed Time |
| `COST_ZERO_HAND` | 1 |  | Every hand card free this turn. | Bullet Time |
| `DMG_MULT_NEXT` | 1 |  | Next turn, Attacks deal double. | Shadow Step |
| `END_TURN` | 1 |  | Playing it ends the turn. | Void Form |
| `ENERGY_MULT` | 1 |  | Double current energy. | Double Energy |
| `EXTRA_TURN` | 1 |  | Take an extra turn. | The Hanged Man |
| `FIRST_PLAY_BONUS` | 1 |  | The first named card each turn deals extra. | Phantom Blades |
| `FREE_FIRST` | 1 |  | The first N cards each turn are free. | Void Form |
| `NEXT_EXTRA_NAMED` | 1 |  | The next named card played twice. `next_extra` needs a name filter. | Ki Verse |
| `NEXT_EXTRA_POWER` | 1 |  | The next Power played twice. `next_extra` needs the Power type from the reader. | Signal Boost |
| `ON_DISLODGE` | 1 |  | Trigger: first dislodge each turn. | Yah! |
| `ON_DRAW_STATUS` | 1 |  | Trigger: the first time you draw a Status each turn. | Iteration |
| `ON_DRAW_TEMP` | 1 |  | "Whenever you draw a card this turn, …". | Corrosive Wave |
| `ON_IMPALE` | 1 |  | Trigger: whenever you Impale (shoot). Hook into `arrows_after`. | Explosive Arrows |
| `SHIV_AOE` | 1 |  | Shivs hit all enemies (matters against groups). | Fan of Knives |

### Per-card state

Cards that change themselves or other cards during a fight (growth, cost changes, upgrades, granted keywords, transforms). `Play` is shared by every copy; these need per-instance state, like the Forge card's damage.

| Code | Cards | ⚠️ | What it needs | Cards |
|---|---:|---:|---|---|
| `GRANT_KEYWORD` | 2 | 1 | Gives a hand card Ethereal or Retain. | Sculpting Strike, Snap |
| `GRANT_REPLAY` | 2 | 1 | Gives a card Replay. | Hidden Gem, Transfigure |
| `TRANSFORM` | 10 |  | Transforms cards into named cards (or random ones). | BEGONE!, CHARGE!!, Compact, Ecstasy, Entropy, Florescence, GUARDS!!!, Primal Force, Punch of Joy, Seance |
| `SELF_GROW` | 5 |  | Damage grows for this card (or every card of its name) each play this combat. | Claw, Hang, Maul, Rampage, Rolling Boulder |
| `UPGRADE` | 4 |  | Upgrades cards in combat. | Aggression, Apotheosis, Armaments, Drain Power |
| `PERMANENT_GROW` | 2 |  | Grows permanently across fights. Price it by the floors left. | Genetic Algorithm, The Scythe |
| `SELF_COST_DOWN` | 2 |  | Its own cost drops after each play. | Momentum Strike, Up My Sleeve |
| `COPY_COST` | 1 |  | The copy it makes costs 0. | Adaptive Strike |
| `COST_DOWN_DRAW` | 1 |  | Its cost drops each time it is drawn. | Kingly Kick |
| `NAMED_DMG_MULT` | 1 |  | A named card deals double this turn. | Conqueror |
| `RETAIN_GRANT` | 1 |  | Gives Shivs Retain. | Phantom Blades |
| `SELF_COST_UP` | 1 |  | Its own cost rises after each play. | Modded |
| `SELF_GROW_DRAW` | 1 |  | Damage grows each time it is drawn. | Kingly Punch |
| `TRANSFORM_SELF` | 1 |  | Turns into another card after play. | Mourning Blow |

### Choices and pile moves

Discards, tutors, putting cards back, autoplay. These need a choice policy (take the best card for the current lean), which the simulator lacks today.

| Code | Cards | ⚠️ | What it needs | Cards |
|---|---:|---:|---|---|
| `RETURN_TO_HAND` | 8 | 1 | Returns to the hand (next turn, or on a trigger). | Bolas, Bounce, Feral, Make It So, Particle Wall, Right Hand Hand, Thrumming Hatchet, Trick Shot |
| `AUTOPLAY` | 12 |  | Plays other cards (top of Draw Pile, random from a pile, cheapest in hand) for free. | Beat Down, Cascade, Catastrophe, Havoc, Hellraiser, Lost Lute, Mayhem, Simpleton, Stampede, The Devil, The Fool, Uproar |
| `PILE_MOVE` | 11 |  | Puts discard cards in the hand or on top of the Draw Pile. | Aggression, All for One, Arrow Stab, Cosmic Indifference, Dredge, Graveblast, Headbutt, Hologram, Neow's Fury, Recall, The World |
| `SLY` | 9 |  | Sly: played for free when discarded. Needs DISCARD_CHOOSE to pick Sly cards first. | Abrasive, Flick-Flack, Hand Trick, Master Planner, Memento Mori, Reflex, Ricochet, Tactician, Untouchable |
| `DISCARD_CHOOSE` | 7 |  | Discard N chosen cards. With Sly, the discarded card plays for free. | Acrobatics, Dagger Throw, Hidden Daggers, Prepared, Survivor, The Hermit, Tools of the Trade |
| `EXH_CHOOSE` | 7 |  | Exhaust a chosen card. Feeds Exhaust-Pile payoffs. | Brand, Burning Pact, Cleanse, Purity, Scavenge, True Grit, Tyranny |
| `TUTOR` | 7 |  | Fetches a chosen or typed card from the Draw Pile. | Anointed, Locked In, Secret Technique, Secret Weapon, Seeker Strike, Stratagem, Wish |
| `PILE_TOP` | 6 |  | Puts a card (from hand, or itself) on top of the Draw Pile. | Glimmer, Nostalgia, Photon Cut, Rebound, Shining Strike, Thinking Ahead |
| `EXH_RANDOM` | 3 |  | Exhaust a random card. | Cinder, Thrash, True Grit |
| `COPY_CHOSEN` | 2 |  | Copy a chosen hand card. | Dual Wield, Heirloom Hammer |
| `EXHAUST_REPLAY` | 2 |  | Plays cards from the Exhaust Pile. | Eidolon, Knife Trap |
| `FILL_HAND` | 2 |  | Fill the hand with a named card. `Make` with n = free hand slots. | Crash Landing, The Moon |
| `AUTOPLAY_TOP` | 1 |  | Plays itself from the top of the Draw Pile. | I Am Invincible |
| `CHOOSE_COPY` | 1 |  | Copies of a chosen card next turn. | Nightmare |
| `CHOOSE_REPLAY` | 1 |  | Plays a chosen hand Skill N times. | Decisions, Decisions |
| `COPY_PLAYED` | 1 |  | Copy of the Nth Attack played. | Juggling |
| `DIG` | 1 |  | Look at the top N, take 1. | Ki Chant |
| `DRAW_FILL` | 1 |  | Draw to a full hand. | Scrawl |
| `DRAW_FILTER` | 1 |  | Draw N, discard the ones that don't cost 0. | Scrape |
| `DRAW_PER_DISCARD` | 1 |  | Draw as many as discarded. | Calculated Gamble |
| `DRAW_UNTIL` | 1 |  | Draw until a non-Attack. | Pillage |
| `PLAY_HAND_NAMED` | 1 |  | Plays every named card in hand. | Cho Chorus |
| `SHUFFLE_ALL` | 1 |  | Shuffle every pile into the Draw Pile. | Reboot |
| `TUTOR_NEXT` | 1 |  | Fetches cards next turn. | Foregone Conclusion |

### Random cards

Cards that make random cards. Treat the result as an average card of the named pool (class, Colorless, Attack, Power), measured from the catalog.

| Code | Cards | ⚠️ | What it needs | Cards |
|---|---:|---:|---|---|
| `RANDOM_GEN` | 22 |  | Makes random cards. | Abundance, Blot out the Sky, Bundle of Joy, Calamity, Call of the Void, Creative AI, Discovery, Distraction, Hello World, Infernal Blade, Jack of All Trades, Jackpot and 10 more |
| `RANDOM_ORB` | 2 |  | Channels random orbs. | Chaos, The Magician |

### Character systems

Mechanics one character owns: Defect's evokes, Necrobinder's Osty and Doom, Regent's Stars, The Fool's Melodies and Arrows, and Status cards in hand.

| Code | Cards | ⚠️ | What it needs | Cards |
|---|---:|---:|---|---|
| `MELODY_TRIGGER` | 6 | 3 | Trigger on gaining a Melody (any, Wah, Cho, every). | Lost Lute, Lullaby, Motif, Song Path, Tune, Violent Chord |
| `MELODY` | 11 | 2 | A Wah, Cho or Ki card. Community guides (not the Codex data) say playing one gains that Melody. Needs Melody state before any Melody payoff can work. | Cho Barbs, Cho Firm, Cho Ready, Ki Hop, Ki Whip, Wah Cho, Wah Cho Ki, Wah Hammer, Wah Ki, Wah Sweep, Wah Touch |
| `STATUS_EFFECT` | 4 | 2 | A Status that hurts while held or drawn (lose energy, draw fewer). | Amnesia, Mind Rot, Void, Waste Away |
| `EVOKE` | 5 | 1 | Evoke orbs on demand (rightmost N times, all twice, leftmost at end of turn). Today orbs evoke only when pushed out of a full row, so Dualcast, a starter card, does nothing. | Consuming Shadow, Dualcast, Multi-Cast, Quadcast, Shatter |
| `MELODY_COND` | 3 | 1 | Effect only with a given Melody. | Cho Limbs, Ki Rest, Wah Blast |
| `COUNTER_OSTY` | 1 | 1 | Per Osty attack this turn. | Rattle |
| `DOOM_SCALE` | 1 | 1 | Doom scaling with Doom already applied. | No Escape |
| `FORCED_PLAY` | 1 | 1 | Must be played first. | Enthralled |
| `OSTY_ALIVE` | 1 | 1 | Needs Osty alive, and kills him. | Bone Shards |
| `MELODY_COUNT` | 5 |  | Scales with Melodies held. | Coda, Harmonic Shield, Melodic Shot, Rhythm Strike, Steady Cadence |
| `OSTY_SCALE` | 4 |  | Scales with Osty's current or Max HP. Needs Osty HP (the sum of Summons). | Protector, Sacrifice, Squeeze, Unleash |
| `ORB_TRIGGER` | 3 |  | Trigger orb passives on demand. | Darkness, Loop, Tesla Coil |
| `SHOT_AMP` | 3 |  | Shots deal more (Honing, Dislike You, Quietus). | Dislike You, Honing, Quietus |
| `DOOM_FROM_DMG` | 2 |  | Doom equal to damage dealt. | Blight Strike, Reaper Form |
| `ARROW_X` | 1 |  | X shots should spend X Arrows. | Repeat Shot |
| `DOOM_DMG` | 1 |  | Damage equal to the enemy's Doom. Make Doom enemy state. | Time's Up |
| `DOOM_EXECUTE` | 1 |  | Kills now when Doom ≥ HP. | End of Days |
| `EXECUTE` | 1 |  | Sets a low-HP enemy to 0. | Judgement |
| `IMPALE_AMP` | 1 |  | More damage to Impaled enemies. | Serrated Arrows |
| `OSTY_HP_LOSS` | 1 |  | Triggers on Osty losing HP (enemy attacks). | Necro Mastery |
| `SELF_DOOM` | 1 |  | Doom on yourself: a survival clock. | Neurosurge |

### Survival effects

What a card does to the enemy's damage or your HP. The output simulator has no enemy, so these belong to the survival check (`survival.rs`), which already walks enemy turn patterns.

| Code | Cards | ⚠️ | What it needs | Cards |
|---|---:|---:|---|---|
| `SELF_HP` | 20 | 3 | Lose HP (or take damage) yourself. Also feeds HP-loss payoffs (Rupture, Inferno, Spite, Tear Asunder), which need a lost-HP counter even in the output simulator. | Bad Luck, Beckon, Blood Wall, Bloodletting, Brand, Breakthrough, Burn, Crimson Mantle, Decay, Disintegration, Hemokinesis, Infection and 8 more |
| `ENEMY_STR` | 11 | 1 | The enemy loses (or gains) Strength. | Crush Under, Dark Shackles, Dying Star, Enfeebling Touch, Fight Me!, Malaise, Mangle, Monarch's Gaze, Piercing Wail, Resonance, Shared Fate |
| `FATAL` | 7 | 1 | Effect on a kill. Only priceable with enemy HP; the turn view has the horizon. | Echoing Slash, Feed, Hand of Greed, Knockout Blow, Sunder, The Hunt, Timber |
| `SELF_DEBUFF` | 2 | 1 | Debuffs you while held (Weak, Frail). | Doubt, Shame |
| `THORNS` | 2 | 1 | Damage back per enemy hit. Needs the enemy's hit count. | Abrasive, Caltrops |
| `THORNS_TEMP` | 3 |  | Damage back per enemy hit this turn. Needs the enemy's hit count. | Cho Barbs, Flame Barrier, Reflect |
| `INTANGIBLE` | 2 |  | Intangible: all damage reduced to 1. | Apparition, Wraith Form |
| `BUFFER` | 1 |  | Prevents the next HP loss. | Buffer |
| `ENEMY_DMG` | 1 |  | Less damage from certain enemies. | Colossus |
| `HEAL` | 1 |  | Heals. | Not Yet |
| `STUN` | 1 |  | The enemy skips its next action. | Whistle |

### Out of scope

Multiplayer-only cards, run value (Gold, card rewards, Max HP), and encounter mechanics.

| Code | Cards | ⚠️ | What it needs | Cards |
|---|---:|---:|---|---|
| `MP` | 43 | 4 | Multiplayer only. Where the effect goes to another player but the numbers are credited to you, the card is marked wrong; fix by skipping cards that target an ally. | Beacon of Hope, Believe in You, Blade Symphony, Blaze, Cacophony, Compost, Concoct, Constellation, Coordinate, Covering Fire, Demonic Shield, Energy Surge and 31 more |
| `OUT_OF_COMBAT` | 8 |  | Run value (Gold, potions, deck removal). | Alchemize, Byrdonis Egg, Debt, Dowsing, Forbidden Grimoire, Lantern Key, Royalties, Spoils Map |
| `ENCOUNTER` | 1 |  | An encounter mechanic. | Frantic Escape |

## Tag reader errors

Tags feed roles, limits and the composition measures, so these matter beyond the simulator. Found by checking every card's tags against its text:

- **"Weak" from your own Strength loss.** The reader tags "lose … Strength" as Weak (cuts enemy damage). That is right for an enemy losing Strength (Mangle, Piercing Wail, Dark Shackles, Enfeebling Touch, Crush Under, Dying Star, Monarch's Gaze, Resonance, Shared Fate). It is wrong when you lose Strength, or when "lose" refers to HP or an orb slot in the same sentence: Brand, Bulk Up, Friendship, Rupture. Read the subject of "lose".
- **"Energy N" from a cost or threshold.** Banshee's Cry, Danse Macabre, Expect a Fight, Melancholy, Right Hand Hand, Transfigure and Tune are tagged as energy. This is the same `ENERGY_VAR` cause as in the simulator.
- **Effects on another player.** Believe in You and Constellation are tagged with energy you don't get.
- **Statuses tagged by their drawback.** Amnesia ("Draw 2, Energy 1"), Void ("Energy 1") and Slimed ("Draw 1") get positive tags from vars that describe a cost. Tags should skip Status and Curse cards, except a playable one's own effect.
- **Misread powers.** The `TRIGGER_FALLBACK` powers carry tags from the misread effect: Inferno "Area 6", Vicious "Draw 1", Smokestack "Area 5", Caltrops "Chip Damage 3". They are the right jobs, but from the wrong timing; the tags will hold once triggers are read.

## Cards

### Ironclad

#### Ironclad: Attacks

**Bash** · 2 energy · Attack · Basic · ✅ read  
Deal 8 damage. Apply 2 Vulnerable.  
*Upgraded:* Deal 10 damage. Apply 3 Vulnerable.  
*Simulator:* damage 8, Vulnerable 2.  

**Strike** · 1 energy · Attack · Basic · ✅ read  
Deal 6 damage.  
*Upgraded:* Deal 9 damage.  
*Simulator:* damage 6.  
Counts for Perfected Strike and Hellraiser ("Strike" in the name).

**Anger** · 0 energy · Attack · Common · ✅ read  
Deal 6 damage. Add a copy of this card into your Discard Pile.  
*Upgraded:* Deal 8 damage. Add a copy of this card into your Discard Pile.  
*Simulator:* damage 6, copy to discard.  
Each play adds a copy to the Discard Pile, so the deck thickens with 0-cost Attacks over a fight. Read through `copy_to`.

**Body Slam** · 1 energy · Attack · Common · ○ not read  
Deal damage equal to your Block.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Damage equals current Block (after Dexterity and Block from earlier plays this turn). Read as 0 damage. Needs the play order to put Block first. `BLOCK_DMG`

**Breakthrough** · 1 energy · Attack · Common · ◐ partly read  
Lose 1 HP. Deal 9 damage to ALL enemies.  
*Upgraded:* Lose 1 HP. Deal 13 damage to ALL enemies.  
*Simulator:* damage 9, all enemies.  
Damage and area read; the 1 HP loss isn't, so it never triggers Rupture, Inferno, Spite or Tear Asunder. `SELF_HP`

**Cinder** · 2 energy · Attack · Common · ◐ partly read  
Deal 18 damage. Exhaust 1 card at random.  
*Upgraded:* Deal 24 damage. Exhaust 1 card at random.  
*Simulator:* damage 18.  
Damage read; the random exhaust isn't, so Feel No Pain, Dark Embrace and the Exhaust Pile count miss it. `EXH_RANDOM`

**Headbutt** · 1 energy · Attack · Common · ◐ partly read  
Deal 9 damage. Put a card from your Discard Pile on top of your Draw Pile.  
*Upgraded:* Deal 12 damage. Put a card from your Discard Pile on top of your Draw Pile.  
*Simulator:* damage 9.  
Damage read; putting a Discard Pile card on top of the Draw Pile isn't. The best card in the discard would be drawn next turn. `PILE_MOVE`

**Iron Wave** · 1 energy · Attack · Common · ✅ read  
Gain 5 Block. Deal 5 damage.  
*Upgraded:* Gain 7 Block. Deal 7 damage.  
*Simulator:* damage 5, Block 5.  

**Molten Fist** · 1 energy · Attack · Common · Exhaust · ◐ partly read  
Deal 10 damage. Double the enemy's Vulnerable.  
*Upgraded:* Deal 14 damage. Double the enemy's Vulnerable.  
*Simulator:* damage 10.  
Damage read; doubling the enemy's Vulnerable isn't. Vulnerable is tracked, so doubling it is one line. `VULN_STACKS`

**Perfected Strike** · 2 energy · Attack · Common · ◐ partly read  
Deal 6 damage. Deals 2 additional damage for ALL your cards containing “Strike”.  
*Upgraded:* Deal 6 damage. Deals 3 additional damage for ALL your cards containing “Strike”.  
*Simulator:* damage 6.  
Base 6 read; +2 per card named "Strike" anywhere in the deck (piles, hand, exhaust) isn't. A Strike-heavy starter deck makes it about 6+2×6 = 18. `DECK_COUNT`

**Pommel Strike** · 1 energy · Attack · Common · ✅ read  
Deal 9 damage. Draw 1 card.  
*Upgraded:* Deal 10 damage. Draw 2 cards.  
*Simulator:* damage 9, draw 1.  

**Setup Strike** · 1 energy · Attack · Common · ◐ partly read  
Deal 7 damage. Gain 3 Strength this turn.  
*Upgraded:* Deal 9 damage. Gain 4 Strength this turn.  
*Simulator:* damage 7.  
Damage read; "Gain 3 Strength this turn" isn't (the card targets an enemy, so the on-self rule skips it). Temporary Strength needs a this-turn pool that resets at end of turn. `STR_TEMP`

**Sword Boomerang** · 1 energy · Attack · Common · ✅ read  
Deal 3 damage to a random enemy 3 times.  
*Upgraded:* Deal 3 damage to a random enemy 4 times.  
*Simulator:* damage 3×3.  
Random target; on a single enemy all 3 hits land on it.

**Thunderclap** · 1 energy · Attack · Common · ✅ read  
Deal 4 damage and apply 1 Vulnerable to ALL enemies.  
*Upgraded:* Deal 7 damage and apply 1 Vulnerable to ALL enemies.  
*Simulator:* damage 4, Vulnerable 1, all enemies.  

**Twin Strike** · 1 energy · Attack · Common · ✅ read  
Deal 5 damage twice.  
*Upgraded:* Deal 7 damage twice.  
*Simulator:* damage 5×2.  

**Ashen Strike** · 1 energy · Attack · Uncommon · ✅ read  
Deal 6 damage. Deals 3 additional damage for each card in your Exhaust Pile.  
*Upgraded:* Deal 6 damage. Deals 4 additional damage for each card in your Exhaust Pile.  
*Simulator:* damage 6, +3 per card in Exhaust Pile.  
6 + 3 per card in the Exhaust Pile, read through `per_exhausted`.

**Bludgeon** · 3 energy · Attack · Uncommon · ✅ read  
Deal 32 damage.  
*Upgraded:* Deal 42 damage.  
*Simulator:* damage 32.  

**Bully** · 0 energy · Attack · Uncommon · ◐ partly read  
Deal 4 damage. Deals 2 additional damage for each Vulnerable on the enemy.  
*Upgraded:* Deal 4 damage. Deals 3 additional damage for each Vulnerable on the enemy.  
*Simulator:* damage 4.  
Base 4 read; +2 per Vulnerable stack on the enemy isn't. Vulnerable is tracked, so this is cheap. `VULN_STACKS`

**Dismantle** · 1 energy · Attack · Uncommon · ⚠️ misread  
Deal 8 damage. If the enemy is Vulnerable, hits twice.  
*Upgraded:* Deal 10 damage. If the enemy is Vulnerable, hits twice.  
*Simulator:* damage 8×2.  
Always read as 2 hits; it hits twice only if the enemy is Vulnerable. Gate the second hit on tracked Vulnerable. `VULN_COND`

**Fight Me!** · 2 energy · Attack · Uncommon · ◐ partly read  
Deal 5 damage twice. Gain 3 Strength. The enemy gains 1 Strength.  
*Upgraded:* Deal 6 damage twice. Gain 4 Strength. The enemy gains 1 Strength.  
*Simulator:* damage 5×2.  
Hits read; +3 Strength for you isn't (it targets an enemy). The enemy's +1 Strength is a survival cost. `STR_SELF_ON_ATTACK` `ENEMY_STR`

**Hemokinesis** · 1 energy · Attack · Uncommon · ◐ partly read  
Lose 2 HP. Deal 15 damage.  
*Upgraded:* Lose 2 HP. Deal 20 damage.  
*Simulator:* damage 15.  
Damage read; 2 HP loss isn't. `SELF_HP`

**Howl from Beyond** · 3 energy · Attack · Uncommon · ✅ read  
Deal 18 damage to ALL enemies. At the end of your turn, if this is in your Exhaust Pile, play it.  
*Upgraded:* Deal 24 damage to ALL enemies. At the end of your turn, if this is in your Exhaust Pile, play it.  
*Simulator:* damage 18, all enemies, replays from Exhaust Pile at TurnEnd.  
Replays itself at end of turn while in the Exhaust Pile; read through `replays_from_exhaust`. It doesn't exhaust itself, so it needs an exhaust outlet.

**Outrage** · 0 energy · Attack · Uncommon · multiplayer · ✅ read  
Deal 9 damage. Add a copy of this card into EVERYONE'S Discard Pile.  
*Upgraded:* Deal 13 damage. Add a copy of this card into EVERYONE'S Discard Pile.  
*Simulator:* damage 9, copy to discard.  
Multiplayer only. Same as Anger for your own pile. `MP`

**Pillage** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 6 damage. Draw cards until you draw a non-Attack card.  
*Upgraded:* Deal 9 damage. Draw cards until you draw a non-Attack card.  
*Simulator:* damage 6.  
Damage read; "draw until a non-Attack" reads as `draws` with no count, so it draws 0. `DRAW_UNTIL`

**Rampage** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 10 damage. Increase this card's damage by 5 this combat.  
*Upgraded:* Deal 10 damage. Increase this card's damage by 10 this combat.  
*Simulator:* damage 10.  
10 read; +5 each time it is played this combat isn't. Needs per-card-instance growth, like the Forge card. `SELF_GROW`

**Spite** · 0 energy · Attack · Uncommon · ⚠️ misread  
Deal 5 damage. If you lost HP this turn, hits 2 times.  
*Upgraded:* Deal 5 damage. If you lost HP this turn, hits 3 times.  
*Simulator:* damage 5×2.  
Always read as 2 hits; the second hit needs HP lost this turn. `SELF_HP`

**Stomp** · 3 energy · Attack · Uncommon · ◐ partly read  
Deal 12 damage to ALL enemies. Costs 1 less 1 Energy for each Attack played this turn.  
*Upgraded:* Deal 15 damage to ALL enemies. Costs 1 less 1 Energy for each Attack played this turn.  
*Simulator:* damage 12, all enemies.  
Damage read; costs 1 less per Attack played this turn isn't, so it always costs 3. `COST_DOWN`

**Unrelenting** · 2 energy · Attack · Uncommon · ◐ partly read  
Deal 14 damage. The next Attack you play costs 0 Energy.  
*Upgraded:* Deal 20 damage. The next Attack you play costs 0 Energy.  
*Simulator:* damage 14.  
Damage read; the next Attack costing 0 isn't. `COST_NEXT`

**Uppercut** · 2 energy · Attack · Uncommon · ◐ partly read  
Deal 13 damage. Apply 1 Weak. Apply 1 Vulnerable.  
*Upgraded:* Deal 13 damage. Apply 2 Weak. Apply 2 Vulnerable.  
*Simulator:* damage 13.  
Damage read; 1 Weak and 1 Vulnerable sit under a generic `power` var the reader doesn't map, so neither is applied. `VAR_POWER`

**Whirlwind** · X energy · Attack · Uncommon · ✅ read  
Deal 5 damage to ALL enemies X times.  
*Upgraded:* Deal 8 damage to ALL enemies X times.  
*Simulator:* damage 5, X-cost, all enemies.  
X-cost, read.

**Conflagration** · 1 energy · Attack · Rare · ✅ read  
Deal 2 damage to ALL enemies 4 times.  
*Upgraded:* Deal 2 damage to ALL enemies 5 times.  
*Simulator:* damage 2×4, all enemies.  

**Feed** · 1 energy · Attack · Rare · Exhaust · ◐ partly read  
Deal 10 damage. If Fatal, raise your Max HP by 3.  
*Upgraded:* Deal 12 damage. If Fatal, raise your Max HP by 4.  
*Simulator:* damage 10.  
Damage read. Max HP on a kill matters for the run, not the fight output. `FATAL`

**Fiend Fire** · 2 energy · Attack · Rare · Exhaust · ✅ read  
Exhaust your Hand. Deal 7 damage for each card Exhausted.  
*Upgraded:* Exhaust your Hand. Deal 10 damage for each card Exhausted.  
*Simulator:* damage 7, exhausts Hand, per card exhausted.  
Exhausts the hand and deals 7 per card exhausted. Read through `exhausts=Hand` and `own_per_card`.

**Mangle** · 3 energy · Attack · Rare · ◐ partly read  
Deal 20 damage. Enemy loses 10 Strength this turn.  
*Upgraded:* Deal 26 damage. Enemy loses 15 Strength this turn.  
*Simulator:* damage 20.  
Damage read; the enemy's −10 Strength this turn is mitigation, and the simulator has no enemy attacks. Survival should count it. `ENEMY_STR`

**Midnight** · 12 energy · Attack · Rare · multiplayer · ✅ read  
Deal 60 damage. Costs 1 less 1 Energy for each card Exhausted this combat by ANYONE.  
*Upgraded:* Deal 72 damage. Costs 1 less 1 Energy for each card Exhausted this combat by ANYONE.  
*Simulator:* damage 60, −1 cost per exhausted card.  
Multiplayer only. Cost reduction per exhausted card is read. `MP`

**Pact's End** · 0 energy · Attack · Rare · ✅ read  
If you have 3 or more cards in your Exhaust Pile, deal 18 damage to ALL enemies.  
*Upgraded:* If you have 3 or more cards in your Exhaust Pile, deal 24 damage to ALL enemies.  
*Simulator:* damage 18, all enemies, needs 3+ in Exhaust Pile.  
Needs 3+ cards in the Exhaust Pile; read through `exhausted_at_least`.

**Tear Asunder** · 2 energy · Attack · Rare · ◐ partly read  
Deal 5 damage. Hits an additional time for each time you lost HP this combat.  
*Upgraded:* Deal 7 damage. Hits an additional time for each time you lost HP this combat.  
*Simulator:* damage 5.  
One hit read; +1 hit per time you lost HP this combat isn't. Needs a lost-HP counter. `SELF_HP`

**Thrash** · 1 energy · Attack · Rare · ◐ partly read  
Deal 4 damage twice. Exhaust a random Attack in your Hand and add its damage to this card.  
*Upgraded:* Deal 6 damage twice. Exhaust a random Attack in your Hand and add its damage to this card.  
*Simulator:* damage 4×2.  
Hits read; exhausting a random Attack in hand and adding its damage isn't. `EXH_RANDOM`

**Break** · 1 energy · Attack · Ancient · ✅ read  
Deal 20 damage. Apply 5 Vulnerable.  
*Upgraded:* Deal 30 damage. Apply 7 Vulnerable.  
*Simulator:* damage 20, Vulnerable 5.  
Ancient. Single-target burst with 5 Vulnerable, which also lifts every later Attack for 5 turns.

#### Ironclad: Skills

**Defend** · 1 energy · Skill · Basic · ✅ read  
Gain 5 Block.  
*Upgraded:* Gain 8 Block.  
*Simulator:* Block 5.  

**Armaments** · 1 energy · Skill · Common · ◐ partly read  
Gain 5 Block. Upgrade a card in your Hand.  
*Upgraded:* Gain 5 Block. Upgrade ALL cards in your Hand.  
*Simulator:* Block 5.  
Block read; upgrading a hand card (all cards when upgraded) isn't. `UPGRADE`

**Blood Wall** · 2 energy · Skill · Common · ◐ partly read  
Lose 2 HP. Gain 16 Block.  
*Upgraded:* Lose 2 HP. Gain 20 Block.  
*Simulator:* Block 16.  
Block read; 2 HP loss isn't. `SELF_HP`

**Havoc** · 1 energy · Skill · Common · ○ not read  
Play the top card of your Draw Pile and Exhaust it.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Plays and exhausts the top card of the Draw Pile. Unread. `AUTOPLAY`

**Shrug It Off** · 1 energy · Skill · Common · ✅ read  
Gain 8 Block. Draw 1 card.  
*Upgraded:* Gain 11 Block. Draw 1 card.  
*Simulator:* Block 8, draw 1.  

**Taunt** · 1 energy · Skill · Common · ✅ read  
Gain 6 Block. Apply 1 Vulnerable.  
*Upgraded:* Gain 7 Block. Apply 2 Vulnerable.  
*Simulator:* Block 6, Vulnerable 1.  

**Tremble** · 1 energy · Skill · Common · Exhaust · ✅ read  
Apply 3 Vulnerable.  
*Upgraded:* Apply 4 Vulnerable.  
*Simulator:* Vulnerable 3.  

**True Grit** · 1 energy · Skill · Common · ◐ partly read  
Gain 7 Block. Exhaust 1 card at random.  
*Upgraded:* Gain 9 Block. Exhaust 1 card.  
*Simulator:* Block 7.  
Block read; exhaust 1 random card (upgraded: your choice) isn't. `EXH_RANDOM` `EXH_CHOOSE`

**Battle Trance** · 0 energy · Skill · Uncommon · ◐ partly read  
Draw 3 cards. You cannot draw additional cards this turn.  
*Upgraded:* Draw 4 cards. You cannot draw additional cards this turn.  
*Simulator:* draw 3.  
Draw read; "no more draws this turn" isn't. `DRAW_LOCK`

**Blaze** · 2 energy · Skill · Uncommon · multiplayer · ○ not read  
Give another player 5 Strength.  
*Upgraded:* Give another player 7 Strength.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Bloodletting** · 0 energy · Skill · Uncommon · ◐ partly read  
Lose 3 HP. Gain 2 Energy.  
*Upgraded:* Lose 3 HP. Gain 3 Energy.  
*Simulator:* energy 2.  
Energy read; 3 HP loss isn't. `SELF_HP`

**Burning Pact** · 1 energy · Skill · Uncommon · ◐ partly read  
Exhaust 1 card. Draw 2 cards.  
*Upgraded:* Exhaust 1 card. Draw 3 cards.  
*Simulator:* draw 2.  
Draw read; the chosen exhaust isn't. `EXH_CHOOSE`

**Colossus** · 1 energy · Skill · Uncommon · ◐ partly read  
Gain 4 Block. You receive 50% less damage from Vulnerable enemies this turn.  
*Upgraded:* Gain 7 Block. You receive 50% less damage from Vulnerable enemies this turn.  
*Simulator:* Block 4.  
Block read; halving damage from Vulnerable enemies is mitigation (survival). `ENEMY_DMG`

**Demonic Shield** · 0 energy · Skill · Uncommon · Exhaust · multiplayer · ○ not read  
Lose 1 HP. Give another player Block equal to your Block.  
*Upgraded:* Loses exhaust  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Drum of Battle** · 1 energy · Skill · Uncommon · ✅ read  
Draw 2 cards. When this card is Exhausted, gain 2 Energy.  
*Upgraded:* Draw 2 cards. When this card is Exhausted, gain 3 Energy.  
*Simulator:* draw 2, when exhausted: [Energy(2)].  
Energy when exhausted; read through `when_exhausted`.

**Evil Eye** · 1 energy · Skill · Uncommon · ✅ read  
Gain 8 Block. Gain another 8 Block if you have Exhausted a card this turn.  
*Upgraded:* Gain 11 Block. Gain another 11 Block if you have Exhausted a card this turn.  
*Simulator:* Block 8, +8 Block after an exhaust.  

**Expect a Fight** · 3 energy · Skill · Uncommon · ⚠️ misread  
Gain 15 Block. Attacks in your Hand cost 1 less 1 Energy until played.  
*Upgraded:* Gain 20 Block. Attacks in your Hand cost 1 less 1 Energy until played.  
*Simulator:* Block 15, energy 1.  
Block read; the cost cut for hand Attacks is misread as +1 energy (`energy` var). Should lower costs in the current hand only. `ENERGY_VAR` `COST_DOWN`

**Flame Barrier** · 2 energy · Skill · Uncommon · ◐ partly read  
Gain 12 Block. Whenever you are attacked this turn, deal 4 damage back.  
*Upgraded:* Gain 16 Block. Whenever you are attacked this turn, deal 6 damage back.  
*Simulator:* Block 12.  
Block read; 4 damage back per enemy hit this turn isn't. Needs the enemy's hit count (from patterns) to price retaliation. `THORNS_TEMP`

**Forgotten Ritual** · 1 energy · Skill · Uncommon · Exhaust · ✅ read  
If you Exhausted a card this turn, gain 3 Energy.  
*Upgraded:* If you Exhausted a card this turn, gain 4 Energy.  
*Simulator:* energy 3, needs an exhaust this turn.  

**Infernal Blade** · 1 energy · Skill · Uncommon · Exhaust · ○ not read  
Add a random Attack into your Hand. It's free to play this turn.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Adds a random Attack, free this turn. Unread. `RANDOM_GEN`

**Rage** · 0 energy · Skill · Uncommon · ○ not read  
Whenever you play an Attack this turn, gain 3 Block.  
*Upgraded:* Whenever you play an Attack this turn, gain 5 Block.  
*Simulator:* nothing.  
Block per Attack played this turn. Unread. Needs a this-turn trigger. `ON_PLAY_TEMP`

**Second Wind** · 1 energy · Skill · Uncommon · ✅ read  
Exhaust all non-Attack cards in your Hand. Gain 5 Block for each card Exhausted.  
*Upgraded:* Exhaust all non-Attack cards in your Hand. Gain 7 Block for each card Exhausted.  
*Simulator:* Block 5, exhausts HandNonAttacks, per card exhausted.  

**Brand** · 0 energy · Skill · Rare · ◐ partly read  
Lose 1 HP. Exhaust 1 card. Gain 1 Strength.  
*Upgraded:* Lose 1 HP. Exhaust 1 card. Gain 2 Strength.  
*Simulator:* Strength 1.  
Strength read; 1 HP loss and exhausting a chosen card aren't. A key exhaust enabler for Pact's End, Ashen Strike and Feel No Pain. `SELF_HP` `EXH_CHOOSE`

**Cascade** · X energy · Skill · Rare · ○ not read  
Play the top X cards of your Draw Pile.  
*Upgraded:* Play the top X+1 cards of your Draw Pile.  
*Simulator:* X-cost.  
X-cost: plays the top X (upgraded X+1) Draw Pile cards. Unread. `AUTOPLAY`

**Dominate** · 1 energy · Skill · Rare · Exhaust · ◐ partly read  
Apply 1 Vulnerable. Gain 1 Strength for each Vulnerable on the enemy.  
*Upgraded:* Apply 2 Vulnerable. Gain 1 Strength for each Vulnerable on the enemy.  
*Simulator:* Vulnerable 1.  
Vulnerable read; +1 Strength per Vulnerable stack on the enemy isn't. `VULN_STACKS`

**Impervious** · 2 energy · Skill · Rare · Exhaust · ✅ read  
Gain 30 Block.  
*Upgraded:* Gain 40 Block.  
*Simulator:* Block 30.  

**Not Yet** · 2 energy · Skill · Rare · Exhaust · ○ not read  
Heal 10 HP.  
*Upgraded:* Heal 13 HP.  
*Simulator:* nothing.  
Heal 10. Outside the output simulator; survival should count it. `HEAL`

**Offering** · 0 energy · Skill · Rare · Exhaust · ◐ partly read  
Lose 6 HP. Gain 2 Energy. Draw 3 cards.  
*Upgraded:* Lose 6 HP. Gain 2 Energy. Draw 5 cards.  
*Simulator:* draw 3, energy 2.  
Energy and draw read; 6 HP loss isn't. `SELF_HP`

**One-Two Punch** · 1 energy · Skill · Rare · ✅ read  
This turn, your next Attack is played an extra time.  
*Upgraded:* This turn, your next 2 Attacks are played an extra time.  
*Simulator:* next Attack played twice.  
Next Attack played twice; read through `next_extra`.

**Primal Force** · 0 energy · Skill · Rare · ○ not read  
Transform all Attacks in your Hand into Giant Rock.  
*Upgraded:* Transform all Attacks in your Hand into Giant Rock+.  
*Makes:* Giant Rock  
*Simulator:* nothing.  
Turns every Attack in hand into Giant Rock. Unread. Needs a hand transform to a named card. `TRANSFORM`

**Stoke** · 1 energy · Skill · Rare · ◐ partly read  
Exhaust your Hand. Add 1 random card into your Hand for each card Exhausted.  
*Upgraded:* Exhaust your Hand. Add 1 random Upgraded card into your Hand for each card Exhausted.  
*Simulator:* exhausts Hand.  
Hand exhaust read; the random replacement cards aren't. Treat them as average class cards. `RANDOM_GEN`

#### Ironclad: Powers

**Cruelty** · 1 energy · Power · Uncommon · ✅ read  
Vulnerable enemies take an additional 25% damage.  
*Upgraded:* Vulnerable enemies take an additional 50% damage.  
*Simulator:* power: Vulnerable +25%.  

**Feel No Pain** · 1 energy · Power · Uncommon · ✅ read  
Whenever a card is Exhausted, gain 3 Block.  
*Upgraded:* Whenever a card is Exhausted, gain 4 Block.  
*Simulator:* power: Whenever a card is exhausted: gain 3 Block.  

**Inferno** · 1 energy · Power · Uncommon · ⚠️ misread  
At the start of your turn, lose 1 HP. Whenever you lose HP on your turn, deal 6 damage to ALL enemies.  
*Upgraded:* At the start of your turn, lose 1 HP. Whenever you lose HP on your turn, deal 9 damage to ALL enemies.  
*Simulator:* power: When played: deal 6 damage to all.  
Misread as a one-time area hit on play. In fact: lose 1 HP each turn, and deal 6 to all enemies whenever you lose HP on your turn. Needs SELF_HP triggers. `TRIGGER_FALLBACK` `SELF_HP`

**Inflame** · 1 energy · Power · Uncommon · ✅ read  
Gain 2 Strength.  
*Upgraded:* Gain 3 Strength.  
*Simulator:* power: When played: gain 2 Strength.  

**Juggling** · 1 energy · Power · Uncommon · ○ not read  
Add a copy of the third Attack you play each turn into your Hand.  
*Upgraded:* Gains innate  
*Simulator:* nothing.  
Every turn, a copy of the third Attack played goes to the hand. Unread. `COPY_PLAYED`

**Rupture** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you lose HP on your turn, gain 1 Strength.  
*Upgraded:* Whenever you lose HP on your turn, gain 2 Strength.  
*Simulator:* power: When played: gain 1 Strength.  
Misread as +1 Strength on play. It is +1 Strength per HP-loss event on your turn. Needs SELF_HP triggers. `TRIGGER_FALLBACK` `SELF_HP`

**Stampede** · 2 energy · Power · Uncommon · ○ not read  
At the end of your turn, 1 random Attack in your Hand is played against a random enemy.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
At end of turn, a random Attack left in the hand is played for free. Unread. `AUTOPLAY`

**Stone Armor** · 1 energy · Power · Uncommon · ✅ read  
Gain 4 Plating.  
*Upgraded:* Gain 6 Plating.  
*Simulator:* power: When played: gain 4 Plating.  

**Vicious** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you apply Vulnerable, draw 1 card.  
*Upgraded:* Whenever you apply Vulnerable, draw 2 cards.  
*Simulator:* power: When played: draw 1.  
Misread as draw 1 on play. It draws whenever you apply Vulnerable. Needs an on-apply trigger. `TRIGGER_FALLBACK` `ON_APPLY`

**Aggression** · 1 energy · Power · Rare · ○ not read  
At the start of your turn, put a random Attack from your Discard Pile into your Hand and Upgrade it.  
*Upgraded:* Gains innate  
*Simulator:* nothing.  
Each turn, returns a random Attack from the Discard Pile to the hand, upgraded. Unread. `PILE_MOVE` `UPGRADE`

**Barricade** · 3 energy · Power · Rare · ○ not read  
Block is not removed at the start of your turn.  
*Upgraded:* Costs 2  
*Simulator:* nothing.  
Block isn't removed at turn start. The simulator resets Block each turn; it needs a carried Block pool. `BLOCK_KEEP`

**Crimson Mantle** · 1 energy · Power · Rare · ◐ partly read  
At the start of your turn, lose 1 HP and gain 7 Block.  
*Upgraded:* At the start of your turn, lose 1 HP and gain 10 Block.  
*Simulator:* power: Each turn: gain 7 Block.  
7 Block per turn read; 1 HP loss per turn isn't (feeds Rupture and Inferno). `SELF_HP`

**Dark Embrace** · 2 energy · Power · Rare · ✅ read  
Whenever a card is Exhausted, draw 1 card.  
*Upgraded:* Costs 1  
*Simulator:* power: Whenever a card is exhausted: draw 1.  
Draw 1 per exhausted card. Read.

**Demon Form** · 3 energy · Power · Rare · ✅ read  
At the start of your turn, gain 3 Strength.  
*Upgraded:* At the start of your turn, gain 4 Strength.  
*Simulator:* power: Each turn: gain 3 Strength.  

**Hellraiser** · 2 energy · Power · Rare · ○ not read  
Whenever you draw a card containing “Strike”, it is played against a random enemy.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Drawn cards named "Strike" are played at once for free. Unread. Needs the draw step to resolve the play. `AUTOPLAY`

**Juggernaut** · 2 energy · Power · Rare · ✅ read  
Whenever you gain Block, deal 6 damage to a random enemy.  
*Upgraded:* Whenever you gain Block, deal 8 damage to a random enemy.  
*Simulator:* power: Whenever you gain Block: deal 6 damage.  
6 damage to a random enemy on every Block gain. Read as an `OnGainBlock` trigger.

**Pyre** · 2 energy · Power · Rare · ✅ read  
Gain 1 Energy at the start of each turn.  
*Upgraded:* Gain 2 Energy at the start of each turn.  
*Simulator:* power: Each turn: gain 1 Energy.  

**Tank** · 1 energy · Power · Rare · multiplayer · ○ not read  
Take 50% more damage from enemies. Allies take 50% less damage from enemies.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Unmovable** · 2 energy · Power · Rare · ○ not read  
The first time you gain Block from a card each turn, double the amount gained.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
The first Block from a card each turn is doubled. Unread. `BLOCK_MULT`

**Corruption** · 3 energy · Power · Ancient · ✅ read  
Skills cost 0 Energy. Whenever you play a Skill, Exhaust it.  
*Upgraded:* Costs 2  
*Simulator:* power: Skills cost 0 and exhaust.  
Skills cost 0 and exhaust. Read as a passive.

### Silent

#### Silent: Attacks

**Neutralize** · 0 energy · Attack · Basic · ✅ read  
Deal 3 damage. Apply 1 Weak.  
*Upgraded:* Deal 4 damage. Apply 2 Weak.  
*Simulator:* damage 3, Weak 1.  

**Strike** · 1 energy · Attack · Basic · ✅ read  
Deal 6 damage.  
*Upgraded:* Deal 9 damage.  
*Simulator:* damage 6.  

**Dagger Spray** · 1 energy · Attack · Common · ✅ read  
Deal 4 damage to ALL enemies twice.  
*Upgraded:* Deal 6 damage to ALL enemies twice.  
*Simulator:* damage 4×2, all enemies.  

**Dagger Throw** · 1 energy · Attack · Common · ◐ partly read  
Deal 9 damage. Draw 1 card. Discard 1 card.  
*Upgraded:* Deal 12 damage. Draw 1 card. Discard 1 card.  
*Simulator:* damage 9.  
Damage and draw read; the discard isn't. Discarding a Sly card plays it for free, which is Silent's main engine. `DISCARD_CHOOSE`

**Flick-Flack** · 1 energy · Attack · Common · Sly · ◐ partly read  
Deal 7 damage to ALL enemies.  
*Upgraded:* Deal 9 damage to ALL enemies.  
*Simulator:* damage 7, all enemies.  
Area damage read. Sly: if discarded from the hand, it is played for free. Unread, because the simulator never discards by choice. `SLY`

**Leading Strike** · 1 energy · Attack · Common · ✅ read  
Deal 3 damage. Add 2 Shivs into your Hand.  
*Upgraded:* Deal 6 damage. Add 2 Shivs into your Hand.  
*Makes:* Shiv  
*Simulator:* damage 3, n=2, pile=Hand }].  
Makes 2 Shivs.

**Poisoned Stab** · 1 energy · Attack · Common · ✅ read  
Deal 6 damage. Apply 3 Poison.  
*Upgraded:* Deal 8 damage. Apply 4 Poison.  
*Simulator:* damage 6, Poison 3.  

**Predator** · 2 energy · Attack · Common · ⚠️ misread  
Deal 15 damage. Next turn, draw 2 cards.  
*Upgraded:* Deal 20 damage. Next turn, draw 2 cards.  
*Simulator:* damage 15.  
Damage read; "Next turn, draw 2" is flagged as a draw but with the count at 0 (the `cards` var is absent), so nothing is drawn. Should add 2 to next turn's draw. `DRAW_NEXT`

**Ricochet** · 2 energy · Attack · Common · Sly · ◐ partly read  
Deal 3 damage to a random enemy 4 times.  
*Upgraded:* Deal 3 damage to a random enemy 5 times.  
*Simulator:* damage 3×4.  
Hits read; Sly unread. `SLY`

**Slice** · 0 energy · Attack · Common · ✅ read  
Deal 6 damage.  
*Upgraded:* Deal 9 damage.  
*Simulator:* damage 6.  

**Sucker Punch** · 1 energy · Attack · Common · ✅ read  
Deal 8 damage. Apply 1 Weak.  
*Upgraded:* Deal 10 damage. Apply 2 Weak.  
*Simulator:* damage 8, Weak 1.  

**Backstab** · 0 energy · Attack · Uncommon · Exhaust, Innate · ✅ read  
Deal 11 damage.  
*Upgraded:* Deal 15 damage.  
*Simulator:* damage 11.  

**Dash** · 2 energy · Attack · Uncommon · ✅ read  
Gain 10 Block. Deal 10 damage.  
*Upgraded:* Gain 13 Block. Deal 13 damage.  
*Simulator:* damage 10, Block 10.  

**Echoing Slash** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 10 damage to ALL enemies. Repeat this effect for each enemy killed.  
*Upgraded:* Deal 13 damage to ALL enemies. Repeat this effect for each enemy killed.  
*Simulator:* damage 10, all enemies.  
Area damage read; repeats on kills aren't (the target is passive and single). `FATAL`

**Finisher** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 6 damage for each Attack already played this turn.  
*Upgraded:* Deal 8 damage for each Attack already played this turn.  
*Simulator:* damage 6.  
6 per Attack already played this turn reads as a flat 6. `attacked_this_turn` exists; a count is needed. `COUNTER_ATTACKS`

**Flechettes** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 5 damage for each Skill in your Hand.  
*Upgraded:* Deal 7 damage for each Skill in your Hand.  
*Simulator:* damage 5.  
5 per Skill in hand reads as a flat 5. Count Skills in hand when played. `COUNT_HAND`

**Memento Mori** · 3 energy · Attack · Uncommon · Sly · ✅ read  
Deal 19 damage to the enemy with the lowest HP.  
*Upgraded:* Deal 23 damage to the enemy with the lowest HP.  
*Simulator:* damage 19.  
Hits the lowest-HP enemy. Sly unread. `SLY`

**Pinpoint** · 3 energy · Attack · Uncommon · ◐ partly read  
Deal 15 damage. Costs 1 less 1 Energy for each Skill played this turn.  
*Upgraded:* Deal 19 damage. Costs 1 less 1 Energy for each Skill played this turn.  
*Simulator:* damage 15.  
Costs 1 less per Skill played this turn; read at full cost 3. `COST_DOWN`

**Pounce** · 2 energy · Attack · Uncommon · ◐ partly read  
Deal 14 damage. The next Skill you play costs 0 Energy.  
*Upgraded:* Deal 20 damage. The next Skill you play costs 0 Energy.  
*Simulator:* damage 14.  
The next Skill costing 0 isn't read. `COST_NEXT`

**Precise Cut** · 0 energy · Attack · Uncommon · ◐ partly read  
Deal 13 damage. Deals 2 less damage for each other card in your Hand.  
*Simulator:* damage 13.  
13 read as flat; −2 per other card in hand isn't. With a typical 4 other cards it is 5. `COUNT_HAND`

**Skewer** · X energy · Attack · Uncommon · ✅ read  
Deal 8 damage X times.  
*Upgraded:* Deal 11 damage X times.  
*Simulator:* damage 8, X-cost.  

**Strangle** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 8 damage. Whenever you play a card this turn, the enemy loses 2 HP.  
*Upgraded:* Deal 10 damage. Whenever you play a card this turn, the enemy loses 3 HP.  
*Simulator:* damage 8.  
Damage read; "enemy loses 2 HP per card played this turn" isn't. Needs a this-turn trigger. `ON_PLAY_TEMP`

**Assassinate** · 0 energy · Attack · Rare · Exhaust, Innate · ✅ read  
Deal 10 damage. Apply 1 Vulnerable.  
*Upgraded:* Deal 13 damage. Apply 2 Vulnerable.  
*Simulator:* damage 10, Vulnerable 1.  

**Grand Finale** · 0 energy · Attack · Rare · ⚠️ misread  
Can only be played if there are no cards in your Draw Pile. Deal 60 damage to ALL enemies.  
*Upgraded:* Can only be played if there are no cards in your Draw Pile. Deal 75 damage to ALL enemies.  
*Simulator:* damage 60, all enemies.  
60 area damage read as always playable; it needs an empty Draw Pile. Gate it on `draw.is_empty()`. `PLAY_COND`

**Murder** · 3 energy · Attack · Rare · ◐ partly read  
Deal 1 damage. Deals 1 additional damage for each card drawn this combat.  
*Upgraded:* Costs 2  
*Simulator:* damage 1.  
1 read; +1 per card drawn this combat isn't. The simulator counts draws (`log.drawn`), so this is cheap. `COUNTER_DRAWN`

**The Hunt** · 1 energy · Attack · Rare · Exhaust · ◐ partly read  
Deal 10 damage. If Fatal, gain an additional card reward.  
*Upgraded:* Deal 15 damage. If Fatal, gain an additional card reward.  
*Simulator:* damage 10.  
Damage read. The extra card reward on a kill is run value. `FATAL`

**Suppress** · 0 energy · Attack · Ancient · Innate · ✅ read  
Deal 11 damage. Apply 3 Weak.  
*Upgraded:* Deal 17 damage. Apply 5 Weak.  
*Simulator:* damage 11, Weak 3.  
Ancient, Innate. Opening burst plus 3 Weak.

#### Silent: Skills

**Defend** · 1 energy · Skill · Basic · ✅ read  
Gain 5 Block.  
*Upgraded:* Gain 8 Block.  
*Simulator:* Block 5.  

**Survivor** · 1 energy · Skill · Basic · ◐ partly read  
Gain 8 Block. Discard 1 card.  
*Upgraded:* Gain 11 Block. Discard 1 card.  
*Simulator:* Block 8.  
Block read; the discard isn't. `DISCARD_CHOOSE`

**Anticipate** · 0 energy · Skill · Common · ⚠️ misread  
Gain 2 Dexterity this turn.  
*Upgraded:* Gain 4 Dexterity this turn.  
*Simulator:* Dexterity 2.  
"2 Dexterity this turn" is kept for the whole fight. Temporary stats need a this-turn pool. `DEX_TEMP`

**Backflip** · 1 energy · Skill · Common · ✅ read  
Gain 5 Block. Draw 2 cards.  
*Upgraded:* Gain 8 Block. Draw 2 cards.  
*Simulator:* Block 5, draw 2.  

**Blade Dance** · 1 energy · Skill · Common · Exhaust · ✅ read  
Add 3 Shivs into your Hand.  
*Upgraded:* Add 4 Shivs into your Hand.  
*Makes:* Shiv  
*Simulator:* n=3, pile=Hand }].  

**Cloak and Dagger** · 1 energy · Skill · Common · ✅ read  
Gain 6 Block. Add 1 Shiv into your Hand.  
*Upgraded:* Gain 6 Block. Add 2 Shivs into your Hand.  
*Makes:* Shiv  
*Simulator:* Block 6, n=1, pile=Hand }].  

**Deadly Poison** · 1 energy · Skill · Common · ✅ read  
Apply 5 Poison.  
*Upgraded:* Apply 7 Poison.  
*Simulator:* Poison 5.  

**Deflect** · 0 energy · Skill · Common · ✅ read  
Gain 4 Block.  
*Upgraded:* Gain 7 Block.  
*Simulator:* Block 4.  

**Dodge and Roll** · 1 energy · Skill · Common · ◐ partly read  
Gain 4 Block. Next turn, gain 4 Block.  
*Upgraded:* Gain 6 Block. Next turn, gain 6 Block.  
*Simulator:* Block 4.  
Block read; next turn's 4 Block isn't. Store it as a start-of-next-turn effect. `BLOCK_NEXT`

**Piercing Wail** · 1 energy · Skill · Common · Exhaust · ◐ partly read  
ALL enemies lose 6 Strength this turn.  
*Upgraded:* ALL enemies lose 8 Strength this turn.  
*Simulator:* all enemies.  
−6 enemy Strength this turn is mitigation. Survival should count it. `ENEMY_STR`

**Prepared** · 0 energy · Skill · Common · ◐ partly read  
Draw 1 card. Discard 1 card.  
*Upgraded:* Draw 2 cards. Discard 2 cards.  
*Simulator:* draw 1.  
Draw read; discard (Sly payoff) isn't. `DISCARD_CHOOSE`

**Snakebite** · 2 energy · Skill · Common · Retain · ✅ read  
Apply 7 Poison.  
*Upgraded:* Apply 10 Poison.  
*Simulator:* Poison 7.  

**Untouchable** · 2 energy · Skill · Common · Sly · ◐ partly read  
Gain 6 Block.  
*Upgraded:* Gain 9 Block.  
*Simulator:* Block 6.  
Block read; Sly unread. `SLY`

**Acrobatics** · 1 energy · Skill · Uncommon · ◐ partly read  
Draw 3 cards. Discard 1 card.  
*Upgraded:* Draw 4 cards. Discard 1 card.  
*Simulator:* draw 3.  
Draw read; discard isn't. `DISCARD_CHOOSE`

**Blade Symphony** · 2 energy · Skill · Uncommon · multiplayer · ✅ read  
Add 2 Shivs to ALL players' Hands.  
*Upgraded:* Costs 1  
*Makes:* Shiv  
*Simulator:* n=2, pile=Hand }].  
Multiplayer only; your own 2 Shivs are read. `MP`

**Blur** · 1 energy · Skill · Uncommon · ◐ partly read  
Gain 5 Block. Block is not removed at the start of your next turn.  
*Upgraded:* Gain 8 Block. Block is not removed at the start of your next turn.  
*Simulator:* Block 5.  
Block read; keeping it into next turn isn't. `BLOCK_KEEP`

**Bouncing Flask** · 2 energy · Skill · Uncommon · ⚠️ misread  
Apply 3 Poison to a random enemy 3 times.  
*Upgraded:* Apply 3 Poison to a random enemy 4 times.  
*Simulator:* Poison 3.  
Read as 3 Poison once; it is 3 Poison 3 times (`repeat` var). Multiply by `repeat`. `VAR_REPEAT`

**Bubble Bubble** · 1 energy · Skill · Uncommon · ◐ partly read  
If the enemy has Poison, apply 9 Poison.  
*Upgraded:* If the enemy has Poison, apply 12 Poison.  
*Simulator:* Poison 9.  
9 Poison read as always applied; it needs the enemy already Poisoned. Gate on tracked Poison. `PLAY_COND`

**Calculated Gamble** · 0 energy · Skill · Uncommon · Exhaust · ◐ partly read  
Discard your Hand, then draw that many cards.  
*Upgraded:* Gains retain  
*Simulator:* discards hand.  
Discards the hand (read) and draws that many (not read). `DRAW_PER_DISCARD`

**Concoct** · 0 energy · Skill · Uncommon · multiplayer · ○ not read  
Choose another player. Whenever their Attacks deal unblocked damage this turn, they apply 3 Poison.  
*Upgraded:* Choose another player. Whenever their Attacks deal unblocked damage this turn, they apply 4 Poison.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Escape Plan** · 0 energy · Skill · Uncommon · ◐ partly read  
Draw 1 card. If you draw a Skill, gain 3 Block.  
*Upgraded:* Draw 1 card. If you draw a Skill, gain 5 Block.  
*Simulator:* Block 3.  
Draw read; the Block is always granted, but it needs the drawn card to be a Skill. Check the drawn card's type. `DRAW_COND`

**Expertise** · 1 energy · Skill · Uncommon · ◐ partly read  
Draw 2 cards. Retain your Hand this turn.  
*Upgraded:* Draw 3 cards. Retain your Hand this turn.  
*Simulator:* draw 2.  
Draw read; "retain your hand this turn" isn't. `RETAIN_TEMP`

**Expose** · 0 energy · Skill · Uncommon · Exhaust · ◐ partly read  
Remove all Artifact and Block from the enemy. Apply 2 Vulnerable.  
*Upgraded:* Remove all Artifact and Block from the enemy. Apply 3 Vulnerable.  
*Simulator:* nothing.  
Vulnerable sits under `power` and isn't applied. Removing enemy Artifact/Block matters only in survival terms. `VAR_POWER`

**Fade** · 0 energy · Skill · Uncommon · Retain · multiplayer · ○ not read  
Another player gains 6 Dexterity this turn.  
*Upgraded:* Another player gains 9 Dexterity this turn.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Hand Trick** · 1 energy · Skill · Uncommon · ◐ partly read  
Gain 7 Block. Add Sly to a Skill in your Hand this turn.  
*Upgraded:* Gain 10 Block. Add Sly to a Skill in your Hand this turn.  
*Simulator:* Block 7.  
Block read; giving Sly to a hand Skill isn't. `SLY`

**Haze** · 2 energy · Skill · Uncommon · ✅ read  
Apply 4 Poison and 1 Weak to ALL enemies.  
*Upgraded:* Apply 6 Poison and 2 Weak to ALL enemies.  
*Simulator:* Poison 4, Weak 1, all enemies.  

**Hidden Daggers** · 0 energy · Skill · Uncommon · ◐ partly read  
Discard 2 cards. Add 2 Shivs into your Hand.  
*Upgraded:* Discard 2 cards. Add 2 Shivs+ into your Hand.  
*Makes:* Shiv  
*Simulator:* n=2, pile=Hand }].  
2 Shivs read; discard 2 (Sly payoff) isn't. `DISCARD_CHOOSE`

**Leg Sweep** · 2 energy · Skill · Uncommon · ✅ read  
Apply 2 Weak. Gain 11 Block.  
*Upgraded:* Apply 3 Weak. Gain 14 Block.  
*Simulator:* Block 11, Weak 2.  

**Mirage** · 1 energy · Skill · Uncommon · Exhaust · ○ not read  
Gain Block equal to Poison on ALL enemies.  
*Upgraded:* Loses exhaust  
*Simulator:* nothing.  
Block equal to enemy Poison. Poison is tracked; cheap to add. `BLOCK_FROM_POISON`

**Reflex** · 3 energy · Skill · Uncommon · Sly · ◐ partly read  
Draw 2 cards.  
*Upgraded:* Draw 3 cards.  
*Simulator:* draw 2.  
Draw read. Sly unread; at cost 3 it is mostly played through a discard. `SLY`

**Sidestep** · 0 energy · Skill · Uncommon · ✅ read  
Next turn, gain 1 Energy.  
*Upgraded:* Next turn, gain 2 Energy.  
*Simulator:* energy next turn 1.  

**Tactician** · 3 energy · Skill · Uncommon · Sly · ◐ partly read  
Gain 1 Energy.  
*Upgraded:* Gain 2 Energy.  
*Simulator:* energy 1.  
Energy read. Sly unread; at cost 3 it is meant to be discarded. `SLY`

**Up My Sleeve** · 2 energy · Skill · Uncommon · ◐ partly read  
Add 3 Shivs into your Hand. Reduce this card's cost by 1.  
*Upgraded:* Add 4 Shivs into your Hand. Reduce this card's cost by 1.  
*Makes:* Shiv  
*Simulator:* n=3, pile=Hand }].  
Shivs read; its own cost dropping by 1 each play isn't. `SELF_COST_DOWN`

**Adrenaline** · 0 energy · Skill · Rare · Exhaust · ✅ read  
Gain 1 Energy. Draw 2 cards.  
*Upgraded:* Gain 2 Energy. Draw 2 cards.  
*Simulator:* draw 2, energy 1.  

**Blade of Ink** · 1 energy · Skill · Rare · ✅ read  
Add 2 Inky Shivs into your Hand.  
*Upgraded:* Add 3 Inky Shivs into your Hand.  
*Makes:* Shiv  
*Simulator:* n=2, pile=Hand }].  
Makes Inky Shivs (token).

**Bullet Time** · 3 energy · Skill · Rare · ○ not read  
You cannot draw additional cards this turn. ALL cards in your Hand are free to play this turn.  
*Upgraded:* Costs 2  
*Simulator:* nothing.  
Every card in hand costs 0 this turn, and no more draws. Unread. `COST_ZERO_HAND` `DRAW_LOCK`

**Burst** · 1 energy · Skill · Rare · ✅ read  
This turn, your next Skill is played an extra time.  
*Upgraded:* This turn, your next 2 Skills are played an extra time.  
*Simulator:* next Skill played twice.  
Next Skill played twice.

**Corrosive Wave** · 1 energy · Skill · Rare · ○ not read  
Whenever you draw a card this turn, apply 2 Poison to ALL enemies.  
*Upgraded:* Whenever you draw a card this turn, apply 3 Poison to ALL enemies.  
*Simulator:* nothing.  
2 Poison to all per card drawn this turn. Flagged as draws with no count. Needs a this-turn on-draw trigger. `ON_DRAW_TEMP`

**Flanking** · 2 energy · Skill · Rare · multiplayer · ○ not read  
The enemy takes double attack damage from other players this turn.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Knife Trap** · 2 energy · Skill · Rare · ○ not read  
Play every Shiv in your Exhaust Pile on the enemy.  
*Upgraded:* Upgrade and play every Shiv in your Exhaust Pile on the enemy.  
*Makes:* Shiv  
*Simulator:* nothing.  
Plays every Shiv in the Exhaust Pile. Shivs exhaust, and the pile is tracked, so it is count(Shivs in Exhaust) × Shiv damage. `EXHAUST_REPLAY`

**Malaise** · X energy · Skill · Rare · Exhaust · ◐ partly read  
Enemy loses X Strength. Apply X Weak.  
*Upgraded:* Enemy loses X+1 Strength. Apply X+1 Weak.  
*Simulator:* X-cost.  
X Weak and X Strength loss are survival effects. X isn't priced. `ENEMY_STR`

**Nightmare** · 3 energy · Skill · Rare · Exhaust · ○ not read  
Choose a card. Next turn, add 3 copies of that card into your Hand.  
*Upgraded:* Costs 2  
*Simulator:* nothing.  
Next turn, 3 copies of a chosen card. Unread. Pick the best card in hand. `CHOOSE_COPY`

**Outbreak** · 3 energy · Skill · Rare · ◐ partly read  
Apply 9 Poison to ALL enemies. Poison triggers immediately.  
*Upgraded:* Apply 12 Poison to ALL enemies. Poison triggers immediately.  
*Simulator:* Poison 9, all enemies.  
9 Poison to all read; "Poison triggers immediately" isn't (one extra tick). `POISON_NOW`

**Shadow Step** · 1 energy · Skill · Rare · ◐ partly read  
Discard your Hand. Next turn, Attacks deal double damage.  
*Upgraded:* Costs 0  
*Simulator:* discards hand.  
Discard hand read; next turn's double Attack damage isn't. `DMG_MULT_NEXT`

**Shadowmeld** · 1 energy · Skill · Rare · ○ not read  
Double your Block gain this turn.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Double Block gained this turn. Unread. `BLOCK_MULT`

**Storm of Steel** · 1 energy · Skill · Rare · ✅ read  
Discard your Hand. Add 1 Shiv into your Hand for each card discarded.  
*Upgraded:* Discard your Hand. Add 1 Shiv+ into your Hand for each card discarded.  
*Makes:* Shiv  
*Simulator:* n=-2, pile=Hand }], discards hand.  
One Shiv per discarded card (n=−2).

#### Silent: Powers

**Accelerant** · 1 energy · Power · Uncommon · ✅ read  
Poison is triggered 1 additional time.  
*Upgraded:* Poison is triggered 2 additional times.  
*Simulator:* power: Poison ticks 1 more time(s).  

**Accuracy** · 1 energy · Power · Uncommon · ✅ read  
Shivs deal 4 additional damage.  
*Upgraded:* Shivs deal 6 additional damage.  
*Makes:* Shiv  
*Simulator:* power: a card named "shiv" deals 4 more damage.  

**Footwork** · 1 energy · Power · Uncommon · ✅ read  
Gain 2 Dexterity.  
*Upgraded:* Gain 3 Dexterity.  
*Simulator:* power: When played: gain 2 Dexterity.  

**Infinite Blades** · 1 energy · Power · Uncommon · ✅ read  
At the start of your turn, add 1 Shiv into your Hand.  
*Upgraded:* Gains innate  
*Makes:* Shiv  
*Simulator:* n=1, pile=Hand }], power: Each turn: add 1 shiv to your Hand.  

**Noxious Fumes** · 1 energy · Power · Uncommon · ✅ read  
At the start of your turn, apply 2 Poison to ALL enemies.  
*Upgraded:* At the start of your turn, apply 3 Poison to ALL enemies.  
*Simulator:* power: Each turn: apply 2 Poison.  

**Phantom Blades** · 1 energy · Power · Uncommon · ○ not read  
Shivs gain Retain. The first Shiv you play each turn deals 8 additional damage.  
*Upgraded:* Shivs gain Retain. The first Shiv you play each turn deals 11 additional damage.  
*Makes:* Shiv  
*Simulator:* nothing.  
Shivs gain Retain and the first Shiv each turn deals +8. Unread. `FirstAttackBonus` exists for relics; it needs a name filter. `FIRST_PLAY_BONUS` `RETAIN_GRANT`

**Speedster** · 2 energy · Power · Uncommon · ✅ read  
Whenever you draw a card during your turn, deal 3 damage to ALL enemies.  
*Upgraded:* Gains innate  
*Simulator:* power: Whenever you draw a card: deal 3 damage to all.  
3 to all enemies per card drawn on your turn. Read.

**Abrasive** · 3 energy · Power · Rare · Sly · ◐ partly read  
Gain 1 Dexterity. Gain 4 Thorns.  
*Upgraded:* Gain 1 Dexterity. Gain 6 Thorns.  
*Simulator:* power: When played: gain 1 Dexterity.  
Dexterity read; 4 Thorns (damage back per enemy hit) isn't. Sly unread. `THORNS` `SLY`

**Afterimage** · 1 energy · Power · Rare · ✅ read  
Whenever you play a card, gain 1 Block.  
*Upgraded:* Gains innate  
*Simulator:* power: Whenever you play a card: gain 1 Block.  

**Envenom** · 2 energy · Power · Rare · ⚠️ misread  
Whenever an Attack deals unblocked damage, apply 1 Poison.  
*Upgraded:* Whenever an Attack deals unblocked damage, apply 2 Poison.  
*Simulator:* power: When played: apply 1 Poison.  
Misread as 1 Poison on play. It applies 1 Poison per unblocked Attack hit, so it scales with hit count (Shivs). Needs a per-hit trigger. `TRIGGER_FALLBACK` `ON_UNBLOCKED`

**Fan of Knives** · 2 energy · Power · Rare · ◐ partly read  
Shivs now hit ALL enemies. Add 4 Shivs into your Hand.  
*Upgraded:* Shivs now hit ALL enemies. Add 5 Shivs into your Hand.  
*Makes:* Shiv  
*Simulator:* n=4, pile=Hand }], power: When played: add 4 shivs to your Hand.  
4 Shivs read; "Shivs hit ALL enemies" isn't. Only matters against groups. `SHIV_AOE`

**Master Planner** · 2 energy · Power · Rare · ○ not read  
When you play a Skill, it gains Sly.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Skills played gain Sly. Unread. `SLY`

**Serpent Form** · 3 energy · Power · Rare · ✅ read  
Whenever you play a card, deal 4 damage to a random enemy.  
*Upgraded:* Whenever you play a card, deal 6 damage to a random enemy.  
*Simulator:* power: Whenever you play a card: deal 4 damage.  

**Sneaky** · 2 energy · Power · Rare · Sly · multiplayer · ○ not read  
Whenever another player attacks an enemy, gain 1 Block.  
*Upgraded:* Whenever another player attacks an enemy, gain 2 Block.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Tools of the Trade** · 1 energy · Power · Rare · ◐ partly read  
At the start of your turn, draw 1 card and discard 1 card.  
*Upgraded:* Costs 0  
*Simulator:* power: Each turn: draw 1.  
Draw 1 per turn read; the discard isn't (Sly payoff). `DISCARD_CHOOSE`

**Tracking** · 2 energy · Power · Rare · ○ not read  
Weak enemies take 50% more damage from your Attacks.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Weak enemies take 50% more Attack damage. Weak is applied but not tracked on the enemy. Needs enemy Weak with duration, like Vulnerable. `WEAK_DMG`

**Well-Laid Plans** · 2 energy · Power · Rare · ✅ read  
At the end of your turn, you no longer discard your Hand.  
*Upgraded:* Costs 1  
*Simulator:* power: Keep your hand.  
The hand isn't discarded at end of turn. Read as keeping the hand.

**Wraith Form** · 3 energy · Power · Ancient · ◐ partly read  
Gain 2 Intangible. At the start of your turn, lose 1 Dexterity.  
*Upgraded:* Gain 3 Intangible. At the start of your turn, lose 1 Dexterity.  
*Simulator:* power: Each turn: lose 1 Dexterity.  
Dexterity loss read; 2 Intangible (all damage taken reduced to 1) isn't. Survival should count it as full mitigation for 2 turns. `INTANGIBLE`

### Defect

#### Defect: Attacks

**Strike** · 1 energy · Attack · Basic · ✅ read  
Deal 6 damage.  
*Upgraded:* Deal 9 damage.  
*Simulator:* damage 6.  

**Ball Lightning** · 1 energy · Attack · Common · ✅ read  
Deal 7 damage. Channel 1 Lightning.  
*Upgraded:* Deal 10 damage. Channel 1 Lightning.  
*Simulator:* damage 7, channel 1 Lightning.  

**Barrage** · 1 energy · Attack · Common · ◐ partly read  
Deal 5 damage for each Channeled Orb.  
*Upgraded:* Deal 7 damage for each Channeled Orb.  
*Simulator:* damage 5.  
5 per channelled orb reads as a flat 5. Orbs are tracked, so multiply by `orbs.len()`. `COUNT_ORBS`

**Beam Cell** · 0 energy · Attack · Common · ✅ read  
Deal 3 damage. Apply 1 Vulnerable.  
*Upgraded:* Deal 4 damage. Apply 2 Vulnerable.  
*Simulator:* damage 3, Vulnerable 1.  

**Claw** · 0 energy · Attack · Common · ◐ partly read  
Deal 3 damage. Increase the damage of ALL Claw cards by 2 this combat.  
*Upgraded:* Deal 4 damage. Increase the damage of ALL Claw cards by 3 this combat.  
*Simulator:* damage 3.  
3 read; +2 to every Claw for the rest of the combat isn't. Needs a shared growth counter keyed by name, like the Forge counter. `SELF_GROW`

**Cold Snap** · 1 energy · Attack · Common · ✅ read  
Deal 6 damage. Channel 1 Frost.  
*Upgraded:* Deal 9 damage. Channel 1 Frost.  
*Simulator:* damage 6, channel 1 Frost.  

**Compile Driver** · 1 energy · Attack · Common · ◐ partly read  
Deal 7 damage. Draw 1 card for each unique Orb you have.  
*Upgraded:* Deal 10 damage. Draw 1 card for each unique Orb you have.  
*Simulator:* damage 7.  
Damage read; draw 1 per unique orb type has no count. Count distinct orb ids. `COUNT_ORBS`

**Focused Strike** · 1 energy · Attack · Common · ◐ partly read  
Deal 9 damage. Gain 1 Focus this turn.  
*Upgraded:* Deal 11 damage. Gain 2 Focus this turn.  
*Simulator:* damage 9.  
Damage read; "1 Focus this turn" isn't (the card targets an enemy). `FOCUS_TEMP`

**Go for the Eyes** · 0 energy · Attack · Common · ✅ read  
Deal 3 damage. If the enemy intends to attack, apply 1 Weak.  
*Upgraded:* Deal 4 damage. If the enemy intends to attack, apply 2 Weak.  
*Simulator:* damage 3, Weak 1.  
Weak only if the enemy intends to attack. Read as always.

**Gunk Up** · 1 energy · Attack · Common · ✅ read  
Deal 4 damage 3 times. Add a Slimed into your Discard Pile.  
*Upgraded:* Deal 5 damage 3 times. Add a Slimed into your Discard Pile.  
*Makes:* Slimed  
*Simulator:* damage 4×3, n=1, pile=Discard }].  
Adds a Slimed (Status) to the discard: food for Trash to Treasure, Smokestack, Iteration and Flak Cannon.

**Momentum Strike** · 1 energy · Attack · Common · ◐ partly read  
Deal 11 damage. Reduce this card's cost to 0 Energy.  
*Upgraded:* Deal 15 damage. Reduce this card's cost to 0 Energy.  
*Simulator:* damage 11.  
Damage read; its cost dropping to 0 after the first play isn't. `SELF_COST_DOWN`

**Sweeping Beam** · 1 energy · Attack · Common · ✅ read  
Deal 6 damage to ALL enemies. Draw 1 card.  
*Upgraded:* Deal 9 damage to ALL enemies. Draw 1 card.  
*Simulator:* damage 6, draw 1, all enemies.  

**Uproar** · 2 energy · Attack · Common · ◐ partly read  
Deal 6 damage twice. Play a random Attack from your Draw Pile.  
*Upgraded:* Deal 8 damage twice. Play a random Attack from your Draw Pile.  
*Simulator:* damage 6×2.  
Hits read; playing a random Attack from the Draw Pile isn't. `AUTOPLAY`

**FTL** · 0 energy · Attack · Uncommon · ✅ read  
Deal 5 damage. If you have played fewer than 3 cards this turn, draw 1 card.  
*Upgraded:* Deal 6 damage. If you have played fewer than 4 cards this turn, draw 1 card.  
*Simulator:* damage 5, draw 1.  
Draws only if fewer than 3 cards were played this turn; read as always (mostly right, since it costs 0).

**Null** · 2 energy · Attack · Uncommon · ✅ read  
Deal 10 damage. Apply 1 Weak. Channel 1 Dark.  
*Upgraded:* Deal 13 damage. Apply 2 Weak. Channel 1 Dark.  
*Simulator:* damage 10, Weak 1, channel 1 Dark.  

**Refract** · 3 energy · Attack · Uncommon · ✅ read  
Deal 10 damage twice. Channel 2 Glass.  
*Upgraded:* Deal 13 damage twice. Channel 2 Glass.  
*Simulator:* damage 10×2, channel 2 Glass.  

**Rocket Punch** · 2 energy · Attack · Uncommon · ◐ partly read  
Deal 13 damage. Draw 1 card. Whenever you create a Status, reduce this card's cost by 1 Energy until played.  
*Upgraded:* Deal 14 damage. Draw 2 cards. Whenever you create a Status, reduce this card's cost by 1 Energy until played.  
*Simulator:* damage 13, draw 1.  
Damage and draw read; −1 cost per Status created isn't. `COST_DOWN`

**Scrape** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 7 damage. Draw 4 cards. Discard all cards drawn this way that do not cost 0 Energy.  
*Upgraded:* Deal 10 damage. Draw 5 cards. Discard all cards drawn this way that do not cost 0 Energy.  
*Simulator:* damage 7, draw 4.  
Draw 4 read; discarding the non-0-cost cards drawn isn't, so it over-draws. `DRAW_FILTER`

**Sunder** · 3 energy · Attack · Uncommon · ⚠️ misread  
Deal 26 damage. If this kills an enemy, gain 3 Energy.  
*Upgraded:* Deal 34 damage. If this kills an enemy, gain 3 Energy.  
*Simulator:* damage 26, energy 3.  
Read as always gaining 3 energy; that needs a kill. Drop the energy unless a kill model exists. `FATAL`

**Synthesis** · 2 energy · Attack · Uncommon · ◐ partly read  
Deal 14 damage. The next Power you play costs 0 Energy.  
*Upgraded:* Deal 20 damage. The next Power you play costs 0 Energy.  
*Simulator:* damage 14.  
The next Power costing 0 isn't read. `COST_NEXT`

**Tesla Coil** · 0 energy · Attack · Uncommon · ◐ partly read  
Deal 3 damage. Trigger all Lightning against the enemy.  
*Upgraded:* Deal 4 damage. Trigger all Lightning against the enemy twice.  
*Simulator:* damage 3.  
3 read; triggering every Lightning's passive against the enemy isn't. `ORB_TRIGGER`

**Adaptive Strike** · 2 energy · Attack · Rare · ◐ partly read  
Deal 18 damage. Add a 0 Energy copy of this card into your Discard Pile.  
*Upgraded:* Deal 23 damage. Add a 0 Energy copy of this card into your Discard Pile.  
*Simulator:* damage 18, copy to discard.  
Copy to discard read, but the copy costs 0 and the simulator gives it full cost. `COPY_COST`

**All for One** · 2 energy · Attack · Rare · ◐ partly read  
Deal 10 damage. Put ALL 0 Energy cards from your Discard Pile into your Hand.  
*Upgraded:* Deal 14 damage. Put ALL 0 Energy cards from your Discard Pile into your Hand.  
*Simulator:* damage 10.  
Damage read; returning every 0-cost card from the Discard Pile to the hand isn't. `PILE_MOVE`

**Flak Cannon** · 2 energy · Attack · Rare · ✅ read  
Exhaust ALL your Status cards. Deal 8 damage to a random enemy for each card Exhausted.  
*Upgraded:* Exhaust ALL your Status cards. Deal 11 damage to a random enemy for each card Exhausted.  
*Simulator:* damage 8, exhausts Statuses, per card exhausted.  
8 per Status exhausted, via `exhausts=Statuses`.

**Helix Drill** · 0 energy · Attack · Rare · ◐ partly read  
Deal 3 damage for each 1 Energy spent on other cards this turn.  
*Upgraded:* Deal 5 damage for each 1 Energy spent on other cards this turn.  
*Simulator:* damage 3.  
3 per energy spent on other cards this turn reads as a flat 3. Track energy spent per turn. `COUNTER_ENERGY`

**Hyperbeam** · 2 energy · Attack · Rare · ◐ partly read  
Deal 24 damage to ALL enemies. Lose 3 Focus this turn.  
*Upgraded:* Deal 30 damage to ALL enemies. Lose 3 Focus this turn.  
*Simulator:* damage 24, all enemies.  
Area damage read; −3 Focus this turn isn't, so orbs aren't weakened that turn. Tag error: "Scaling (damage) 3" comes from the Focus loss. `FOCUS_TEMP`

**Ice Lance** · 3 energy · Attack · Rare · ✅ read  
Deal 19 damage. Channel 3 Frost.  
*Upgraded:* Deal 24 damage. Channel 3 Frost.  
*Simulator:* damage 19, channel 3 Frost.  

**Meteor Strike** · 5 energy · Attack · Rare · ✅ read  
Deal 24 damage. Channel 3 Plasma.  
*Upgraded:* Deal 30 damage. Channel 3 Plasma.  
*Simulator:* damage 24, channel 3 Plasma.  

**Shatter** · 1 energy · Attack · Rare · Exhaust · ◐ partly read  
Deal 7 damage to ALL enemies. Evoke all of your Orbs twice.  
*Upgraded:* Deal 11 damage to ALL enemies. Evoke all of your Orbs twice.  
*Simulator:* damage 7, all enemies.  
Area damage read; "Evoke all orbs twice" isn't. The simulator only evokes when an orb is pushed out of a full slot row. `EVOKE`

#### Defect: Skills

**Defend** · 1 energy · Skill · Basic · ✅ read  
Gain 5 Block.  
*Upgraded:* Gain 8 Block.  
*Simulator:* Block 5.  

**Dualcast** · 1 energy · Skill · Basic · ○ not read  
Evoke your rightmost Orb twice.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Evokes the rightmost orb twice. Unread. A starter card, so every Defect deck is undercounted. `EVOKE`

**Zap** · 1 energy · Skill · Basic · ✅ read  
Channel 1 Lightning.  
*Upgraded:* Costs 0  
*Simulator:* channel 1 Lightning.  

**Boost Away** · 0 energy · Skill · Common · ✅ read  
Gain 6 Block. Add a Dazed into your Discard Pile.  
*Upgraded:* Gain 9 Block. Add a Dazed into your Discard Pile.  
*Makes:* Dazed  
*Simulator:* Block 6, n=1, pile=Discard }].  
Adds a Dazed to the discard.

**Charge Battery** · 1 energy · Skill · Common · ✅ read  
Gain 7 Block. Next turn, gain 1 Energy.  
*Upgraded:* Gain 10 Block. Next turn, gain 1 Energy.  
*Simulator:* Block 7, energy next turn 1.  

**Coolheaded** · 1 energy · Skill · Common · ✅ read  
Channel 1 Frost. Draw 1 card.  
*Upgraded:* Channel 1 Frost. Draw 2 cards.  
*Simulator:* draw 1, channel 1 Frost.  

**Hologram** · 1 energy · Skill · Common · Exhaust · ◐ partly read  
Gain 3 Block. Put a card from your Discard Pile into your Hand.  
*Upgraded:* Gain 5 Block. Put a card from your Discard Pile into your Hand.  
*Simulator:* Block 3.  
Block read; returning a Discard Pile card to the hand isn't. `PILE_MOVE`

**Hotfix** · 0 energy · Skill · Common · Exhaust · ⚠️ misread  
Gain 2 Focus this turn.  
*Upgraded:* Loses exhaust  
*Simulator:* Focus 2.  
"2 Focus this turn" is kept for the whole fight. `FOCUS_TEMP`

**Leap** · 1 energy · Skill · Common · ✅ read  
Gain 9 Block.  
*Upgraded:* Gain 12 Block.  
*Simulator:* Block 9.  

**Lightning Rod** · 1 energy · Skill · Common · ⚠️ misread  
Gain 4 Block. At the start of the next 2 turns, Channel 1 Lightning.  
*Upgraded:* Gain 7 Block. At the start of the next 2 turns, Channel 1 Lightning.  
*Simulator:* Block 4, channel 1 Lightning.  
Read as 1 Lightning now; it is 1 Lightning at the start of each of the next 2 turns. `DELAYED_CHANNEL`

**TURBO** · 0 energy · Skill · Common · ✅ read  
Gain 2 Energy. Add a Void into your Discard Pile.  
*Upgraded:* Gain 3 Energy. Add a Void into your Discard Pile.  
*Makes:* Void  
*Simulator:* energy 2, n=1, pile=Discard }].  
Adds a Void to the discard.

**Boot Sequence** · 0 energy · Skill · Uncommon · Exhaust, Innate · ✅ read  
Gain 10 Block.  
*Upgraded:* Gain 13 Block.  
*Simulator:* Block 10.  

**Chaos** · 1 energy · Skill · Uncommon · ○ not read  
Channel 1 random Orb.  
*Upgraded:* Channel 2 random Orbs.  
*Simulator:* nothing.  
Channels a random orb. Unread. Use the average orb. `RANDOM_ORB`

**Chill** · 0 energy · Skill · Uncommon · Exhaust · ✅ read  
Channel 1 Frost for each enemy.  
*Upgraded:* Loses exhaust  
*Simulator:* channel 1 Frost.  
1 Frost per enemy; read as 1 (a single target).

**Compact** · 1 energy · Skill · Uncommon · ◐ partly read  
Gain 6 Block. Transform all Status cards in your Hand into Fuel.  
*Upgraded:* Gain 7 Block. Transform all Status cards in your Hand into Fuel+.  
*Makes:* Fuel  
*Simulator:* Block 6.  
Block read; turning hand Statuses into Fuel isn't. `TRANSFORM`

**Darkness** · 1 energy · Skill · Uncommon · ◐ partly read  
Channel 1 Dark. Trigger the passive ability of all Dark Orbs.  
*Upgraded:* Channel 1 Dark. Trigger the passive ability of all Dark Orbs twice.  
*Simulator:* channel 1 Dark.  
Channel read; triggering every Dark orb's passive isn't. `ORB_TRIGGER`

**Double Energy** · 1 energy · Skill · Uncommon · Exhaust · ○ not read  
Double your Energy.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Doubles current energy. Unread. `ENERGY_MULT`

**Energy Surge** · 1 energy · Skill · Uncommon · Exhaust · multiplayer · ✅ read  
ALL players gain 2 Energy.  
*Upgraded:* ALL players gain 3 Energy.  
*Simulator:* energy 2.  
Multiplayer only; your own energy is read. `MP`

**Fight Through** · 1 energy · Skill · Uncommon · ✅ read  
Gain 13 Block. Add 2 Wounds into your Discard Pile.  
*Upgraded:* Gain 17 Block. Add 2 Wounds into your Discard Pile.  
*Makes:* Wound  
*Simulator:* Block 13, n=2, pile=Discard }].  
Adds 2 Wounds.

**Fusion** · 1 energy · Skill · Uncommon · Exhaust · ✅ read  
Channel 1 Plasma.  
*Upgraded:* Loses exhaust  
*Simulator:* channel 1 Plasma.  

**Glacier** · 2 energy · Skill · Uncommon · ✅ read  
Gain 6 Block. Channel 2 Frost.  
*Upgraded:* Gain 9 Block. Channel 2 Frost.  
*Simulator:* Block 6, channel 2 Frost.  

**Glasswork** · 1 energy · Skill · Uncommon · ✅ read  
Gain 5 Block. Channel 1 Glass.  
*Upgraded:* Gain 8 Block. Channel 1 Glass.  
*Simulator:* Block 5, channel 1 Glass.  

**Hibernate** · 2 energy · Skill · Uncommon · multiplayer · ◐ partly read  
This turn, your Frost grants ALL allies Block. Channel 2 Frost.  
*Upgraded:* This turn, your Frost grants ALL allies Block. Channel 3 Frost.  
*Simulator:* channel 2 Frost.  
Multiplayer only; your own Frost is read. `MP`

**Ignition** · 1 energy · Skill · Uncommon · Exhaust · multiplayer · ○ not read  
Another player Channels Plasma.  
*Upgraded:* Loses exhaust  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Overclock** · 0 energy · Skill · Uncommon · ✅ read  
Draw 2 cards. Add a Burn into your Discard Pile.  
*Upgraded:* Draw 3 cards. Add a Burn into your Discard Pile.  
*Makes:* Burn  
*Simulator:* draw 2, n=1, pile=Discard }].  
Adds a Burn.

**Scavenge** · 1 energy · Skill · Uncommon · ◐ partly read  
Exhaust a card. Next turn, gain 2 Energy.  
*Upgraded:* Exhaust a card. Next turn, gain 3 Energy.  
*Simulator:* energy next turn 2.  
Next-turn energy read; exhausting a card isn't. `EXH_CHOOSE`

**Shadow Shield** · 2 energy · Skill · Uncommon · ✅ read  
Gain 11 Block. Channel 1 Dark.  
*Upgraded:* Gain 15 Block. Channel 1 Dark.  
*Simulator:* Block 11, channel 1 Dark.  

**Skim** · 1 energy · Skill · Uncommon · ✅ read  
Draw 3 cards.  
*Upgraded:* Draw 4 cards.  
*Simulator:* draw 3.  

**Synchronize** · 1 energy · Skill · Uncommon · ○ not read  
Gain 1 Focus this turn for each unique Orb you have.  
*Upgraded:* Gain 2 Focus this turn for each unique Orb you have.  
*Simulator:* nothing.  
Focus this turn per unique orb. Unread. `COUNT_ORBS` `FOCUS_TEMP`

**Tempest** · X energy · Skill · Uncommon · ✅ read  
Channel X Lightning.  
*Upgraded:* Channel X+1 Lightning.  
*Simulator:* channel X Lightning, X-cost.  
X Lightning (the upgrade is X+1, not read).

**White Noise** · 1 energy · Skill · Uncommon · Exhaust · ○ not read  
Add a random Power into your Hand. It's free to play this turn.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
A random free Power. Unread. `RANDOM_GEN`

**Genetic Algorithm** · 1 energy · Skill · Rare · Exhaust · ◐ partly read  
Gain 1 Block. Permanently increase this card's Block by 3.  
*Upgraded:* Gain 1 Block. Permanently increase this card's Block by 4.  
*Simulator:* Block 1.  
1 Block read; the permanent +3 per play (across fights) is run growth. Price it from the floor count. `PERMANENT_GROW`

**Imitation Learning** · 1 energy · Skill · Rare · Exhaust · multiplayer · ○ not read  
Choose another player. The next 2 times they play a Power, you play a copy of it.  
*Upgraded:* Choose another player. The next 3 times they play a Power, you play a copy of it.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Modded** · 0 energy · Skill · Rare · ◐ partly read  
Gain 1 Orb Slot. Draw 1 card. Increase this card's cost by 1.  
*Upgraded:* Gain 1 Orb Slot. Draw 2 cards. Increase this card's cost by 1.  
*Simulator:* draw 1.  
Draw read; +1 orb slot and its own cost rising each play aren't. `SELF_COST_UP`

**Multi-Cast** · X energy · Skill · Rare · ○ not read  
Evoke your rightmost Orb X times.  
*Upgraded:* Evoke your rightmost Orb X+1 times.  
*Simulator:* X-cost.  
X evokes of the rightmost orb. Unread. `EVOKE`

**Rainbow** · 2 energy · Skill · Rare · Exhaust · ✅ read  
Channel 1 Lightning. Channel 1 Frost. Channel 1 Dark.  
*Upgraded:* Loses exhaust  
*Simulator:* channel 1 Lightning, channel 1 Frost, channel 1 Dark.  

**Reboot** · 0 energy · Skill · Rare · Exhaust · ◐ partly read  
Shuffle ALL your cards into your Draw Pile. Draw 4 cards.  
*Upgraded:* Shuffle ALL your cards into your Draw Pile. Draw 6 cards.  
*Simulator:* draw 4.  
Draw read; shuffling every pile back first isn't. `SHUFFLE_ALL`

**Signal Boost** · 1 energy · Skill · Rare · Exhaust · ◐ partly read  
The next Power you play is played an additional time.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
The next Power played twice isn't read. `next_extra` exists; the reader needs "Power". `NEXT_EXTRA_POWER`

**Supercritical** · 0 energy · Skill · Rare · Exhaust · ✅ read  
Gain 4 Energy.  
*Upgraded:* Gain 6 Energy.  
*Simulator:* energy 4.  

**Voltaic** · 3 energy · Skill · Rare · Exhaust · ○ not read  
Channel Lightning equal to the Lightning already Channeled this combat.  
*Upgraded:* Loses exhaust  
*Simulator:* nothing.  
Channels Lightning equal to the Lightning already channelled this combat. Unread. Needs a per-orb channel counter. `COUNTER_CHANNEL`

**Quadcast** · 1 energy · Skill · Ancient · ○ not read  
Evoke your rightmost Orb 4 times.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Evokes the rightmost orb 4 times. Unread. `EVOKE`

#### Defect: Powers

**Bulk Up** · 2 energy · Power · Uncommon · ✅ read  
Lose 1 Orb Slot. Gain 2 Strength. Gain 2 Dexterity.  
*Upgraded:* Lose 1 Orb Slot. Gain 3 Strength. Gain 3 Dexterity.  
*Simulator:* power: When played: lose 1 orb slots, When played: gain 2 Strength, When played: gain 2 Dexterity.  
−1 orb slot, +2 Strength, +2 Dexterity. Tag error: "Weak".

**Capacitor** · 1 energy · Power · Uncommon · ✅ read  
Gain 2 Orb Slots.  
*Upgraded:* Gain 3 Orb Slots.  
*Simulator:* power: When played: gain 2 orb slots.  

**Feral** · 2 energy · Power · Uncommon · ○ not read  
The first time you play a 0 Energy Attack each turn, return it to your Hand.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
The first 0-cost Attack each turn returns to the hand. Unread (the reader also splits the sentence on the energy icon). `RETURN_TO_HAND`

**Hailstorm** · 1 energy · Power · Uncommon · ◐ partly read  
At the end of your turn, if you have Frost, deal 6 damage to ALL enemies.  
*Upgraded:* At the end of your turn, if you have Frost, deal 8 damage to ALL enemies.  
*Simulator:* power: End of each turn: deal 6 damage to all.  
End-of-turn area damage read as always; it needs a Frost orb. Gate on Frost in the row. `ORB_COND`

**Iteration** · 1 energy · Power · Uncommon · ○ not read  
The first time you draw a Status each turn, draw 2 cards.  
*Upgraded:* The first time you draw a Status each turn, draw 3 cards.  
*Simulator:* nothing.  
Draw 2 the first time a Status is drawn each turn. Unread. `ON_DRAW_STATUS`

**Loop** · 1 energy · Power · Uncommon · ○ not read  
At the start of your turn, trigger the passive ability of your rightmost Orb.  
*Upgraded:* At the start of your turn, trigger the passive ability of your rightmost Orb 2 times.  
*Simulator:* nothing.  
Triggers the rightmost orb's passive again each turn. Unread. `ORB_TRIGGER`

**Smokestack** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you create a Status, deal 5 damage to ALL enemies.  
*Upgraded:* Whenever you create a Status, deal 7 damage to ALL enemies.  
*Simulator:* power: When played: deal 5 damage to all.  
Misread as 5 area damage on play. It is 5 to all enemies per Status created. `TRIGGER_FALLBACK` `ON_STATUS`

**Storm** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you play a Power, Channel 1 Lightning.  
*Upgraded:* Whenever you play a Power, Channel 2 Lightning.  
*Simulator:* channel 1 Lightning, power: Whenever you play a Power: Channel 1 lightning.  
"Channel 1 Lightning per Power played" read; the card's channel list also channels one on play, which is wrong. `DOUBLE_CHANNEL`

**Subroutine** · 1 energy · Power · Uncommon · ✅ read  
Whenever you play a Power, gain 1 Energy.  
*Upgraded:* Costs 0  
*Simulator:* power: Whenever you play a Power: gain 1 Energy.  

**Thunder** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you Evoke Lightning, deal 8 damage to each enemy hit.  
*Upgraded:* Whenever you Evoke Lightning, deal 11 damage to each enemy hit.  
*Simulator:* power: When played: deal 8 damage.  
Misread as 8 damage on play. It is +8 per Lightning evoked. Needs an evoke hook. `TRIGGER_FALLBACK` `ON_EVOKE`

**Buffer** · 2 energy · Power · Rare · ○ not read  
Prevent the next time you would lose HP.  
*Upgraded:* Prevent the next 2 times you would lose HP.  
*Simulator:* nothing.  
Prevents the next HP loss. Survival should count it. `BUFFER`

**Consuming Shadow** · 2 energy · Power · Rare · ⚠️ misread  
Channel 2 Dark. At the end of your turn, Evoke your leftmost Orb.  
*Upgraded:* Channel 3 Dark. At the end of your turn, Evoke your leftmost Orb.  
*Simulator:* channel 2 Dark, power: When played: Channel 2 dark.  
Channels 2 Dark twice (the card's channel list and the power's on-play trigger both fire). The end-of-turn evoke of the leftmost orb isn't read. `DOUBLE_CHANNEL` `EVOKE`

**Coolant** · 1 energy · Power · Rare · ○ not read  
At the start of your turn, gain 2 Block for each unique Orb you have.  
*Upgraded:* At the start of your turn, gain 3 Block for each unique Orb you have.  
*Simulator:* nothing.  
Block per unique orb each turn. Unread. `COUNT_ORBS`

**Creative AI** · 3 energy · Power · Rare · ○ not read  
At the start of your turn, add a random Power into your Hand.  
*Upgraded:* Costs 2  
*Simulator:* nothing.  
A random Power each turn. Unread. `RANDOM_GEN`

**Defragment** · 1 energy · Power · Rare · ✅ read  
Gain 1 Focus.  
*Upgraded:* Gain 2 Focus.  
*Simulator:* power: When played: gain 1 Focus.  

**Echo Form** · 3 energy · Power · Rare · Ethereal · ✅ read  
The first card you play each turn is played an extra time.  
*Upgraded:* Loses ethereal  
*Simulator:* power: First card each turn plays twice.  
First card each turn plays twice. Ethereal unless upgraded.

**Machine Learning** · 1 energy · Power · Rare · ✅ read  
At the start of your turn, draw 1 additional card.  
*Upgraded:* Gains innate  
*Simulator:* power: Each turn: draw 1.  

**One for All** · 1 energy · Power · Rare · multiplayer · ○ not read  
EVERYONE'S 0 Energy Attacks deal 3 additional damage.  
*Upgraded:* EVERYONE'S 0 Energy Attacks deal 4 additional damage.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Spinner** · 1 energy · Power · Rare · ⚠️ misread  
At the start of your turn, Channel 1 Glass.  
*Upgraded:* Channel 1 Glass. At the start of your turn, Channel 1 Glass.  
*Simulator:* channel 1 Glass, power: Each turn: Channel 1 glass.  
Each-turn Glass read, but the card's channel list also channels a Glass on play, which only the upgrade does. `DOUBLE_CHANNEL`

**Trash to Treasure** · 1 energy · Power · Rare · ○ not read  
Whenever you create a Status, Channel 1 random Orb.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
A random orb whenever you create a Status. Unread. Needs an on-make trigger for Status cards. `ON_STATUS`

**Biased Cognition** · 1 energy · Power · Ancient · ✅ read  
Gain 5 Focus. At the start of your turn, lose 1 Focus.  
*Upgraded:* Gain 6 Focus. At the start of your turn, lose 1 Focus.  
*Simulator:* power: When played: gain 5 Focus, Each turn: lose 1 Focus.  
Ancient. +5 Focus, −1 per turn.

### Necrobinder

#### Necrobinder: Attacks

**Strike** · 1 energy · Attack · Basic · ✅ read  
Deal 6 damage.  
*Upgraded:* Deal 9 damage.  
*Simulator:* damage 6.  

**Unleash** · 1 energy · Attack · Basic · ◐ partly read  
Osty deals 6 damage. Deals additional damage equal to Osty's current HP.  
*Simulator:* damage 6.  
Starter. Osty deals 6 plus his current HP. Read as a flat 6 player attack. With Bound Phylactery summoning 1 a turn and Bodyguard 5, it is often 12–20. `OSTY_SCALE`

**Blight Strike** · 1 energy · Attack · Common · ◐ partly read  
Deal 8 damage. Apply Doom equal to damage dealt.  
*Upgraded:* Deal 10 damage. Apply Doom equal to damage dealt.  
*Simulator:* damage 8.  
Damage read; Doom equal to damage dealt isn't. `DOOM_FROM_DMG`

**Defile** · 1 energy · Attack · Common · Ethereal · ✅ read  
Deal 13 damage.  
*Upgraded:* Deal 17 damage.  
*Simulator:* damage 13.  
Ethereal: exhausted if still in hand at end of turn (read).

**Drain Power** · 1 energy · Attack · Common · ◐ partly read  
Deal 10 damage. Upgrade 2 random cards in your Discard Pile.  
*Upgraded:* Deal 12 damage. Upgrade 3 random cards in your Discard Pile.  
*Simulator:* damage 10.  
Damage read; upgrading 2 random Discard Pile cards isn't. `UPGRADE`

**Fear** · 1 energy · Attack · Common · Ethereal · ✅ read  
Deal 7 damage. Apply 1 Vulnerable.  
*Upgraded:* Deal 8 damage. Apply 2 Vulnerable.  
*Simulator:* damage 7, Vulnerable 1.  

**Flatten** · 2 energy · Attack · Common · ⚠️ misread  
Osty deals 12 damage. This card costs 0 Energy if Osty has attacked this turn.  
*Upgraded:* Osty deals 16 damage. This card costs 0 Energy if Osty has attacked this turn.  
*Simulator:* damage 12, Osty damage 12.  
Counted twice: the data has both `damage` and `ostydamage` at 12, and the simulator adds both. Also costs 0 if Osty attacked this turn (unread). `OSTY_DOUBLE` `COST_DOWN`

**Graveblast** · 1 energy · Attack · Common · Exhaust · ◐ partly read  
Deal 4 damage. Put a card from your Discard Pile into your Hand.  
*Upgraded:* Deal 6 damage. Put a card from your Discard Pile into your Hand.  
*Simulator:* damage 4.  
Damage read; returning a Discard Pile card to the hand isn't. `PILE_MOVE`

**Poke** · 0 energy · Attack · Common · ⚠️ misread  
Osty deals 6 damage.  
*Upgraded:* Osty deals 9 damage.  
*Simulator:* damage 6, Osty damage 6.  
Osty's 6 is counted twice (`damage` + `ostydamage`). `OSTY_DOUBLE`

**Reap** · 3 energy · Attack · Common · Retain · ✅ read  
Deal 27 damage.  
*Upgraded:* Deal 33 damage.  
*Simulator:* damage 27.  
Retain.

**Reave** · 1 energy · Attack · Common · ✅ read  
Deal 10 damage. Add a Soul into your Draw Pile.  
*Upgraded:* Deal 13 damage. Add a Soul+ into your Draw Pile.  
*Makes:* Soul  
*Simulator:* damage 10, n=1, pile=Draw }].  
Soul into the Draw Pile.

**Sculpting Strike** · 1 energy · Attack · Common · ◐ partly read  
Deal 9 damage. Add Ethereal to a card in your Hand.  
*Upgraded:* Deal 12 damage. Add Ethereal to a card in your Hand.  
*Simulator:* damage 9.  
Damage read; giving Ethereal to a hand card (fuel for Ethereal payoffs) isn't. `GRANT_KEYWORD`

**Snap** · 1 energy · Attack · Common · ⚠️ misread  
Osty deals 7 damage. Add Retain to a card in your Hand.  
*Upgraded:* Osty deals 10 damage. Add Retain to a card in your Hand.  
*Simulator:* damage 7, Osty damage 7.  
Osty's 7 counted twice; giving Retain to a card is unread. `OSTY_DOUBLE` `GRANT_KEYWORD`

**Sow** · 1 energy · Attack · Common · Retain · ✅ read  
Deal 8 damage to ALL enemies.  
*Upgraded:* Deal 11 damage to ALL enemies.  
*Simulator:* damage 8, all enemies.  

**Bone Shards** · 1 energy · Attack · Uncommon · ⚠️ misread  
If Osty is alive, he deals 9 damage to ALL enemies and you gain 9 Block. Osty dies.  
*Upgraded:* If Osty is alive, he deals 12 damage to ALL enemies and you gain 12 Block. Osty dies.  
*Simulator:* damage 9, Osty damage 9, Block 9, all enemies.  
Area damage and Block read; the damage is counted twice. Osty must be alive, and he dies, so the next Osty card needs a new Summon. `OSTY_DOUBLE` `OSTY_ALIVE`

**Bury** · 4 energy · Attack · Uncommon · ✅ read  
Deal 52 damage.  
*Upgraded:* Deal 63 damage.  
*Simulator:* damage 52.  

**Death March** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 8 damage. Deals 4 additional damage for each card drawn during your turn.  
*Upgraded:* Deal 8 damage. Deals 6 additional damage for each card drawn during your turn.  
*Simulator:* damage 8.  
8 read; +4 per card drawn this turn isn't. Draws are counted per turn. `COUNTER_DRAWN`

**Debilitate** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 10 damage. Vulnerable and Weak are twice as effective against the enemy for the next 2 turns.  
*Upgraded:* Deal 12 damage. Vulnerable and Weak are twice as effective against the enemy for the next 3 turns.  
*Simulator:* damage 10.  
Damage read; doubling Vulnerable and Weak for 2 turns isn't. `DEBUFF_AMP`

**Fetch** · 0 energy · Attack · Uncommon · ⚠️ misread  
Osty deals 3 damage. If this is the first time this card has been played this turn, draw 1 card.  
*Upgraded:* Osty deals 6 damage. If this is the first time this card has been played this turn, draw 1 card.  
*Simulator:* damage 3, Osty damage 3, draw 1.  
Osty's 3 counted twice. Draw read (it only draws the first time each turn). `OSTY_DOUBLE`

**High Five** · 2 energy · Attack · Uncommon · ⚠️ misread  
Osty deals 11 damage and applies 2 Vulnerable to ALL enemies.  
*Upgraded:* Osty deals 13 damage and applies 3 Vulnerable to ALL enemies.  
*Simulator:* damage 11, Osty damage 11, Vulnerable 2, all enemies.  
Area 11 counted twice; Vulnerable read. `OSTY_DOUBLE`

**Pull from Below** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 5 damage for each Ethereal card played this combat.  
*Upgraded:* Deal 7 damage for each Ethereal card played this combat.  
*Simulator:* damage 5.  
5 per Ethereal card played this combat reads as a flat 5. `COUNTER_ETHEREAL`

**Rattle** · 1 energy · Attack · Uncommon · ⚠️ misread  
Osty deals 7 damage. Hits an additional time for each other time he has attacked this turn.  
*Upgraded:* Osty deals 9 damage. Hits an additional time for each other time he has attacked this turn.  
*Simulator:* damage 7, Osty damage 7.  
7 counted twice; +1 hit per other Osty attack this turn isn't. `OSTY_DOUBLE` `COUNTER_OSTY`

**Right Hand Hand** · 0 energy · Attack · Uncommon · ⚠️ misread  
Osty deals 4 damage. Whenever you play a card that costs 2 Energy or more, return this to your Hand from the Discard Pile.  
*Upgraded:* Osty deals 6 damage. Whenever you play a card that costs 2 Energy or more, return this to your Hand from the Discard Pile.  
*Simulator:* damage 4, Osty damage 4, energy 2.  
Osty's 4 counted twice; the "costs 2 or more" threshold is misread as +2 energy. Returning to the hand from the discard after a 2+ cost card isn't read. `OSTY_DOUBLE` `ENERGY_VAR` `RETURN_TO_HAND`

**Severance** · 2 energy · Attack · Uncommon · ✅ read  
Deal 13 damage. Add a Soul into your Draw Pile, Hand, and Discard Pile.  
*Upgraded:* Deal 18 damage. Add a Soul into your Draw Pile, Hand, and Discard Pile.  
*Makes:* Soul  
*Simulator:* damage 13, n=1, pile=Draw }, Make { name: "soul", n=1, pile=Hand }, Make { name: "soul", n=1, pile=Discard }].  
A Soul into the Draw Pile, hand and Discard Pile.

**Sic 'Em** · 1 energy · Attack · Uncommon · ⚠️ misread  
Osty deals 5 damage. Whenever Osty hits this enemy this turn, Summon 3.  
*Upgraded:* Osty deals 6 damage. Whenever Osty hits this enemy this turn, Summon 4.  
*Simulator:* damage 5, Osty damage 5.  
Osty's 5 counted twice; Summon 3 per Osty hit this turn isn't. `OSTY_DOUBLE` `ON_OSTY_HIT`

**Veilpiercer** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 10 damage. The next Ethereal card you play costs 0 Energy.  
*Upgraded:* Deal 13 damage. The next Ethereal card you play costs 0 Energy.  
*Simulator:* damage 10.  
The next Ethereal card costing 0 isn't read. `COST_NEXT`

**Banshee's Cry** · 9 energy · Attack · Rare · ⚠️ misread  
Deal 33 damage to ALL enemies. Costs 2 Energy less for each Ethereal card played this combat.  
*Upgraded:* Costs 7  
*Simulator:* damage 33, energy 2, all enemies.  
Area damage read; "costs 2 less per Ethereal card played this combat" is misread as +2 energy on play. Needs an Ethereal-played counter. `ENERGY_VAR` `COST_DOWN`

**Eradicate** · X energy · Attack · Rare · Retain · ✅ read  
Deal 11 damage X times.  
*Upgraded:* Deal 14 damage X times.  
*Simulator:* damage 11, X-cost.  

**Hang** · 1 energy · Attack · Rare · ◐ partly read  
Deal 10 damage. Double the damage ALL Hang cards deal to this enemy.  
*Upgraded:* Deal 13 damage. Double the damage ALL Hang cards deal to this enemy.  
*Simulator:* damage 10.  
10 read; doubling every Hang's damage against that enemy isn't. `SELF_GROW`

**Misery** · 0 energy · Attack · Rare · ✅ read  
Deal 7 damage. Apply any debuffs on the enemy to ALL other enemies.  
*Upgraded:* Deal 9 damage. Apply any debuffs on the enemy to ALL other enemies.  
*Simulator:* damage 7.  
Spreads debuffs to other enemies; nothing to spread on a single target.

**Soul Storm** · 1 energy · Attack · Rare · ✅ read  
Deal 9 damage. Deals 4 additional damage for each Soul in your Exhaust Pile.  
*Upgraded:* Deal 9 damage. Deals 6 additional damage for each Soul in your Exhaust Pile.  
*Makes:* Soul  
*Simulator:* damage 9, +4 per soul in Exhaust Pile.  
9 + 4 per Soul in the Exhaust Pile (Souls exhaust when played).

**Squeeze** · 3 energy · Attack · Rare · ◐ partly read  
Osty deals 25 damage. Deals 5 additional damage for ALL your other Osty Attacks.  
*Upgraded:* Osty deals 25 damage. Deals 6 additional damage for ALL your other Osty Attacks.  
*Simulator:* damage 25.  
25 read as a player attack; +5 per other Osty Attack in the deck isn't. `DECK_COUNT` `OSTY_SCALE`

**The Scythe** · 2 energy · Attack · Rare · Exhaust · ◐ partly read  
Deal 13 damage. Permanently increase this card's damage by 5.  
*Upgraded:* Deal 13 damage. Permanently increase this card's damage by 7.  
*Simulator:* damage 13.  
13 read; +5 permanently per play is run growth. `PERMANENT_GROW`

**Time's Up** · 2 energy · Attack · Rare · ○ not read  
Deal damage equal to the enemy's Doom.  
*Upgraded:* Gains retain  
*Simulator:* nothing.  
Damage equal to the enemy's Doom. Doom is tallied as a resource; make it enemy state and read it here. `DOOM_DMG`

**Protector** · 1 energy · Attack · Ancient · ◐ partly read  
Osty deals 10 damage. Deals additional damage equal to Osty's Max HP.  
*Upgraded:* Costs 0  
*Simulator:* damage 10.  
Ancient. Osty deals 10 plus his Max HP. Read as a flat 10 player attack. Needs Osty's Max HP (sum of Summons) and Osty damage routing. `OSTY_SCALE`

#### Necrobinder: Skills

**Bodyguard** · 1 energy · Skill · Basic · ✅ read  
Summon 5.  
*Upgraded:* Summon 7.  
*Simulator:* Summon 5.  
Starter. Summon 5.

**Defend** · 1 energy · Skill · Basic · ✅ read  
Gain 5 Block.  
*Upgraded:* Gain 8 Block.  
*Simulator:* Block 5.  

**Afterlife** · 1 energy · Skill · Common · Exhaust · ✅ read  
Summon 6.  
*Upgraded:* Summon 9.  
*Simulator:* Summon 6.  

**Defy** · 1 energy · Skill · Common · Ethereal · ✅ read  
Gain 6 Block. Apply 1 Weak.  
*Upgraded:* Gain 9 Block. Apply 1 Weak.  
*Simulator:* Block 6, Weak 1.  

**Grave Warden** · 1 energy · Skill · Common · ✅ read  
Gain 8 Block. Add a Soul into your Draw Pile.  
*Upgraded:* Gain 11 Block. Add a Soul into your Draw Pile.  
*Makes:* Soul  
*Simulator:* Block 8, n=1, pile=Draw }].  

**Invoke** · 1 energy · Skill · Common · ✅ read  
Next turn, Summon 2 and gain 2 Energy.  
*Upgraded:* Next turn, Summon 3 and gain 3 Energy.  
*Simulator:* energy next turn 2, Summon 2.  
Next turn: Summon 2 and 2 energy. (Summon is applied now, a turn early.)

**Negative Pulse** · 1 energy · Skill · Common · ✅ read  
Gain 5 Block. Apply 7 Doom to ALL enemies.  
*Upgraded:* Gain 6 Block. Apply 11 Doom to ALL enemies.  
*Simulator:* Block 5, Doom 7, all enemies.  

**Pull Aggro** · 2 energy · Skill · Common · ✅ read  
Summon 4. Gain 7 Block.  
*Upgraded:* Summon 5. Gain 9 Block.  
*Simulator:* Block 7, Summon 4.  

**Scourge** · 1 energy · Skill · Common · ✅ read  
Apply 13 Doom. Draw 1 card.  
*Upgraded:* Apply 16 Doom. Draw 2 cards.  
*Simulator:* draw 1, Doom 13.  

**Wisp** · 0 energy · Skill · Common · Exhaust · ✅ read  
Gain 1 Energy.  
*Upgraded:* Gains retain  
*Simulator:* energy 1.  

**Borrowed Time** · 1 energy · Skill · Uncommon · ◐ partly read  
Gain 4 Energy. Cards cost an additional 1 Energy this turn.  
*Upgraded:* Gain 6 Energy. Cards cost an additional 1 Energy this turn.  
*Simulator:* energy 4.  
4 energy read; +1 cost on every card this turn isn't, so it is overvalued. `COST_UP_TEMP`

**Capture Spirit** · 1 energy · Skill · Uncommon · ✅ read  
Enemy loses 3 HP. Add 3 Souls into your Draw Pile.  
*Upgraded:* Enemy loses 4 HP. Add 4 Souls into your Draw Pile.  
*Makes:* Soul  
*Simulator:* damage 3, n=3, pile=Draw }].  
HP loss read as damage (Strength wrongly applies). 3 Souls.

**Cleanse** · 1 energy · Skill · Uncommon · ◐ partly read  
Summon 3. Exhaust 1 card from your Draw Pile.  
*Upgraded:* Summon 5. Exhaust 1 card from your Draw Pile.  
*Simulator:* Summon 3.  
Summon read; exhausting a Draw Pile card isn't. `EXH_CHOOSE`

**Death's Door** · 1 energy · Skill · Uncommon · ◐ partly read  
Gain 6 Block. If you applied Doom this turn, gain Block 2 additional times.  
*Upgraded:* Gain 7 Block. If you applied Doom this turn, gain Block 2 additional times.  
*Simulator:* Block 6.  
6 Block read; ×3 if you applied Doom this turn isn't. `COND_DOOM`

**Deathbringer** · 2 energy · Skill · Uncommon · ✅ read  
Apply 21 Doom and 1 Weak to ALL enemies.  
*Upgraded:* Apply 26 Doom and 1 Weak to ALL enemies.  
*Simulator:* Doom 21, Weak 1, all enemies.  

**Delay** · 2 energy · Skill · Uncommon · ✅ read  
Gain 11 Block. Next turn, gain 1 Energy.  
*Upgraded:* Gain 13 Block. Next turn, gain 2 Energy.  
*Simulator:* Block 11, energy next turn 1.  

**Dirge** · X energy · Skill · Uncommon · Exhaust · ✅ read  
Summon 3 X times. Add X Souls into your Draw Pile.  
*Upgraded:* Summon 4 X times. Add X Souls+ into your Draw Pile.  
*Makes:* Soul  
*Simulator:* Summon 3, X-cost, n=-1, pile=Draw }].  
X Summons of 3 and X Souls.

**Dredge** · 1 energy · Skill · Uncommon · Exhaust · ○ not read  
Put 3 cards from your Discard Pile into your Hand.  
*Upgraded:* Gains retain  
*Simulator:* nothing.  
3 cards from the Discard Pile to the hand. Unread. `PILE_MOVE`

**Enfeebling Touch** · 1 energy · Skill · Uncommon · Ethereal · ◐ partly read  
Enemy loses 8 Strength this turn.  
*Upgraded:* Enemy loses 11 Strength this turn.  
*Simulator:* nothing.  
−8 enemy Strength this turn is mitigation. Survival should count it. `ENEMY_STR`

**Legion of Bone** · 2 energy · Skill · Uncommon · multiplayer · ✅ read  
ALL players Summon 6.  
*Upgraded:* ALL players Summon 8.  
*Simulator:* Summon 6.  
Multiplayer only; your own Summon 6 is read. `MP`

**Melancholy** · 3 energy · Skill · Uncommon · ⚠️ misread  
Gain 13 Block. Reduce this card's cost by 1 Energy whenever ANYONE dies.  
*Upgraded:* Gain 17 Block. Reduce this card's cost by 1 Energy whenever ANYONE dies.  
*Simulator:* Block 13, energy 1.  
Block read; "cost −1 whenever anyone dies" misread as +1 energy. `ENERGY_VAR`

**No Escape** · 1 energy · Skill · Uncommon · ⚠️ misread  
Apply 10 Doom, plus an additional 5 Doom for every 10 Doom already on this enemy.  
*Upgraded:* Apply 15 Doom, plus an additional 5 Doom for every 10 Doom already on this enemy.  
*Simulator:* damage 10.  
Read as 10 damage (calculation base). It applies 10 Doom plus 5 per 10 already on the enemy. `DOOM_SCALE`

**Parse** · 1 energy · Skill · Uncommon · Ethereal · ✅ read  
Draw 3 cards.  
*Upgraded:* Draw 4 cards.  
*Simulator:* draw 3.  

**Putrefy** · 1 energy · Skill · Uncommon · Exhaust · ⚠️ misread  
Apply 2 Weak. Apply 2 Vulnerable.  
*Upgraded:* Apply 3 Weak. Apply 3 Vulnerable.  
*Simulator:* nothing.  
2 Weak and 2 Vulnerable sit under `power` and are dropped. `VAR_POWER`

**Spur** · 1 energy · Skill · Uncommon · Retain · ✅ read  
Summon 3. Osty heals 5 HP.  
*Upgraded:* Summon 5. Osty heals 7 HP.  
*Simulator:* Summon 3.  
Summon read; Osty's heal isn't (Osty HP isn't modelled).

**Underworld** · 2 energy · Skill · Uncommon · Exhaust · multiplayer · ○ not read  
Whenever other players deal attack damage this turn, apply that much Doom.  
*Upgraded:* Loses exhaust  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Eidolon** · 2 energy · Skill · Rare · Exhaust · ○ not read  
Play ALL Ethereal cards in your Exhaust Pile.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Plays every Ethereal card in the Exhaust Pile. Unread; the pile is tracked. `EXHAUST_REPLAY`

**End of Days** · 3 energy · Skill · Rare · ◐ partly read  
Apply 29 Doom to ALL enemies. Kill enemies with at least as much Doom as HP.  
*Upgraded:* Apply 37 Doom to ALL enemies. Kill enemies with at least as much Doom as HP.  
*Simulator:* Doom 29, all enemies.  
29 Doom to all read; the immediate execute isn't. `DOOM_EXECUTE`

**Glimpse Beyond** · 1 energy · Skill · Rare · Exhaust · multiplayer · ✅ read  
ALL players add 3 Souls into their Draw Pile.  
*Upgraded:* ALL players add 4 Souls into their Draw Pile.  
*Makes:* Soul  
*Simulator:* n=3, pile=Draw }].  
Multiplayer only; your own 3 Souls are read. `MP`

**Oblivion** · 0 energy · Skill · Rare · ⚠️ misread  
Whenever you play a card this turn, apply 3 Doom to the enemy.  
*Upgraded:* Whenever you play a card this turn, apply 4 Doom to the enemy.  
*Simulator:* Doom 3.  
Read as 3 Doom once; it is 3 Doom per card played this turn. `ON_PLAY_TEMP`

**Reanimate** · 3 energy · Skill · Rare · Exhaust · ✅ read  
Summon 20.  
*Upgraded:* Summon 25.  
*Simulator:* Summon 20.  
Summon 20.

**Sacrifice** · 1 energy · Skill · Rare · Retain · ○ not read  
If Osty is alive, he dies and you gain Block equal to triple his Max HP.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Block equal to 3× Osty's Max HP; Osty dies. Unread. `OSTY_SCALE`

**Seance** · 1 energy · Skill · Rare · Ethereal · ○ not read  
Transform a card in your Draw Pile into Soul.  
*Upgraded:* Costs 0  
*Makes:* Soul  
*Simulator:* nothing.  
Turns a Draw Pile card into a Soul. Unread. `TRANSFORM`

**Shared Fate** · 0 energy · Skill · Rare · Exhaust · ◐ partly read  
Lose 2 Strength. Enemy loses 2 Strength.  
*Upgraded:* Lose 2 Strength. Enemy loses 3 Strength.  
*Simulator:* nothing.  
−2 Strength to you and the enemy. Neither is read. Tag error: "Weak". `ENEMY_STR`

**Transfigure** · 1 energy · Skill · Rare · Exhaust · ⚠️ misread  
Add Replay to a card in your Hand. It costs an extra 1 Energy.  
*Upgraded:* Loses exhaust  
*Simulator:* energy 1.  
"Costs an extra 1" misread as +1 energy. Giving Replay to a hand card isn't read. `ENERGY_VAR` `GRANT_REPLAY`

**Undeath** · 0 energy · Skill · Rare · ✅ read  
Gain 7 Block. Add a copy of this card into your Discard Pile.  
*Upgraded:* Gain 9 Block. Add a copy of this card into your Discard Pile.  
*Simulator:* Block 7, copy to discard.  
A 0-cost copy to the Discard Pile each play.

#### Necrobinder: Powers

**Calcify** · 1 energy · Power · Uncommon · ✅ read  
Osty's attacks deal 4 additional damage.  
*Upgraded:* Osty's attacks deal 6 additional damage.  
*Simulator:* power: Osty deals 4 more damage.  
+4 to Osty's attacks.

**Countdown** · 1 energy · Power · Uncommon · ✅ read  
At the start of your turn, apply 6 Doom to a random enemy.  
*Upgraded:* At the start of your turn, apply 9 Doom to a random enemy.  
*Simulator:* power: Each turn: apply 6 Doom.  
6 Doom each turn.

**Danse Macabre** · 1 energy · Power · Uncommon · ✅ read  
Whenever you play a card that costs 2 Energy or more, gain 4 Block.  
*Upgraded:* Whenever you play a card that costs 2 Energy or more, gain 6 Block.  
*Simulator:* power: Whenever you play a card costing 2+: gain 4 Block.  
4 Block per card costing 2+. Tag error: "Energy 2".

**Friendship** · 1 energy · Power · Uncommon · ✅ read  
Lose 2 Strength. Gain 1 Energy at the start of each turn.  
*Upgraded:* Lose 1 Strength. Gain 1 Energy at the start of each turn.  
*Simulator:* power: When played: lose 2 Strength, Each turn: gain 1 Energy.  
−2 Strength, +1 energy each turn. Tag error: "Weak" and "Scaling (damage) 2" come from the Strength loss.

**Haunt** · 1 energy · Power · Uncommon · ○ not read  
Whenever you play a Soul, a random enemy loses 7 HP.  
*Upgraded:* Whenever you play a Soul, a random enemy loses 9 HP.  
*Makes:* Soul  
*Simulator:* nothing.  
7 HP loss to a random enemy per Soul played. Unread; `OnPlay(Named("soul"))` + Damage covers it. `ON_PLAY_NAMED`

**Lethality** · 1 energy · Power · Uncommon · Ethereal · ✅ read  
The first Attack each turn deals 50% additional damage.  
*Upgraded:* The first Attack each turn deals 75% additional damage.  
*Simulator:* power: First attack each turn +50%.  
First Attack each turn +50%. Ethereal power.

**Pagestorm** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you draw an Ethereal card, draw 1 card.  
*Upgraded:* Costs 0  
*Simulator:* power: When played: draw 1.  
Misread as draw 1 on play. It draws 1 per Ethereal card drawn. `TRIGGER_FALLBACK` `ON_DRAW_ETHEREAL`

**Shroud** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you apply Doom, gain 3 Block.  
*Upgraded:* Whenever you apply Doom, gain 4 Block.  
*Simulator:* power: When played: gain 3 Block.  
Misread as 3 Block on play. It is 3 Block per Doom application. `TRIGGER_FALLBACK` `ON_APPLY`

**Sleight of Flesh** · 2 energy · Power · Uncommon · ○ not read  
Whenever you apply a debuff to an enemy, they take 9 damage.  
*Upgraded:* Whenever you apply a debuff to an enemy, they take 13 damage.  
*Simulator:* nothing.  
9 damage per debuff applied. Unread. The simulator counts debuffs applied (`log.debuffs`), so a trigger is cheap. `ON_APPLY`

**Soulbound** · 1 energy · Power · Uncommon · multiplayer · ○ not read  
Choose an ally. Whenever you create a Soul, add a Soul to their Draw Pile.  
*Upgraded:* Gains innate  
*Makes:* Soul  
*Simulator:* n=1, pile=Draw }], power: When played: add 1 soul to your Draw.  
Multiplayer only (misread as one Soul on play). `MP`

**Cacophony** · 2 energy · Power · Rare · multiplayer · ○ not read  
Every 33 cards drawn by ALL players, deal 66 damage to a random enemy.  
*Upgraded:* Every 33 cards drawn by ALL players, deal 99 damage to a random enemy.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Call of the Void** · 1 energy · Power · Rare · ○ not read  
At the start of your turn, add 1 random card into your Hand. It gains Ethereal.  
*Upgraded:* Gains innate  
*Simulator:* nothing.  
A random Ethereal card each turn. Unread. `RANDOM_GEN`

**Demesne** · 3 energy · Power · Rare · Ethereal · ✅ read  
At the start of your turn, gain 1 Energy and draw 1 additional card.  
*Upgraded:* Costs 2  
*Simulator:* power: Each turn: gain 1 Energy, draw 1.  
Ethereal power: +1 energy and +1 draw each turn.

**Devour Life** · 1 energy · Power · Rare · ⚠️ misread  
Whenever you play a Soul, Summon 1.  
*Upgraded:* Whenever you play a Soul, Summon 2.  
*Makes:* Soul  
*Simulator:* power: When played: Summon 1.  
Misread as Summon 1 on play. It is Summon 1 per Soul played; `OnPlay(Named("soul"))` covers it. `TRIGGER_FALLBACK` `ON_PLAY_NAMED`

**Necro Mastery** · 2 energy · Power · Rare · ◐ partly read  
Summon 5. Whenever Osty loses HP, ALL enemies lose that much HP as well.  
*Upgraded:* Summon 8. Whenever Osty loses HP, ALL enemies lose that much HP as well.  
*Simulator:* power: When played: Summon 5.  
Summon read; enemies losing HP when Osty does needs enemy attacks on Osty. `OSTY_HP_LOSS`

**Neurosurge** · 0 energy · Power · Rare · ◐ partly read  
Gain 3 Energy. Draw 2 cards. At the start of your turn, apply 3 Doom to yourself.  
*Upgraded:* Gain 4 Energy. Draw 2 cards. At the start of your turn, apply 3 Doom to yourself.  
*Simulator:* power: When played: gain 3 Energy, When played: draw 2.  
Energy and draw read; 3 Doom on yourself each turn is a survival clock: you die when your HP falls to your Doom. `SELF_DOOM`

**Reaper Form** · 3 energy · Power · Rare · ○ not read  
Whenever Attacks deal damage, apply that much Doom.  
*Upgraded:* Gains retain  
*Simulator:* nothing.  
Doom equal to Attack damage dealt. Unread. Doom needs to be enemy state first. `DOOM_FROM_DMG`

**Sentry Mode** · 2 energy · Power · Rare · ✅ read  
At the start of your turn, add 1 Sweeping Gaze into your Hand.  
*Upgraded:* Costs 1  
*Makes:* Sweeping Gaze  
*Simulator:* n=1, pile=Hand }], power: Each turn: add 1 sweeping gaze to your Hand.  
A Sweeping Gaze each turn.

**Spirit of Ash** · 1 energy · Power · Rare · ⚠️ misread  
Whenever you play an Ethereal card, gain 4 Block.  
*Upgraded:* Whenever you play an Ethereal card, gain 5 Block.  
*Simulator:* power: When played: gain 4 Block.  
Misread as 4 Block on play. It is 4 Block per Ethereal card played. Needs an Ethereal filter. `TRIGGER_FALLBACK` `ON_PLAY_ETHEREAL`

**Forbidden Grimoire** · 2 energy · Power · Ancient · Eternal · ○ not read  
At the end of combat, you may remove a card from your Deck.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Eternal. Removes a card after each combat (deck thinning). Run value, not fight output. `OUT_OF_COMBAT`

### Regent

#### Regent: Attacks

**Falling Star** · 0 energy + 2 Stars · Attack · Basic · ✅ read  
Deal 8 damage. Apply 1 Weak. Apply 1 Vulnerable.  
*Upgraded:* Deal 12 damage. Apply 1 Weak. Apply 1 Vulnerable.  
*Simulator:* damage 8, Vulnerable 1, Weak 1, 2 Stars to play.  
Starter. Costs 0 energy and 2 Stars.

**Strike** · 1 energy · Attack · Basic · ✅ read  
Deal 6 damage.  
*Upgraded:* Deal 9 damage.  
*Simulator:* damage 6.  

**Astral Pulse** · 0 energy + 3 Stars · Attack · Common · ✅ read  
Deal 6 damage to ALL enemies twice.  
*Upgraded:* Deal 8 damage to ALL enemies twice.  
*Simulator:* damage 6×2, 3 Stars to play, all enemies.  

**Celestial Might** · 2 energy · Attack · Common · ✅ read  
Deal 6 damage 3 times.  
*Upgraded:* Deal 6 damage 4 times.  
*Simulator:* damage 6×3.  

**Collision Course** · 0 energy · Attack · Common · ✅ read  
Deal 10 damage. Add a Debris into your Hand.  
*Upgraded:* Deal 14 damage. Add a Debris into your Hand.  
*Makes:* Debris  
*Simulator:* damage 10, n=1, pile=Hand }].  
Adds a Debris (Status-like token) to the hand.

**Crescent Spear** · 1 energy + 1 Stars · Attack · Common · ◐ partly read  
Deal 8 damage. Deals 2 additional damage for ALL your cards that have a 1 Star cost.  
*Upgraded:* Deal 8 damage. Deals 3 additional damage for ALL your cards that have a 1 Star cost.  
*Simulator:* damage 8, 1 Stars to play.  
8 read; +2 per Star-cost card in the deck isn't. `DECK_COUNT`

**Crush Under** · 1 energy · Attack · Common · ◐ partly read  
Deal 8 damage to ALL enemies. All enemies lose 1 Strength this turn.  
*Upgraded:* Deal 9 damage to ALL enemies. All enemies lose 2 Strength this turn.  
*Simulator:* damage 8, all enemies.  
Area damage read; −1 enemy Strength this turn is survival. Tag error: "Weak". `ENEMY_STR`

**Guiding Star** · 1 energy + 1 Stars · Attack · Common · ⚠️ misread  
Deal 12 damage. Next turn, draw 2 cards.  
*Upgraded:* Deal 13 damage. Next turn, draw 3 cards.  
*Simulator:* damage 12, draw 2, 1 Stars to play.  
"Next turn, draw 2" is drawn now. Should add 2 to next turn's draw. `DRAW_NEXT`

**Photon Cut** · 1 energy · Attack · Common · ◐ partly read  
Deal 10 damage. Draw 1 card. Put 1 card from your Hand on top of your Draw Pile.  
*Upgraded:* Deal 13 damage. Draw 2 cards. Put 1 card from your Hand on top of your Draw Pile.  
*Simulator:* damage 10, draw 1.  
Damage and draw read; putting a hand card on top of the Draw Pile isn't. `PILE_TOP`

**Solar Strike** · 1 energy · Attack · Common · ✅ read  
Deal 9 damage. Gain 1 Star.  
*Upgraded:* Deal 10 damage. Gain 2 Star.  
*Simulator:* damage 9, Stars 1.  

**Wrought in War** · 1 energy · Attack · Common · ✅ read  
Deal 7 damage. Forge 7.  
*Upgraded:* Deal 9 damage. Forge 9.  
*Simulator:* damage 7, Forge 7.  
Forge 7.

**Devastate** · 1 energy + 4 Stars · Attack · Uncommon · ✅ read  
Deal 35 damage.  
*Upgraded:* Deal 45 damage.  
*Simulator:* damage 35, 4 Stars to play.  

**Gamma Blast** · 0 energy + 3 Stars · Attack · Uncommon · ✅ read  
Deal 13 damage. Apply 2 Weak. Apply 2 Vulnerable.  
*Upgraded:* Deal 18 damage. Apply 2 Weak. Apply 2 Vulnerable.  
*Simulator:* damage 13, Vulnerable 2, Weak 2, 3 Stars to play.  

**Hegemony** · 2 energy · Attack · Uncommon · ✅ read  
Deal 15 damage. Next turn, gain 2 Energy.  
*Upgraded:* Deal 18 damage. Next turn, gain 3 Energy.  
*Simulator:* damage 15, energy next turn 2.  

**Kingly Kick** · 4 energy · Attack · Uncommon · ◐ partly read  
Deal 27 damage. Whenever you draw this card, reduce its cost by 1.  
*Upgraded:* Deal 35 damage. Whenever you draw this card, reduce its cost by 1.  
*Simulator:* damage 27.  
27 read at cost 4; −1 cost each time it is drawn (this combat) isn't. `COST_DOWN_DRAW`

**Kingly Punch** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 8 damage. Whenever you draw this card, increase its damage by 4 this combat.  
*Upgraded:* Deal 10 damage. Whenever you draw this card, increase its damage by 6 this combat.  
*Simulator:* damage 8.  
8 read; +4 each time it is drawn isn't. `SELF_GROW_DRAW`

**Knockout Blow** · 3 energy · Attack · Uncommon · ◐ partly read  
Deal 30 damage. If this kills an enemy, gain 5 Star.  
*Upgraded:* Deal 38 damage. If this kills an enemy, gain 5 Star.  
*Simulator:* damage 30.  
Damage read; 5 Stars on a kill isn't. `FATAL`

**Lunar Blast** · 0 energy · Attack · Uncommon · ◐ partly read  
Deal 4 damage for each Skill already played this turn.  
*Upgraded:* Deal 5 damage for each Skill already played this turn.  
*Simulator:* damage 4.  
4 per Skill already played this turn reads as a flat 4. `COUNTER_SKILLS`

**Radiate** · 0 energy · Attack · Uncommon · ◐ partly read  
Deal 3 damage to ALL enemies for each 1 Star gained this turn.  
*Upgraded:* Deal 4 damage to ALL enemies for each 1 Star gained this turn.  
*Simulator:* damage 3, all enemies.  
3 to all per Star gained this turn reads as a flat 3. `COUNTER_STARS`

**Shining Strike** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 8 damage. Gain 2 Star. Put this card on top of your Draw Pile.  
*Upgraded:* Deal 11 damage. Gain 2 Star. Put this card on top of your Draw Pile.  
*Simulator:* damage 8, Stars 2.  
Damage and Stars read; going back on top of the Draw Pile (redrawn next turn) isn't. `PILE_TOP`

**Stardust** · 0 energy + X Stars · Attack · Uncommon · ⚠️ misread  
Deal 5 damage to a random enemy X times.  
*Upgraded:* Deal 7 damage to a random enemy X times.  
*Simulator:* damage 5.  
X is paid in Stars, but read as one hit with no Star cost. Hits should equal the Stars spent. `X_STARS`

**Supermassive** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 5 damage. Deals 3 additional damage for each card you created this combat.  
*Upgraded:* Deal 5 damage. Deals 4 additional damage for each card you created this combat.  
*Simulator:* damage 5.  
5 read; +3 per card created this combat isn't. Made cards are tracked, so count them. `COUNTER_MADE`

**Beat into Shape** · 1 energy · Attack · Rare · ◐ partly read  
Deal 5 damage. Forge 5. Forges an additional 5 for every other time you've hit the enemy this turn.  
*Upgraded:* Deal 7 damage. Forge 7. Forges an additional 7 for every other time you've hit the enemy this turn.  
*Simulator:* damage 5.  
Damage read; Forge 5 (+5 per other hit on the enemy this turn) isn't (`forge` sits under `calculationbase`). `COUNTER_HITS`

**Bombardment** · 3 energy · Attack · Rare · Exhaust · ✅ read  
Deal 18 damage. At the start of your turn, if this is in your Exhaust Pile, play it.  
*Upgraded:* Deal 24 damage. At the start of your turn, if this is in your Exhaust Pile, play it.  
*Simulator:* damage 18, replays from Exhaust Pile at TurnStart.  
Replays from the Exhaust Pile each turn start.

**Comet** · 0 energy + 5 Stars · Attack · Rare · ✅ read  
Deal 33 damage. Apply 3 Weak. Apply 3 Vulnerable.  
*Upgraded:* Deal 44 damage. Apply 3 Weak. Apply 3 Vulnerable.  
*Simulator:* damage 33, Vulnerable 3, Weak 3, 5 Stars to play.  

**Crash Landing** · 1 energy · Attack · Rare · ◐ partly read  
Deal 21 damage to ALL enemies. Fill your Hand with Debris.  
*Upgraded:* Deal 26 damage to ALL enemies. Fill your Hand with Debris.  
*Makes:* Debris  
*Simulator:* damage 21, all enemies.  
Area damage read; filling the hand with Debris isn't. Debris clogs the next draws and feeds Arsenal, Pillar of Creation and Supermassive. `FILL_HAND`

**Dying Star** · 1 energy + 3 Stars · Attack · Rare · Ethereal · ◐ partly read  
Deal 9 damage to ALL enemies. ALL enemies lose 9 Strength this turn.  
*Upgraded:* Deal 11 damage to ALL enemies. ALL enemies lose 11 Strength this turn.  
*Simulator:* damage 9, 3 Stars to play, all enemies.  
Area damage read; −9 enemy Strength this turn is survival. Tag error: "Weak". `ENEMY_STR`

**Heavenly Drill** · X energy · Attack · Rare · ⚠️ misread  
Deal 8 damage X times. Double X if it's 4 or more.  
*Upgraded:* Deal 10 damage X times. Double X if it's 4 or more.  
*Simulator:* damage 8, energy 4, X-cost.  
"Double X if it's 4 or more" is misread as +4 energy. Should be hits = X, or 2X when X ≥ 4. `ENERGY_VAR` `X_DOUBLE`

**Heirloom Hammer** · 2 energy · Attack · Rare · ◐ partly read  
Deal 20 damage. Choose a Colorless card in your Hand. Add a copy of that card into your Hand.  
*Upgraded:* Deal 25 damage. Choose a Colorless card in your Hand. Add a copy of that card into your Hand.  
*Simulator:* damage 20.  
Damage read; copying a Colorless card in the hand isn't. `COPY_CHOSEN`

**Make It So** · 0 energy · Attack · Rare · ◐ partly read  
Deal 6 damage. Every 3 Skills you play in a turn, put this into your Hand.  
*Upgraded:* Deal 9 damage. Every 3 Skills you play in a turn, put this into your Hand.  
*Simulator:* damage 6.  
Damage read; returning to the hand every 3 Skills in a turn isn't. `RETURN_TO_HAND`

**Seven Stars** · 2 energy + 7 Stars · Attack · Rare · ✅ read  
Deal 7 damage to ALL enemies 7 times.  
*Upgraded:* Costs 1  
*Simulator:* damage 7×7, 7 Stars to play, all enemies.  

**Meteor Shower** · 0 energy + 2 Stars · Attack · Ancient · ✅ read  
Deal 14 damage to ALL enemies. Apply 2 Weak and Vulnerable to ALL enemies.  
*Upgraded:* Deal 21 damage to ALL enemies. Apply 2 Weak and Vulnerable to ALL enemies.  
*Simulator:* damage 14, Vulnerable 2, Weak 2, 2 Stars to play, all enemies.  
Ancient. 2 Stars: area damage, 2 Weak and 2 Vulnerable.

#### Regent: Skills

**Defend** · 1 energy · Skill · Basic · ✅ read  
Gain 5 Block.  
*Upgraded:* Gain 8 Block.  
*Simulator:* Block 5.  

**Venerate** · 1 energy · Skill · Basic · ✅ read  
Gain 2 Star.  
*Upgraded:* Gain 3 Star.  
*Simulator:* Stars 2.  
Starter. 2 Stars.

**BEGONE!** · 1 energy · Skill · Common · ○ not read  
Choose a card in your Hand to Transform into Minion Strike.  
*Upgraded:* Choose a card in your Hand to Transform into Minion Strike+.  
*Makes:* Minion Strike  
*Simulator:* nothing.  
Turns a hand card into Minion Strike. Unread. `TRANSFORM`

**Cloak of Stars** · 0 energy + 1 Stars · Skill · Common · ✅ read  
Gain 7 Block.  
*Upgraded:* Gain 10 Block.  
*Simulator:* Block 7, 1 Stars to play.  

**Cosmic Indifference** · 1 energy · Skill · Common · ◐ partly read  
Gain 6 Block. Put a card from your Discard Pile on top of your Draw Pile.  
*Upgraded:* Gain 9 Block. Put a card from your Discard Pile on top of your Draw Pile.  
*Simulator:* Block 6.  
Block read; putting a Discard Pile card on top of the Draw Pile isn't. `PILE_MOVE`

**Gather Light** · 1 energy · Skill · Common · ✅ read  
Gain 8 Block. Gain 1 Star.  
*Upgraded:* Gain 11 Block. Gain 1 Star.  
*Simulator:* Block 8, Stars 1.  

**Glitterstream** · 2 energy · Skill · Common · ◐ partly read  
Gain 11 Block. Next turn, gain 5 Block.  
*Upgraded:* Gain 13 Block. Next turn, gain 7 Block.  
*Simulator:* Block 11.  
11 Block read; 5 Block next turn isn't. `BLOCK_NEXT`

**Glow** · 1 energy · Skill · Common · ◐ partly read  
Gain 1 Star. Draw 1 card. Next turn, draw 1 card.  
*Upgraded:* Gain 2 Star. Draw 1 card. Next turn, draw 1 card.  
*Simulator:* draw 1, Stars 1.  
Stars and the draw now read; next turn's draw isn't. `DRAW_NEXT`

**Hidden Cache** · 1 energy · Skill · Common · ◐ partly read  
Gain 1 Star. Next turn, gain 3 Star.  
*Upgraded:* Gain 1 Star. Next turn, gain 4 Star.  
*Simulator:* Stars 1.  
1 Star read; 3 Stars next turn isn't. `STARS_NEXT`

**Know Thy Place** · 0 energy · Skill · Common · Exhaust · ✅ read  
Apply 1 Weak. Apply 1 Vulnerable.  
*Upgraded:* Loses exhaust  
*Simulator:* Vulnerable 1, Weak 1.  

**Patter** · 1 energy · Skill · Common · ✅ read  
Gain 8 Block. Gain 2 Vigor.  
*Upgraded:* Gain 10 Block. Gain 3 Vigor.  
*Simulator:* Block 8, Vigor 2.  

**Refine Blade** · 1 energy · Skill · Common · ✅ read  
Forge 8. Next turn, gain 1 Energy.  
*Upgraded:* Forge 12. Next turn, gain 1 Energy.  
*Simulator:* energy next turn 1, Forge 8.  

**Spoils of Battle** · 1 energy · Skill · Common · ✅ read  
Forge 6. Draw 2 cards.  
*Upgraded:* Forge 9. Draw 2 cards.  
*Simulator:* draw 2, Forge 6.  

**Alignment** · 0 energy + 2 Stars · Skill · Uncommon · ✅ read  
Gain 2 Energy.  
*Upgraded:* Gain 3 Energy.  
*Simulator:* energy 2, 2 Stars to play.  

**Bulwark** · 2 energy · Skill · Uncommon · ✅ read  
Gain 12 Block. Forge 10.  
*Upgraded:* Gain 15 Block. Forge 13.  
*Simulator:* Block 12, Forge 10.  

**CHARGE!!** · 1 energy · Skill · Uncommon · ○ not read  
Choose 2 cards in your Draw Pile to Transform into Minion Dive Bombs.  
*Upgraded:* Choose 2 cards in your Draw Pile to Transform into Minion Dive Bombs+.  
*Makes:* Minion Dive Bomb  
*Simulator:* nothing.  
Turns 2 Draw Pile cards into Minion Dive Bombs. Unread. `TRANSFORM`

**Conqueror** · 1 energy · Skill · Uncommon · ◐ partly read  
Forge 3. Sovereign Blade deals double damage to the enemy this turn.  
*Upgraded:* Forge 5. Sovereign Blade deals double damage to the enemy this turn.  
*Simulator:* Forge 3.  
Forge read; the Blade's double damage this turn isn't. `NAMED_DMG_MULT`

**Constellation** · 0 energy + 2 Stars · Skill · Uncommon · multiplayer · ⚠️ misread  
Another player draws 1 card, gains 1 Energy, and gains 9 Block.  
*Upgraded:* Another player draws 1 card, gains 1 Energy, and gains 12 Block.  
*Simulator:* Block 9, energy 1, 2 Stars to play.  
Multiplayer only, and the effects go to another player, yet Block and energy are credited to you. `MP`

**Convergence** · 1 energy · Skill · Uncommon · ◐ partly read  
Next turn, gain 1 Energy and 1 Star. Retain your Hand this turn.  
*Upgraded:* Next turn, gain 1 Energy and 2 Star. Retain your Hand this turn.  
*Simulator:* energy next turn 1.  
Next-turn energy read; next-turn Star and retaining the hand aren't. `STARS_NEXT` `RETAIN_TEMP`

**Glimmer** · 1 energy · Skill · Uncommon · ◐ partly read  
Draw 3 cards. Put 1 card from your Hand on top of your Draw Pile.  
*Upgraded:* Draw 4 cards. Put 1 card from your Hand on top of your Draw Pile.  
*Simulator:* draw 3.  
Draw read; putting one back on top isn't. `PILE_TOP`

**Largesse** · 0 energy · Skill · Uncommon · multiplayer · ○ not read  
Another player adds 1 random Colorless card to their Hand.  
*Upgraded:* Another player adds 1 random Upgraded Colorless card to their Hand.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Manifest Authority** · 1 energy · Skill · Uncommon · ◐ partly read  
Gain 7 Block. Add 1 random Colorless card into your Hand.  
*Upgraded:* Gain 8 Block. Add 1 random Upgraded Colorless card into your Hand.  
*Simulator:* Block 7.  
Block read; the random Colorless card isn't. `RANDOM_GEN`

**Monologue** · 0 energy · Skill · Uncommon · ○ not read  
Whenever you play a card this turn, gain 1 Strength this turn.  
*Upgraded:* Gains retain  
*Simulator:* nothing.  
+1 Strength this turn per card played. Unread. `ON_PLAY_TEMP`

**Particle Wall** · 0 energy + 2 Stars · Skill · Uncommon · ◐ partly read  
Gain 9 Block. Return this card to your Hand.  
*Upgraded:* Gain 12 Block. Return this card to your Hand.  
*Simulator:* Block 9, 2 Stars to play.  
Block read; returning to the hand (replayable for Stars) isn't. `RETURN_TO_HAND`

**Plot** · 1 energy · Skill · Uncommon · multiplayer · ⚠️ misread  
Next turn, ALL players draw 2 cards.  
*Upgraded:* Next turn, ALL players draw 3 cards.  
*Simulator:* draw 2.  
Multiplayer only; read as drawing 2 now. `MP` `DRAW_NEXT`

**Prophesize** · 2 energy · Skill · Uncommon · ✅ read  
Draw 6 cards.  
*Upgraded:* Draw 9 cards.  
*Simulator:* draw 6.  

**Quasar** · 0 energy + 2 Stars · Skill · Uncommon · ○ not read  
Choose 1 of 3 random Colorless cards to add into your Hand.  
*Upgraded:* Choose 1 of 3 random Upgraded Colorless cards to add into your Hand.  
*Simulator:* 2 Stars to play.  
Choose 1 of 3 Colorless cards. Unread. `RANDOM_GEN`

**Reflect** · 1 energy + 3 Stars · Skill · Uncommon · ◐ partly read  
Gain 15 Block. Blocked attack damage is reflected to your attacker this turn.  
*Upgraded:* Gain 20 Block. Blocked attack damage is reflected to your attacker this turn.  
*Simulator:* Block 15, 3 Stars to play.  
Block read; reflecting blocked damage needs enemy attacks. `THORNS_TEMP`

**Resonance** · 1 energy + 2 Stars · Skill · Uncommon · ⚠️ misread  
Gain 1 Strength. ALL enemies lose 1 Strength.  
*Upgraded:* Gain 2 Strength. ALL enemies lose 1 Strength.  
*Simulator:* 2 Stars to play, all enemies.  
+1 Strength isn't read because the card targets all enemies. The enemies' −1 Strength is survival. Tag error: "Weak". `STR_SELF_ON_ATTACK` `ENEMY_STR`

**Royal Gamble** · 0 energy + 5 Stars · Skill · Uncommon · Exhaust · ⚠️ misread  
Gain 9 Star.  
*Upgraded:* Gains retain  
*Simulator:* 5 Stars to play.  
"Gain 9 Stars" reads as 0. The Stars check looks for the base value in the text and misses it. `STARS_VAR`

**Summon Forth** · 1 energy · Skill · Uncommon · ✅ read  
Put Sovereign Blade into your Hand from anywhere. Forge 8.  
*Upgraded:* Put Sovereign Blade into your Hand from anywhere. Forge 11.  
*Makes:* Sovereign Blade  
*Simulator:* Forge 8.  
Forge read; fetching the Blade from anywhere isn't.

**Terraforming** · 1 energy · Skill · Uncommon · ✅ read  
Gain 7 Vigor.  
*Upgraded:* Gain 10 Vigor.  
*Simulator:* Vigor 7.  
7 Vigor.

**Big Bang** · 0 energy · Skill · Rare · Exhaust · ✅ read  
Draw 1 card. Gain 1 Energy. Gain 1 Star. Forge 5.  
*Upgraded:* Gains innate  
*Simulator:* draw 1, energy 1, Stars 1, Forge 5.  

**Bundle of Joy** · 1 energy · Skill · Rare · Exhaust · ○ not read  
Add 3 random Colorless cards into your Hand.  
*Upgraded:* Add 4 random Colorless cards into your Hand.  
*Simulator:* nothing.  
3 random Colorless cards. Unread. `RANDOM_GEN`

**Decisions, Decisions** · 0 energy + 6 Stars · Skill · Rare · Exhaust · ◐ partly read  
Draw 3 cards. Choose a Skill in your Hand and play it 3 times.  
*Upgraded:* Draw 5 cards. Choose a Skill in your Hand and play it 3 times.  
*Simulator:* draw 3, 6 Stars to play.  
Draw read; playing a chosen hand Skill 3 times isn't. `CHOOSE_REPLAY`

**Foregone Conclusion** · 1 energy · Skill · Rare · ○ not read  
Next turn, put 2 cards from your Draw Pile into your Hand.  
*Upgraded:* Next turn, put 3 cards from your Draw Pile into your Hand.  
*Simulator:* nothing.  
Next turn, 2 chosen Draw Pile cards go to the hand. Unread (works as +2 draw next turn). `TUTOR_NEXT`

**GUARDS!!!** · 2 energy · Skill · Rare · Exhaust · ○ not read  
Transform any number of cards in your Hand into Minion Sacrifice.  
*Upgraded:* Transform any number of cards in your Hand into Minion Sacrifice+.  
*Makes:* Minion Sacrifice  
*Simulator:* nothing.  
Turns any number of hand cards into Minion Sacrifice. Unread. `TRANSFORM`

**I Am Invincible** · 1 energy · Skill · Rare · ◐ partly read  
Gain 10 Block. At the end of your turn, if this is on top of your Draw Pile, play it.  
*Upgraded:* Gain 13 Block. At the end of your turn, if this is on top of your Draw Pile, play it.  
*Simulator:* Block 10.  
Block read; playing itself from the top of the Draw Pile at end of turn isn't. `AUTOPLAY_TOP`

**The Smith** · 1 energy + 4 Stars · Skill · Rare · ✅ read  
Forge 30.  
*Upgraded:* Forge 40.  
*Simulator:* Forge 30, 4 Stars to play.  
Forge 30 for 1 energy and 4 Stars.

**Tutor** · 1 energy · Skill · Rare · multiplayer · ○ not read  
Another player chooses a card in their Draw Pile to add into their Hand.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Multiplayer only. `MP`

#### Regent: Powers

**Black Hole** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you spend or gain 1 Star, deal 3 damage to ALL enemies.  
*Upgraded:* Whenever you spend or gain 1 Star, deal 4 damage to ALL enemies.  
*Simulator:* power: When played: gain 1 Stars, deal 3 damage to all.  
Misread as +1 Star and 3 area damage on play. It deals 3 to all per Star spent or gained. `TRIGGER_FALLBACK` `ON_STARS`

**Child of the Stars** · 1 energy · Power · Uncommon · ○ not read  
Whenever you spend 1 Star, gain 2 Block for each 1 Star spent.  
*Upgraded:* Whenever you spend 1 Star, gain 3 Block for each 1 Star spent.  
*Simulator:* nothing.  
2 Block per Star spent. Unread. Star spending is tracked (`spend(Store::Stars)`), so it is cheap. `ON_STARS`

**Furnace** · 1 energy · Power · Uncommon · ✅ read  
At the start of your turn, Forge 5.  
*Upgraded:* At the start of your turn, Forge 7.  
*Simulator:* power: Each turn: Forge 5.  

**Orbit** · 2 energy · Power · Uncommon · ⚠️ misread  
Every 4 Energy you spend, gain 1 Energy.  
*Upgraded:* Costs 1  
*Simulator:* power: When played: gain 1 Energy.  
Misread as +1 energy on play. It is +1 energy per 4 energy spent. `TRIGGER_FALLBACK` `COUNTER_ENERGY`

**Pale Blue Dot** · 1 energy · Power · Uncommon · ○ not read  
If you play 5 or more cards in a turn, draw 1 card at the start of your next turn.  
*Upgraded:* If you play 5 or more cards in a turn, draw 2 cards at the start of your next turn.  
*Simulator:* nothing.  
Draws next turn if you played 5+ cards. `played_this_turn` exists. `COUNTER_PLAYS`

**Parry** · 1 energy · Power · Uncommon · ⚠️ misread  
Sovereign Blade now gains 10 Block.  
*Upgraded:* Sovereign Blade now gains 14 Block.  
*Makes:* Sovereign Blade  
*Simulator:* power: When played: gain 10 Block.  
Misread as 10 Block on play. It is 10 Block whenever Sovereign Blade is played. `TRIGGER_FALLBACK` `ON_PLAY_NAMED`

**Pillar of Creation** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you create a card, gain 2 Block.  
*Upgraded:* Whenever you create a card, gain 3 Block.  
*Simulator:* power: When played: gain 2 Block.  
Misread as 2 Block on play. It is 2 Block per card created. `TRIGGER_FALLBACK` `ON_MAKE`

**Spectrum Shift** · 2 energy · Power · Uncommon · ○ not read  
At the start of your turn, add 1 random Colorless card into your Hand.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
A random Colorless card each turn. Unread. `RANDOM_GEN`

**Arsenal** · 1 energy · Power · Rare · ⚠️ misread  
Whenever you create a card, gain 1 Strength.  
*Upgraded:* Gains innate  
*Simulator:* power: When played: gain 1 Strength.  
Misread as +1 Strength on play. It is +1 Strength per card created. `TRIGGER_FALLBACK` `ON_MAKE`

**Genesis** · 2 energy · Power · Rare · ✅ read  
At the start of your turn, gain 2 Star.  
*Upgraded:* At the start of your turn, gain 3 Star.  
*Simulator:* power: Each turn: gain 2 Stars.  

**Hammer Time** · 2 energy · Power · Rare · multiplayer · ○ not read  
Whenever you Forge, all allies Forge as well.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Monarch's Gaze** · 2 energy · Power · Rare · ○ not read  
Whenever you attack an enemy, it loses 1 Strength this turn.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Each attack gives the enemy −1 Strength this turn. Survival. `ENEMY_STR`

**Neutron Aegis** · 1 energy + 5 Stars · Power · Rare · ✅ read  
Gain 8 Plating.  
*Upgraded:* Gain 11 Plating.  
*Simulator:* 5 Stars to play, power: When played: gain 8 Plating.  
8 Plating for 5 Stars.

**Royalties** · 1 energy · Power · Rare · ○ not read  
At the end of combat, gain 30 Gold.  
*Upgraded:* At the end of combat, gain 40 Gold.  
*Simulator:* nothing.  
Gold at end of combat. `OUT_OF_COMBAT`

**Seeking Edge** · 1 energy · Power · Rare · ✅ read  
Forge 7. Sovereign Blade now deals damage to ALL enemies.  
*Upgraded:* Forge 11. Sovereign Blade now deals damage to ALL enemies.  
*Simulator:* power: When played: Forge 7.  
Forge 7; the Blade hitting all enemies only matters against groups.

**Sword Sage** · 2 energy · Power · Rare · ✅ read  
Sovereign Blade gains Replay 1.  
*Upgraded:* Costs 1  
*Makes:* Sovereign Blade  
*Simulator:* gives Replay.  
The Blade gains Replay 1 (read through `gives_replay`).

**Tyranny** · 1 energy · Power · Rare · Exhaust · ◐ partly read  
At the start of your turn, draw 1 card and Exhaust 1 card from your Hand.  
*Upgraded:* Gains innate  
*Simulator:* power: Each turn: draw 1.  
Draw 1 each turn read; exhausting a hand card each turn isn't. `EXH_CHOOSE`

**Void Form** · 3 energy · Power · Rare · Ethereal · ○ not read  
End your turn. The first 2 cards you play each turn are free to play.  
*Upgraded:* Loses ethereal  
*Simulator:* nothing.  
Ends your turn when played; the first 2 cards each turn are free. Unread. `FREE_FIRST` `END_TURN`

**The Sealed Throne** · 1 energy + 3 Stars · Power · Ancient · ✅ read  
Whenever you play a card, gain 1 Star.  
*Upgraded:* Costs 0  
*Simulator:* 3 Stars to play, power: Whenever you play a card: gain 1 Stars.  
Ancient. +1 Star per card played.

### The Fool

#### The Fool: Attacks

**Foolish Shot** · 1 energy · Attack · Basic · ✅ read  
Shoot an Arrow to deal 15 damage.  
*Upgraded:* Shoot an Arrow to deal 20 damage.  
*Simulator:* damage 15, shoots.  
Starter shot (15). Needs an Arrow; Song Bow starts combat with 6 Aim, spent on the first shot.

**Strike** · 1 energy · Attack · Basic · ✅ read  
Deal 6 damage.  
*Upgraded:* Deal 9 damage.  
*Simulator:* damage 6.  

**Arrow Stab** · 1 energy · Attack · Common · ◐ partly read  
Deal 9 damage. If you have an Arrow, put a card that can Shoot from your Discard Pile into your Hand.  
*Upgraded:* Deal 12 damage. If you have an Arrow, put a card that can Shoot from your Discard Pile into your Hand.  
*Simulator:* damage 9.  
Damage read; fetching a Shoot card from the discard while you hold an Arrow isn't. `IMPALE_COND` `PILE_MOVE`

**Bow Smack** · 1 energy · Attack · Common · ⚠️ misread  
Deal 8 damage. If the enemy is Impaled, gain 1 Energy.  
*Upgraded:* Deal 11 damage. If the enemy is Impaled, gain 1 Energy.  
*Simulator:* damage 8, energy 1.  
+1 energy read as always; it needs the enemy Impaled (`stuck > 0`, which is tracked). `IMPALE_COND`

**Heavy Shot** · 2 energy · Attack · Common · ⚠️ misread  
Shoot an Arrow to deal 20 damage. Aim affects this card 2 times.  
*Upgraded:* Shoot an Arrow to deal 25 damage. Aim affects this card 2 times.  
*Simulator:* damage 20, shoots.  
Shoot read, but "Aim affects this card 2 times" isn't: `aim_multiplier` looks for an `aimmultiplier` var the card lacks. `AIM_MULT`

**Ki Hop** · 1 energy · Attack · Common · ⚠️ misread  
Deal 7 damage. If the enemy is Impaled, draw 2 cards.  
*Upgraded:* Deal 8 damage. If the enemy is Impaled, draw 3 cards.  
*Simulator:* damage 7, draw 2.  
Draw 2 read as always; it needs the enemy Impaled. As a "Ki" card it presumably grants a Ki Melody (not stated in the data). `IMPALE_COND` `MELODY`

**Ki Whip** · 1 energy · Attack · Common · ◐ partly read  
Deal 5 damage. Next turn, gain 2 Energy.  
*Upgraded:* Deal 6 damage. Next turn, gain 3 Energy.  
*Simulator:* damage 5, energy next turn 2.  
Damage and next-turn energy read. Ki card. `MELODY`

**Mourning Blow** · 0 energy · Attack · Common · ◐ partly read  
Deal 10 damage. Transform this into Sadness and return it to your Hand.  
*Upgraded:* Deal 15 damage. Transform this into Sadness and return it to your Hand.  
*Makes:* Sadness  
*Simulator:* damage 10.  
10 for 0 read; turning into Sadness and returning to the hand isn't. Sadness is a Retain, Unplayable Status. `TRANSFORM_SELF`

**Quick Shot** · 0 energy · Attack · Common · ✅ read  
Shoot an Arrow to deal 10 damage.  
*Upgraded:* Shoot an Arrow to deal 13 damage.  
*Simulator:* damage 10, shoots.  

**Rain Shot** · 1 energy · Attack · Common · ✅ read  
Shoot an Arrow to deal 12 damage. Damage ALL other enemies equal to the damage dealt.  
*Upgraded:* Shoot an Arrow to deal 15 damage. Damage ALL other enemies equal to the damage dealt.  
*Simulator:* damage 12, shoots.  
Splash to other enemies matters only against groups.

**Sorrowful Strike** · 1 energy · Attack · Common · ✅ read  
Deal 13 damage. Add 2 Sadness into your Hand.  
*Upgraded:* Deal 17 damage. Add 2 Sadness into your Hand.  
*Makes:* Sadness  
*Simulator:* damage 13, n=2, pile=Hand }].  
2 Sadness into the hand.

**Wah Hammer** · 1 energy · Attack · Common · ⚠️ misread  
Deal 9 damage. Deals double damage to Impaled enemies.  
*Upgraded:* Deal 12 damage. Deals double damage to Impaled enemies.  
*Simulator:* damage 9.  
9 read; double damage on an Impaled enemy isn't. Gate on `stuck > 0`. Wah card. `IMPALE_COND` `MELODY`

**Wah Sweep** · 1 energy · Attack · Common · ◐ partly read  
Deal 8 damage to ALL enemies.  
*Upgraded:* Deal 11 damage to ALL enemies.  
*Simulator:* damage 8, all enemies.  
Area damage read. Wah card. `MELODY`

**Wah Touch** · 1 energy · Attack · Common · ◐ partly read  
Deal 6 damage. Apply 1 Vulnerable.  
*Upgraded:* Deal 8 damage. Apply 2 Vulnerable.  
*Simulator:* damage 6, Vulnerable 1.  
Read. Wah card. `MELODY`

**Anguish** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 6 damage. Deals 6 additional damage for each Sadness you have created this combat.  
*Upgraded:* Deal 6 damage. Deals 7 additional damage for each Sadness you have created this combat.  
*Makes:* Sadness  
*Simulator:* damage 6.  
6 read; +6 per Sadness created this combat isn't. `COUNTER_MADE`

**Blossom Shot** · 2 energy · Attack · Uncommon · ◐ partly read  
Shoot an Arrow to deal 11 damage and gain Stretch equal to the damage dealt.  
*Upgraded:* Shoot an Arrow to deal 14 damage and gain Stretch equal to the damage dealt.  
*Simulator:* damage 11, shoots.  
Shoot read; Stretch equal to damage dealt isn't. `STRETCH_FROM_DMG`

**Dislike You** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 8 damage. The enemy takes 50% additional damage from Shots this turn.  
*Upgraded:* Deal 9 damage. The enemy takes 75% additional damage from Shots this turn.  
*Simulator:* damage 8.  
Damage read; +50% Shot damage this turn isn't. `SHOT_AMP`

**Epic Retrieval** · 2 energy · Attack · Uncommon · ⚠️ misread  
Deal 15 damage. If this Dislodges an Arrow, gain 3 Energy.  
*Upgraded:* Deal 20 damage. If this Dislodges an Arrow, gain 4 Energy.  
*Simulator:* damage 15, energy 3, dislodges.  
+3 energy read as always; it needs a stuck Arrow to dislodge. `DISLODGE_COND`

**Hobbling Shot** · 1 energy · Attack · Uncommon · ⚠️ misread  
Shoot an Arrow to deal 13 damage and apply 1 Weak and Vulnerable.  
*Upgraded:* Shoot an Arrow to deal 14 damage and apply 2 Weak and Vulnerable.  
*Simulator:* damage 13, shoots.  
Shoot read; 1 Weak and 1 Vulnerable sit under `power` and are dropped. `VAR_POWER`

**Punch of Joy** · 2 energy · Attack · Uncommon · ◐ partly read  
Deal 13 damage. Transform 1 random Sadness in your Hand into Joy.  
*Upgraded:* Deal 17 damage. Transform 1 random Sadness in your Hand into Joy.  
*Makes:* Joy, Sadness  
*Simulator:* damage 13.  
Damage read; turning a hand Sadness into Joy (+1 energy, draw 1) isn't. `TRANSFORM`

**Rapid Shot** · 1 energy · Attack · Uncommon · ✅ read  
Shoot an Arrow to deal 3 damage 3 times.  
*Upgraded:* Shoot an Arrow to deal 3 damage 4 times.  
*Simulator:* damage 3×3, shoots.  

**Rhythm Strike** · 0 energy · Attack · Uncommon · ◐ partly read  
Deal 4 damage. Hits an additional time for each Melody you have.  
*Upgraded:* Deal 5 damage. Hits an additional time for each Melody you have.  
*Simulator:* damage 4.  
4 read; +1 hit per Melody isn't. `MELODY_COUNT`

**Steady Cadence** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 10 damage. Draw 1 card for each Melody you have.  
*Upgraded:* Deal 13 damage. Draw 1 card for each Melody you have.  
*Simulator:* damage 10.  
Damage read; draw 1 per Melody has no count. `MELODY_COUNT`

**Timber** · 3 energy · Attack · Uncommon · ◐ partly read  
Deal 20 damage. If this kills an enemy, deal 30 damage to ALL enemies.  
*Upgraded:* Deal 25 damage. If this kills an enemy, deal 35 damage to ALL enemies.  
*Simulator:* damage 20.  
20 read; 30 to all on a kill isn't. `FATAL`

**Trick Shot** · 1 energy · Attack · Uncommon · ◐ partly read  
Shoot an Arrow to deal 12 damage and return this card to your Hand.  
*Upgraded:* Shoot an Arrow to deal 15 damage and return this card to your Hand.  
*Simulator:* damage 12, shoots.  
Shoot read; returning to the hand isn't. Each replay needs another Arrow. `RETURN_TO_HAND`

**Wah Blast** · 2 energy · Attack · Uncommon · ◐ partly read  
Deal 14 damage to ALL enemies. If you have a Cho Melody or Ki Melody, hits twice.  
*Upgraded:* Deal 18 damage to ALL enemies. If you have a Cho Melody or Ki Melody, hits twice.  
*Simulator:* damage 14, all enemies.  
Area 14 read; the second hit with a Cho or Ki Melody isn't. `MELODY_COND`

**Wah Cho** · 0 energy · Attack · Uncommon · ◐ partly read  
Gain 3 Block. Deal 3 damage.  
*Upgraded:* Gain 5 Block. Deal 5 damage.  
*Simulator:* damage 3, Block 3.  
Read. Wah and Cho card. `MELODY`

**Wah Ki** · 1 energy · Attack · Uncommon · ✅ read  
Deal 2 damage 2 times. Next turn, gain 1 Energy and draw 1 card.  
*Upgraded:* Deal 4 damage 2 times. Next turn, gain 1 Energy and draw 1 card.  
*Simulator:* damage 2×2, draw 1, energy next turn 1.  
Read. Wah and Ki card. `MELODY`

**Coda** · 6 energy · Attack · Rare · ◐ partly read  
Deal 40 damage. Costs 2 less 1 Energy for each Melody you have.  
*Upgraded:* Deal 55 damage. Costs 2 less 1 Energy for each Melody you have.  
*Simulator:* damage 40.  
40 read at cost 6; −2 cost per Melody held isn't. Needs Melody state. `MELODY_COUNT`

**Melodic Shot** · 1 energy · Attack · Rare · ◐ partly read  
Shoot an Arrow to deal 12 damage. Hits an additional time for each Melody you have.  
*Upgraded:* Shoot an Arrow to deal 13 damage. Hits an additional time for each Melody you have.  
*Simulator:* damage 12, shoots.  
Shoot read; +1 hit per Melody isn't. `MELODY_COUNT`

**Nail** · 2 energy · Attack · Rare · ⚠️ misread  
If the enemy is Impaled, Deal 20 damage and apply 2 Weak and Vulnerable.  
*Upgraded:* If the enemy is Impaled, Deal 25 damage and apply 3 Weak and Vulnerable.  
*Simulator:* damage 20.  
20 damage read as always; needs Impaled. Weak and Vulnerable sit under `power` and are dropped. `IMPALE_COND` `VAR_POWER`

**Repeat Shot** · X energy · Attack · Rare · ◐ partly read  
Shoot an Arrow to deal 15 damage X times.  
*Upgraded:* Shoot an Arrow to deal 20 damage X times.  
*Simulator:* damage 15, X-cost, shoots.  
X shots read as X hits spending one Arrow; each shot should spend an Arrow. `ARROW_X`

**Sky Shot** · 1 energy · Attack · Rare · Exhaust, Innate · ✅ read  
Shoot an Arrow to deal 30 damage.  
*Upgraded:* Shoot an Arrow to deal 40 damage.  
*Simulator:* damage 30, shoots.  

**Wah Cho Ki** · 1 energy · Attack · Rare · ◐ partly read  
Gain 9 Block. Deal 9 damage. Draw 1 card.  
*Upgraded:* Gain 12 Block. Deal 12 damage. Draw 2 cards.  
*Simulator:* damage 9, Block 9, draw 1.  
Read. Counts as Wah, Cho and Ki. `MELODY`

**Wah Symphony** · 2 energy · Attack · Rare · ◐ partly read  
Deal 6 damage to ALL enemies. Deals 4 additional damage for ALL your cards containing “Wah”, “Cho”, or “Ki”.  
*Upgraded:* Deal 6 damage to ALL enemies. Deals 6 additional damage for ALL your cards containing “Wah”, “Cho”, or “Ki”.  
*Simulator:* damage 6, all enemies.  
6 to all read; +4 per Wah/Cho/Ki card in the deck isn't. `DECK_COUNT`

**Divine Shot** · 0 energy · Attack · Ancient · Retain · ✅ read  
Shoot an Arrow to deal 40 damage.  
*Upgraded:* Shoot an Arrow to deal 50 damage.  
*Simulator:* damage 40, shoots.  
Ancient. Shoots for 40. Retain.

#### The Fool: Skills

**Defend** · 1 energy · Skill · Basic · ✅ read  
Gain 5 Block.  
*Upgraded:* Gain 8 Block.  
*Simulator:* Block 5.  

**Cho Barbs** · 1 energy · Skill · Common · ◐ partly read  
Gain 7 Block. Gain 3 Thorns this turn.  
*Upgraded:* Gain 9 Block. Gain 4 Thorns this turn.  
*Simulator:* Block 7.  
Block read; 3 Thorns this turn isn't. Cho card. `THORNS_TEMP` `MELODY`

**Cho Firm** · 1 energy · Skill · Common · ◐ partly read  
Gain 6 Block. If you have an Arrow, gain Block an additional time.  
*Upgraded:* Gain 8 Block. If you have an Arrow, gain Block an additional time.  
*Simulator:* Block 6.  
6 Block read; a second 6 with an Arrow ready isn't (`arrows > 0`). `ARROW_COND` `MELODY`

**Cho Ready** · 1 energy · Skill · Common · ✅ read  
Gain 8 Block. Gain 3 Aim.  
*Upgraded:* Gain 11 Block. Gain 5 Aim.  
*Simulator:* Block 8, Aim 3.  
Block and Aim read. Cho card. `MELODY`

**Deep Stretch** · 1 energy · Skill · Common · ✅ read  
Gain 7 Stretch. Draw 1 card.  
*Upgraded:* Gain 10 Stretch. Draw 1 card.  
*Simulator:* draw 1, Stretch 7.  

**Jest** · 2 energy · Skill · Common · ✅ read  
Gain 14 Block. Add 1 Sadness into your Hand.  
*Upgraded:* Gain 18 Block. Add 1 Sadness into your Hand.  
*Makes:* Sadness  
*Simulator:* Block 14, n=1, pile=Hand }].  

**Ki Chant** · 1 energy · Skill · Common · ○ not read  
Look at the top 3 cards of your Draw Pile. Choose 1 to add to your Hand and discard the rest.  
*Upgraded:* Look at the top 4 cards of your Draw Pile. Choose 1 to add to your Hand and discard the rest.  
*Simulator:* nothing.  
Look at 3, take 1, discard the rest. Unread (worth about +1 draw). `DIG`

**Nock** · 0 energy · Skill · Common · ✅ read  
Gain 5 Aim.  
*Upgraded:* Gain 7 Aim.  
*Simulator:* Aim 5.  

**Recall** · 1 energy · Skill · Common · ○ not read  
If the enemy is Impaled, Dislodge and put a card from your Discard Pile on top of your Draw Pile.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
With the enemy Impaled: dislodge and put a discard card on top. Unread. `IMPALE_COND` `PILE_MOVE`

**Blot out the Sky** · 1 energy · Skill · Uncommon · Exhaust · ○ not read  
Add a random card that can Shoot into your Hand. It's free to play this turn.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
A random free Shoot card. Unread. `RANDOM_GEN`

**Brace** · X energy · Skill · Uncommon · ✅ read  
Gain 8 Aim X times.  
*Upgraded:* Gain 11 Aim X times.  
*Simulator:* Aim 8, X-cost.  
X × 8 Aim.

**Cho Ki** · 1 energy · Skill · Uncommon · ◐ partly read  
Gain 8 Block. Regain any spent Stretch.  
*Upgraded:* Gain 11 Block. Regain any spent Stretch.  
*Simulator:* Block 8.  
Block read; regaining spent Stretch isn't. Stretch spending is tracked. `STRETCH_REGAIN`

**Cho Limbs** · 2 energy · Skill · Uncommon · ⚠️ misread  
Gain 5 Block twice. Costs 2 less 1 Energy if you have a Wah Melody or Ki Melody.  
*Upgraded:* Gain 7 Block twice. Costs 2 less 1 Energy if you have a Wah Melody or Ki Melody.  
*Simulator:* Block 5.  
"Gain 5 Block twice" read once, and −2 cost with a Wah or Ki Melody isn't read. `VAR_HITS` `MELODY_COND`

**Enrapture** · 1 energy · Skill · Uncommon · multiplayer · ○ not read  
Give another player 6 Vigor and 6 Stretch.  
*Upgraded:* Give another player 9 Vigor and 9 Stretch.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Growth** · 1 energy · Skill · Uncommon · ✅ read  
Exhaust ALL Sadness in your Hand. Gain 4 Stretch for each card Exhausted.  
*Upgraded:* Exhaust ALL Sadness in your Hand. Gain 6 Stretch for each card Exhausted.  
*Makes:* Sadness  
*Simulator:* exhausts HandNamed("sadness"), per card exhausted: [Stretch(4)].  
Exhausts hand Sadness for 4 Stretch each.

**Harmonic Shield** · 1 energy · Skill · Uncommon · ○ not read  
Gain 5 Block for each Melody you have.  
*Upgraded:* Gain 7 Block for each Melody you have.  
*Simulator:* nothing.  
5 Block per Melody. Unread. `MELODY_COUNT`

**Ki Rest** · 2 energy · Skill · Uncommon · ◐ partly read  
Next turn, gain 5 Energy. If you have a Wah Melody or Cho Melody, Retain your Hand this turn.  
*Upgraded:* Next turn, gain 6 Energy. If you have a Wah Melody or Cho Melody, Retain your Hand this turn.  
*Simulator:* energy next turn 5.  
5 energy next turn read; retaining the hand with a Wah or Cho Melody isn't. `MELODY_COND` `RETAIN_TEMP`

**Locked In** · 1 energy · Skill · Uncommon · ○ not read  
Put a card that can Shoot from your Draw Pile into your Hand.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Fetches a Shoot card from the Draw Pile. Unread (worth +1 targeted draw). `TUTOR`

**Rummage** · 1 energy · Skill · Uncommon · ✅ read  
Draw 3 cards. Add a Sadness into your Hand.  
*Upgraded:* Draw 4 cards. Add a Sadness into your Hand.  
*Makes:* Sadness  
*Simulator:* draw 3, n=1, pile=Hand }].  

**Uproot** · 1 energy · Skill · Uncommon · ⚠️ misread  
If the enemy is Impaled, Dislodge and apply 3 Vulnerable.  
*Upgraded:* If the enemy is Impaled, Dislodge and apply 4 Vulnerable.  
*Simulator:* Vulnerable 3.  
3 Vulnerable read as always; needs the enemy Impaled. The dislodge isn't read. `IMPALE_COND`

**Vitriol** · 0 energy · Skill · Uncommon · ✅ read  
Apply 2 Weak. Add a Sadness into your Hand.  
*Upgraded:* Apply 3 Weak. Add a Sadness into your Hand.  
*Makes:* Sadness  
*Simulator:* Weak 2, n=1, pile=Hand }].  

**Whistling Arrows** · 0 energy · Skill · Uncommon · ⚠️ misread  
Gain 3 Block. Whenever you Shoot an Arrow, return this to your Hand from the Discard Pile.  
*Upgraded:* Gain 4 Block. Whenever you Shoot an Arrow, return this to your Hand from the Discard Pile.  
*Simulator:* Block 3, shoots.  
Read as a shooting card because its text contains "Shoot an Arrow", so it spends an Arrow it doesn't use. It is 3 Block and returns to the hand whenever you shoot. `SHOOT_FLAG`

**Wrest** · 0 energy · Skill · Uncommon · ⚠️ misread  
If the enemy is Impaled, Dislodge and draw 2 cards.  
*Upgraded:* If the enemy is Impaled, Dislodge and draw 3 cards.  
*Simulator:* draw 2.  
Draw 2 read as always; needs the enemy Impaled. Dislodge isn't read. `IMPALE_COND`

**Yoga** · 1 energy · Skill · Uncommon · multiplayer · ✅ read  
ALL players gain 6 Stretch.  
*Upgraded:* ALL players gain 8 Stretch.  
*Simulator:* Stretch 6.  
Multiplayer only; your own Stretch is read. `MP`

**Bounce** · 1 energy · Skill · Rare · ◐ partly read  
Gain 10 Stretch. At the start of your next turn, return this to your Hand.  
*Upgraded:* Gain 13 Stretch. At the start of your next turn, return this to your Hand.  
*Simulator:* Stretch 10.  
Stretch read; returning to the hand next turn isn't. `RETURN_TO_HAND`

**Brighten Up** · 1 energy · Skill · Rare · Exhaust · ✅ read  
Add 2 Sadness to your Hand. Add 2 Joy to your Hand.  
*Upgraded:* Add 2 Sadness to your Hand. Add 3 Joy to your Hand.  
*Makes:* Joy, Sadness  
*Simulator:* n=2, pile=Hand }, Make { name: "joy", n=2, pile=Hand }].  
2 Sadness and 2 Joy.

**Cho Chorus** · 3 energy · Skill · Rare · ○ not read  
Gain 12 Block. Play ALL “Wah” and “Ki” cards in your Hand.  
*Upgraded:* Gain 15 Block. Play ALL “Wah” and “Ki” cards in your Hand.  
*Simulator:* Block 12.  
Block read; playing every Wah and Ki card in the hand for free isn't. `PLAY_HAND_NAMED`

**Encore** · 0 energy · Skill · Rare · ◐ partly read  
Gain 1 Energy. Draw 2 cards. You can only play 1 more card this turn.  
*Upgraded:* Gain 1 Energy. Draw 3 cards. You can only play 1 more card this turn.  
*Simulator:* draw 2, energy 1.  
Energy and draw read; "only 1 more card this turn" isn't, so it is overvalued. `PLAY_LIMIT`

**Ensemble** · 3 energy · Skill · Rare · Exhaust · multiplayer · ○ not read  
The next 4 cards played by other players are free to play.  
*Upgraded:* The next 5 cards played by other players are free to play.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Florescence** · 0 energy · Skill · Rare · Exhaust · ○ not read  
Exhaust ALL of your cards that Shoot and create The Shot.  
*Upgraded:* Exhaust ALL of your cards that Shoot and create The Shot+.  
*Makes:* The Shot  
*Simulator:* nothing.  
Exhausts every Shoot card and creates The Shot. Unread. `TRANSFORM`

**Ki Verse** · 1 energy · Skill · Rare · ○ not read  
The next “Wah”, “Cho”, or “Ki” card you play this turn is played an additional time.  
*Upgraded:* The next 2 “Wah”, “Cho”, or “Ki” cards you play this turn are played an additional time.  
*Simulator:* nothing.  
The next Wah/Cho/Ki card is played twice. Unread; `next_extra` needs a name filter. `NEXT_EXTRA_NAMED`

**Quietus** · 1 energy · Skill · Rare · Exhaust · ○ not read  
This turn, your next Shot deals double damage but the Arrow is destroyed.  
*Upgraded:* This turn, your next Shot deals triple damage but the Arrow is destroyed.  
*Simulator:* nothing.  
The next Shot deals double, but the Arrow is destroyed. Unread. `SHOT_AMP`

**Self Deprecate** · 1 energy · Skill · Rare · multiplayer · ✅ read  
Add a Sadness to your Hand. Add 2 Joy to all allies' Hands.  
*Upgraded:* Add a Sadness to your Hand. Add 3 Joy to all allies' Hands.  
*Makes:* Joy, Sadness  
*Simulator:* n=1, pile=Hand }].  
Multiplayer only; your Sadness is read. `MP`

**Show Off** · 1 energy · Skill · Rare · Exhaust · ✅ read  
Gain 5 Aim. If a non-minion enemy dies this turn, permanently increase this card's Aim by 2.  
*Upgraded:* Gain 5 Aim. If a non-minion enemy dies this turn, permanently increase this card's Aim by 3.  
*Simulator:* Aim 5.  
5 Aim; the permanent growth on a kill is run value.

#### The Fool: Powers

**All Is Well** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you create a Sadness, gain 4 Block.  
*Upgraded:* Whenever you create a Sadness, gain 5 Block.  
*Makes:* Sadness  
*Simulator:* power: When played: gain 4 Block.  
Misread as 4 Block on play. It is 4 Block per Sadness created. `TRIGGER_FALLBACK` `ON_MAKE_NAMED`

**Archery** · 1 energy · Power · Uncommon · ✅ read  
At the start of your turn, gain 4 Aim.  
*Upgraded:* At the start of your turn, gain 5 Aim.  
*Simulator:* power: Each turn: gain 4 Aim.  
4 Aim each turn.

**Compost** · 0 energy · Power · Uncommon · multiplayer · ○ not read  
Whenever an enemy dies, ALL players gain 1 Energy.  
*Upgraded:* Whenever an enemy dies, ALL players gain 2 Energy.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Hidden Roots** · 1 energy · Power · Uncommon · ✅ read  
At the start of your turn, gain 3 Stretch.  
*Upgraded:* At the start of your turn, gain 4 Stretch.  
*Simulator:* power: Each turn: gain 3 Stretch.  

**Motif** · 2 energy · Power · Uncommon · ⚠️ misread  
Whenever you gain a Melody, draw 1 card.  
*Upgraded:* Costs 1  
*Simulator:* power: When played: draw 1.  
Misread as draw 1 on play. It draws per Melody gained. `TRIGGER_FALLBACK` `MELODY_TRIGGER`

**Serrated Arrows** · 1 energy · Power · Uncommon · ○ not read  
Impaled enemies take 25% more damage from your Attacks.  
*Upgraded:* Impaled enemies take 50% more damage from your Attacks.  
*Simulator:* nothing.  
+25% Attack damage on Impaled enemies. Unread; gate on `stuck > 0`. `IMPALE_AMP`

**Song Path** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you gain a Cho Melody, gain 5 Block.  
*Upgraded:* Whenever you gain a Cho Melody, gain 7 Block.  
*Simulator:* power: When played: gain 5 Block.  
Misread as 5 Block on play. It is 5 Block per Cho Melody gained. `TRIGGER_FALLBACK` `MELODY_TRIGGER`

**Tune** · 2 energy · Power · Uncommon · ○ not read  
At the end of your turn, if you have a Ki Melody, the next card you play costs 0 Energy.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
With a Ki Melody, the next card(s) cost 0 (the card says 1, the power text says 2). Unread. `MELODY_TRIGGER`

**Violent Chord** · 1 energy · Power · Uncommon · ⚠️ misread  
Whenever you gain a Wah Melody, deal 9 damage to ALL enemies.  
*Upgraded:* Whenever you gain a Wah Melody, deal 12 damage to ALL enemies.  
*Simulator:* power: When played: deal 9 damage to all.  
Misread as 9 area damage on play. It is 9 to all per Wah Melody gained. `TRIGGER_FALLBACK` `MELODY_TRIGGER`

**Covering Fire** · 1 energy · Power · Rare · multiplayer · ○ not read  
Whenever you Shoot, allies gain 8 Block.  
*Upgraded:* Whenever you Shoot, allies gain 11 Block.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Ecstasy** · 2 energy · Power · Rare · ○ not read  
At the start of your turn, Transform 1 random Sadness in your Hand into Joy.  
*Upgraded:* Costs 1  
*Makes:* Joy, Sadness  
*Simulator:* nothing.  
Turns a hand Sadness into Joy each turn. Unread. `TRANSFORM`

**Ennui** · 1 energy · Power · Rare · ✅ read  
At the start of your turn, add 1 Sadness to your Hand and draw 2 additional cards.  
*Upgraded:* At the start of your turn, add 1 Sadness to your Hand and draw 3 additional cards.  
*Makes:* Sadness  
*Simulator:* n=1, pile=Hand }], power: Each turn: add 1 sadness to your Hand, draw 2.  
A Sadness and +2 draw each turn.

**Explosive Arrows** · 1 energy · Power · Rare · ○ not read  
Whenever you Impale an enemy, deal 6 damage to ALL enemies.  
*Upgraded:* Whenever you Impale an enemy, deal 8 damage to ALL enemies.  
*Simulator:* nothing.  
6 to all whenever you Impale (every shot). Unread; hook it onto `arrows_after`. `ON_IMPALE`

**Honing** · 3 energy · Power · Rare · ○ not read  
Arrows deal 50% additional damage.  
*Upgraded:* Costs 2  
*Simulator:* nothing.  
Arrows deal +50%. Unread; a multiplier on shots. `SHOT_AMP`

**Lullaby** · 0 energy · Power · Rare · ○ not read  
When you gain every Melody, gain 20 Block.  
*Upgraded:* Gains innate  
*Simulator:* nothing.  
20 Block on completing every Melody. Unread. `MELODY_TRIGGER`

**Mythic Form** · 3 energy · Power · Rare · ○ not read  
At the start of your turn, add a random Tarot card into your Hand.  
*Upgraded:* At the start of your turn, add a random Upgraded Tarot card into your Hand.  
*Makes:* Death, Judgement, Justice, Strength, Temperance, … 17 more  
*Simulator:* nothing.  
A random Tarot card each turn (22 Tarot tokens). Unread. `RANDOM_GEN`

**Quiver** · 1 energy · Power · Rare · ✅ read  
Gain 1 additional Arrow.  
*Upgraded:* Gain 2 additional Arrows.  
*Simulator:* power: When played: gain 1 Arrows.  
+1 Arrow.

**Simpleton** · 2 energy · Power · Rare · ○ not read  
At the end of your turn, play the least expensive card in your Hand.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Plays the cheapest hand card at end of turn. Unread. `AUTOPLAY`

**Yah!** · 1 energy · Power · Rare · ○ not read  
The first time you Dislodge an Arrow each turn, gain 1 Energy.  
*Upgraded:* The first time you Dislodge an Arrow each turn, gain 2 Energy.  
*Simulator:* nothing.  
+1 energy on the first dislodge each turn. Unread; dislodges are tracked. `ON_DISLODGE`

**Lost Lute** · 3 energy · Power · Ancient · ○ not read  
Whenever you gain a Melody, play the top card of your Draw Pile.  
*Upgraded:* Costs 2  
*Simulator:* nothing.  
Ancient. Plays the top card(s) of the Draw Pile per Melody gained (the power text says 2). Unread. `MELODY_TRIGGER` `AUTOPLAY`

### Colorless

#### Colorless: Attacks

**Dramatic Entrance** · 0 energy · Attack · Uncommon · Exhaust, Innate · ✅ read  
Deal 11 damage to ALL enemies.  
*Upgraded:* Deal 15 damage to ALL enemies.  
*Simulator:* damage 11, all enemies.  

**Fisticuffs** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 7 damage. Gain Block equal to damage dealt.  
*Upgraded:* Deal 9 damage. Gain Block equal to damage dealt.  
*Simulator:* damage 7.  
Damage read; Block equal to damage dealt isn't. `BLOCK_FROM_DMG`

**Flash of Steel** · 0 energy · Attack · Uncommon · ✅ read  
Deal 5 damage. Draw 1 card.  
*Upgraded:* Deal 8 damage. Draw 1 card.  
*Simulator:* damage 5, draw 1.  

**Gang Up** · 1 energy · Attack · Uncommon · multiplayer · ✅ read  
Deal 5 damage. Deals 5 additional damage for each time another player has attacked the enemy this turn.  
*Upgraded:* Deal 5 damage. Deals 7 additional damage for each time another player has attacked the enemy this turn.  
*Simulator:* damage 5.  
Multiplayer only. `MP`

**Mind Blast** · 1 energy · Attack · Uncommon · Innate · ○ not read  
Deal damage equal to the number of cards in your Draw Pile.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Damage equal to Draw Pile size. Read as 0; `draw.len()` exists. `COUNT_DRAW_PILE`

**Omnislice** · 0 energy · Attack · Uncommon · ✅ read  
Deal 8 damage. Damage ALL other enemies equal to the damage dealt.  
*Upgraded:* Deal 11 damage. Damage ALL other enemies equal to the damage dealt.  
*Simulator:* damage 8.  

**Salvo** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 10 damage. Retain your Hand this turn.  
*Upgraded:* Deal 14 damage. Retain your Hand this turn.  
*Simulator:* damage 10.  
Damage read; retaining the hand this turn isn't. `RETAIN_TEMP`

**Seeker Strike** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 9 damage. Choose 1 of 3 cards in your Draw Pile to add into your Hand.  
*Upgraded:* Deal 12 damage. Choose 1 of 3 cards in your Draw Pile to add into your Hand.  
*Simulator:* damage 9.  
Damage read; picking 1 of 3 Draw Pile cards isn't (worth a targeted draw). `TUTOR`

**Tag Team** · 2 energy · Attack · Uncommon · multiplayer · ✅ read  
Deal 11 damage. The next Attack another player plays on the enemy is played an extra time.  
*Upgraded:* Deal 15 damage. The next Attack another player plays on the enemy is played an extra time.  
*Simulator:* damage 11.  
Multiplayer only. `MP`

**The Ball** · 1 energy · Attack · Uncommon · multiplayer · ✅ read  
Deal 10 damage. Increase this card's damage by 10 this combat and give it to a random ally.  
*Upgraded:* Deal 10 damage. Increase this card's damage by 15 this combat and give it to a random ally.  
*Simulator:* damage 10.  
Multiplayer only. `MP`

**Thrumming Hatchet** · 1 energy · Attack · Uncommon · ◐ partly read  
Deal 11 damage. At the start of your next turn, return this to your Hand.  
*Upgraded:* Deal 14 damage. At the start of your next turn, return this to your Hand.  
*Simulator:* damage 11.  
11 read; returning to the hand next turn isn't, so it is undercounted by about one play a turn. `RETURN_TO_HAND`

**Ultimate Strike** · 1 energy · Attack · Uncommon · ✅ read  
Deal 14 damage.  
*Upgraded:* Deal 20 damage.  
*Simulator:* damage 14.  
Counts as a Strike.

**Volley** · X energy · Attack · Uncommon · ✅ read  
Deal 10 damage to a random enemy X times.  
*Upgraded:* Deal 14 damage to a random enemy X times.  
*Simulator:* damage 10, X-cost.  

**Bolas** · 0 energy · Attack · Rare · ◐ partly read  
Deal 3 damage. At the start of your next turn, return this to your Hand.  
*Upgraded:* Deal 4 damage. At the start of your next turn, return this to your Hand.  
*Simulator:* damage 3.  
3 read; returning to the hand next turn isn't. `RETURN_TO_HAND`

**Gold Axe** · 1 energy · Attack · Rare · ○ not read  
Deal damage equal to the number of cards played this combat.  
*Upgraded:* Gains retain  
*Simulator:* nothing.  
Damage equal to cards played this combat. Read as 0; the simulator counts plays. `COUNTER_PLAYS`

**Hand of Greed** · 2 energy · Attack · Rare · ✅ read  
Deal 20 damage. If Fatal, gain 20 Gold.  
*Upgraded:* Deal 25 damage. If Fatal, gain 25 Gold.  
*Simulator:* damage 20.  
Gold on a kill is run value. `FATAL`

**Jackpot** · 3 energy · Attack · Rare · ◐ partly read  
Deal 25 damage. Add 3 random 0 Energy cards into your Hand.  
*Upgraded:* Deal 30 damage. Add 3 random Upgraded 0 Energy cards into your Hand.  
*Simulator:* damage 25.  
25 read; 3 random 0-cost cards aren't. `RANDOM_GEN`

**Knockdown** · 3 energy · Attack · Rare · multiplayer · ✅ read  
Deal 10 damage. The enemy takes double damage from other players this turn.  
*Upgraded:* Deal 14 damage. The enemy takes triple damage from other players this turn.  
*Simulator:* damage 10.  
Multiplayer only. `MP`

**My Shinies** · 1 energy · Attack · Rare · ◐ partly read  
Deal 1 damage for every 20 Gold you have.  
*Upgraded:* Deal 1 damage for every 10 Gold you have.  
*Simulator:* damage 1.  
1 per 20 Gold reads as 1. Gold is run state (`RunState.gold`), so pass it in. `GOLD`

**Rend** · 1 energy · Attack · Rare · ◐ partly read  
Deal 10 damage. Deals 5 additional damage for each unique debuff on the enemy.  
*Upgraded:* Deal 10 damage. Deals 8 additional damage for each unique debuff on the enemy.  
*Simulator:* damage 10.  
10 read; +5 per unique enemy debuff isn't. Vulnerable and Poison are tracked; Weak and Doom aren't yet. `COUNT_DEBUFFS`

#### Colorless: Skills

**Believe in You** · 0 energy · Skill · Uncommon · multiplayer · ⚠️ misread  
Another player gains 2 Energy.  
*Upgraded:* Another player gains 3 Energy.  
*Simulator:* energy 2.  
Multiplayer only; the energy goes to another player but is credited to you. `MP`

**Catastrophe** · 2 energy · Skill · Uncommon · ○ not read  
Play 2 random cards from your Draw Pile.  
*Upgraded:* Play 3 random cards from your Draw Pile.  
*Simulator:* nothing.  
Plays 2 random Draw Pile cards. Unread. `AUTOPLAY`

**Coordinate** · 1 energy · Skill · Uncommon · multiplayer · ○ not read  
Give another player 5 Strength this turn.  
*Upgraded:* Give another player 8 Strength this turn.  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Dark Shackles** · 0 energy · Skill · Uncommon · Exhaust · ◐ partly read  
Enemy loses 9 Strength this turn.  
*Upgraded:* Enemy loses 15 Strength this turn.  
*Simulator:* nothing.  
−9 enemy Strength this turn is mitigation. Survival. `ENEMY_STR`

**Discovery** · 1 energy · Skill · Uncommon · Exhaust · ○ not read  
Choose 1 of 3 random cards to add into your Hand. It's free to play this turn.  
*Upgraded:* Loses exhaust  
*Simulator:* nothing.  
Choose 1 of 3 random cards, free this turn. Unread. `RANDOM_GEN`

**Equilibrium** · 2 energy · Skill · Uncommon · ◐ partly read  
Gain 13 Block. Retain your Hand this turn.  
*Upgraded:* Gain 16 Block. Retain your Hand this turn.  
*Simulator:* Block 13.  
Block read; retaining the hand isn't. `RETAIN_TEMP`

**Finesse** · 0 energy · Skill · Uncommon · ✅ read  
Gain 4 Block. Draw 1 card.  
*Upgraded:* Gain 7 Block. Draw 1 card.  
*Simulator:* Block 4, draw 1.  

**Huddle Up** · 1 energy · Skill · Uncommon · Exhaust · multiplayer · ✅ read  
ALL players draw 2 cards.  
*Upgraded:* ALL players draw 3 cards.  
*Simulator:* draw 2.  
Multiplayer only; your own draw is read. `MP`

**Impatience** · 0 energy · Skill · Uncommon · ◐ partly read  
If you have no Attacks in your Hand, draw 2 cards.  
*Upgraded:* If you have no Attacks in your Hand, draw 3 cards.  
*Simulator:* draw 2.  
Draw read as always; it needs no Attacks in hand. `DRAW_COND`

**Intercept** · 1 energy · Skill · Uncommon · multiplayer · ✅ read  
Gain 9 Block. Redirect all incoming attacks that would be dealt to another player this turn to you.  
*Upgraded:* Gain 13 Block. Redirect all incoming attacks that would be dealt to another player this turn to you.  
*Simulator:* Block 9.  
Multiplayer only; your own Block is read. `MP`

**Jack of All Trades** · 0 energy · Skill · Uncommon · Exhaust · ○ not read  
Add 1 random Colorless card into your Hand.  
*Upgraded:* Add 2 random Colorless cards into your Hand.  
*Simulator:* nothing.  
A random Colorless card. Unread. `RANDOM_GEN`

**Lift** · 1 energy · Skill · Uncommon · multiplayer · ⚠️ misread  
Give another player 11 Block.  
*Upgraded:* Give another player 16 Block.  
*Simulator:* Block 11.  
Multiplayer only; the Block goes to another player but is credited to you. `MP`

**Panic Button** · 0 energy · Skill · Uncommon · Exhaust · ◐ partly read  
Gain 30 Block. You cannot gain Block from cards for 2 turns.  
*Upgraded:* Gain 40 Block. You cannot gain Block from cards for 2 turns.  
*Simulator:* Block 30.  
30 Block read; no Block from cards for 2 turns isn't, so it is overvalued. `BLOCK_LOCK`

**Production** · 0 energy · Skill · Uncommon · Exhaust · ✅ read  
Gain 2 Energy.  
*Upgraded:* Gain 3 Energy.  
*Simulator:* energy 2.  

**Prolong** · 0 energy · Skill · Uncommon · Exhaust · ○ not read  
Next turn, gain Block equal to your current Block.  
*Upgraded:* Loses exhaust  
*Simulator:* nothing.  
Next turn, Block equal to current Block. Unread. `BLOCK_NEXT`

**Purity** · 0 energy · Skill · Uncommon · Exhaust, Retain · ○ not read  
Exhaust up to 3 cards in your Hand.  
*Upgraded:* Exhaust up to 5 cards in your Hand.  
*Simulator:* nothing.  
Exhausts up to 3 hand cards. Unread (exhaust fuel and deck cleaning). `EXH_CHOOSE`

**Restlessness** · 0 energy · Skill · Uncommon · Retain · ◐ partly read  
If your Hand is empty, draw 2 cards and gain 2 Energy.  
*Upgraded:* If your Hand is empty, draw 3 cards and gain 3 Energy.  
*Simulator:* draw 2, energy 2.  
Draw and energy read as always; it needs an empty hand. `DRAW_COND`

**Shockwave** · 2 energy · Skill · Uncommon · Exhaust · ⚠️ misread  
Apply 3 Weak and Vulnerable to ALL enemies.  
*Upgraded:* Apply 5 Weak and Vulnerable to ALL enemies.  
*Simulator:* all enemies.  
3 Weak and 3 Vulnerable to all sit under `power` and are dropped. `VAR_POWER`

**The Bomb** · 2 energy · Skill · Uncommon · ○ not read  
At the end of 3 turns, deal 40 damage to ALL enemies.  
*Upgraded:* At the end of 3 turns, deal 50 damage to ALL enemies.  
*Simulator:* nothing.  
40 to all after 3 turns. Unread; a delayed trigger. `DELAYED_DMG`

**Thinking Ahead** · 0 energy · Skill · Uncommon · Exhaust · ◐ partly read  
Draw 2 cards. Put 1 card from your Hand on top of your Draw Pile.  
*Upgraded:* Loses exhaust  
*Simulator:* draw 2.  
Draw read; putting a card back isn't. `PILE_TOP`

**Ultimate Defend** · 1 energy · Skill · Uncommon · ✅ read  
Gain 11 Block.  
*Upgraded:* Gain 15 Block.  
*Simulator:* Block 11.  
Counts as a Defend for Fasten.

**Alchemize** · 1 energy · Skill · Rare · Exhaust · ○ not read  
Procure a random potion.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
A random potion. `OUT_OF_COMBAT`

**Anointed** · 1 energy · Skill · Rare · Exhaust · ○ not read  
Put every Rare card from your Draw Pile into your Hand.  
*Upgraded:* Gains retain  
*Simulator:* nothing.  
Every Rare in the Draw Pile to the hand. Unread. `TUTOR`

**Beat Down** · 3 energy · Skill · Rare · ○ not read  
Play 3 random Attacks from your Discard Pile.  
*Upgraded:* Play 4 random Attacks from your Discard Pile.  
*Simulator:* nothing.  
Plays 3 random Attacks from the Discard Pile. Unread. `AUTOPLAY`

**Hidden Gem** · 1 energy · Skill · Rare · ◐ partly read  
A random card without Replay in your Draw Pile gains Replay 2.  
*Upgraded:* A random card without Replay in your Draw Pile gains Replay 3.  
*Simulator:* gives Replay.  
Replay 2 to a random Draw Pile card. Read as a named replay that matches nothing. `GRANT_REPLAY`

**Master of Strategy** · 0 energy · Skill · Rare · Exhaust · ✅ read  
Draw 3 cards.  
*Upgraded:* Draw 4 cards.  
*Simulator:* draw 3.  

**Mimic** · 1 energy · Skill · Rare · Exhaust · multiplayer · ○ not read  
Gain Block equal to the Block on another player.  
*Upgraded:* Loses exhaust  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Rally** · 2 energy · Skill · Rare · multiplayer · ✅ read  
ALL players gain 12 Block.  
*Upgraded:* ALL players gain 17 Block.  
*Simulator:* Block 12.  
Multiplayer only; your own Block is read. `MP`

**Scrawl** · 1 energy · Skill · Rare · Exhaust · ○ not read  
Draw cards until your Hand is full.  
*Upgraded:* Gains retain  
*Simulator:* nothing.  
Draw to a full hand. Unread; easy (`max_hand − hand.len()`). `DRAW_FILL`

**Secret Technique** · 0 energy · Skill · Rare · Exhaust · ○ not read  
Put a Skill from your Draw Pile into your Hand.  
*Upgraded:* Loses exhaust  
*Simulator:* nothing.  
A Skill from the Draw Pile. Unread. `TUTOR`

**Secret Weapon** · 0 energy · Skill · Rare · Exhaust · ○ not read  
Put an Attack from your Draw Pile into your Hand.  
*Upgraded:* Loses exhaust  
*Simulator:* nothing.  
An Attack from the Draw Pile. Unread. `TUTOR`

**Splash** · 1 energy · Skill · Rare · ○ not read  
Choose 1 of 3 random Attacks from another character to add into your Hand. It's free to play this turn.  
*Upgraded:* Choose 1 of 3 random Upgraded Attacks from another character to add into your Hand. It's free to play this turn.  
*Simulator:* nothing.  
A free Attack from another character. Unread. `RANDOM_GEN`

**The Gambit** · 0 energy · Skill · Rare · ✅ read  
Gain 50 Block. If you take unblocked attack damage this combat, die.  
*Upgraded:* Gain 75 Block. If you take unblocked attack damage this combat, die.  
*Simulator:* Block 50.  
50 Block for 0 read. The real cost (death on any unblocked attack) is survival; flag it there.

#### Colorless: Powers

**Automation** · 1 energy · Power · Uncommon · ⚠️ misread  
Every 10 cards you draw, gain 1 Energy.  
*Upgraded:* Costs 0  
*Simulator:* power: When played: gain 1 Energy.  
Misread as +1 energy on play. It is +1 energy per 10 cards drawn. `TRIGGER_FALLBACK` `COUNTER_DRAWN`

**Fasten** · 1 energy · Power · Uncommon · ✅ read  
Gain an additional 4 Block from Defend cards.  
*Upgraded:* Gain an additional 6 Block from Defend cards.  
*Simulator:* power: a card named "defend" gives 4 more Block.  

**Panache** · 0 energy · Power · Uncommon · ✅ read  
Every time you play 5 cards in a single turn, deal 10 damage to ALL enemies.  
*Upgraded:* Every time you play 5 cards in a single turn, deal 14 damage to ALL enemies.  
*Simulator:* power: Every 5 plays of a card in a turn: deal 10 damage to all.  

**Prep Time** · 1 energy · Power · Uncommon · ✅ read  
At the start of your turn, gain 4 Vigor.  
*Upgraded:* At the start of your turn, gain 6 Vigor.  
*Simulator:* power: Each turn: gain 4 Vigor.  

**Prowess** · 1 energy · Power · Uncommon · ✅ read  
Gain 1 Strength. Gain 1 Dexterity.  
*Upgraded:* Gain 2 Strength. Gain 2 Dexterity.  
*Simulator:* power: When played: gain 1 Strength, When played: gain 1 Dexterity.  

**Stratagem** · 1 energy · Power · Uncommon · ○ not read  
Whenever you shuffle your Draw Pile, choose a card from it to put into your Hand.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
On each shuffle, a chosen Draw Pile card to the hand. Unread. `TUTOR`

**Beacon of Hope** · 2 energy · Power · Rare · multiplayer · ○ not read  
Whenever you gain Block on your turn, other players gain half that much Block.  
*Upgraded:* Gains innate  
*Simulator:* nothing.  
Multiplayer only. `MP`

**Calamity** · 3 energy · Power · Rare · ○ not read  
Whenever you play an Attack, add a random Attack into your Hand.  
*Upgraded:* Costs 2  
*Simulator:* nothing.  
A random Attack per Attack played. Unread. `RANDOM_GEN`

**Entropy** · 1 energy · Power · Rare · ○ not read  
At the start of your turn, Transform 1 card in your Hand.  
*Upgraded:* Gains innate  
*Simulator:* nothing.  
Transforms a hand card each turn. Unread. `TRANSFORM`

**Eternal Armor** · 3 energy · Power · Rare · ✅ read  
Gain 9 Plating.  
*Upgraded:* Gain 12 Plating.  
*Simulator:* power: When played: gain 9 Plating.  
9 Plating.

**Mayhem** · 2 energy · Power · Rare · ○ not read  
At the start of your turn, play the top card of your Draw Pile.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Plays the top Draw Pile card each turn. Unread. `AUTOPLAY`

**Nostalgia** · 1 energy · Power · Rare · ○ not read  
The first Attack or Skill you play each turn is placed on top of your Draw Pile.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
The first Attack or Skill each turn goes on top of the Draw Pile. Unread. `PILE_TOP`

**Rolling Boulder** · 3 energy · Power · Rare · ◐ partly read  
At the start of your turn, deal 5 damage to ALL enemies and increase this damage by 5.  
*Upgraded:* At the start of your turn, deal 10 damage to ALL enemies and increase this damage by 5.  
*Simulator:* power: Each turn: deal 5 damage to all.  
5 to all each turn read; +5 more each turn isn't. `SELF_GROW`

### Tokens (made in combat)

#### Tokens (made in combat): Attacks

**Giant Rock** · 1 energy · Attack · Token · ✅ read  
Deal 20 damage.  
*Upgraded:* Deal 24 damage.  
*Simulator:* damage 20.  
Made by Primal Force.

**Justice** · 0 energy · Attack · Token · Exhaust · ✅ read  
Deal 30 damage.  
*Upgraded:* Deal 35 damage.  
*Simulator:* damage 30.  
Tarot (Mythic Form).

**Minion Dive Bomb** · 0 energy · Attack · Token · Exhaust · ✅ read  
Deal 11 damage.  
*Upgraded:* Deal 13 damage.  
*Simulator:* damage 11.  
Made by CHARGE!!.

**Minion Strike** · 0 energy · Attack · Token · Exhaust · ✅ read  
Deal 6 damage. Draw 1 card.  
*Upgraded:* Deal 9 damage. Draw 1 card.  
*Simulator:* damage 6, draw 1.  
Made by BEGONE!. Counts as a Strike.

**Shiv** · 0 energy · Attack · Token · Exhaust · ✅ read  
Deal 4 damage.  
*Upgraded:* Deal 6 damage.  
*Simulator:* damage 4.  
0-cost, Exhaust, 4 damage. Made by Blade Dance, Cloak and Dagger, Leading Strike, Infinite Blades, Storm of Steel, Hidden Daggers, Up My Sleeve, Fan of Knives, The Moon. Accuracy, Phantom Blades, Fan of Knives and Knife Trap modify it.

**Sovereign Blade** · 2 energy · Attack · Token · Retain · ✅ read  
Deal 10 damage.  
*Upgraded:* Costs 1  
*Simulator:* damage 10.  
Regent's Forge card: the first Forge each combat adds it; every Forge adds damage. Retain. Read as the rules' `forge_card`.

**Sweeping Gaze** · 0 energy · Attack · Token · Exhaust, Ethereal · ⚠️ misread  
Osty deals 10 damage to a random enemy.  
*Upgraded:* Osty deals 15 damage to a random enemy.  
*Simulator:* damage 10, Osty damage 10.  
Osty's 10 counted twice. Made by Sentry Mode. `OSTY_DOUBLE`

**The Chariot** · 0 energy · Attack · Token · Exhaust · ✅ read  
Deal 25 damage to ALL enemies.  
*Upgraded:* Deal 30 damage to ALL enemies.  
*Simulator:* damage 25, all enemies.  
Tarot.

**The Shot** · 1 energy · Attack · Token · ○ not read  
Shoot an Arrow to deal 10 damage for each card Exhausted by Florescence.  
*Upgraded:* Shoot an Arrow to deal 13 damage for each card Exhausted by Florescence.  
*Makes:* Florescence  
*Simulator:* shoots, per card exhausted.  
10 per card Florescence exhausted. Florescence is unread, so this has no count. `COUNT_SOURCE`

**The Tower** · 0 energy · Attack · Token · Exhaust · ○ not read  
Deal 2 damage for each floor traveled.  
*Upgraded:* Deal 3 damage for each floor traveled.  
*Simulator:* nothing.  
2 per floor travelled. The floor is run state; pass it in. `FLOOR`

#### Tokens (made in combat): Skills

**Death** · 0 energy · Skill · Token · Exhaust · ✅ read  
Apply 75 Doom.  
*Upgraded:* Apply 99 Doom.  
*Simulator:* Doom 75.  
Tarot. 75 Doom (Doom kills at the end of the enemy turn if HP ≤ Doom).

**Fuel** · 0 energy · Skill · Token · Exhaust · ✅ read  
Gain 1 Energy.  
*Upgraded:* Gain 2 Energy.  
*Simulator:* energy 1.  
Made by Compact.

**Joy** · 0 energy · Skill · Token · Exhaust · ✅ read  
Gain 1 Energy. Draw 1 card.  
*Upgraded:* Gain 2 Energy. Draw 1 card.  
*Simulator:* draw 1, energy 1.  
Made by Brighten Up, Punch of Joy, Ecstasy, Self Deprecate. +1 energy, draw 1.

**Judgement** · 0 energy · Skill · Token · Exhaust · ○ not read  
If the enemy has 30 or less HP, set their HP to 0.  
*Upgraded:* If the enemy has 40 or less HP, set their HP to 0.  
*Simulator:* nothing.  
Tarot. Sets an enemy at ≤ 30 HP to 0. Execute threshold. `EXECUTE`

**Luminesce** · 0 energy · Skill · Token · Exhaust, Retain · ✅ read  
Gain 2 Energy.  
*Upgraded:* Gain 3 Energy.  
*Simulator:* energy 2.  
Retain energy token. (Its maker isn't named in the card data.)

**Minion Sacrifice** · 0 energy · Skill · Token · Exhaust · ✅ read  
Gain 7 Block.  
*Upgraded:* Gain 10 Block.  
*Simulator:* Block 7.  
Made by GUARDS!!!.

**Soul** · 0 energy · Skill · Token · Exhaust · ✅ read  
Draw 2 cards.  
*Upgraded:* Draw 3 cards.  
*Simulator:* draw 2.  
0-cost, Exhaust, draw 2. Made by Reave, Severance, Grave Warden, Capture Spirit, Dirge, Seance, Glimpse Beyond. Paid off by Soul Storm, Devour Life, Haunt.

**Strength** · 0 energy · Skill · Token · Exhaust · ✅ read  
Gain 4 Strength.  
*Upgraded:* Gain 5 Strength.  
*Simulator:* Strength 4.  
Tarot. +4 Strength.

**Temperance** · 0 energy · Skill · Token · Exhaust · ✅ read  
Gain 30 Block.  
*Upgraded:* Gain 35 Block.  
*Simulator:* Block 30.  
Tarot.

**The Devil** · 0 energy · Skill · Token · Exhaust · ◐ partly read  
Exhaust your Hand. Play the top card of your Draw Pile for each card Exhausted this way.  
*Upgraded:* Exhaust your Hand. Upgrade and play the top card of your Draw Pile for each card Exhausted this way.  
*Simulator:* exhausts Hand.  
Tarot. Hand exhaust read; playing that many top cards isn't. `AUTOPLAY`

**The Emperor** · 0 energy · Skill · Token · Exhaust · ✅ read  
Gain 7 Energy.  
*Upgraded:* Gain 9 Energy.  
*Simulator:* energy 7.  
Tarot. +7 energy.

**The Empress** · 0 energy · Skill · Token · Exhaust · ⚠️ misread  
Apply 3 Weak. Apply 3 Vulnerable.  
*Upgraded:* Apply 4 Weak. Apply 4 Vulnerable.  
*Simulator:* nothing.  
Tarot. 3 Weak and 3 Vulnerable under `power`, dropped. `VAR_POWER`

**The Fool** · 0 energy · Skill · Token · Exhaust · ○ not read  
Play 2 random cards from your Hand.  
*Upgraded:* Play 3 random cards from your Hand.  
*Simulator:* nothing.  
Tarot. Plays 2 random hand cards. Unread. `AUTOPLAY`

**The Hanged Man** · 0 energy · Skill · Token · Exhaust · ○ not read  
Lose 6 HP. Take an extra turn after this one.  
*Upgraded:* Lose 3 HP. Take an extra turn after this one.  
*Simulator:* nothing.  
Tarot. Lose 6 HP, take an extra turn. Unread. `EXTRA_TURN`

**The Hermit** · 0 energy · Skill · Token · Exhaust · ◐ partly read  
Discard a card. Draw 2 cards. Discarded Curses are Exhausted.  
*Upgraded:* Discard a card. Draw 3 cards. Discarded Curses are Exhausted.  
*Simulator:* draw 2.  
Tarot. Draw read; discard isn't. `DISCARD_CHOOSE`

**The Hierophant** · 0 energy · Skill · Token · Exhaust · ○ not read  
Add 3 random Colorless cards into your Hand. They cost 1 less 1 Energy this turn.  
*Upgraded:* Add 4 random Colorless cards into your Hand. They cost 1 less 1 Energy this turn.  
*Simulator:* nothing.  
Tarot. 3 random Colorless cards, 1 cheaper. Unread. `RANDOM_GEN`

**The High Priestess** · 0 energy · Skill · Token · Exhaust, Retain · ✅ read  
Draw 4 cards.  
*Upgraded:* Draw 5 cards.  
*Simulator:* draw 4.  
Tarot.

**The Lovers** · 0 energy · Skill · Token · Exhaust · ○ not read  
Fill your Hand with random cards from other characters.  
*Upgraded:* Fill your Hand with random upgraded cards from other characters.  
*Simulator:* nothing.  
Tarot. Fills the hand with other characters' cards. Unread. `RANDOM_GEN`

**The Magician** · 0 energy · Skill · Token · Exhaust · ○ not read  
Gain 3 Orb Slots. Channel 3 random Orbs.  
*Upgraded:* Gain 4 Orb Slots. Channel 4 random Orbs.  
*Simulator:* nothing.  
Tarot. +3 orb slots and 3 random orbs. Unread. `RANDOM_ORB`

**The Moon** · 0 energy · Skill · Token · Exhaust · ○ not read  
Fill your Hand with Shivs.  
*Upgraded:* Fill your Hand with Shivs+.  
*Makes:* Shiv  
*Simulator:* nothing.  
Tarot. Fills the hand with Shivs. Unread; `Make` with n = free hand slots. `FILL_HAND`

**The Star** · 0 energy · Skill · Token · Exhaust · ⚠️ misread  
Next turn, gain 3 Energy and draw 3 cards.  
*Upgraded:* Next turn, gain 4 Energy and draw 4 cards.  
*Simulator:* draw 3, energy next turn 3.  
Tarot. Next-turn energy read; next-turn draw 3 is drawn now. `DRAW_NEXT`

**The Sun** · 0 energy · Skill · Token · Exhaust · ✅ read  
Draw 2 cards. Gain 2 Energy. Add a Soot into your Discard Pile.  
*Upgraded:* Draw 3 cards. Gain 3 Energy. Add a Soot into your Discard Pile.  
*Makes:* Soot  
*Simulator:* draw 2, energy 2, n=1, pile=Discard }].  
Tarot.

**The World** · 0 energy · Skill · Token · Exhaust · ○ not read  
Put 5 random cards from your Discard Pile into your Hand.  
*Upgraded:* Put 7 random cards from your Discard Pile into your Hand.  
*Simulator:* nothing.  
Tarot. 5 random discard cards to the hand. Unread. `PILE_MOVE`

**Wheel of Fortune** · 0 energy · Skill · Token · Exhaust · ✅ read  
Discard your Hand. Draw 7 cards.  
*Upgraded:* Discard your Hand. Draw 9 cards.  
*Simulator:* draw 7, discards hand.  
Tarot.

#### Tokens (made in combat): Status

**Amnesia** · Unplayable · Status · Status · ⚠️ misread  
This turn, draw 2 fewer cards and lose 1 Energy.  
*Simulator:* draw 2, energy 1, unplayable (dead draw).  
Status: this turn, draw 2 fewer and lose 1 energy. Unplayable, so its vars never fire, but the tag reader calls it "Draw 2, Energy 1". The drawback isn't modelled. `STATUS_EFFECT`

**Disintegration** · Unplayable · Status · Status · ○ not read  
At the end of your turn, take 6 damage.  
*Simulator:* unplayable (dead draw).  
Status: take 6 at end of turn. Survival. `SELF_HP`

**Mind Rot** · Unplayable · Status · Status · ○ not read  
Draw 1 fewer card each turn.  
*Simulator:* unplayable (dead draw).  
Status: draw 1 fewer each turn. Unread. `STATUS_EFFECT`

**Sloth** · Unplayable · Status · Status · ○ not read  
You cannot play more than 3 cards each turn.  
*Simulator:* unplayable (dead draw).  
Status: max 3 cards per turn. Unread. `PLAY_LIMIT`

**Waste Away** · Unplayable · Status · Status · ○ not read  
Gain 1 less 1 Energy per turn.  
*Simulator:* unplayable (dead draw).  
Status: 1 less energy per turn. Unread. `STATUS_EFFECT`

### Event and Ancient cards

#### Event and Ancient cards: Attacks

**Maul** · 1 energy · Attack · Ancient · ◐ partly read  
Deal 5 damage twice. Increase the damage of ALL Maul cards by 2 this combat.  
*Upgraded:* Deal 6 damage twice. Increase the damage of ALL Maul cards by 3 this combat.  
*Simulator:* damage 5×2.  
Ancient. Hits read; +2 to every Maul each play isn't. `SELF_GROW`

**Neow's Fury** · 1 energy · Attack · Ancient · Exhaust · ◐ partly read  
Deal 10 damage. Put up to 2 cards from your Discard Pile into your Hand.  
*Upgraded:* Deal 14 damage. Put up to 3 cards from your Discard Pile into your Hand.  
*Simulator:* damage 10.  
Ancient. Damage read; up to 2 discard cards to the hand isn't. `PILE_MOVE`

**Whistle** · 2 energy · Attack · Ancient · Exhaust · ◐ partly read  
Deal 33 damage. Stun the enemy.  
*Upgraded:* Deal 44 damage. Stun the enemy.  
*Simulator:* damage 33.  
Ancient. 33 read; Stun (the enemy skips its next action) is strong mitigation. Survival. `STUN`

**Byrd Swoop** · 0 energy · Attack · Event · ✅ read  
Deal 14 damage.  
*Upgraded:* Deal 18 damage.  
*Simulator:* damage 14.  

**Clash** · 0 energy · Attack · Event · ⚠️ misread  
Can only be played if every card in your Hand is an Attack. Deal 14 damage.  
*Upgraded:* Can only be played if every card in your Hand is an Attack. Deal 18 damage.  
*Simulator:* damage 14.  
14 for 0 read as always playable; it needs every hand card to be an Attack. `PLAY_COND`

**DingyDagger** · 0 energy · Attack · Event · ✅ read  
Deal 7 damage. Can be upgraded any number of times.  
*Simulator:* damage 7.  
Can be upgraded any number of times (no upgrade data).

**Exterminate** · 1 energy · Attack · Event · ✅ read  
Deal 3 damage to ALL enemies 4 times.  
*Upgraded:* Deal 4 damage to ALL enemies 4 times.  
*Simulator:* damage 3×4, all enemies.  

**Mad Science** · 1 energy · Attack · Event · ⚠️ misread  
Deal 12 damage.  
*Upgraded:* Gains innate  
*Simulator:* damage 12, Block 8.  
Event card whose type and effect are set by the event. Read as 12 damage plus 8 Block (both vars), which no variant does at once. Read the variant from the run record. `EVENT_VARIANT`

**Mighty Weapon** · 1 energy · Attack · Event · ✅ read  
Deal 25 damage to ALL enemies. Add 2 Dazed into your Draw Pile.  
*Upgraded:* Deal 30 damage to ALL enemies. Add 2 Dazed into your Draw Pile.  
*Makes:* Dazed  
*Simulator:* damage 25, all enemies, n=2, pile=Draw }].  

**Peck** · 1 energy · Attack · Event · ✅ read  
Deal 2 damage 3 times.  
*Upgraded:* Deal 2 damage 4 times.  
*Simulator:* damage 2×3.  

**Rebound** · 1 energy · Attack · Event · ◐ partly read  
Deal 9 damage. Put the next card you play this turn on top of your Draw Pile.  
*Upgraded:* Deal 12 damage. Put the next card you play this turn on top of your Draw Pile.  
*Simulator:* damage 9.  
Damage read; the next card played going on top isn't. `PILE_TOP`

**Rip and Tear** · 1 energy · Attack · Event · ✅ read  
Deal 7 damage to a random enemy twice.  
*Upgraded:* Deal 9 damage to a random enemy twice.  
*Simulator:* damage 7×2.  

**Squash** · 1 energy · Attack · Event · ✅ read  
Deal 10 damage. Apply 2 Vulnerable.  
*Upgraded:* Deal 12 damage. Apply 3 Vulnerable.  
*Simulator:* damage 10, Vulnerable 2.  

#### Event and Ancient cards: Skills

**Abundance** · 1 energy · Skill · Ancient · Exhaust · ○ not read  
Choose 1 of 3 Upgraded Powers to add into your Hand. It's free to play this turn.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
Ancient. Choose 1 of 3 upgraded Powers, free. Unread. `RANDOM_GEN`

**Apotheosis** · 2 energy · Skill · Ancient · Exhaust, Innate · ○ not read  
Upgrade ALL your cards.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Ancient. Upgrades every card this combat. Unread; switch every play to its upgraded version. `UPGRADE`

**Apparition** · 1 energy · Skill · Ancient · Exhaust, Ethereal · ○ not read  
Gain 1 Intangible.  
*Upgraded:* Loses ethereal  
*Simulator:* nothing.  
Ancient. 1 Intangible. Survival. `INTANGIBLE`

**Brightest Flame** · 0 energy · Skill · Ancient · ✅ read  
Gain 2 Energy. Draw 2 cards. Lose 2 Max HP.  
*Upgraded:* Gain 3 Energy. Draw 3 cards. Lose 2 Max HP.  
*Simulator:* draw 2, energy 2.  
Ancient. Energy and draw; −2 Max HP is run cost.

**Relax** · 3 energy · Skill · Ancient · Exhaust · ⚠️ misread  
Gain 16 Block. Next turn, draw 2 cards and gain 2 Energy.  
*Upgraded:* Gain 18 Block. Next turn, draw 3 cards and gain 3 Energy.  
*Simulator:* Block 16, draw 2, energy next turn 2.  
Ancient. Next-turn energy read; next-turn draw happens now. `DRAW_NEXT`

**Wish** · 0 energy · Skill · Ancient · Exhaust · ○ not read  
Put a card from your Draw Pile into your Hand.  
*Upgraded:* Gains retain  
*Simulator:* nothing.  
Ancient. Any Draw Pile card to the hand. Unread. `TUTOR`

**Distraction** · 1 energy · Skill · Event · Exhaust · ○ not read  
Add a random Skill into your Hand. It's free to play this turn.  
*Upgraded:* Costs 0  
*Simulator:* nothing.  
A random free Skill. Unread. `RANDOM_GEN`

**Dual Wield** · 1 energy · Skill · Event · ○ not read  
Choose an Attack or Power card. Add a copy of that card into your Hand.  
*Upgraded:* Choose an Attack or Power card. Add 2 copies of that card into your Hand.  
*Simulator:* nothing.  
Copy an Attack or Power in hand. Unread. `COPY_CHOSEN`

**Enlightenment** · 0 energy · Skill · Event · ○ not read  
Reduce the cost of ALL cards in your Hand to 1 this turn.  
*Upgraded:* Reduce the cost of ALL cards in your Hand to 1 this combat.  
*Simulator:* nothing.  
Hand costs capped at 1 this turn. Unread. `COST_CAP`

**Entrench** · 2 energy · Skill · Event · ○ not read  
Double your Block.  
*Upgraded:* Costs 1  
*Simulator:* nothing.  
Doubles current Block. Unread. `BLOCK_MULT`

**Feeding Frenzy** · 0 energy · Skill · Event · ⚠️ misread  
Gain 5 Strength this turn.  
*Upgraded:* Gain 7 Strength this turn.  
*Simulator:* Strength 5.  
"5 Strength this turn" kept for the whole fight. `STR_TEMP`

**God Byrd** · 0 energy · Skill · Event · ⚠️ misread  
At the start of your next turn, deal 30 damage to ALL enemies.  
*Upgraded:* At the start of your next turn, deal 40 damage to ALL enemies.  
*Simulator:* damage 30.  
Read as 30 single-target damage now; it is 30 to all next turn. `DELAYED_DMG`

**Metamorphosis** · 2 energy · Skill · Event · Exhaust · ○ not read  
Add 3 random Attacks into your Draw Pile. They're free to play this combat.  
*Upgraded:* Add 5 random Attacks into your Draw Pile. They're free to play this combat.  
*Simulator:* nothing.  
3 random free Attacks into the Draw Pile. Unread. `RANDOM_GEN`

**Mighty Shield** · 1 energy · Skill · Event · ✅ read  
Gain 20 Block. Add 3 Dazed into your Draw Pile.  
*Upgraded:* Gain 25 Block. Add 3 Dazed into your Draw Pile.  
*Makes:* Dazed  
*Simulator:* Block 20, n=3, pile=Draw }].  

**Outmaneuver** · 1 energy · Skill · Event · ✅ read  
Next turn, gain 2 Energy.  
*Upgraded:* Next turn, gain 3 Energy.  
*Simulator:* energy next turn 2.  

**Stack** · 1 energy · Skill · Event · ○ not read  
Gain Block equal to the number of cards in your Discard Pile.  
*Upgraded:* Gain Block equal to the number of cards in your Discard Pile +3.  
*Simulator:* nothing.  
Block equal to Discard Pile size. Unread; `discard.len()`. `COUNT_DISCARD`

**Toric Toughness** · 2 energy · Skill · Event · ◐ partly read  
Gain 5 Block. Gain 5 Block at the start of the next 2 turns.  
*Upgraded:* Gain 7 Block. Gain 7 Block at the start of the next 2 turns.  
*Simulator:* Block 5.  
5 read; 5 more on each of the next 2 turns isn't. `BLOCK_NEXT`

#### Event and Ancient cards: Powers

**Caltrops** · 1 energy · Power · Event · ⚠️ misread  
Whenever you are attacked, deal 3 damage back.  
*Upgraded:* Whenever you are attacked, deal 5 damage back.  
*Simulator:* power: When played: deal 3 damage.  
Misread as 3 damage on play. It is 3 Thorns (damage back per enemy hit). `TRIGGER_FALLBACK` `THORNS`

**Hello World** · 1 energy · Power · Event · ○ not read  
At the start of your turn, add a random Common card into your Hand.  
*Upgraded:* Gains innate  
*Simulator:* nothing.  
A random Common card each turn. Unread. `RANDOM_GEN`

### Statuses

#### Statuses: Status

**Beckon** · 1 energy · Status · Status · ○ not read  
At the end of your turn, if this is in your Hand, lose 6 HP.  
*Simulator:* unplayable (dead draw).  
Costs 1 (played to clear it); 6 HP at end of turn if held. Treated as dead. `SELF_HP`

**Burn** · Unplayable · Status · Status · Unplayable · ○ not read  
At the end of your turn, if this is in your Hand, take 2 damage.  
*Simulator:* damage 2, unplayable (dead draw).  
2 damage at end of turn if held. Feeds Trash to Treasure and Smokestack when made. `SELF_HP`

**Dazed** · Unplayable · Status · Status · Ethereal, Unplayable · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Ethereal, Unplayable.

**Debris** · 1 energy · Status · Status · Exhaust · ⚠️ misread  
(no text)  
*Simulator:* unplayable (dead draw).  
Costs 1 and exhausts: playable to clear, and an exhaust for Feel No Pain and Dark Embrace. Treated as a dead draw. `DEAD_PLAYABLE`

**Frantic Escape** · 1 energy · Status · Status · ○ not read  
Get farther away. Increase Sandpit by 1. Increase the cost of this card by 1.  
*Simulator:* unplayable (dead draw).  
Encounter status (Sandpit). Out of scope. `ENCOUNTER`

**Infection** · Unplayable · Status · Status · Unplayable · ○ not read  
At the end of your turn, if this is in your Hand, take 3 damage.  
*Simulator:* damage 3, unplayable (dead draw).  
3 damage at end of turn if held. `SELF_HP`

**Sadness** · Unplayable · Status · Status · Retain, Unplayable · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Retain, Unplayable. Made by The Fool's cards; Growth, Punch of Joy and Ecstasy turn it into value. Held Sadness shrinks later hands, which the simulator models through Retain.

**Slimed** · 1 energy · Status · Status · Exhaust · ⚠️ misread  
Draw 1 card.  
*Simulator:* draw 1, unplayable (dead draw).  
Costs 1, exhausts, draws 1: a playable clear. Treated as dead. Tag error: "Draw 1". `DEAD_PLAYABLE`

**Soot** · Unplayable · Status · Status · Unplayable · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Unplayable dead draw (from The Sun).

**Toxic** · 1 energy · Status · Status · Exhaust · ○ not read  
At the end of your turn, if this is in your Hand, take 5 damage.  
*Simulator:* damage 5, unplayable (dead draw).  
Costs 1 to clear; 5 damage at end of turn if held. `SELF_HP` `DEAD_PLAYABLE`

**Void** · Unplayable · Status · Status · Ethereal, Unplayable · ⚠️ misread  
Whenever you draw this card, lose 1 Energy.  
*Simulator:* energy 1, unplayable (dead draw).  
Lose 1 energy when drawn. Read as dead; the energy loss isn't. Tag error: "Energy 1". `STATUS_EFFECT`

**Wither** · Unplayable · Status · Status · Unplayable · ○ not read  
At the end of your turn, if this is in your Hand, take 3 damage.  
*Upgraded:* At the end of your turn, if this is in your Hand, take 6 damage.  
*Simulator:* damage 3, unplayable (dead draw).  
3 damage at end of turn if held. `SELF_HP`

**Wound** · Unplayable · Status · Status · Unplayable · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Dead draw.

### Curses

#### Curses: Curses

**Ascender's Bane** · Unplayable · Curse · Curse · Ethereal, Unplayable, Eternal · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Ethereal, Unplayable, Eternal. A dead draw that exhausts at end of turn.

**Bad Luck** · Unplayable · Curse · Curse · Unplayable, Eternal · ○ not read  
At the end of your turn, if this is in your Hand, lose 13 HP.  
*Simulator:* unplayable (dead draw).  
Lose 13 HP at end of turn if held. Survival. `SELF_HP`

**Clumsy** · Unplayable · Curse · Curse · Ethereal, Unplayable · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Ethereal, Unplayable: a dead draw that exhausts itself.

**Curse of the Bell** · Unplayable · Curse · Curse · Unplayable, Eternal · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Dead draw. Eternal.

**Debt** · Unplayable · Curse · Curse · Unplayable · ○ not read  
At the end of your turn, if this is in your Hand, lose 10 Gold.  
*Simulator:* unplayable (dead draw).  
−10 Gold per turn held. `OUT_OF_COMBAT`

**Decay** · Unplayable · Curse · Curse · Unplayable · ○ not read  
At the end of your turn, if this is in your Hand, take 2 damage.  
*Simulator:* damage 2, unplayable (dead draw).  
2 damage at end of turn if held. The `damage` var is the self-damage, not an attack (unplayable, so harmless in play). `SELF_HP`

**Doubt** · Unplayable · Curse · Curse · Unplayable · ⚠️ misread  
At the end of your turn, if this is in your Hand, gain 1 Weak.  
*Simulator:* unplayable (dead draw).  
Gives you 1 Weak if held. The tag reader calls it "Weak 1" and the role reader "Survival": an enemy debuff. `SELF_DEBUFF`

**Enthralled** · 2 energy · Curse · Curse · Eternal · ⚠️ misread  
If this is in your Hand, it must be played before other cards.  
*Simulator:* unplayable (dead draw).  
Costs 2 and must be played before other cards; treated as a dead draw. It actually eats 2 energy. `FORCED_PLAY`

**Folly** · Unplayable · Curse · Curse · Innate, Ethereal, Unplayable, Eternal · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Innate, Ethereal, Unplayable, Eternal: always a dead opening card.

**Greed** · Unplayable · Curse · Curse · Unplayable, Eternal · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Dead draw. Eternal.

**Guilty** · Unplayable · Curse · Curse · Unplayable · ✅ read  
Removed from your Deck after 5 combats.  
*Simulator:* unplayable (dead draw).  
Dead draw; leaves the deck after 5 combats.

**Injury** · Unplayable · Curse · Curse · Unplayable · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Dead draw.

**Normality** · Unplayable · Curse · Curse · Unplayable · ⚠️ misread  
You cannot play more than 3 cards this turn.  
*Simulator:* damage 3, unplayable (dead draw).  
Max 3 cards a turn while held. Read as a dead draw with a stray `damage=3`. The limit isn't modelled. `PLAY_LIMIT`

**Poor Sleep** · Unplayable · Curse · Curse · Retain, Unplayable · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Retain, Unplayable: a dead card that stays in the hand and shrinks later draws.

**Regret** · Unplayable · Curse · Curse · Unplayable · ○ not read  
At the end of your turn, if this is in your Hand, lose 1 HP for each card in your Hand.  
*Simulator:* unplayable (dead draw).  
Lose 1 HP per hand card at end of turn if held. `SELF_HP`

**Shame** · Unplayable · Curse · Curse · Unplayable · ○ not read  
At the end of your turn, if this is in your Hand, gain 1 Frail.  
*Simulator:* unplayable (dead draw).  
1 Frail if held. `SELF_DEBUFF`

**Spore Mind** · 1 energy · Curse · Curse · Exhaust · ⚠️ misread  
(no text)  
*Simulator:* unplayable (dead draw).  
Costs 1 and exhausts. Treated as a dead draw, but it can be paid off to clear it. `DEAD_PLAYABLE`

**Writhe** · Unplayable · Curse · Curse · Innate, Unplayable · ✅ read  
(no text)  
*Simulator:* unplayable (dead draw).  
Innate dead draw.

### Quest cards

#### Quest cards: Quests

**Byrdonis Egg** · Unplayable · Quest · Quest · Unplayable · ✅ read  
Can be hatched at a Rest Site.  
*Simulator:* unplayable (dead draw).  
Quest card: a dead draw until hatched at a rest site. `OUT_OF_COMBAT`

**Dowsing** · Unplayable · Quest · Quest · Unplayable · ✅ read  
After entering 4 more ? rooms, transforms into an Abundance.  
*Makes:* Abundance  
*Simulator:* unplayable (dead draw).  
Quest card: a dead draw; becomes Abundance after 4 ? rooms. `OUT_OF_COMBAT`

**Lantern Key** · Unplayable · Quest · Quest · Unplayable · ✅ read  
Unlocks a special event in the next Act.  
*Simulator:* unplayable (dead draw).  
Quest card: a dead draw. `OUT_OF_COMBAT`

**Spoils Map** · Unplayable · Quest · Quest · Unplayable · ✅ read  
Marks a site of 600 extra Gold in the next Act.  
*Simulator:* unplayable (dead draw).  
Quest card: a dead draw. `OUT_OF_COMBAT`
