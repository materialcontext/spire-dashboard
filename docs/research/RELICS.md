# Relic dossier

Every relic in the Slay the Spire 2 beta (Spire Codex data, v0.113.1): what it does, what the simulator reads, and what is missing. Companion to [CARDS.md](CARDS.md), whose [Gaps](CARDS.md#gaps) codes this file reuses; [INTERACTIONS.md](INTERACTIONS.md) builds on both.

Relics reach the simulator through `mechanics::read` on the relic text (not as a played card): timed triggers ("At the start of each combat", "Every 3 turns", "Whenever you shuffle") and passives (extra damage, Vulnerable %, kept hand, carried energy, extra hits, echoes). A relic with no combat effect is marked · out of combat. Those are run value: the deck, Max HP and Gold they change already show in the run state.

## Summary

| Pool | Relics | ✅ | ◐ | ○ | ⚠️ | · out of combat |
|---|---:|---:|---:|---:|---:|---:|
| Ironclad | 9 | 2 | 1 | 4 | 0 | 2 |
| Silent | 9 | 4 | 0 | 5 | 0 | 0 |
| Defect | 9 | 4 | 1 | 4 | 0 | 0 |
| Necrobinder | 9 | 4 | 0 | 4 | 0 | 1 |
| Regent | 9 | 4 | 0 | 5 | 0 | 0 |
| The Fool | 9 | 4 | 0 | 4 | 1 | 0 |
| Shared | 261 | 54 | 9 | 66 | 1 | 131 |
| **All** | **315** | **76** | **11** | **92** | **2** | **134** |

Of the 181 relics with a combat effect, 76 are read in full. The unread ones fall into a few groups:

- **Encounter-kind relics** (Booming Conch, Sling of Courage, Tormentous Collar, Pantograph). The simulator doesn't know whether it is simulating an elite or a boss. The fight view does, so pass the encounter kind in.
- **Enchanting relics** (Nutritious Soup, Pael's Claw, Tri-Boomerang, Gnarled Hammer, Beautiful Bracelet and others). The enchantments are recorded on the deck cards but never simulated; see [Enchantments](#enchantments). Nutritious Soup alone turns every Strike into a 0-cost, +3 damage card.
- **Triggers the reader lacks**: on discard (Tingsha, Tough Bandages), on Star spent (Mini Regent, Galactic Dust), on card made (Regalite, Fertilizer), on dislodge (Twine), on Osty attack (Bone Flute), every N exhausts (Joss Paper), the Nth Attack (Pen Nib). These are the same trigger kinds the cards need (`TRIGGER_FALLBACK` in CARDS.md), so one change serves both.
- **End-of-turn conditions** on Block or hand size (Orichalcum, Parrying Shield, Cloak Clasp, Screaming Flagon, Obsidian Vice). The turn log already has Block and hand size.
- **Weak as an effect** (Red Mask, Tormentous Collar). The trigger `Effect` enum has Vulnerable but not Weak.
- **Survival relics** (Bronze Scales, Lizard Tail, Tungsten Rod, Beating Remnant, Wuppy, Paper Krane, Undying Sigil). These belong to the survival check, not the output simulator.
- **Two misreads**: Bread reads as −2 energy on turn 1 and nothing else, because the "+1 energy on all other turns" clause is dropped. Bizarre Lamp makes nothing, because "a Joy and a Sadness" is parsed as one card name.

## Enchantments

Several relics (and events) put an enchantment on deck cards. The run records it (`DeckCard.enchantment`, from the game save in monitored runs), but the simulator ignores it, and the record keeps only the enchantment's id, not its amount ("Swift 2", "Sharp 3"). Most enchantments are one-line modifiers on a `Play`, so this is a cheap, high-value addition: carry the amount, then apply the enchantment in `play_of`.

| Enchantment | Effect (Codex) | Applied by | How to simulate |
|---|---|---|---|
| Sharp X | +X damage (Attacks) | Gnarled Hammer | `damage += X` |
| Instinct | Attack damage doubled | Tri-Boomerang | damage × 2 (whether Strength is doubled too isn't in the data; double the printed damage) |
| Tezcatara's Ember | Costs 0, +3 damage, Eternal | Nutritious Soup (every Strike) | `cost = 0`, `damage += 3` |
| Vigorous X | +X damage the first time played | (event) | first-play bonus |
| Momentum X | +X damage this combat each play | Punch Dagger | per-instance growth (`SELF_GROW`) |
| Corrupted | +50% damage, lose 2 HP | (event) | damage × 1.5; HP loss to survival |
| Nimble X | +X Block from the card | Fresnel Lens | `block += X` |
| Adroit X | Gain X Block | Kifuda | `block += X` |
| Goopy | Gains Exhaust; Block +1 permanently per play | Pael's Claw (every Defend) | `exhausts = true`; growth is run value |
| Swift X | Draw X the first time played | Beautiful Bracelet, Wing Charm | first-play draw |
| Sown X | X energy the first time played each combat | (event) | first-play energy |
| Glam | Replay once per combat | Glitter, Silken Tress | first-play replay |
| Spiral | Replay 1 (Basic Strike or Defend) | (event) | plays twice |
| Inky | Applies 1 Weak | (event) | `weak += 1` |
| Royally Approved | Innate and Retain | Royal Stamp | flags |
| Steady | Retain | (event) | flag |
| Soul's Power | Loses Exhaust | (event) | flag |
| Imbued | Played automatically at combat start (Skills) | Electric Shrymp | play on turn 1 for free |
| Perfect Fit | Goes on top when shuffled | (event) | draw-order rule |
| Slumbering Essence | −1 cost each turn held, until played | (event) | per-instance cost |
| Slither | Cost randomised 0–3 on draw | (event) | per-draw cost |
| Clone | Can be copied at rest sites | Pael's Growth | run value |

"(event)" means no relic text names it; the Codex enchantment data doesn't list sources.

## Gaps

Codes from [CARDS.md](CARDS.md#gaps), plus relic-only ones. Count is relics needing it.

### Reader bugs

| Code | Relics | What it needs | Relics |
|---|---:|---|---|
| `MAKE_PARSE` | 3 | The reader misses a card-making phrase ("a Joy and a Sadness", "shuffle 3 Dazed into"). | Bizarre Lamp, Blessed Antler, Tea of Discourtesy |
| `WEAK_EFFECT` | 2 | Weak as a trigger effect: `Effect` has Vulnerable but no Weak. | Red Mask, Tormentous Collar |
| `DEX_TEMP` | 1 | "N Dexterity this turn" is kept all fight. | Helical Dart |
| `TRAILING_TIMING` | 1 | A second timing in the same sentence ("At the start of all other turns") is dropped. | Bread |

### Conditions and counts on state the simulator already tracks

| Code | Relics | What it needs | Relics |
|---|---:|---|---|
| `BLOCK_COND` | 3 | Conditions on Block at end of turn. | Orichalcum, Orichalcum???, Parrying Shield |
| `ENCOUNTER_KIND` | 3 | Only in elite (or boss) fights. Pass the encounter kind into the simulation. | Booming Conch, Sling of Courage, Tormentous Collar |
| `COND_NO_ATTACK` | 2 | If no Attack was played this turn. | Art of War, Ripple Basin |
| `COUNTER_PLAYS` | 2 | Per card played this turn or combat. | Brilliant Scarf, Pocketwatch |
| `COUNT_HAND` | 2 | Per (other) card or Skill in hand. | Cloak Clasp, Obsidian Vice |
| `ENERGY_NEXT_COMBAT` | 2 | Bonus in the fight after a given room. | Venerable Tea Set, Venerable Tea Set??? |
| `COUNTER_CHANNEL` | 1 | Per Lightning channelled this combat. | Metronome |
| `DEBUFF_AMP` | 1 | Vulnerable and Weak doubled for N turns. | Unsettling Lamp |
| `DRAW_PENALTY` | 1 | Fewer cards on turn 1. | Big Mushroom |
| `EMPTY_HAND` | 1 | If the hand is empty at end of turn. | Screaming Flagon |
| `HIT_THRESHOLD` | 1 | Triggers on a single hit of N+. | Mighty Wings |
| `LIMITED_COMBATS` | 1 | Lasts N combats; the run needs a counter. | Ember Tea |
| `MIN_HIT` | 1 | Raises small hits to a minimum. | The Boot |
| `POISON_BONUS` | 1 | +N to each Poison application. | Snecko Skull |
| `TURN_SCALE` | 1 | Scales with the turn number. | Mr. Struggles |
| `TYPE_SET` | 1 | Playing an Attack, Skill and Power in one turn. | Rainbow Ring |
| `WEAK_DMG` | 1 | Weak enemies take more damage. Needs enemy Weak tracked like Vulnerable. | Paper Krane |
| `X_BONUS` | 1 | X-cost cards count X+2. | Chemical X |

### This-turn and next-turn effects

| Code | Relics | What it needs | Relics |
|---|---:|---|---|
| `BLOCK_KEEP` | 2 | Block carries into the next turn. The simulator logs Block per turn; it needs a carried pool. | Diamond Diadem, Sturdy Clamp |
| `CONFUSED` | 2 | Card costs randomised 0–3 on draw. | Snecko Eye, Snecko Eye??? |
| `FIRST_GAIN_MULT` | 2 | Doubles the first gain of a stat (Strength, Block) each combat. | Ruined Helmet, Vambrace |
| `ON_DISCARD` | 2 | Trigger per discarded card (needs `DISCARD_CHOOSE` first). | Tingsha, Tough Bandages |
| `ON_STARS` | 2 | Trigger: whenever you gain or spend Stars. `gain`/`spend(Store::Stars)` are the hooks. | Galactic Dust, Mini Regent |
| `BLOCK_MULT` | 1 | Double Block gained (this turn, or the first gain) or current Block. | Pael's Legion |
| `BLOCK_NEXT` | 1 | Block at the start of the next turn(s). | Self-Forming Clay |
| `DELAYED_DMG` | 1 | Damage after N turns. | Stone Calendar |
| `DRAW_LOCK` | 1 | No more draws this turn. | Fiddle |
| `ENERGY_CARRY` | 1 | Bonus energy next turn when energy is left unspent. | Pael's Tears |
| `EVERY_ATTACK_MULT` | 1 | Every Nth Attack deals double. | Pen Nib |
| `EVERY_EXHAUST` | 1 | Every N exhausts. `EveryPlays` needs an exhaust variant. | Joss Paper |
| `EXTRA_TURN` | 1 | Take an extra turn. | Pael's Eye |
| `FIRST_POWER` | 1 | On the first Power each combat. | Permafrost |
| `FREE_RANDOM` | 1 | A random hand card becomes free. | Mummified Hand |
| `ON_DISLODGE` | 1 | Trigger: first dislodge each turn. | Twine |
| `ON_EXHAUST_TYPE` | 1 | Trigger on exhausting a card of a type. | Burning Sticks |
| `ON_MAKE` | 1 | Trigger: whenever you create a card. `make` is the single entry point. | Regalite |
| `ON_MAKE_NAMED` | 1 | Trigger: whenever you create a named card (Sadness). | Fertilizer |
| `ON_OSTY_ATTACK` | 1 | Trigger per Osty attack. | Bone Flute |
| `ON_PLAY_NAMED` | 1 | Trigger: whenever you play a named card (Soul, Sovereign Blade). `OnPlay(Named)` exists; the reader needs the phrase. | Helical Dart |
| `PLAY_LIMIT` | 1 | Only N more cards this turn. | Velvet Choker |
| `RETAIN_TEMP` | 1 | Retain your hand this turn. | Ringing Triangle |

### Per-card state

| Code | Relics | What it needs | Relics |
|---|---:|---|---|
| `ENCHANT` | 14 | Enchantments on deck cards. `DeckCard.enchantment` is recorded but the simulator ignores it; see [Enchantments](#enchantments). | Beautiful Bracelet, Electric Shrymp, Fresnel Lens, Glitter, Gnarled Hammer, Kifuda, Mystic Lighter, Nutritious Soup, Pael's Claw, Pael's Growth, Punch Dagger, Royal Stamp and 2 more |
| `UPGRADE` | 4 | Upgrades cards in combat. | Bellows, Bone Tea, Razor Tooth, Stone Cracker |
| `COST_UP_TYPE` | 1 | A card type costs more. | Spiked Gauntlets |
| `GRANT_KEYWORD` | 1 | Gives a hand card Ethereal or Retain. | Ghost Seed |
| `NAMED_DMG_MULT` | 1 | A named card deals double this turn. | Vitruvian Minion |
| `RETAIN_COST` | 1 | Retained cards get cheaper. | Bookmark |

### Choices and pile moves

| Code | Relics | What it needs | Relics |
|---|---:|---|---|
| `TUTOR` | 2 | Fetches a chosen or typed card from the Draw Pile. | Jeweled Mask, Power Cell |
| `AUTOPLAY` | 1 | Plays other cards (top of Draw Pile, random from a pile, cheapest in hand) for free. | History Course |
| `COPY_PLAYED` | 1 | Copy of the Nth Attack played. | Music Box |
| `DRAW_EMPTY` | 1 | Draw when the hand is empty. | Unceasing Top |
| `EXH_CHOOSE` | 1 | Exhaust a chosen card. Feeds Exhaust-Pile payoffs. | Toasty Mittens |
| `MULLIGAN` | 1 | Redraw the opening hand. | Gambling Chip |

### Random cards

| Code | Relics | What it needs | Relics |
|---|---:|---|---|
| `RANDOM_GEN` | 7 | Makes random cards. | Big Hat, Choices Paradox, Crossbow, Ocarina, Orange Dough, Toolbox, Vexing Puzzlebox |

### Character systems

| Code | Relics | What it needs | Relics |
|---|---:|---|---|
| `COMPANION` | 3 | A companion creature; the data doesn't describe it. | Byrdine, Byrdpip, Wuppy |
| `ORB_TRIGGER` | 2 | Trigger orb passives on demand. | Emotion Chip, Gold-Plated Cables |
| `DOOM_EXECUTE` | 1 | Kills now when Doom ≥ HP. | Undying Sigil |
| `MELODY` | 1 | A Wah, Cho or Ki card. Community guides (not the Codex data) say playing one gains that Melody. Needs Melody state before any Melody payoff can work. | Ocarina |
| `MELODY_TRIGGER` | 1 | Trigger on gaining a Melody (any, Wah, Cho, every). | Conductor's Baton |
| `ORB_BONUS` | 1 | Flat bonus to one orb type's value. | Infused Core |

### Survival effects

| Code | Relics | What it needs | Relics |
|---|---:|---|---|
| `HEAL` | 17 | Heals. | Bath Robes, Bath Towel, Black Blood, Blood Vial, Blood Vial???, Book Repair Knife, Book of Five Rings, Burning Blood, Demon Tongue, Eternal Feather, Lee's Waffle, Lee's Waffle??? and 5 more |
| `SELF_HP` | 5 | Lose HP (or take damage) yourself. Also feeds HP-loss payoffs (Rupture, Inferno, Spite, Tear Asunder), which need a lost-HP counter even in the output simulator. | Centennial Puzzle, Demon Tongue, Emotion Chip, Royal Poison, Self-Forming Clay |
| `ENEMY_STR` | 2 | The enemy loses (or gains) Strength. | Brimstone, Philosopher's Stone |
| `DMG_CAP` | 1 | Caps HP lost per turn. | Beating Remnant |
| `DMG_REDUCE` | 1 | Reduces each HP loss. | Tungsten Rod |
| `ENEMY_BLOCK` | 1 | Needs enemy Block. | Hand Drill |
| `FATAL` | 1 | Effect on a kill. Only priceable with enemy HP; the turn view has the horizon. | Gremlin Horn |
| `HP_COND` | 1 | Active at or below 50% HP. Pass the run's current HP into the simulation. | Red Skull |
| `REVIVE` | 1 | Revive once. | Lizard Tail |
| `THORNS` | 1 | Damage back per enemy hit. Needs the enemy's hit count. | Bronze Scales |

### Out of scope

| Code | Relics | What it needs | Relics |
|---|---:|---|---|
| `POTION` | 9 | Potions and potion slots (the simulator has no potions). | Alchemical Coffer, Belt Buckle, Cauldron, Delicate Frond, Petrified Toad, Phial Holster, Potion Belt, Reptile Trinket, Tiny Mailbox |
| `AUTOPILOT` | 1 | The game plays a turn for you. | Whispering Earring |

## Relics

### Ironclad

**Black Blood** · Starter · · out of combat  
At the end of combat, heal 12 HP.  
Ancient starter: heal 12 after combat. `HEAL`

**Burning Blood** · Starter · · out of combat  
At the end of combat, heal 6 HP.  
Ironclad starter: heal 6 after combat. `HEAL`

**Red Skull** · Common · ○ not read  
While your HP is at or below 50%, you have 3 additional Strength.  
+3 Strength at or below 50% HP. Needs the player's HP; the run state has it, so the simulator could take current HP as given. `HP_COND`

**Paper Phrog** · Uncommon · ✅ read  
Enemies with Vulnerable take 75% more damage rather than 50%.  
*Simulator:* Vulnerable is 75%.  
Vulnerable is 75%.

**Self-Forming Clay** · Uncommon · ○ not read  
Whenever you lose HP in combat, gain 3 Block next turn.  
3 Block next turn per HP loss. Needs HP loss. `SELF_HP` `BLOCK_NEXT`

**Charon's Ashes** · Rare · ✅ read  
Whenever you Exhaust a card, deal 3 damage to ALL enemies.  
*Simulator:* Whenever a card is exhausted: deal 3 damage to all.  
3 to all per exhaust.

**Demon Tongue** · Rare · ○ not read  
The first time you lose HP on your turn, heal HP equal to the amount lost.  
Heals the first HP loss on your turn. Survival; makes Ironclad's self-damage free once a turn. `SELF_HP` `HEAL`

**Ruined Helmet** · Rare · ○ not read  
The first time you gain Strength each combat, double the amount gained.  
Doubles the first Strength gain each combat. Unread. `FIRST_GAIN_MULT`

**Brimstone** · Shop · ◐ partly read  
At the start of your turn, gain 2 Strength and ALL enemies gain 1 Strength.  
*Simulator:* Each turn: gain 2 Strength.  
+2 Strength each turn read; enemies' +1 each turn is a survival cost. `ENEMY_STR`

### Silent

**Ring of the Drake** · Starter · ✅ read  
At the start of your first 3 turns, draw 2 additional cards.  
*Simulator:* First 3 turns: draw 2.  
Ancient starter: +2 draw on the first 3 turns.

**Ring of the Snake** · Starter · ✅ read  
At the start of each combat, draw 2 additional cards.  
*Simulator:* Combat start: draw 2.  
Silent starter: +2 draw on turn 1.

**Snecko Skull** · Common · ○ not read  
Whenever you apply Poison, apply an additional 1 Poison.  
+1 to each Poison application. Unread; one line in `apply`. `POISON_BONUS`

**Tingsha** · Uncommon · ○ not read  
Whenever you discard a card during your turn, deal 3 damage to a random enemy for each card discarded.  
3 damage per card discarded. Unread. `ON_DISCARD`

**Twisted Funnel** · Uncommon · ✅ read  
At the start of each combat, apply 4 Poison to ALL enemies.  
*Simulator:* Combat start: apply 4 Poison.  
4 Poison at combat start.

**Helical Dart** · Rare · ○ not read  
Whenever you play a Shiv, gain 1 Dexterity this turn.  
+1 Dexterity this turn per Shiv played. Unread. `ON_PLAY_NAMED` `DEX_TEMP`

**Paper Krane** · Rare · ○ not read  
Enemies with Weak deal 40% less damage to you rather than 25%.  
Weak enemies deal 40% less (not 25%). Survival. `WEAK_DMG`

**Tough Bandages** · Rare · ○ not read  
Whenever you discard a card during your turn, gain 3 Block.  
3 Block per discard. Unread; needs discards first. `ON_DISCARD`

**Ninja Scroll** · Shop · ✅ read  
At the start of each combat, add 3 Shivs into your Hand.  
*Simulator:* Combat start: add 3 shivs to your Hand.  
3 Shivs at combat start.

### Defect

**Cracked Core** · Starter · ✅ read  
At the start of each combat, Channel 1 Lightning.  
*Simulator:* Combat start: Channel 1 lightning.  
Defect starter.

**Infused Core** · Starter · ◐ partly read  
At the start of each combat, Channel 3 Lightning. Lightning deals 1 additional damage.  
*Simulator:* Combat start: Channel 3 lightning.  
Ancient starter upgrade. 3 Lightning read; +1 Lightning damage isn't. Add a per-orb Focus-like bonus. `ORB_BONUS`

**Data Disk** · Common · ✅ read  
Start each combat with 1 Focus.  
*Simulator:* Combat start: gain 1 Focus.  

**Gold-Plated Cables** · Uncommon · ○ not read  
Your rightmost Orb triggers its passive an additional time.  
Rightmost orb's passive triggers twice. Unread. `ORB_TRIGGER`

**Symbiotic Virus** · Uncommon · ✅ read  
At the start of each combat, Channel 1 Dark.  
*Simulator:* Combat start: Channel 1 dark.  

**Emotion Chip** · Rare · ○ not read  
If you lost HP during the previous turn, trigger the passive ability of all Orbs at the start of your turn.  
Triggers every orb passive at turn start if you lost HP last turn. Needs HP loss (enemy attacks). `SELF_HP` `ORB_TRIGGER`

**Metronome** · Rare · ○ not read  
The first time you Channel 7 Orbs each combat, deal 30 damage to ALL enemies.  
30 to all the first time 7 orbs have been channelled this combat. Needs a channel counter. `COUNTER_CHANNEL`

**Power Cell** · Rare · ○ not read  
At the start of each combat, add 2 zero-cost cards from your Draw Pile into your Hand.  
Two 0-cost Draw Pile cards to the opening hand. Unread. `TUTOR`

**Runic Capacitor** · Shop · ✅ read  
Start each combat with 3 additional Orb Slots.  
*Simulator:* Combat start: gain 3 orb slots.  

### Necrobinder

**Bound Phylactery** · Starter · ✅ read  
At the start of your turn, Summon 1.  
*Simulator:* Each turn: Summon 1.  
Necrobinder starter: Summon 1 a turn.

**Phylactery Unbound** · Starter · ✅ read  
At the start of each combat, Summon 5. At the start of your turn, Summon 2.  
*Simulator:* Combat start: Summon 5; Each turn: Summon 2.  
Ancient starter.

**Bone Flute** · Common · ○ not read  
Whenever Osty attacks, gain 2 Block.  
2 Block per Osty attack. Unread; Osty attacks are plays with `osty > 0`. `ON_OSTY_ATTACK`

**Book Repair Knife** · Uncommon · · out of combat  
Whenever a non-Minion enemy dies to Doom, heal 3 HP.  
Heal on Doom kills. `HEAL`

**Funerary Mask** · Uncommon · ✅ read  
At the start of each combat, add 3 Souls into your Draw Pile.  
*Simulator:* Combat start: add 3 souls to your Draw.  
3 Souls into the Draw Pile at combat start.

**Big Hat** · Rare · ○ not read  
At the start of each combat, add 2 random Ethereal cards into your Hand.  
2 random Ethereal cards at combat start. `RANDOM_GEN`

**Bookmark** · Rare · ○ not read  
At the end of each turn, lower the cost of a random Retained card by 1 until played.  
Retained cards get cheaper each turn. Unread. `RETAIN_COST`

**Ivory Tile** · Rare · ✅ read  
Whenever you play a card that costs 3 Energy or more, gain 1 Energy.  
*Simulator:* Whenever you play a card costing 3+: gain 1 Energy.  
+1 energy per card costing 3+.

**Undying Sigil** · Shop · ○ not read  
Enemies with at least as much Doom as HP deal 50% less damage.  
Doomed enemies (Doom ≥ HP) deal half damage. Survival. `DOOM_EXECUTE`

### Regent

**Divine Destiny** · Starter · ✅ read  
At the start of each combat, gain 7 Star.  
*Simulator:* Combat start: gain 7 Stars.  
Ancient starter: 7 Stars.

**Divine Right** · Starter · ✅ read  
At the start of each combat, gain 3 Star.  
*Simulator:* Combat start: gain 3 Stars.  
Regent starter: 3 Stars.

**Fencing Manual** · Common · ✅ read  
At the start of each combat, Forge 10.  
*Simulator:* Combat start: Forge 10.  
Forge 10 at combat start.

**Galactic Dust** · Uncommon · ○ not read  
For every 10 1 Star spent, gain 10 Block.  
10 Block per 10 Stars spent. Unread. `ON_STARS`

**Regalite** · Uncommon · ○ not read  
The first time you create a card each turn, gain 4 Block.  
4 Block on the first card created each turn. Unread. `ON_MAKE`

**Lunar Pastry** · Rare · ✅ read  
At the end of your turn, gain 1 Star.  
*Simulator:* End of each turn: gain 1 Stars.  

**Mini Regent** · Rare · ○ not read  
The first time you spend 1 Star each turn, gain 1 Strength.  
+1 Strength on the first Star spent each turn. Unread; Star spending is tracked. `ON_STARS`

**Orange Dough** · Rare · ○ not read  
At the start of each combat, add 2 random Colorless cards into your Hand.  
2 random Colorless cards at combat start. `RANDOM_GEN`

**Vitruvian Minion** · Shop · ○ not read  
Cards containing “Minion” deal double damage and gain double Block.  
"Minion" cards deal double damage and Block. Needs a named multiplier (`ExtraDamage` is additive). `NAMED_DMG_MULT`

### The Fool

**Song Bow** · Starter · ✅ read  
Start each combat with 6 Aim.  
*Simulator:* Combat start: gain 6 Aim.  
Fool starter: 6 Aim at combat start.

**Storyteller** · Starter · ✅ read  
Start each combat with 12 Aim. At the end of your turn, gain 3 Aim.  
*Simulator:* Combat start: gain 12 Aim; End of each turn: gain 3 Aim.  
Ancient Fool starter.

**Exercise Mat** · Common · ✅ read  
Start each combat with 6 Stretch.  
*Simulator:* Combat start: gain 6 Stretch.  

**Ocarina** · Uncommon · ○ not read  
At the start of each combat, add a random “Wah”, “Cho”, or “Ki” card into your Hand.  
A random Wah/Cho/Ki card at combat start. Unread. `RANDOM_GEN` `MELODY`

**Whittling Knife** · Uncommon · ✅ read  
Whenever you shuffle your Draw Pile, gain 5 Aim.  
*Simulator:* Whenever you shuffle: gain 5 Aim.  

**Conductor's Baton** · Rare · ○ not read  
When you gain every Melody, receive 25% less attack damage until your next turn.  
Less damage taken after completing every Melody. Survival, gated on Melody state. `MELODY_TRIGGER`

**Fertilizer** · Rare · ○ not read  
Whenever you create a Sadness, gain 1 Strength.  
+1 Strength per Sadness created. Unread. `ON_MAKE_NAMED`

**Twine** · Rare · ○ not read  
Whenever you Dislodge an Arrow, gain 4 Block.  
4 Block per dislodge. Unread; dislodges are tracked. `ON_DISLODGE`

**Bizarre Lamp** · Shop · ⚠️ misread  
At the start of each combat, add a Joy and a Sadness into your Hand.  
*Simulator:* Combat start: add 1 joy and a sadness to your Hand.  
"add a Joy and a Sadness" is parsed as one card named "joy and a sadness", which matches nothing, so neither is made. Split on "and". `MAKE_PARSE`

### Shared

**Amethyst Aubergine** · Common · · out of combat  
Enemies drop 15 additional Gold.  

**Anchor** · Common · ✅ read  
Start each combat with 10 Block.  
*Simulator:* Combat start: gain 10 Block.  
10 Block at combat start.

**Bag of Marbles** · Common · ✅ read  
At the start of each combat, apply 1 Vulnerable to ALL enemies.  
*Simulator:* Combat start: apply 1 Vulnerable.  
1 Vulnerable to all at combat start.

**Bag of Preparation** · Common · ✅ read  
At the start of each combat, draw 2 additional cards.  
*Simulator:* Combat start: draw 2.  

**Blood Vial** · Common · · out of combat  
At the start of each combat, heal 2 HP.  
Heal 2 at combat start. `HEAL`

**Book of Five Rings** · Common · · out of combat  
Every 5 cards you add to your Deck, heal 20 HP.  
`HEAL`

**Bronze Scales** · Common · ○ not read  
Start each combat with 3 Thorns.  
3 Thorns. Survival-side damage (per enemy hit). `THORNS`

**Centennial Puzzle** · Common · ○ not read  
The first time you lose HP each combat, draw 3 cards.  
Draw 3 on the first HP loss. Needs HP loss; self-damage cards (Hemokinesis, Offering) trigger it on your turn. `SELF_HP`

**Festive Popper** · Common · ✅ read  
At the start of each combat, deal 9 damage to ALL enemies.  
*Simulator:* Combat start: deal 9 damage to all.  
9 to all at combat start.

**Gorget** · Common · ✅ read  
At the start of each combat, gain 4 Plating.  
*Simulator:* Combat start: gain 4 Plating.  
4 Plating at combat start.

**Happy Flower** · Common · ✅ read  
Every 3 turns, gain 1 Energy.  
*Simulator:* Every 3 turns: gain 1 Energy.  
+1 energy every 3 turns.

**Juzu Bracelet** · Common · · out of combat  
Regular enemy combats are no longer encountered in ? rooms.  

**Lantern** · Common · ✅ read  
Start each combat with an additional 1 Energy.  
*Simulator:* Combat start: gain 1 Energy.  

**Meal Ticket** · Common · · out of combat  
Whenever you enter a shop room, heal 15 HP.  
`HEAL`

**Oddly Smooth Stone** · Common · ✅ read  
Start each combat with 1 Dexterity.  
*Simulator:* Combat start: gain 1 Dexterity.  

**Pendulum** · Common · ✅ read  
Every 3 turns, draw 1 card.  
*Simulator:* Every 3 turns: draw 1.  

**Potion Belt** · Common · · out of combat  
Upon pickup, gain 2 potion slots.  
`POTION`

**Red Mask** · Common · ○ not read  
At the start of each combat, apply 1 Weak to ALL enemies.  
1 Weak to all at combat start. Unread: the trigger effects have no Weak (`Effect` lacks it). `WEAK_EFFECT`

**Regal Pillow** · Common · · out of combat  
Whenever you Rest, heal an additional 15 HP.  
`HEAL`

**Strawberry** · Common · · out of combat  
Upon pickup, raise your Max HP by 7.  
+7 Max HP.

**Strike Dummy** · Common · ✅ read  
Cards containing “Strike” deal 3 additional damage.  
*Simulator:* a card named "strike" deals 3 more damage.  
+3 to Strike cards.

**Vajra** · Common · ✅ read  
Start each combat with 1 Strength.  
*Simulator:* Combat start: gain 1 Strength.  

**Venerable Tea Set** · Common · ○ not read  
Whenever you enter a Rest Site, start the next combat with an additional 2 Energy.  
+2 energy in the fight after a rest site. Run-dependent; the run state knows the previous room. `ENERGY_NEXT_COMBAT`

**War Paint** · Common · · out of combat  
Upon pickup, Upgrade 2 random Skills.  

**Whetstone** · Common · · out of combat  
Upon pickup, Upgrade 2 random Attacks.  

**Akabeko** · Uncommon · ✅ read  
At the start of each combat, gain 8 Vigor.  
*Simulator:* Combat start: gain 8 Vigor.  
8 Vigor at combat start.

**Bowler Hat** · Uncommon · · out of combat  
Gain 25% additional Gold.  

**Candelabra** · Uncommon · ✅ read  
At the start of your 2nd turn, gain 2 Energy.  
*Simulator:* Turn 2: gain 2 Energy.  

**Eternal Feather** · Uncommon · · out of combat  
For every 5 cards in your Deck, heal 3 HP whenever you enter a Rest Site.  
`HEAL`

**Gremlin Horn** · Uncommon · ○ not read  
Whenever an enemy dies, gain 1 Energy and draw 1 card.  
+1 energy and draw 1 per enemy death. Multi-enemy fights only. `FATAL`

**Horn Cleat** · Uncommon · ✅ read  
At the start of your 2nd turn, gain 14 Block.  
*Simulator:* Turn 2: gain 14 Block.  

**Joss Paper** · Uncommon · ○ not read  
Every 5 times you Exhaust a card, draw 1 card.  
Draw 1 every 5 exhausts. Unread; `EveryPlays` has the counter, it needs an exhaust filter. `EVERY_EXHAUST`

**Kusarigama** · Uncommon · ✅ read  
Every time you play 3 Attacks in a single turn, deal 6 damage to a random enemy.  
*Simulator:* Every 3 plays of an Attack in a turn: deal 6 damage.  

**Lasting Candy** · Uncommon · · out of combat  
Every other combat, your card rewards gain an additional Power.  

**Letter Opener** · Uncommon · ✅ read  
Every time you play 3 Skills in a single turn, deal 5 damage to ALL enemies.  
*Simulator:* Every 3 plays of a Skill in a turn: deal 5 damage to all.  

**Lucky Fysh** · Uncommon · · out of combat  
Whenever you add a card to your Deck, gain 15 Gold.  

**Mercury Hourglass** · Uncommon · ✅ read  
At the start of your turn, deal 3 damage to ALL enemies.  
*Simulator:* Each turn: deal 3 damage to all.  

**Miniature Cannon** · Uncommon · ✅ read  
Upgraded Attacks deal 3 additional damage.  
*Simulator:* an upgraded Attack deals 3 more damage.  
+3 to upgraded Attacks.

**Nunchaku** · Uncommon · ✅ read  
Every time you play 10 Attacks, gain 1 Energy.  
*Simulator:* Every 10 plays of an Attack: gain 1 Energy.  

**Orichalcum** · Uncommon · ○ not read  
If you end your turn without Block, gain 6 Block.  
6 Block if you end the turn with none. Cheap: the turn's Block is logged. `BLOCK_COND`

**Ornamental Fan** · Uncommon · ✅ read  
Every time you play 3 Attacks in a single turn, gain 4 Block.  
*Simulator:* Every 3 plays of an Attack in a turn: gain 4 Block.  

**Pantograph** · Uncommon · · out of combat  
At the start of each Boss combat, heal 25 HP.  
Heal 25 at the start of boss fights (counts for the boss's survival check). `HEAL`

**Parrying Shield** · Uncommon · ○ not read  
If you end a turn with at least 10 Block, deal 6 damage to a random enemy.  
6 damage if you end the turn with 10+ Block. `BLOCK_COND`

**Pear** · Uncommon · · out of combat  
Upon pickup, raise your Max HP by 10.  

**Pen Nib** · Uncommon · ○ not read  
Every 10th Attack you play deals double damage.  
Every 10th Attack deals double. `EveryPlays` + a damage multiplier. `EVERY_ATTACK_MULT`

**Permafrost** · Uncommon · ○ not read  
The first time you play a Power each combat, gain 7 Block.  
7 Block on the first Power each combat. `FIRST_POWER`

**Petrified Toad** · Uncommon · · out of combat  
At the start of each combat, procure a Potion-Shaped Rock.  
`POTION`

**Planisphere** · Uncommon · · out of combat  
Whenever you enter a ? room, heal 5 HP.  
`HEAL`

**Reptile Trinket** · Uncommon · ○ not read  
Whenever you use a potion, gain 3 Strength this turn.  
`POTION`

**Ripple Basin** · Uncommon · ○ not read  
If you did not play any Attacks during your turn, gain 4 Block.  
4 Block if no Attacks were played this turn. `COND_NO_ATTACK`

**Sparkling Rouge** · Uncommon · ✅ read  
At the start of your 3rd turn, gain 1 Strength and 1 Dexterity.  
*Simulator:* Turn 3: gain 1 Strength, gain 1 Dexterity.  

**Stone Cracker** · Uncommon · ○ not read  
At the start of each combat, Upgrade 2 random cards in your Draw Pile for the rest of combat.  
2 random Draw Pile cards upgraded at combat start. `UPGRADE`

**Tiny Mailbox** · Uncommon · · out of combat  
Whenever you Rest, procure 2 random Potions.  
`POTION`

**Tuning Fork** · Uncommon · ✅ read  
Every time you play 10 Skills, gain 7 Block.  
*Simulator:* Every 10 plays of a Skill: gain 7 Block.  

**Vambrace** · Uncommon · ○ not read  
The first time you gain Block from a card each combat, double the amount gained.  
Doubles the first Block from a card each combat. `FIRST_GAIN_MULT`

**Art of War** · Rare · ○ not read  
If you do not play any Attacks during your turn, gain an additional 1 Energy next turn.  
+1 energy next turn if no Attacks were played. Unread. `COND_NO_ATTACK`

**Beating Remnant** · Rare · ○ not read  
You cannot lose more than 20 HP in a single turn.  
HP loss capped at 20 a turn. Survival. `DMG_CAP`

**Bellows** · Rare · ○ not read  
The first Hand you draw each combat is Upgraded.  
The opening hand is upgraded. Unread. `UPGRADE`

**Captain's Wheel** · Rare · ✅ read  
At the start of your 3rd turn, gain 18 Block.  
*Simulator:* Turn 3: gain 18 Block.  
18 Block on turn 3.

**Chandelier** · Rare · ✅ read  
At the start of your 3rd turn, gain 3 Energy.  
*Simulator:* Turn 3: gain 3 Energy.  
+3 energy on turn 3.

**Cloak Clasp** · Rare · ○ not read  
At the end of your turn, gain 1 Block for each card in your Hand.  
1 Block per card left in hand at end of turn. `COUNT_HAND`

**Frozen Egg** · Rare · · out of combat  
Whenever you add a Power into your Deck, Upgrade it.  

**Gambling Chip** · Rare · ○ not read  
At the start of each combat, discard any number of cards then draw that many.  
Discard any number and redraw at combat start. `MULLIGAN`

**Game Piece** · Rare · ✅ read  
Whenever you play a Power, draw 1 card.  
*Simulator:* Whenever you play a Power: draw 1.  
Draw 1 per Power.

**Girya** · Rare · · out of combat  
You can now gain Strength at Rest Sites. (3 times max)  
Strength at rest sites (3 max); permanent Strength lives in the run.

**Ice Cream** · Rare · ✅ read  
Energy is now conserved between turns.  
*Simulator:* Unspent Energy carries over.  
Energy carries over.

**Intimidating Helmet** · Rare · ✅ read  
Whenever you play a card that costs 2 Energy or more, gain 4 Block.  
*Simulator:* Whenever you play a card costing 2+: gain 4 Block.  
4 Block per card costing 2+.

**Kunai** · Rare · ✅ read  
Every time you play 3 Attacks in a single turn, gain 1 Dexterity.  
*Simulator:* Every 3 plays of an Attack in a turn: gain 1 Dexterity.  

**Lizard Tail** · Rare · ○ not read  
When your HP would be reduced to 0, heal to 50% of your Max HP instead (works once).  
Revive at 50% once. Survival. `REVIVE`

**Mango** · Rare · · out of combat  
Upon pickup, raise your Max HP by 14.  

**Meat on the Bone** · Rare · · out of combat  
If your HP is at or below 50% at the end of combat, heal 12 HP.  
`HEAL`

**Molten Egg** · Rare · · out of combat  
Whenever you add an Attack card to your Deck, Upgrade it.  

**Mummified Hand** · Rare · ○ not read  
Whenever you play a Power, a random card in your Hand is free to play that turn.  
A random hand card is free after each Power. Unread. `FREE_RANDOM`

**Old Coin** · Rare · · out of combat  
Upon pickup, gain 300 Gold.  

**Pocketwatch** · Rare · ○ not read  
Whenever you play 3 or fewer cards during your turn, draw 3 additional cards at the start of your next turn.  
+3 draw next turn after a turn of 3 or fewer plays. Unread. `COUNTER_PLAYS`

**Prayer Wheel** · Rare · · out of combat  
Normal enemies drop an additional card reward.  

**Rainbow Ring** · Rare · ○ not read  
The first time you play an Attack, Skill, and Power each turn, gain 1 Strength and 1 Dexterity.  
+1 Strength and +1 Dexterity when an Attack, Skill and Power are all played in a turn. Unread. `TYPE_SET`

**Razor Tooth** · Rare · ○ not read  
Every time you play an Attack or Skill, Upgrade it for the remainder of combat.  
Attacks and Skills upgrade when played. Unread; switch each play to its upgraded version after the first. `UPGRADE`

**Shovel** · Rare · · out of combat  
You can now dig at Rest Sites to obtain a random Relic.  

**Shuriken** · Rare · ✅ read  
Every time you play 3 Attacks in a single turn, gain 1 Strength.  
*Simulator:* Every 3 plays of an Attack in a turn: gain 1 Strength.  

**Stone Calendar** · Rare · ○ not read  
At the end of turn 7, deal 52 damage to ALL enemies.  
52 to all at the end of turn 7. Unread; `TurnNumber(7)` + `DamageAll` exist, so the reader just needs "end of turn 7". `DELAYED_DMG`

**Sturdy Clamp** · Rare · ○ not read  
Up to 10 Block persists across turns.  
Up to 10 Block carries over. `BLOCK_KEEP`

**The Courier** · Rare · · out of combat  
The merchant no longer runs out of cards, relics, or potions and his prices are reduced by 20%.  

**Toxic Egg** · Rare · · out of combat  
Whenever you add a Skill into your Deck, Upgrade it.  

**Tungsten Rod** · Rare · ○ not read  
Whenever you would lose HP, lose 1 less.  
−1 HP loss per instance. Survival. `DMG_REDUCE`

**Unceasing Top** · Rare · ○ not read  
Whenever you have no cards in Hand during your turn, draw a card.  
Draw when the hand empties. Unread. `DRAW_EMPTY`

**Unsettling Lamp** · Rare · ○ not read  
Each combat, the first time you play a card that Debuffs an enemy, double its effect.  
Doubles the first debuffing card each combat. `DEBUFF_AMP`

**Vexing Puzzlebox** · Rare · ○ not read  
At the start of each combat, add a random card into your Hand. It's free to play this turn.  
A random free card at combat start. `RANDOM_GEN`

**White Beast Statue** · Rare · · out of combat  
Potions always appear in combat rewards.  

**White Star** · Rare · · out of combat  
Elites drop an additional Rare card reward.  

**Belt Buckle** · Shop · ○ not read  
While you have no Potions, you have 2 additional Dexterity.  
+2 Dexterity while you hold no potions. Potions are run state. `POTION`

**Bread** · Shop · ⚠️ misread  
At the start of your first turn, lose 2 Energy. At the start of all other turns, gain 1 Energy.  
*Simulator:* Turn 1: lose 2 Energy.  
Turn 1 −2 energy read; "+1 energy on all other turns" isn't, so it reads as a pure loss. `TRAILING_TIMING`

**Burning Sticks** · Shop · ○ not read  
The first time each combat you Exhaust a Skill, add a copy of it into your Hand.  
A copy of the first Skill exhausted each combat. Unread. `ON_EXHAUST_TYPE`

**Cauldron** · Shop · · out of combat  
Upon pickup, brews 5 random Potions.  
`POTION`

**Chemical X** · Shop · ○ not read  
The effects of your cost X cards are increased by 2.  
X-cost cards get X+2. Unread; one line where `times` is set. `X_BONUS`

**Dingy Rug** · Shop · · out of combat  
Card rewards can now contain Colorless cards.  

**Dolly's Mirror** · Shop · · out of combat  
Upon pickup, obtain an additional copy of a card in your Deck.  

**Dragon Fruit** · Shop · · out of combat  
Whenever you gain Gold, raise your Max HP by 1.  

**Ghost Seed** · Shop · ○ not read  
Strikes and Defends gain Ethereal.  
Strikes and Defends gain Ethereal (exhaust when unplayed). Turns starter cards into exhaust fuel. `GRANT_KEYWORD`

**Gnarled Hammer** · Shop · · out of combat  
Upon pickup, Enchant up to 3 Attacks with Sharp 3.  
Sharp 3 on 3 Attacks. `ENCHANT`

**Kifuda** · Shop · · out of combat  
Upon pickup, Enchant up to 3 cards with Adroit.  
Adroit on 3 cards. `ENCHANT`

**Lava Lamp** · Shop · · out of combat  
At the end of combat, Upgrade all card rewards if you took no damage.  

**Lee's Waffle** · Shop · · out of combat  
Upon pickup, raise your Max HP by 7 and heal all of your HP.  
`HEAL`

**Membership Card** · Shop · · out of combat  
50% discount on all products!  

**Miniature Tent** · Shop · · out of combat  
You may choose any number of options at Rest Sites.  

**Mystic Lighter** · Shop · ○ not read  
Enchanted Attacks deal 9 additional damage.  
Enchanted Attacks deal +9. Needs enchantments in the simulator. `ENCHANT`

**Orrery** · Shop · · out of combat  
Upon pickup, gain 5 card rewards.  

**Punch Dagger** · Shop · · out of combat  
Upon pickup, Enchant an Attack with Momentum 5.  
Momentum 5 on an Attack. `ENCHANT`

**Ringing Triangle** · Shop · ○ not read  
Retain your Hand on the first turn of combat.  
Retain the hand on turn 1. `RETAIN_TEMP`

**Royal Stamp** · Shop · · out of combat  
Upon pickup, choose an Attack or Skill in your Deck to Enchant with Royally Approved.  
Royally Approved (Innate and Retain) on one card. `ENCHANT`

**Screaming Flagon** · Shop · ○ not read  
If you end your turn with no cards in your Hand, deal 20 damage to ALL enemies.  
20 to all if you end the turn with an empty hand. Unread; cheap. `EMPTY_HAND`

**Sling of Courage** · Shop · ○ not read  
Start each Elite combat with 2 Strength.  
+2 Strength in elite fights. `ENCOUNTER_KIND`

**The Abacus** · Shop · ✅ read  
Whenever you shuffle your Draw Pile, gain 6 Block.  
*Simulator:* Whenever you shuffle: gain 6 Block.  
6 Block per shuffle.

**Toolbox** · Shop · ○ not read  
At the start of each combat, choose 1 of 3 random Colorless cards and add the chosen card into your Hand.  
1 of 3 Colorless cards at combat start. `RANDOM_GEN`

**Wing Charm** · Shop · · out of combat  
A random card in each card reward is Enchanted with Swift 1.  
`ENCHANT`

**Anchor???** · Event · ✅ read  
Start each combat with 4 Block.  
*Simulator:* Combat start: gain 4 Block.  

**Bath Robes** · Event · · out of combat  
At the start of the next Boss combat, heal all of your HP.  
`HEAL`

**Bath Towel** · Event · · out of combat  
At the start of each combat, heal 8 HP.  
`HEAL`

**Big Mushroom** · Event · ○ not read  
Upon pickup, raise your Max HP by 20. At the start of each combat, draw 2 fewer cards.  
+20 Max HP, and draw 2 fewer on turn 1. The draw penalty is unread. `DRAW_PENALTY`

**Bing Bong** · Event · · out of combat  
Whenever you add a card to your Deck, add one additional copy.  

**Blood Vial???** · Event · · out of combat  
At the start of each combat, heal 1 HP.  
`HEAL`

**Bone Tea** · Event · ○ not read  
At the start of the next combat, Upgrade your starting hand.  
Next combat's opening hand is upgraded. One fight only. `UPGRADE`

**Byrdine** · Event · ○ not read  
Upon pickup, Transform Byrd Swoop into God Byrd. A Byrdine will accompany you in battles.  
A Byrdine companion in battle. The data doesn't say what it does. `COMPANION`

**Byrdpip** · Event · ○ not read  
Upon pickup, gain the card Byrd Swoop. A Byrdpip will accompany you in battles. Once you play Byrd Swoop 6 times, you can evolve Byrdpip at a Rest Site.  
A Byrdpip companion. The data doesn't say what it does. `COMPANION`

**Darkstone Periapt** · Event · · out of combat  
Whenever you obtain a Curse, raise your Max HP by 6.  

**Daughter of the Wind** · Event · ✅ read  
Whenever you play an Attack, gain 1 Block.  
*Simulator:* Whenever you play an Attack: gain 1 Block.  
1 Block per Attack.

**Dream Catcher** · Event · · out of combat  
Whenever you Rest, you may add a card to your Deck.  

**Ember Tea** · Event · ○ not read  
At the start of the next 5 combats, gain 2 Strength.  
+2 Strength for the next 5 combats. Run state needs a remaining-uses counter (the game save may carry it). `LIMITED_COMBATS`

**Forgotten Soul** · Event · ✅ read  
Whenever you Exhaust a card, deal 1 damage to a random enemy.  
*Simulator:* Whenever a card is exhausted: deal 1 damage.  
1 damage per exhaust.

**Fragrant Mushroom** · Event · · out of combat  
Upon pickup, lose 15 HP and Upgrade 2 random cards.  

**Fresnel Lens** · Event · · out of combat  
Whenever you add a card that gains Block to your Deck, Enchant it with Nimble 2.  
`ENCHANT`

**Hand Drill** · Event · ○ not read  
Whenever you break an enemy's Block, apply 2 Vulnerable.  
2 Vulnerable when you break an enemy's Block. The simulator's target has no Block. `ENEMY_BLOCK`

**Happy Flower???** · Event · ✅ read  
Every 5 turns, gain 1 Energy.  
*Simulator:* Every 5 turns: gain 1 Energy.  

**History Course** · Event · ○ not read  
At the start of your turn, play a copy of your last played Attack.  
Replays the last Attack each turn. Unread. `AUTOPLAY`

**Lee's Waffle???** · Event · · out of combat  
Upon pickup, heal 10% of your HP.  
`HEAL`

**Lost Wisp** · Event · ✅ read  
Whenever you play a Power, deal 8 damage to ALL enemies.  
*Simulator:* Whenever you play a Power: deal 8 damage to all.  
8 to all per Power played.

**Mango???** · Event · · out of combat  
Upon pickup, raise your Max HP by 3.  

**Maw Bank** · Event · · out of combat  
Whenever you climb a floor, gain 12 Gold. No longer works when you spend any Gold at the shop.  

**Mighty Wings** · Event · ○ not read  
Whenever you deal 30 or more attack damage in a single hit, gain 2 Strength.  
+2 Strength on a 30+ hit. Hit sizes are computed per hit, so this is cheap. `HIT_THRESHOLD`

**Mr. Struggles** · Event · ○ not read  
At the start of your turn, deal damage equal to the turn number to ALL enemies.  
Damage to all equal to the turn number, each turn. Unread; cheap. `TURN_SCALE`

**Obsidian Vice** · Event · ○ not read  
At the end of your turn, deal 1 damage to a random enemy for each card in your Hand.  
1 damage per card left in hand at end of turn. `COUNT_HAND`

**Orichalcum???** · Event · ○ not read  
If you end your turn without Block, gain 3 Block.  
3 Block if you end the turn with none. `BLOCK_COND`

**Pollinous Core** · Event · ✅ read  
Every 4 turns, draw 2 additional cards.  
*Simulator:* Every 4 turns: draw 2.  
+2 draw every 4 turns.

**Royal Poison** · Event · ○ not read  
At the start of each combat, lose 4 HP.  
Lose 4 HP at combat start. Survival. `SELF_HP`

**Snecko Eye???** · Event · ○ not read  
Start each combat Confused.  
Confused with no upside. `CONFUSED`

**Strike Dummy???** · Event · ✅ read  
Cards containing “Strike” deal 1 additional damage.  
*Simulator:* a card named "strike" deals 1 more damage.  

**Sword of Jade** · Event · ✅ read  
Start each combat with 3 Strength.  
*Simulator:* Combat start: gain 3 Strength.  

**Sword of Stone** · Event · · out of combat  
Transforms into a powerful Relic after defeating 5 Elites.  

**Tea of Discourtesy** · Event · ○ not read  
At the start of the next combat, shuffle 2 Dazed into your Draw Pile.  
2 Dazed in the next combat's Draw Pile. `MAKE_PARSE`

**The Boot** · Event · ○ not read  
Whenever you would deal 4 or less unblocked attack damage, increase it to 5.  
Attack hits under 5 become 5. Matters for Shivs, multi-hits and chip damage. Cheap: `max(5)` per hit. `MIN_HIT`

**The Chosen Cheese** · Event · · out of combat  
At the end of combat, gain 1 Max HP.  

**The Merchant's Rug???** · Event · · out of combat  
Poor imitation. Does nothing.  
Does nothing.

**Thumbscrew** · Event · ✅ read  
Attacks that hit multiple times hit one additional time.  
*Simulator:* Multi-hit attacks hit once more.  
Multi-hit attacks hit once more.

**Tormentous Collar** · Event · ○ not read  
At the start of Elite combats, apply 2 Weak to ALL enemies.  
2 Weak to all at the start of elite fights. `ENCOUNTER_KIND` `WEAK_EFFECT`

**Venerable Tea Set???** · Event · ○ not read  
Whenever you enter a Rest Site, start the next combat with an additional 1 Energy.  
`ENERGY_NEXT_COMBAT`

**Wongo Customer Appreciation Badge** · Event · · out of combat  
Does nothing.  
Does nothing.

**Wongo's Mystery Ticket** · Event · · out of combat  
Receive 3 random Relics after 5 combats.  

**Wuppy** · Event · ○ not read  
Wuppy has 60/60 HP. Wuppy absorbs ALL attack damage in combat.  
A 60-HP Wuppy absorbs all attack damage. Survival: 60 extra effective HP per fight. `COMPANION`

**Alchemical Coffer** · Ancient · · out of combat  
Upon pickup, gain 4 potion slots filled with random Potions.  
`POTION`

**Arcane Scroll** · Ancient · · out of combat  
Upon pickup, obtain a random Rare Card to add to your Deck.  
Adds a card; the deck reflects it.

**Archaic Tooth** · Ancient · · out of combat  
Upon pickup, Transform a starter card into an ancient version.  
Transforms a starter card into its Ancient version.

**Astrolabe** · Ancient · · out of combat  
Upon pickup, Transform 3 cards, then Upgrade them.  

**Beautiful Bracelet** · Ancient · · out of combat  
Upon pickup, Enchant 4 random cards in your Deck with Swift 2.  
Swift 2 on 4 cards: those cards draw 2 the first time they are played. The enchantment sits on the deck card; the simulator ignores it. `ENCHANT`

**Biiig Hug** · Ancient · ✅ read  
Upon pickup, remove 4 cards from your Deck. Whenever you shuffle your Draw Pile, add a Soot into your Draw Pile.  
*Simulator:* Whenever you shuffle: add 1 soot to your Draw.  
Soot into the Draw Pile on each shuffle (read); the removal is already in the deck.

**Black Star** · Ancient · · out of combat  
Elites drop an additional Relic when defeated.  

**Blessed Antler** · Ancient · ◐ partly read  
Gain 1 Energy at the start of each turn. At the start of each combat, shuffle 3 Dazed into your Draw Pile.  
*Simulator:* Each turn: gain 1 Energy.  
+1 energy each turn read; "shuffle 3 Dazed into your Draw Pile" at combat start isn't (the reader knows "add", not "shuffle"). `MAKE_PARSE`

**Blood-Soaked Rose** · Ancient · ✅ read  
Upon pickup, add 1 Enthralled to your Deck. Gain 1 Energy at the start of each turn.  
*Simulator:* Each turn: gain 1 Energy.  
+1 energy each turn; the Enthralled curse is in the deck.

**Booming Conch** · Ancient · ○ not read  
At the start of Elite combats, draw 2 additional cards and gain 1 Energy.  
Elite fights only: +2 draw and +1 energy at the start. Needs the encounter kind passed into the simulation. `ENCOUNTER_KIND`

**Brilliant Scarf** · Ancient · ○ not read  
The 6th card you play from your hand each turn is free.  
The 6th card each turn is free. Unread. `COUNTER_PLAYS`

**Calling Bell** · Ancient · · out of combat  
Upon pickup, obtain a unique Curse and 3 Relics.  

**Choices Paradox** · Ancient · ○ not read  
At the start of each combat, add 1 of 5 random cards into your Hand. Add Retain to the chosen card.  
1 of 5 random cards at combat start, with Retain. `RANDOM_GEN`

**Claws** · Ancient · · out of combat  
Upon pickup, Transform up to 6 cards into Maul.  
Transforms cards into Maul.

**Crossbow** · Ancient · ○ not read  
At the start of your turn, add a random Attack into your Hand. It's free to play this turn.  
A random free Attack each turn. `RANDOM_GEN`

**Cursed Pearl** · Ancient · · out of combat  
Upon pickup, receive Greed. Gain 333 Gold.  

**Delicate Frond** · Ancient · · out of combat  
At the start of each combat, fill all empty potion slots with random Potions.  
`POTION`

**Diamond Diadem** · Ancient · ◐ partly read  
Start combat with 20 Block. Your Block is not removed at the start of your 2nd turn.  
*Simulator:* Combat start: gain 20 Block.  
20 Block at combat start read; keeping Block into turn 2 isn't. The simulator logs Block per turn, so the 20 counts once. `BLOCK_KEEP`

**Distinguished Cape** · Ancient · · out of combat  
Upon pickup, add 2 random Curses and 3 Apparitions to your Deck.  
Adds curses and Apparitions.

**Dowsing Rod** · Ancient · · out of combat  
Upon pickup, add 1 Dowsing to your Deck.  

**Driftwood** · Ancient · · out of combat  
You may reroll each card reward once.  

**Dusty Tome** · Ancient · · out of combat  
Upon pickup, obtain an Ancient Card.  

**Ectoplasm** · Ancient · ✅ read  
You can no longer gain Gold. Gain 1 Energy at the start of each turn.  
*Simulator:* Each turn: gain 1 Energy.  
+1 energy each turn.

**Electric Shrymp** · Ancient · · out of combat  
Upon pickup, Enchant a Skill with Imbued.  
Imbued: that Skill plays itself at the start of each combat. Not simulated. `ENCHANT`

**Empty Cage** · Ancient · · out of combat  
Upon pickup, remove 2 cards from your Deck.  

**Fiddle** · Ancient · ◐ partly read  
At the start of each turn, draw 2 additional cards. You may not draw cards during your turn.  
*Simulator:* Each turn: draw 2.  
+2 draw at turn start read; "no draws during your turn" isn't, so draw cards still draw. `DRAW_LOCK`

**Fishing Rod** · Ancient · · out of combat  
Every 3 normal combats, Upgrade a random card in your Deck.  

**Fur Coat** · Ancient · · out of combat  
Upon pickup, mark 8 random combats. Enemies in those rooms have 1 HP.  

**Glass Eye** · Ancient · · out of combat  
Upon pickup, obtain 2 Common cards, 2 Uncommon cards, and 1 Rare card.  

**Glitter** · Ancient · · out of combat  
Enchant all card rewards with Glam.  
Glam on card rewards (Replay once per combat). `ENCHANT`

**Golden Compass** · Ancient · · out of combat  
Upon pickup, replace the Act 2 Map with a single special path.  

**Golden Pearl** · Ancient · · out of combat  
Upon pickup, gain 150 Gold.  

**Hefty Tablet** · Ancient · · out of combat  
Upon pickup, choose 1 of 3 Rare cards to add to your Deck, and add 1 Injury to your Deck.  

**Iron Club** · Ancient · ✅ read  
Every 4 cards you play, draw 1 card.  
*Simulator:* Every 4 plays of a card: draw 1.  
Draw 1 every 4 plays.

**Jeweled Mask** · Ancient · ○ not read  
At the start of each combat, put a random Power from your Draw Pile into your Hand. It's free to play this combat.  
A Draw Pile Power to the opening hand, free. Unread. `TUTOR`

**Jewelry Box** · Ancient · · out of combat  
Upon pickup, add 1 Apotheosis to your Deck.  

**Kaleidoscope** · Ancient · · out of combat  
Upon pickup, obtain 2 card rewards from other characters.  

**Large Capsule** · Ancient · · out of combat  
Upon pickup, obtain 2 random Relics. Add an additional Strike and Defend to your Deck.  

**Lava Rock** · Ancient · · out of combat  
The Act 1 Boss drops 2 Relics.  

**Lead Paperweight** · Ancient · · out of combat  
Upon pickup, choose 1 of 2 Colorless cards to add to your Deck.  

**Leafy Poultice** · Ancient · · out of combat  
Upon pickup, Transform 1 of your Strikes and 1 of your Defends and lose 12 Max HP.  

**Looming Fruit** · Ancient · · out of combat  
Upon pickup, raise your Max HP by 31.  
+31 Max HP.

**Lord's Parasol** · Ancient · · out of combat  
When you encounter the Merchant, immediately obtain EVERYTHING he sells.  

**Lost Coffer** · Ancient · · out of combat  
Upon pickup, gain 1 card reward and procure 1 random potion.  

**Massive Scroll** · Ancient · · out of combat  
Upon pickup, choose 1 of 3 Multiplayer Cards to add to your Deck.  

**Meat Cleaver** · Ancient · · out of combat  
You may Cook at Rest Sites.  

**Music Box** · Ancient · ○ not read  
Create an Ethereal copy of the first Attack you play each turn.  
Ethereal copy of the first Attack each turn. Unread. `COPY_PLAYED`

**Neow's Bones** · Ancient · · out of combat  
Upon pickup, gain 2 random Neow Relics. Add 1 random Curse to your Deck.  

**Neow's Sacrifice** · Ancient · · out of combat  
Upon pickup, procure 1 Ambergris and add 1 Guilty to your Deck.  

**Neow's Talisman** · Ancient · · out of combat  
Upon pickup, Upgrade 1 of your Strikes and 1 of your Defends.  

**Neow's Torment** · Ancient · · out of combat  
Upon pickup, add 1 Neow's Fury to your Deck.  

**New Leaf** · Ancient · · out of combat  
Upon pickup, Transform 1 card.  

**Nutritious Oyster** · Ancient · · out of combat  
Upon pickup, raise your Max HP by 11.  

**Nutritious Soup** · Ancient · · out of combat  
Upon pickup, Enchant all Strikes in your Deck with Tezcatara's Ember.  
Tezcatara's Ember on every Strike: cost 0, +3 damage, Eternal. A big change the simulator misses. `ENCHANT`

**Pael's Blood** · Ancient · ✅ read  
At the start of your turn, draw 1 additional card.  
*Simulator:* Each turn: draw 1.  
+1 draw each turn.

**Pael's Claw** · Ancient · · out of combat  
Upon pickup, Enchant all Defends with Goopy.  
Goopy on every Defend: Exhaust, and permanent Block growth per play. `ENCHANT`

**Pael's Eye** · Ancient · ○ not read  
The first time each combat you end your turn without playing cards, Exhaust your Hand, and take an extra turn.  
The first turn you play nothing: exhaust the hand and take an extra turn. `EXTRA_TURN`

**Pael's Flesh** · Ancient · ✅ read  
Gain an additional 1 Energy at the start of your 3rd turn, and every turn after that.  
*Simulator:* Turn 3 onward: gain 1 Energy.  
+1 energy from turn 3.

**Pael's Growth** · Ancient · · out of combat  
Upon pickup, Enchant a card with Clone.  
`ENCHANT`

**Pael's Horn** · Ancient · · out of combat  
Upon pickup, add 2 Relax to your Deck.  
Adds 2 Relax.

**Pael's Legion** · Ancient · ○ not read  
Doubles Block gained from a card, then goes to sleep for 2 turns.  
Doubles a card's Block, then sleeps 2 turns. `BLOCK_MULT`

**Pael's Tears** · Ancient · ○ not read  
If you end your turn with unspent 1 Energy, gain an additional 2 Energy next turn.  
+2 energy next turn if energy was left unspent. Unread. `ENERGY_CARRY`

**Pael's Tooth** · Ancient · · out of combat  
Upon pickup, remove 5 cards from your Deck. After each combat, randomly add 1 back Upgraded.  

**Pael's Wing** · Ancient · · out of combat  
You may sacrifice card rewards to Pael. Every 2 sacrifices, obtain a Relic.  

**Pandora's Box** · Ancient · · out of combat  
Transform ALL Strikes and Defends.  

**Phial Holster** · Ancient · · out of combat  
Upon pickup, gain 1 potion slot and procure 2 random Potions.  
`POTION`

**Philosopher's Stone** · Ancient · ◐ partly read  
Gain 1 Energy at the start of each turn. ALL enemies start combat with 1 Strength.  
*Simulator:* Each turn: gain 1 Energy.  
+1 energy each turn read; enemies start with +1 Strength (survival). `ENEMY_STR`

**Pomander** · Ancient · · out of combat  
Upon pickup, Upgrade a card.  

**Precarious Shears** · Ancient · · out of combat  
Upon pickup, remove 2 cards from your Deck and lose 16 HP.  

**Precise Scissors** · Ancient · · out of combat  
Upon pickup, remove 1 card from your Deck.  

**Preserved Fog** · Ancient · · out of combat  
Upon pickup, remove 3 cards from your Deck. Add Folly to your Deck.  

**Prismatic Gem** · Ancient · ✅ read  
Gain 1 Energy at the start of each turn. Card rewards now contain cards from other colors.  
*Simulator:* Each turn: gain 1 Energy.  
+1 energy each turn; mixed card rewards are run value.

**Pumpkin Candle** · Ancient · ✅ read  
Gain 1 Energy at the start of each turn. Extinguishes after 5 combats. Can be Kindled at Rest Sites.  
*Simulator:* Each turn: gain 1 Energy.  
+1 energy each turn while lit (5 combats; can be rekindled at rest sites). The simulator assumes it is lit.

**Radiant Pearl** · Ancient · ✅ read  
At the start of each combat, add 1 Luminesce into your Hand.  
*Simulator:* Combat start: add 1 luminesce to your Hand.  
A Luminesce (Retain, 2 energy) at combat start.

**Runic Pyramid** · Ancient · ✅ read  
At the end of your turn, you no longer discard your Hand.  
*Simulator:* Keep your hand.  
The hand is kept.

**Sai** · Ancient · ✅ read  
At the start of your turn, gain 7 Block.  
*Simulator:* Each turn: gain 7 Block.  
7 Block each turn.

**Sand Castle** · Ancient · · out of combat  
Upon pickup, Upgrade 6 random cards.  

**Scroll Boxes** · Ancient · · out of combat  
Upon pickup, choose 1 of 2 packs of cards to add to your Deck.  

**Sea Glass** · Ancient · · out of combat  
See 15 cards from another character. Choose any number of them to add to your Deck.  

**Seal of Gold** · Ancient · ✅ read  
At the start of your turn, spend 3 Gold to gain 1 Energy.  
*Simulator:* Each turn: gain 1 Energy.  
+1 energy each turn for 3 Gold (assumes Gold lasts).

**Sere Talon** · Ancient · · out of combat  
Upon pickup, lose 9 Max HP. Add 3 Wishes to your Deck.  

**Signet Ring** · Ancient · · out of combat  
Upon pickup, gain 888 Gold.  

**Silken Tress** · Ancient · · out of combat  
Upon pickup, lose all Gold. Enchant all cards in your first card reward with Glam.  

**Silver Crucible** · Ancient · · out of combat  
The first 3 card rewards you see are Upgraded. The first Treasure Chest you open is empty.  

**Small Capsule** · Ancient · · out of combat  
Upon pickup, obtain a random Relic.  

**Snecko Eye** · Ancient · ◐ partly read  
At the start of your turn, draw 2 additional cards. Start each combat Confused.  
*Simulator:* Each turn: draw 2.  
+2 draw read; Confused (card costs randomised 0–3 on draw) isn't. Cheap decks get worse, expensive ones better. `CONFUSED`

**Sozu** · Ancient · ✅ read  
Gain 1 Energy at the start of each turn. You can no longer obtain Potions.  
*Simulator:* Each turn: gain 1 Energy.  

**Spiked Gauntlets** · Ancient · ◐ partly read  
Gain 1 Energy at the start of each turn. Powers cost 1 more 1 Energy.  
*Simulator:* Each turn: gain 1 Energy.  
+1 energy read; Powers costing 1 more isn't. `COST_UP_TYPE`

**Stone Humidifier** · Ancient · · out of combat  
Whenever you Rest at a Rest Site, raise your Max HP by 5.  

**Storybook** · Ancient · · out of combat  
Upon pickup, add 1 Brightest Flame to your Deck.  

**Tanx's Whistle** · Ancient · · out of combat  
Upon pickup, add 1 Whistle to your Deck.  

**Throwing Axe** · Ancient · ✅ read  
The first card you play each combat is played an extra time.  
*Simulator:* First card of combat plays twice.  
The first card each combat plays twice.

**Toasty Mittens** · Ancient · ◐ partly read  
At the start of your turn, Exhaust 1 card from your Hand and gain 1 Strength.  
*Simulator:* Each turn: gain 1 Strength.  
+1 Strength each turn read; exhausting a hand card each turn isn't (feeds exhaust payoffs). `EXH_CHOOSE`

**Touch of Orobas** · Ancient · · out of combat  
Upon pickup, replace your starter Relic with an Ancient version.  

**Toy Box** · Ancient · · out of combat  
Upon pickup, obtain 5 Wax Relics. Every 3 combats, your left-most Wax Relic will melt away.  
Wax relics: temporary copies of normal relics.

**Tri-Boomerang** · Ancient · · out of combat  
Choose 3 Attacks in your Deck. Enchant them with Instinct.  
Instinct (double attack damage) on 3 Attacks. Not simulated. `ENCHANT`

**Velvet Choker** · Ancient · ◐ partly read  
Gain 1 Energy at the start of each turn. You cannot play more than 6 cards per turn.  
*Simulator:* Each turn: gain 1 Energy.  
+1 energy read; the 6-card limit isn't. `PLAY_LIMIT`

**Very Hot Cocoa** · Ancient · ✅ read  
Start each combat with an additional 4 Energy.  
*Simulator:* Combat start: gain 4 Energy.  
+4 energy on turn 1.

**War Hammer** · Ancient · · out of combat  
Whenever you kill an Elite, Upgrade 4 random cards.  

**Whispering Earring** · Ancient · ◐ partly read  
Gain 1 Energy at the start of each turn. Vakuu plays your first turn for you.  
*Simulator:* Each turn: gain 1 Energy.  
+1 energy read; the game plays your first turn (presumably weaker than the greedy policy). `AUTOPILOT`

**Winged Boots** · Ancient · · out of combat  
You may ignore paths when choosing the next rooms to travel to 3 times.  

**Yummy Cookie** · Ancient · · out of combat  
Upon pickup, Upgrade 4 cards.  

**Circlet** · Relic · · out of combat  
It's a circlet.  
Placeholder relic.
