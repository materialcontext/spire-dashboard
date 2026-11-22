# Card and relic interactions

How cards and relics feed each other, class by class: first-order interactions (a producer feeds a payoff), second-order ones (a producer feeds a converter, which feeds a payoff), and loops. Each one notes whether the simulator can see it today. Built on [CARDS.md](CARDS.md) and [RELICS.md](RELICS.md), whose coverage marks (✅ read, ◐ partly, ○ not read, ⚠️ misread, · out of combat) appear on every name here.

Two layers:

- **Engines** (per class): hand-written from the card text. They cover the interactions that decide how a deck plays, what each link does, and what the simulator needs to follow them. Where the text leaves a rule open (does a cost-modified card count as "costs 2 or more"?), the entry says so.
- **Index and chains** (per class): generated from a resource model (`made`, `exhaust`, `Shiv`, `HP loss` and so on) that reads what each card and relic produces and pays off from its text. It lists every payoff with every producer in the class pool, then every converter chain. It is a pattern match on text, so treat it as a complete-but-rough map: it can miss an unusual phrasing or include a weak link. The engines section is the checked one.

A class pool is the class's cards, Colorless cards, tokens, event cards, the class's relics and the shared relics. Multiplayer-only cards are left out.

**What stands out**

- The simulator sees between 8% and 21% of the specific first-order interactions in each class (table below). The misses aren't spread evenly. A handful of untracked resources (your HP loss, discards and Sly, Melodies, Osty's HP, Doom as enemy state, kills) cut whole archetypes out.
- Ironclad's exhaust engine (Corruption, Feel No Pain, Dark Embrace, Juggernaut, Charon's Ashes) is the one engine the simulator follows end to end, second order included. The chosen-card exhausters that feed it (Brand, Burning Pact, True Grit) are not read.
- Five archetypes are invisible because one resource is missing: Ironclad self-damage, Silent discard and Sly, Necrobinder Osty scaling, Necrobinder Doom, and The Fool's Melodies.
- Several engines are misread rather than missed. Inferno, Rupture, Vicious, Shroud, Spirit of Ash, Arsenal, Pillar of Creation, Black Hole, Parry, Violent Chord, Song Path, Motif and All Is Well fire once on play instead of per event (`TRIGGER_FALLBACK`), so a single play gets credited with what the engine does over a whole fight.

## Resources

Each card and relic is modelled by what it produces and what it pays off. "Tracked" says whether the simulator carries the resource today; an interaction through an untracked resource is invisible to it whatever the cards' own coverage. Counts are producers / payoffs in each class pool (class cards, Colorless, tokens, event cards, class and shared relics).

| Resource | Tracked | Ironclad | Silent | Defect | Necrobinder | Regent | The Fool |
|---|---|---:|---:|---:|---:|---:|---:|
| Exhaust | yes | 77 / 17 | 69 / 6 | 74 / 6 | 70 / 7 | 66 / 6 | 65 / 6 |
| Discard | partly (whole-hand discards only) | 3 / 1 | 13 / 12 | 3 / 1 | 3 / 1 | 3 / 1 | 3 / 1 |
| Sly card discarded | no |  | 0 / 2 |  |  |  |  |
| Extra draw | yes | 41 / 2 | 44 / 4 | 44 / 3 | 38 / 3 | 42 / 3 | 42 / 1 |
| Shuffle | yes | 0 / 2 | 0 / 2 | 0 / 2 | 0 / 2 | 0 / 2 | 0 / 3 |
| HP loss (yours) | no | 10 / 9 | 2 / 2 | 2 / 3 | 3 / 2 | 2 / 2 | 2 / 2 |
| Block gain | yes | 44 / 12 | 42 / 10 | 41 / 8 | 42 / 8 | 42 / 8 | 43 / 8 |
| Energy | yes | 39 / 1 | 37 / 1 | 41 / 1 | 42 / 1 | 40 / 2 | 41 / 1 |
| Strength | yes | 20 / 14 | 11 / 8 | 12 / 10 | 11 / 8 | 15 / 11 | 12 / 10 |
| Vulnerable | yes | 13 / 8 |  |  | 8 / 1 |  |  |
| Weak | no (applied, not tracked) |  | 10 / 2 |  | 7 / 1 |  |  |
| Any debuff | counted | 16 / 2 | 27 / 2 | 11 / 2 | 22 / 4 | 13 / 2 | 13 / 2 |
| Poison | yes |  | 12 / 5 |  |  |  |  |
| Doom | tallied, not enemy state |  |  |  | 11 / 7 |  |  |
| Card created | yes |  |  |  |  | 49 / 4 |  |
| Status created | yes |  |  | 10 / 6 |  |  |  |
| Shiv | yes |  | 13 / 5 |  |  |  |  |
| Soul | yes |  |  |  | 7 / 3 |  |  |
| Sadness | yes |  |  |  |  |  | 7 / 6 |
| Stars gained | yes |  |  |  |  | 14 / 2 |  |
| Stars spent | yes |  |  |  |  | 22 / 3 |  |
| Sovereign Blade | yes |  |  |  |  | 12 / 4 |  |
| Orb channelled | yes |  |  | 27 / 6 |  |  |  |
| Lightning | yes |  |  | 9 / 4 |  |  |  |
| Frost | yes |  |  | 6 / 1 |  |  |  |
| Dark | yes |  |  | 6 / 1 |  |  |  |
| Evoke | partly (overflow only) |  |  | 5 / 1 |  |  |  |
| Summon (Osty HP) | tallied (no Osty HP) |  |  |  | 13 / 6 |  |  |
| Osty attack | partly (counted twice) |  |  |  | 12 / 6 |  |  |
| Ethereal card | yes |  |  |  | 10 / 7 |  |  |
| Retain | yes |  |  |  | 13 / 1 |  |  |
| Shot (Arrow spent) | yes |  |  |  |  |  | 13 / 8 |
| Impaled enemy | yes |  |  |  |  |  | 13 / 9 |
| Dislodge | yes |  |  |  |  |  | 3 / 3 |
| Aim | yes |  |  |  |  |  | 8 / 1 |
| Stretch | yes |  |  |  |  |  | 7 / 1 |
| Melody (Wah/Cho/Ki card) | no |  |  |  |  |  | 19 / 15 |
| Power played | yes | 33 / 4 | 31 / 4 | 34 / 8 | 32 / 4 | 32 / 4 | 32 / 4 |
| Skill played | yes | 96 / 3 | 109 / 5 | 104 / 2 | 100 / 2 | 103 / 4 | 97 / 2 |
| Attack played | yes | 76 / 15 | 66 / 11 | 68 / 10 | 74 / 11 | 71 / 11 | 75 / 10 |
| Multi-hit | yes | 13 / 1 | 8 / 2 | 10 / 1 | 7 / 3 | 10 / 2 | 10 / 1 |
| Cards played | yes | 205 / 6 | 206 / 9 | 206 / 7 | 206 / 7 | 206 / 10 | 204 / 6 |
| 0-cost card | yes |  |  | 76 / 4 |  |  |  |
| Card costing 2+ | yes | 44 / 1 | 46 / 1 | 40 / 1 | 40 / 4 | 35 / 1 | 38 / 1 |
| "Strike" card | yes | 9 / 4 | 5 / 2 | 8 / 2 | 6 / 2 | 6 / 2 | 6 / 2 |
| "Defend" card | yes | 2 / 3 | 2 / 3 | 2 / 3 | 2 / 3 | 2 / 3 | 2 / 3 |
| "Minion" card | yes |  |  |  |  | 3 / 1 |  |
| X-cost card | yes | 3 / 1 | 3 / 1 | 3 / 1 | 3 / 1 | 3 / 1 | 3 / 1 |
| Kill (Fatal) | no | 1 / 4 | 1 / 5 | 1 / 4 | 2 / 4 | 1 / 4 | 1 / 5 |
| Upgrade | no | 13 / 1 | 12 / 1 | 11 / 1 | 12 / 1 | 11 / 1 | 11 / 1 |
| Replay | partly | 9 / 1 | 8 / 1 | 8 / 1 | 8 / 1 | 8 / 1 | 9 / 1 |

## How much of it the simulator sees

A first-order interaction is one producer feeding one payoff through a resource. It is visible when the resource is tracked and both cards are read in full (✅). Wide resources (any Attack, any Skill, any card, any Block) are left out of the pair counts, because nearly every card produces them.

| Class | Payoff cards and relics | Producer → payoff pairs | Visible | Share |
|---|---:|---:|---:|---:|
| Ironclad | 62 | 1838 | 393 | 21% |
| Silent | 55 | 838 | 77 | 9% |
| Defect | 43 | 720 | 105 | 14% |
| Necrobinder | 59 | 973 | 91 | 9% |
| Regent | 44 | 753 | 109 | 14% |
| The Fool | 74 | 1119 | 93 | 8% |

## Across classes

Shared relics and Colorless cards that join any class's engine.

**Attack counters × cheap Attacks.** Kunai (Dexterity), Shuriken (Strength), Ornamental Fan (Block), Kusarigama (damage), Letter Opener (Skills), Nunchaku (energy per 10) and Pen Nib (every 10th Attack doubles) count plays, so 0-cost Attacks feed them. The feeders are Shivs, Anger and Adaptive Strike copies, Claw, Poke and Fetch, Wah Cho, Quick Shot, Slice and Flash of Steel. Second order: Shuriken's Strength lands on every later hit, so Shivs and multi-hit cards gain twice. Visible: the counters and most feeders are ✅. Pen Nib ○ and Finisher ◐ are the gaps.

**Exhaust payoffs × exhausting tokens.** Every Shiv, Soul, Joy, Fuel, Minion card and Tarot card exhausts when played, so Forgotten Soul ✅, Charon's Ashes ✅ (Ironclad), Joss Paper ○ and Burning Sticks ○ pay off token makers in any class. Toasty Mittens exhausts a hand card each turn, but only its Strength is read (◐).

**Ghost Seed** (Strikes and Defends gain Ethereal) turns the starter cards into exhaust fuel (Feel No Pain, Dark Embrace, Charon's Ashes) and into Necrobinder Ethereal fuel (Spirit of Ash, Pagestorm, Pull from Below, Banshee's Cry, Eidolon). Not read.

**Strike payoffs.** Strike Dummy ✅, Perfected Strike ◐ and Hellraiser ○ count "Strike" in a card's name. That includes Minion Strike (Regent), Ultimate Strike, Seeker Strike and every class's Strike cards. Nutritious Soup puts Tezcatara's Ember on every Strike (cost 0, +3 damage), which also makes them 0-cost Attacks for the counters above. The enchantment isn't simulated. Pandora's Box and Leafy Poultice transform Strikes away and work against all of this.

**Multi-hit.** Strength (any source), Thumbscrew ✅ (one more hit), The Boot ○ (hits under 5 become 5: Shivs, Sword Boomerang, Peck, Rapid Shot), Envenom ⚠️ (Poison per unblocked hit) and Reaper Form ○ (Doom per damage) all scale per hit.

**Big single hits.** Mighty Wings ○ gives +2 Strength on a 30+ hit. Bludgeon, Bury, Comet, Devastate, Divine Shot, Sky Shot and a Forged Sovereign Blade reach it. Pen Nib ○ doubles every 10th Attack, best on the same cards.

**X-cost.** Chemical X ○ (+2 to X) and Ice Cream ✅ (carried energy) feed Whirlwind, Skewer, Eradicate, Tempest, Multi-Cast, Brace, Volley, Dirge and Repeat Shot.

**Vulnerable and Weak amplifiers.** Paper Phrog ✅ and Cruelty ✅ work on Vulnerable. Paper Krane ○ and Tracking ○ work on Weak, which isn't tracked. Unsettling Lamp ○ doubles the first debuffing card each combat, and Debilitate ◐ doubles both debuffs.

**Statuses as fuel.** Biiig Hug (Soot each shuffle), Blessed Antler (3 Dazed), Tea of Discourtesy and Mighty Weapon/Shield (Dazed) add Statuses, which Defect's Status payoffs (Smokestack ⚠️, Trash to Treasure ○, Flak Cannon ✅, Iteration ○, Compact ◐, Rocket Punch ◐) turn into value. For other classes they are just dead draws.

**Anti-synergies.**

- Draw locks stop in-turn draw engines. Fiddle ◐ (no draws during your turn) and Battle Trance and Bullet Time (no more draws this turn) stop Dark Embrace, Speedster, Souls, Pagestorm, Corrosive Wave and Murder. The simulator reads Fiddle's bonus but not its lock.
- Play caps. Velvet Choker ◐ (6 cards a turn), Sloth and Normality (3) cap Shiv and 0-cost engines.
- Snecko Eye and Confused randomise costs 0–3 on draw. That helps decks of 2–3 cost cards and hurts decks of 0-cost cards. Not read.
- Spiked Gauntlets (Powers cost 1 more) taxes Power engines (Storm, Subroutine, Creative AI). Not read.
- Runic Pyramid and Well-Laid Plans keep the hand, so Sadness and other Retain dead cards pile up until the hand is full. The simulator caps the hand (at the playbook's `max_hand`), so this is partly visible.

## Ironclad

### Ironclad: engines

**Exhaust.**

- Producers:
  - Corruption ✅ (every Skill costs 0 and exhausts)
  - chosen exhausts: Brand ◐, Burning Pact ◐, True Grit ◐ (random unless upgraded)
  - hand exhausts: Second Wind ✅ (non-Attacks), Fiend Fire ✅ and Stoke ◐ (the whole hand)
  - random exhausts: Cinder ◐, Thrash ◐ (an Attack), Havoc ○
  - every Exhaust-keyword card
- Payoffs:
  - per card: Feel No Pain ✅ (3 Block), Dark Embrace ✅ (draw 1), Charon's Ashes ✅ (3 to all), Forgotten Soul ✅
  - on the Exhaust Pile: Ashen Strike ✅ (+3 per card), Pact's End ✅ (needs 3), Howl from Beyond ✅ (replays itself from the pile)
  - this turn: Evil Eye ✅ (double Block), Forgotten Ritual ✅ (+3 energy)
  - on itself: Drum of Battle ✅ (2 energy when exhausted)
- Second order:
  - Corruption makes every Skill an exhaust. Feel No Pain turns that into Block, and Juggernaut ✅ turns each Block gain into 6 damage.
  - Dark Embrace draws a card per exhaust, which with Corruption is usually another free Skill: a draw loop that runs until the Skills run out.
  - Drum of Battle under Corruption refunds 2 energy for its own exhaust.
- Simulator: follows Corruption → Feel No Pain → Juggernaut and → Dark Embrace in full. It misses every chosen-card exhaust (`EXH_CHOOSE`), so decks that exhaust with Brand, Burning Pact or True Grit instead of Corruption are undercounted on every payoff above.

**Self-damage.**

- Producers: Bloodletting ◐, Offering ◐, Hemokinesis ◐, Breakthrough ◐, Blood Wall ◐, Brand ◐, Crimson Mantle ◐ (1 HP every turn), Inferno ⚠️ (1 HP every turn).
- Payoffs:
  - per loss on your turn: Rupture ⚠️ (+1 Strength), Inferno ⚠️ (6 to all)
  - Spite ⚠️ (second hit if you lost HP this turn), Tear Asunder ◐ (+1 hit per HP-loss event this combat)
  - relics: Centennial Puzzle ○ (draw 3 on the first loss), Self-Forming Clay ○ (3 Block next turn per loss), Red Skull ○ (+3 Strength at ≤ 50% HP), Demon Tongue ○ (heals the first loss on your turn, making one self-hit a turn free)
- Second order:
  - Crimson Mantle or Inferno costs 1 HP at the start of every turn. That alone triggers Rupture (+1 Strength a turn) and Inferno's own 6 to all, before a card is played.
  - Rupture's Strength then lands on every hit of Twin Strike, Sword Boomerang, Conflagration, Whirlwind and Tear Asunder.
  - Tear Asunder scales twice from one source: hits from the number of losses, damage per hit from Rupture.
- Simulator: none of it. Your HP loss isn't tracked (`SELF_HP`), and Inferno and Rupture are misread as one-time effects (`TRIGGER_FALLBACK`). This is the largest Ironclad gap. A lost-HP counter (per turn and per combat) and an on-HP-loss trigger would close it without modelling enemy damage.

**Strength.**

- Producers: Demon Form ✅, Inflame ✅, Rupture ⚠️, Brand ◐, Dominate ◐ (+1 per Vulnerable stack), Fight Me! ◐, Setup Strike ◐ (this turn), Brimstone ◐, Shuriken ✅, Vajra ✅, Sparkling Rouge ✅.
- Payoffs: every hit. Multi-hit cards gain most: Twin Strike, Sword Boomerang, Conflagration (4 hits on every enemy), Whirlwind (X hits), Fight Me!, Dismantle.
- Relic amplifiers: Ruined Helmet ○ doubles the first Strength gain each combat (Demon Form's first trigger, or Inflame). Thumbscrew ✅ adds a hit to every multi-hit.
- Simulator: Strength is tracked and applied per hit. Temporary Strength (Setup Strike) and Strength from enemy-targeted cards (Fight Me!) are dropped (`STR_TEMP`, `STR_SELF_ON_ATTACK`).

**Vulnerable.**

- Producers: Bash ✅, Tremble ✅, Thunderclap ✅, Taunt ✅, Uppercut ◐ (dropped, `VAR_POWER`), Break ✅, Bag of Marbles ✅; Molten Fist ◐ doubles the stacks.
- Payoffs:
  - multipliers: Cruelty ✅ (+25%), Paper Phrog ✅ (75%)
  - per stack: Bully ◐ (+2 per stack), Dominate ◐ (Strength per stack)
  - conditional or on apply: Dismantle ⚠️ (second hit only if Vulnerable), Vicious ⚠️ (draw per application), Colossus ◐ (half damage from Vulnerable enemies)
- Second order: Molten Fist doubles the stacks before Dominate or Bully reads them. Tremble, then Molten Fist, then Dominate is 6 stacks and 6 Strength.
- Simulator: stacks are tracked, so Bully, Dominate and Molten Fist are cheap fixes (`VULN_STACKS`). Dismantle and Vicious are misread.

**Block.**

- Payoffs: Juggernaut ✅ (6 damage per Block gain), Barricade ○ (Block kept), Body Slam ○ (damage = Block), Unmovable ○ (first card Block doubled), Entrench ○ (double Block).
- Second order:
  - Barricade with Stone Armor's Plating, Crimson Mantle and Feel No Pain stacks Block turn over turn, and Body Slam converts it into damage.
  - Juggernaut fires on every Block gain, so Plating's end-of-turn Block should fire it too. The simulator adds Plating to the turn's Block without firing `OnGainBlock`, a small bug worth fixing with this group.
- Simulator: Juggernaut only. Barricade and Body Slam need a carried Block pool (`BLOCK_KEEP`, `BLOCK_DMG`).

**Strike.**

- Strike cards: Strike, Pommel Strike, Twin Strike, Setup Strike, Ashen Strike, Perfected Strike, plus Colorless Ultimate Strike and Seeker Strike.
- Payoffs: Perfected Strike ◐ (+2 per Strike in the deck), Hellraiser ○ (drawn Strikes play themselves), Strike Dummy ✅ (+3 each).
- Second order: Hellraiser with Dark Embrace or Pommel Strike draws turns each draw into a free Strike. Strike Dummy and Strength apply to each.
- Simulator: Strike Dummy only.

**Attack count.**

- Feeders: Anger ✅ makes a 0-cost copy each play, a growing stream of Attacks.
- Payoffs: Rage ○ (Block per Attack this turn), Stomp ◐ (cheaper per Attack), Juggling ○ (copies the 3rd Attack), and the attack-counter relics (see Across classes).
- Simulator: Anger and the relics are read; Rage, Stomp and Juggling aren't.

**Anti-synergies.**

- Battle Trance stops Dark Embrace's draws for the turn.
- Corruption exhausts Armaments, Shrug It Off and other reusable Skills, so a long fight runs out of them.
- Offering, Bloodletting and Hemokinesis cost HP that only pays off with the self-damage payoffs.

### Ironclad: first-order index

Every payoff in the pool, with what feeds it. Class cards come first, then Colorless, tokens, event cards and relics.

- **Exhaust** (tracked: yes). Payoffs: Ashen Strike ✅, Dark Embrace ✅, Drum of Battle ✅, Evil Eye ✅, Feel No Pain ✅, Fiend Fire ✅, Forgotten Ritual ✅, Howl from Beyond ✅, Pact's End ✅, Second Wind ✅, Stoke ◐, The Devil ◐, The Shot ○, Charon's Ashes ✅, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○. Fed by: 77 cards and relics, for example Brand ◐, Burning Pact ◐, Cinder ◐, Corruption ✅, Dominate ◐, Feed ◐, Fiend Fire ✅, Forgotten Ritual ✅ and 11 more.
- **Discard** (tracked: partly (whole-hand discards only)). Payoffs: Gambling Chip ○. Fed by: The Hermit ◐, Wheel of Fortune ✅, Runic Pyramid ✅.
- **Extra draw** (tracked: yes). Payoffs: Hellraiser ○, Automation ⚠️. Fed by: 41 cards and relics, for example Battle Trance ◐, Burning Pact ◐, Dark Embrace ✅, Drum of Battle ✅, Offering ◐, Pillage ◐, Pommel Strike ✅, Shrug It Off ✅ and 1 more.
- **Shuffle** (tracked: yes). Payoffs: Stratagem ○, The Abacus ✅. Fed by: nothing in the pool.
- **HP loss (yours)** (tracked: no). Payoffs: Inferno ⚠️, Rupture ⚠️, Spite ⚠️, Tear Asunder ◐, Demon Tongue ○, Red Skull ○, Self-Forming Clay ○, Centennial Puzzle ○, Meat on the Bone ·. Fed by: Blood Wall ◐, Bloodletting ◐, Brand ◐, Breakthrough ◐, Crimson Mantle ◐, Hemokinesis ◐, Inferno ⚠️, Offering ◐, The Hanged Man ○, Royal Poison ○.
- **Block gain** (tracked: yes). Payoffs: Barricade ○, Body Slam ○, Juggernaut ✅, Unmovable ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○. Fed by: 44 cards and relics, for example Armaments ◐, Blood Wall ◐, Colossus ◐, Crimson Mantle ◐, Defend ✅, Evil Eye ✅, Expect a Fight ⚠️, Feel No Pain ✅ and 9 more.
- **Energy** (tracked: yes). Payoffs: Ice Cream ✅. Fed by: Bloodletting ◐, Drum of Battle ✅, Forgotten Ritual ✅, Offering ◐, Pyre ✅, Automation ⚠️, Brightest Flame ✅, Fuel ✅, Joy ✅, Luminesce ✅, Outmaneuver ✅, Production ✅, Relax ⚠️, Restlessness ◐, The Emperor ✅, The Star ⚠️, The Sun ✅, Art of War ○, Blessed Antler ◐, Blood-Soaked Rose ✅, Booming Conch ○, Bread ⚠️, Candelabra ✅, Chandelier ✅, Ectoplasm ✅, Gremlin Horn ○, Happy Flower ✅, Happy Flower??? ✅, Nunchaku ✅, Pael's Flesh ✅, Pael's Tears ○, Philosopher's Stone ◐, Prismatic Gem ✅, Pumpkin Candle ✅, Seal of Gold ✅, Sozu ✅, Spiked Gauntlets ◐, Velvet Choker ◐, Whispering Earring ◐.
- **Strength** (tracked: yes). Payoffs: Conflagration ✅, Dismantle ⚠️, Fight Me! ◐, Spite ⚠️, Sword Boomerang ✅, Thrash ◐, Twin Strike ✅, Whirlwind ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅, Ruined Helmet ○. Fed by: Brand ◐, Demon Form ✅, Dominate ◐, Fight Me! ◐, Inflame ✅, Rupture ⚠️, Setup Strike ◐, Feeding Frenzy ⚠️, Prowess ✅, Strength ✅, Brimstone ◐, Red Skull ○, Ember Tea ○, Girya ·, Mighty Wings ○, Rainbow Ring ○, Reptile Trinket ○, Shuriken ✅, Sparkling Rouge ✅, Toasty Mittens ◐.
- **Vulnerable** (tracked: yes). Payoffs: Bully ◐, Colossus ◐, Cruelty ✅, Dismantle ⚠️, Dominate ◐, Molten Fist ◐, Vicious ⚠️, Paper Phrog ✅. Fed by: Bash ✅, Break ✅, Dominate ◐, Molten Fist ◐, Taunt ✅, Thunderclap ✅, Tremble ✅, Uppercut ◐, Shockwave ⚠️, Squash ✅, The Empress ⚠️, Bag of Marbles ✅, Hand Drill ○.
- **Any debuff** (tracked: counted). Payoffs: Rend ◐, Unsettling Lamp ○. Fed by: Bash ✅, Break ✅, Dominate ◐, Molten Fist ◐, Taunt ✅, Thunderclap ✅, Tremble ✅, Uppercut ◐, Death ✅, Shockwave ⚠️, Squash ✅, The Empress ⚠️, Bag of Marbles ✅, Hand Drill ○, Red Mask ○, Tormentous Collar ○.
- **Multi-hit** (tracked: yes). Payoffs: Thumbscrew ✅. Fed by: Conflagration ✅, Dismantle ⚠️, Fight Me! ◐, Spite ⚠️, Sword Boomerang ✅, Thrash ◐, Twin Strike ✅, Whirlwind ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅.
- **Card costing 2+** (tracked: yes). Payoffs: Intimidating Helmet ✅. Fed by: 44 cards and relics, for example Barricade ○, Bash ✅, Blood Wall ◐, Bludgeon ✅, Cinder ◐, Corruption ✅, Dark Embrace ✅, Demon Form ✅ and 18 more.
- **"Strike" card** (tracked: yes). Payoffs: Hellraiser ○, Perfected Strike ◐, Strike Dummy ✅, Strike Dummy??? ✅. Fed by: Ashen Strike ✅, Perfected Strike ◐, Pommel Strike ✅, Setup Strike ◐, Strike ✅, Twin Strike ✅, Minion Strike ✅, Seeker Strike ◐, Ultimate Strike ✅.
- **"Defend" card** (tracked: yes). Payoffs: Fasten ✅, Ghost Seed ○, Pandora's Box ·. Fed by: Defend ✅, Ultimate Defend ✅.
- **X-cost card** (tracked: yes). Payoffs: Chemical X ○. Fed by: Cascade ○, Whirlwind ✅, Volley ✅.
- **Kill (Fatal)** (tracked: no). Payoffs: Feed ◐, Hand of Greed ✅, Gremlin Horn ○, War Hammer ·. Fed by: Judgement ○.
- **Upgrade** (tracked: no). Payoffs: Miniature Cannon ✅. Fed by: Aggression ○, Armaments ◐, Apotheosis ○, The Devil ◐, Bone Tea ○, Fishing Rod ·, Frozen Egg ·, Lava Lamp ·, Molten Egg ·, Razor Tooth ○, Stone Cracker ○, Toxic Egg ·, War Hammer ·.
- **Replay** (tracked: partly). Payoffs: Hidden Gem ◐. Fed by: Cascade ○, Havoc ○, One-Two Punch ✅, Beat Down ○, Catastrophe ○, Mayhem ○, The Devil ◐, The Fool ○.

### Ironclad: second-order chains

A converter turns one resource into another, so a producer reaches a payoff through it. Listed only where the far end has a payoff in the pool. "Visible" means the simulator reads all three links.

| From | Converter | To | Far payoffs | Visible |
|---|---|---|---|---|
| "Defend" card (2) | Fasten ✅ | Block gain | Barricade ○, Body Slam ○, Juggernaut ✅, Unmovable ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | yes |
| Card costing 2+ (44) | Intimidating Helmet ✅ | Block gain | Barricade ○, Body Slam ○, Juggernaut ✅, Unmovable ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | yes |
| Exhaust (77) | Dark Embrace ✅ | Extra draw | Hellraiser ○, Automation ⚠️ | no |
| Exhaust (77) | Drum of Battle ✅ | Energy | Ice Cream ✅ | yes |
| Exhaust (77) | Drum of Battle ✅ | Extra draw | Hellraiser ○, Automation ⚠️ | no |
| Exhaust (77) | Evil Eye ✅ | Block gain | Barricade ○, Body Slam ○, Juggernaut ✅, Unmovable ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | yes |
| Exhaust (77) | Feel No Pain ✅ | Block gain | Barricade ○, Body Slam ○, Juggernaut ✅, Unmovable ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | yes |
| Exhaust (77) | Forgotten Ritual ✅ | Energy | Ice Cream ✅ | yes |
| Exhaust (77) | Joss Paper ○ | Extra draw | Hellraiser ○, Automation ⚠️ | no |
| Exhaust (77) | Second Wind ✅ | Block gain | Barricade ○, Body Slam ○, Juggernaut ✅, Unmovable ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | yes |
| Extra draw (41) | Automation ⚠️ | Energy | Ice Cream ✅ | no |
| HP loss (yours) (10) | Centennial Puzzle ○ | Extra draw | Hellraiser ○, Automation ⚠️ | no |
| HP loss (yours) (10) | Red Skull ○ | Strength | Conflagration ✅, Dismantle ⚠️, Fight Me! ◐, Spite ⚠️, Sword Boomerang ✅, Thrash ◐, Twin Strike ✅, Whirlwind ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅, Ruined Helmet ○ | no |
| HP loss (yours) (10) | Rupture ⚠️ | Strength | Conflagration ✅, Dismantle ⚠️, Fight Me! ◐, Spite ⚠️, Sword Boomerang ✅, Thrash ◐, Twin Strike ✅, Whirlwind ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅, Ruined Helmet ○ | no |
| HP loss (yours) (10) | Self-Forming Clay ○ | Block gain | Barricade ○, Body Slam ○, Juggernaut ✅, Unmovable ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Kill (Fatal) (1) | Gremlin Horn ○ | Energy | Ice Cream ✅ | no |
| Kill (Fatal) (1) | Gremlin Horn ○ | Extra draw | Hellraiser ○, Automation ⚠️ | no |
| Kill (Fatal) (1) | War Hammer · | Upgrade | Miniature Cannon ✅ | no |
| Vulnerable (13) | Colossus ◐ | Block gain | Barricade ○, Body Slam ○, Juggernaut ✅, Unmovable ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Vulnerable (13) | Dominate ◐ | Strength | Conflagration ✅, Dismantle ⚠️, Fight Me! ◐, Spite ⚠️, Sword Boomerang ✅, Thrash ◐, Twin Strike ✅, Whirlwind ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅, Ruined Helmet ○ | no |
| Vulnerable (13) | Dominate ◐ | Any debuff | Rend ◐, Unsettling Lamp ○ | no |
| Vulnerable (13) | Molten Fist ◐ | Any debuff | Rend ◐, Unsettling Lamp ○ | no |
| Vulnerable (13) | Vicious ⚠️ | Extra draw | Hellraiser ○, Automation ⚠️ | no |

## Silent

### Silent: engines

**Shivs.**

- Makers: Blade Dance ✅ (3), Cloak and Dagger ✅, Leading Strike ✅, Hidden Daggers ◐, Up My Sleeve ◐, Storm of Steel ✅ (one per discarded card), Infinite Blades ✅ (1 a turn), Fan of Knives ◐ (4), Blade of Ink ✅ (Inky Shivs), Ninja Scroll ✅.
- Shivs are 0-cost, Exhaust Attacks.
- Payoffs:
  - damage: Accuracy ✅ (+4 each), Phantom Blades ○ (+8 to the first each turn, and Retain), Fan of Knives ◐ (hit all), Knife Trap ○ (replays every Shiv in the Exhaust Pile)
  - per card played: Serpent Form ✅ (4 damage), Afterimage ✅ (1 Block)
  - per Shiv: Helical Dart ○ (+1 Dexterity this turn), Finisher ◐ (6 per Attack this turn)
  - per hit: Envenom ⚠️ (Poison per unblocked hit), The Boot ○ (4 → 5)
  - the attack-counter relics
- Second order:
  - Shuriken turns every 3 Shivs into +1 Strength, which every later Shiv deals. Accuracy and Shuriken compound.
  - Shivs exhaust, so the Exhaust Pile fills with them and Knife Trap replays the whole pile: late in a fight that is 10+ Shivs for 2 energy.
  - Envenom turns each Shiv hit into Poison, which Accelerant ✅ makes tick more and Bubble Bubble ◐ multiplies.
- Simulator: Shiv making, Accuracy, Serpent Form, Afterimage and the relics are followed (the most visible Silent engine). Knife Trap, Phantom Blades and Helical Dart are unread. Envenom is misread as one Poison on play.

**Discard and Sly.**

- Sly: a Sly card discarded from the hand is played for free.
- Discarders: Survivor ◐, Dagger Throw ◐, Prepared ◐, Acrobatics ◐, Hidden Daggers ◐, Tools of the Trade ◐ (each turn), Calculated Gamble ◐ (whole hand, then draws as many), Storm of Steel ✅ (whole hand), Shadow Step ◐ (whole hand), Gambling Chip ○.
- Sly cards: Tactician (+1 energy), Reflex (draw 2), Flick-Flack (7 to all), Ricochet (4 hits), Untouchable (6 Block), Memento Mori (19), Abrasive (a cost-3 Power, free).
  - Their own costs are high (Reflex 3, Tactician 3, Memento Mori 3, Abrasive 3): they are meant to be discarded, not played.
  - Master Planner ○ gives every played Skill Sly; Hand Trick ◐ gives one Skill Sly.
- Discard payoffs: Tingsha ○ (3 damage per card), Tough Bandages ○ (3 Block per card).
- Second order:
  - Calculated Gamble discards the hand: every Sly card in it plays for free, Tingsha hits once per card, and you draw that many again.
  - Storm of Steel instead turns the discards into Shivs, which feeds the Shiv engine.
- Simulator: almost none of it. Only whole-hand discards happen (Storm of Steel, Calculated Gamble, Shadow Step), and Sly never triggers, so Sly cards are valued at their printed (high) cost. This needs a discard choice (`DISCARD_CHOOSE`: discard Sly cards first, then dead cards) and the Sly rule (`SLY`). It is the largest Silent gap.

**Poison.**

- Producers: Deadly Poison ✅, Poisoned Stab ✅, Snakebite ✅, Bouncing Flask ⚠️ (3 × 3, read once), Noxious Fumes ✅ (each turn), Haze ✅, Outbreak ◐, Corrosive Wave ○ (per card drawn this turn), Envenom ⚠️, Twisted Funnel ✅.
- Payoffs: Accelerant ✅ (extra ticks), Bubble Bubble ◐ (+9 if Poisoned), Outbreak ◐ (ticks immediately), Mirage ○ (Block = Poison), Rend ◐ (per debuff).
- Second order:
  - Snecko Skull ○ adds 1 to every application, so it rewards many small applications: Bouncing Flask (3), Corrosive Wave (one per draw), Envenom (one per hit), Noxious Fumes (each turn).
  - Corrosive Wave with a draw burst (Acrobatics, Calculated Gamble, Backflip) applies Poison once per card drawn.
- Simulator: Poison is tracked and ticks, Accelerant is read. Bouncing Flask is read at a third of its Poison (`VAR_REPEAT`), and Corrosive Wave, Mirage and Snecko Skull are unread.

**Weak.**

- Producers: Neutralize ✅, Sucker Punch ✅, Leg Sweep ✅, Haze ✅, Suppress ✅, Malaise ◐, Red Mask ○.
- Payoffs: Tracking ○ (+50% Attack damage to Weak enemies), Paper Krane ○ (Weak enemies deal 40% less).
- Simulator: Weak is applied but not kept as enemy state, so Tracking can't be read (`WEAK_DMG`).

**Draw.**

- Speedster ✅ (3 to all per card drawn on your turn), Corrosive Wave ○, Murder ◐ (+1 per card drawn this combat), Tools of the Trade ◐.
- Second order: Calculated Gamble and Acrobatics are draw bursts, so they feed Speedster and Corrosive Wave per card.
- Simulator: Speedster is followed; Murder is read flat.

**Retain.** Well-Laid Plans ✅ keeps the hand. Snakebite ✅ and Phantom Blades' Shivs retain. Expertise ◐ retains for one turn. Bullet Time ○ makes the whole (retained) hand free, so Well-Laid Plans into Bullet Time is a stored-up turn.

**Anti-synergies.**

- Bullet Time, and Battle Trance from other pools, stop the draw engines for the turn.
- Velvet Choker caps the Shiv engine at 6 cards.
- Hidden Daggers and Survivor discard by choice, so without Sly targets they only thin the hand.

### Silent: first-order index

Every payoff in the pool, with what feeds it. Class cards come first, then Colorless, tokens, event cards and relics.

- **Exhaust** (tracked: yes). Payoffs: Knife Trap ○, The Devil ◐, The Shot ○, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○. Fed by: 69 cards and relics, for example Adrenaline ✅, Assassinate ✅, Backstab ✅, Blade Dance ✅, Calculated Gamble ◐, Expose ◐, Malaise ◐, Mirage ○ and 3 more.
- **Discard** (tracked: partly (whole-hand discards only)). Payoffs: Abrasive ◐, Calculated Gamble ◐, Flick-Flack ◐, Memento Mori ✅, Reflex ◐, Ricochet ◐, Storm of Steel ✅, Tactician ◐, Untouchable ◐, Tingsha ○, Tough Bandages ○, Gambling Chip ○. Fed by: Acrobatics ◐, Calculated Gamble ◐, Dagger Throw ◐, Hidden Daggers ◐, Prepared ◐, Shadow Step ◐, Storm of Steel ✅, Survivor ◐, Tools of the Trade ◐, Well-Laid Plans ✅, The Hermit ◐, Wheel of Fortune ✅, Runic Pyramid ✅.
- **Sly card discarded** (tracked: no). Payoffs: Hand Trick ◐, Master Planner ○. Fed by: nothing in the pool.
- **Extra draw** (tracked: yes). Payoffs: Corrosive Wave ○, Murder ◐, Speedster ✅, Automation ⚠️. Fed by: 44 cards and relics, for example Acrobatics ◐, Adrenaline ✅, Backflip ✅, Dagger Throw ◐, Escape Plan ◐, Expertise ◐, Predator ⚠️, Prepared ◐ and 4 more.
- **Shuffle** (tracked: yes). Payoffs: Stratagem ○, The Abacus ✅. Fed by: nothing in the pool.
- **HP loss (yours)** (tracked: no). Payoffs: Centennial Puzzle ○, Meat on the Bone ·. Fed by: The Hanged Man ○, Royal Poison ○.
- **Block gain** (tracked: yes). Payoffs: Blur ◐, Shadowmeld ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○. Fed by: 42 cards and relics, for example Afterimage ✅, Backflip ✅, Blur ◐, Cloak and Dagger ✅, Dash ✅, Defend ✅, Deflect ✅, Dodge and Roll ◐ and 7 more.
- **Energy** (tracked: yes). Payoffs: Ice Cream ✅. Fed by: Adrenaline ✅, Sidestep ✅, Tactician ◐, Automation ⚠️, Brightest Flame ✅, Fuel ✅, Joy ✅, Luminesce ✅, Outmaneuver ✅, Production ✅, Relax ⚠️, Restlessness ◐, The Emperor ✅, The Star ⚠️, The Sun ✅, Art of War ○, Blessed Antler ◐, Blood-Soaked Rose ✅, Booming Conch ○, Bread ⚠️, Candelabra ✅, Chandelier ✅, Ectoplasm ✅, Gremlin Horn ○, Happy Flower ✅, Happy Flower??? ✅, Nunchaku ✅, Pael's Flesh ✅, Pael's Tears ○, Philosopher's Stone ◐, Prismatic Gem ✅, Pumpkin Candle ✅, Seal of Gold ✅, Sozu ✅, Spiked Gauntlets ◐, Velvet Choker ◐, Whispering Earring ◐.
- **Strength** (tracked: yes). Payoffs: Dagger Spray ✅, Ricochet ◐, Skewer ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅. Fed by: Feeding Frenzy ⚠️, Prowess ✅, Strength ✅, Ember Tea ○, Girya ·, Mighty Wings ○, Rainbow Ring ○, Reptile Trinket ○, Shuriken ✅, Sparkling Rouge ✅, Toasty Mittens ◐.
- **Weak** (tracked: no (applied, not tracked)). Payoffs: Tracking ○, Paper Krane ○. Fed by: Haze ✅, Leg Sweep ✅, Malaise ◐, Neutralize ✅, Sucker Punch ✅, Suppress ✅, Shockwave ⚠️, The Empress ⚠️, Red Mask ○, Tormentous Collar ○.
- **Any debuff** (tracked: counted). Payoffs: Rend ◐, Unsettling Lamp ○. Fed by: Assassinate ✅, Bouncing Flask ⚠️, Bubble Bubble ◐, Corrosive Wave ○, Deadly Poison ✅, Envenom ⚠️, Expose ◐, Haze ✅, Leg Sweep ✅, Malaise ◐, Neutralize ✅, Noxious Fumes ✅, Outbreak ◐, Poisoned Stab ✅, Snakebite ✅, Sucker Punch ✅, Suppress ✅, Death ✅, Shockwave ⚠️, Squash ✅, The Empress ⚠️, Snecko Skull ○, Twisted Funnel ✅, Bag of Marbles ✅, Hand Drill ○, Red Mask ○, Tormentous Collar ○.
- **Poison** (tracked: yes). Payoffs: Accelerant ✅, Bubble Bubble ◐, Mirage ○, Outbreak ◐, Snecko Skull ○. Fed by: Bouncing Flask ⚠️, Bubble Bubble ◐, Corrosive Wave ○, Deadly Poison ✅, Envenom ⚠️, Haze ✅, Noxious Fumes ✅, Outbreak ◐, Poisoned Stab ✅, Snakebite ✅, Snecko Skull ○, Twisted Funnel ✅.
- **Shiv** (tracked: yes). Payoffs: Accuracy ✅, Fan of Knives ◐, Knife Trap ○, Phantom Blades ○, Helical Dart ○. Fed by: Blade Dance ✅, Blade of Ink ✅, Cloak and Dagger ✅, Fan of Knives ◐, Hidden Daggers ◐, Infinite Blades ✅, Knife Trap ○, Leading Strike ✅, Phantom Blades ○, Storm of Steel ✅, Up My Sleeve ◐, The Moon ○, Ninja Scroll ✅.
- **Multi-hit** (tracked: yes). Payoffs: Envenom ⚠️, Thumbscrew ✅. Fed by: Dagger Spray ✅, Ricochet ◐, Skewer ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅.
- **Card costing 2+** (tracked: yes). Payoffs: Intimidating Helmet ✅. Fed by: 46 cards and relics, for example Abrasive ◐, Bouncing Flask ⚠️, Bullet Time ○, Dash ✅, Envenom ⚠️, Fan of Knives ◐, Haze ✅, Knife Trap ○ and 20 more.
- **"Strike" card** (tracked: yes). Payoffs: Strike Dummy ✅, Strike Dummy??? ✅. Fed by: Leading Strike ✅, Strike ✅, Minion Strike ✅, Seeker Strike ◐, Ultimate Strike ✅.
- **"Defend" card** (tracked: yes). Payoffs: Fasten ✅, Ghost Seed ○, Pandora's Box ·. Fed by: Defend ✅, Ultimate Defend ✅.
- **X-cost card** (tracked: yes). Payoffs: Chemical X ○. Fed by: Malaise ◐, Skewer ✅, Volley ✅.
- **Kill (Fatal)** (tracked: no). Payoffs: Echoing Slash ◐, The Hunt ◐, Hand of Greed ✅, Gremlin Horn ○, War Hammer ·. Fed by: Judgement ○.
- **Upgrade** (tracked: no). Payoffs: Miniature Cannon ✅. Fed by: Knife Trap ○, Apotheosis ○, The Devil ◐, Bone Tea ○, Fishing Rod ·, Frozen Egg ·, Lava Lamp ·, Molten Egg ·, Razor Tooth ○, Stone Cracker ○, Toxic Egg ·, War Hammer ·.
- **Replay** (tracked: partly). Payoffs: Hidden Gem ◐. Fed by: Burst ✅, Knife Trap ○, Beat Down ○, Catastrophe ○, Mayhem ○, The Devil ◐, The Fool ○.

### Silent: second-order chains

A converter turns one resource into another, so a producer reaches a payoff through it. Listed only where the far end has a payoff in the pool. "Visible" means the simulator reads all three links.

| From | Converter | To | Far payoffs | Visible |
|---|---|---|---|---|
| "Defend" card (2) | Fasten ✅ | Block gain | Blur ◐, Shadowmeld ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Card costing 2+ (46) | Intimidating Helmet ✅ | Block gain | Blur ◐, Shadowmeld ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Discard (13) | Reflex ◐ | Extra draw | Corrosive Wave ○, Murder ◐, Speedster ✅, Automation ⚠️ | no |
| Discard (13) | Storm of Steel ✅ | Shiv | Accuracy ✅, Fan of Knives ◐, Knife Trap ○, Phantom Blades ○, Helical Dart ○ | no |
| Discard (13) | Tactician ◐ | Energy | Ice Cream ✅ | no |
| Discard (13) | Tough Bandages ○ | Block gain | Blur ◐, Shadowmeld ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Discard (13) | Untouchable ◐ | Block gain | Blur ◐, Shadowmeld ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Exhaust (69) | Joss Paper ○ | Extra draw | Corrosive Wave ○, Murder ◐, Speedster ✅, Automation ⚠️ | no |
| Exhaust (69) | Knife Trap ○ | Replay | Hidden Gem ◐ | no |
| Exhaust (69) | Knife Trap ○ | Shiv | Accuracy ✅, Fan of Knives ◐, Phantom Blades ○, Helical Dart ○ | no |
| Exhaust (69) | Knife Trap ○ | Upgrade | Miniature Cannon ✅ | no |
| Extra draw (44) | Automation ⚠️ | Energy | Ice Cream ✅ | no |
| Extra draw (44) | Corrosive Wave ○ | Poison | Accelerant ✅, Bubble Bubble ◐, Mirage ○, Outbreak ◐, Snecko Skull ○ | no |
| Extra draw (44) | Corrosive Wave ○ | Any debuff | Rend ◐, Unsettling Lamp ○ | no |
| HP loss (yours) (2) | Centennial Puzzle ○ | Extra draw | Corrosive Wave ○, Murder ◐, Speedster ✅, Automation ⚠️ | no |
| Kill (Fatal) (1) | Gremlin Horn ○ | Energy | Ice Cream ✅ | no |
| Kill (Fatal) (1) | Gremlin Horn ○ | Extra draw | Corrosive Wave ○, Murder ◐, Speedster ✅, Automation ⚠️ | no |
| Kill (Fatal) (1) | War Hammer · | Upgrade | Miniature Cannon ✅ | no |
| Multi-hit (8) | Envenom ⚠️ | Poison | Accelerant ✅, Bubble Bubble ◐, Mirage ○, Outbreak ◐, Snecko Skull ○ | no |
| Multi-hit (8) | Envenom ⚠️ | Any debuff | Rend ◐, Unsettling Lamp ○ | no |
| Poison (12) | Bubble Bubble ◐ | Any debuff | Rend ◐, Unsettling Lamp ○ | no |
| Poison (12) | Mirage ○ | Block gain | Blur ◐, Shadowmeld ○, Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Poison (12) | Outbreak ◐ | Any debuff | Rend ◐, Unsettling Lamp ○ | no |
| Poison (12) | Snecko Skull ○ | Any debuff | Rend ◐, Unsettling Lamp ○ | no |
| Shiv (13) | Knife Trap ○ | Replay | Hidden Gem ◐ | no |
| Shiv (13) | Knife Trap ○ | Upgrade | Miniature Cannon ✅ | no |
| Shiv (13) | Shiv ✅ | Exhaust | Knife Trap ○, The Devil ◐, The Shot ○, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○ | yes |
| Shiv (13) | Shiv ✅ | Attack played | Finisher ◐, Calamity ○, Nostalgia ○, Daughter of the Wind ✅, Kunai ✅, Kusarigama ✅, Music Box ○, Nunchaku ✅, Ornamental Fan ✅, Pen Nib ○, Shuriken ✅ | yes |

Loops (a resource that comes back to itself through converters):

- Exhaust → [Knife Trap] → Shiv → [Shiv] → Exhaust

## Defect

### Defect: engines

**Orbs and Focus.**

- Producers:
  - Lightning: Zap ✅, Ball Lightning ✅, Tempest ✅, Lightning Rod ⚠️, Storm ⚠️ (per Power), Voltaic ○, Rainbow ✅, Cracked Core ✅
  - Frost: Cold Snap ✅, Coolheaded ✅, Glacier ✅, Chill ✅, Ice Lance ✅
  - Dark: Null ✅, Shadow Shield ✅, Darkness ◐, Consuming Shadow ⚠️, Symbiotic Virus ✅
  - Glass: Glasswork ✅, Refract ✅, Spinner ⚠️
  - Plasma: Fusion ✅, Meteor Strike ✅
  - random: Chaos ○, Trash to Treasure ○, The Magician ○
- Focus (adds to every orb but Plasma): Defragment ✅, Biased Cognition ✅, Data Disk ✅, Hotfix ⚠️ and Focused Strike ◐ (this turn), Synchronize ○; Hyperbeam ◐ lowers it.
- Slots: Capacitor ✅, Modded ◐, Runic Capacitor ✅, The Magician ○; Bulk Up ✅ costs one.
- Evoke on demand: Dualcast ○ (starter), Multi-Cast ○, Quadcast ○, Shatter ◐, Consuming Shadow ⚠️. Channelling into full slots also evokes the first orb (read).
- Extra passives: Loop ○, Gold-Plated Cables ○, Darkness ◐ (Dark only), Tesla Coil ◐ (Lightning only), Emotion Chip ○.
- Payoffs:
  - per orb: Barrage ◐ (5 per orb), Compile Driver ◐ (draw per unique orb), Coolant ○ (Block per unique orb)
  - per channel: Voltaic ○ (Lightning per Lightning channelled), Metronome ○ (30 to all at 7 channels)
  - per evoke: Thunder ⚠️ (+8 per Lightning evoked)
  - Frost: Hailstorm ◐ (needs Frost)
- Second order:
  - Focus raises every passive every turn, and every evoke. Loop and Gold-Plated Cables trigger the rightmost passive again, so they multiply Focus too.
  - A Dark orb gains 6 each end of turn, so Dualcast or Multi-Cast on an old Dark orb is a large hit.
  - Thunder adds 8 to every Lightning evoke, including the overflow evokes Storm causes. Storm channels a Lightning per Power, pushing older orbs out (evoking them).
  - Capacitor adds slots, which raises Barrage and Coolant and delays overflow evokes.
- Simulator:
  - Channelling, passives, Focus, slots and overflow evokes are followed.
  - On-demand evoke isn't (`EVOKE`), so Dualcast, a starter card, does nothing. This undercounts every Defect deck from the first floor.
  - Storm, Spinner and Consuming Shadow channel an extra orb on play (`DOUBLE_CHANNEL`). Lightning Rod channels early. Hotfix keeps its Focus all fight.

**Statuses.**

- Makers: Gunk Up ✅ (Slimed), Boost Away ✅ (Dazed), TURBO ✅ (Void), Fight Through ✅ (2 Wounds), Overclock ✅ (Burn), plus shared relics (Biiig Hug, Blessed Antler).
- Payoffs:
  - per Status created: Smokestack ⚠️ (5 to all), Trash to Treasure ○ (random orb), Rocket Punch ◐ (cheaper)
  - on draw: Iteration ○ (draw 2 on the first Status drawn)
  - on the Statuses themselves: Flak Cannon ✅ (exhausts all, 8 each), Compact ◐ (turns hand Statuses into Fuel, +1 energy each)
- Second order:
  - Trash to Treasure turns every Status into an orb, which feeds the orb engine.
  - Overclock's Burn is made by a 0-cost draw card, so it's the cheapest Status source.
  - Compact turns the clog back into energy.
- Simulator: Statuses are made and clog the hand, so the cost side is visible, but most payoffs are not. Smokestack is misread as one area hit.

**Powers.**

- Payoffs per Power played: Storm ⚠️ (Lightning), Subroutine ✅ (+1 energy), Game Piece ✅ (draw), Lost Wisp ✅ (8 to all), Permafrost ○ (first Power: 7 Block), Mummified Hand ○ (a random hand card free).
- Discounts and copies: Signal Boost ◐ (next Power twice), Synthesis ◐ (next Power free).
- Sources: Creative AI ○ (a random Power each turn), White Noise ○ (one free random Power).
- Second order: Creative AI makes a Power every turn, so Subroutine refunds energy toward it, Storm channels Lightning and Game Piece draws. That is the most self-sustaining Defect loop, and the simulator sees none of it (`RANDOM_GEN`).

**0-cost.**

- Claw ◐ (every Claw gains +2 per play this combat), All for One ◐ (returns every 0-cost card from the Discard Pile), Feral ○ (first 0-cost Attack each turn returns), Scrape ◐ (keeps the 0-costs it draws), Momentum Strike ◐ (becomes 0), Adaptive Strike ◐ (0-cost copies).
- Second order: Claws grow for the combat, and All for One brings them all back, so a Claw deck plays every Claw twice a turn. The attack-counter relics count each play.
- Simulator: 0-cost plays are cheap and played, but Claw's growth (`SELF_GROW`) and the returns (`PILE_MOVE`, `RETURN_TO_HAND`) aren't read.

**Anti-synergies.**

- Hyperbeam lowers Focus for the turn, which hurts its own orb deck.
- Bulk Up gives up a slot.
- Fusion's Plasma ignores Focus.
- TURBO's Void costs energy when drawn, unless a Status payoff uses it.

### Defect: first-order index

Every payoff in the pool, with what feeds it. Class cards come first, then Colorless, tokens, event cards and relics.

- **Exhaust** (tracked: yes). Payoffs: Flak Cannon ✅, The Devil ◐, The Shot ○, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○. Fed by: 74 cards and relics, for example Boot Sequence ✅, Chill ✅, Double Energy ○, Flak Cannon ✅, Fusion ✅, Genetic Algorithm ◐, Hologram ◐, Hotfix ⚠️ and 8 more.
- **Discard** (tracked: partly (whole-hand discards only)). Payoffs: Gambling Chip ○. Fed by: The Hermit ◐, Wheel of Fortune ✅, Runic Pyramid ✅.
- **Extra draw** (tracked: yes). Payoffs: Iteration ○, Scrape ◐, Automation ⚠️. Fed by: 44 cards and relics, for example Compile Driver ◐, Coolheaded ✅, FTL ✅, Iteration ○, Machine Learning ✅, Modded ◐, Overclock ✅, Reboot ◐ and 4 more.
- **Shuffle** (tracked: yes). Payoffs: Stratagem ○, The Abacus ✅. Fed by: nothing in the pool.
- **HP loss (yours)** (tracked: no). Payoffs: Emotion Chip ○, Centennial Puzzle ○, Meat on the Bone ·. Fed by: The Hanged Man ○, Royal Poison ○.
- **Block gain** (tracked: yes). Payoffs: Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○. Fed by: 41 cards and relics, for example Boost Away ✅, Boot Sequence ✅, Charge Battery ✅, Compact ◐, Coolant ○, Defend ✅, Fight Through ✅, Genetic Algorithm ◐ and 6 more.
- **Energy** (tracked: yes). Payoffs: Ice Cream ✅. Fed by: 41 cards and relics, for example Charge Battery ✅, Double Energy ○, Scavenge ◐, Subroutine ✅, Sunder ⚠️, Supercritical ✅, TURBO ✅.
- **Strength** (tracked: yes). Payoffs: Gunk Up ✅, Refract ✅, Shatter ◐, Tesla Coil ◐, Uproar ◐, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅. Fed by: Bulk Up ✅, Feeding Frenzy ⚠️, Prowess ✅, Strength ✅, Ember Tea ○, Girya ·, Mighty Wings ○, Rainbow Ring ○, Reptile Trinket ○, Shuriken ✅, Sparkling Rouge ✅, Toasty Mittens ◐.
- **Any debuff** (tracked: counted). Payoffs: Rend ◐, Unsettling Lamp ○. Fed by: Beam Cell ✅, Go for the Eyes ✅, Null ✅, Death ✅, Shockwave ⚠️, Squash ✅, The Empress ⚠️, Bag of Marbles ✅, Hand Drill ○, Red Mask ○, Tormentous Collar ○.
- **Status created** (tracked: yes). Payoffs: Compact ◐, Flak Cannon ✅, Iteration ○, Rocket Punch ◐, Smokestack ⚠️, Trash to Treasure ○. Fed by: Boost Away ✅, Fight Through ✅, Gunk Up ✅, Overclock ✅, TURBO ✅, Mighty Shield ✅, Mighty Weapon ✅, The Sun ✅, Blessed Antler ◐, Tea of Discourtesy ○.
- **Orb channelled** (tracked: yes). Payoffs: Barrage ◐, Compile Driver ◐, Coolant ○, Synchronize ○, Voltaic ○, Metronome ○. Fed by: Ball Lightning ✅, Chaos ○, Chill ✅, Cold Snap ✅, Consuming Shadow ⚠️, Coolheaded ✅, Darkness ◐, Fusion ✅, Glacier ✅, Glasswork ✅, Ice Lance ✅, Lightning Rod ⚠️, Meteor Strike ✅, Null ✅, Rainbow ✅, Refract ✅, Shadow Shield ✅, Spinner ⚠️, Storm ⚠️, Tempest ✅, Trash to Treasure ○, Voltaic ○, Zap ✅, The Magician ○, Cracked Core ✅, Infused Core ◐, Symbiotic Virus ✅.
- **Lightning** (tracked: yes). Payoffs: Tesla Coil ◐, Thunder ⚠️, Voltaic ○, Infused Core ◐. Fed by: Ball Lightning ✅, Lightning Rod ⚠️, Rainbow ✅, Storm ⚠️, Tempest ✅, Voltaic ○, Zap ✅, Cracked Core ✅, Infused Core ◐.
- **Frost** (tracked: yes). Payoffs: Hailstorm ◐. Fed by: Chill ✅, Cold Snap ✅, Coolheaded ✅, Glacier ✅, Ice Lance ✅, Rainbow ✅.
- **Dark** (tracked: yes). Payoffs: Darkness ◐. Fed by: Consuming Shadow ⚠️, Null ✅, Rainbow ✅, Shadow Shield ✅, Symbiotic Virus ✅.
- **Evoke** (tracked: partly (overflow only)). Payoffs: Thunder ⚠️. Fed by: Consuming Shadow ⚠️, Dualcast ○, Multi-Cast ○, Quadcast ○, Shatter ◐.
- **Multi-hit** (tracked: yes). Payoffs: Thumbscrew ✅. Fed by: Gunk Up ✅, Refract ✅, Shatter ◐, Tesla Coil ◐, Uproar ◐, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅.
- **0-cost card** (tracked: yes). Payoffs: All for One ◐, Feral ○, Scrape ◐, Power Cell ○. Fed by: 76 cards and relics, for example Beam Cell ✅, Boost Away ✅, Boot Sequence ✅, Chill ✅, Claw ◐, FTL ✅, Go for the Eyes ✅, Helix Drill ◐ and 9 more.
- **Card costing 2+** (tracked: yes). Payoffs: Intimidating Helmet ✅. Fed by: Adaptive Strike ◐, All for One ◐, Buffer ○, Bulk Up ✅, Consuming Shadow ⚠️, Creative AI ○, Echo Form ✅, Feral ○, Flak Cannon ✅, Glacier ✅, Hyperbeam ◐, Ice Lance ✅, Meteor Strike ✅, Null ✅, Rainbow ✅, Refract ✅, Rocket Punch ◐, Shadow Shield ✅, Sunder ⚠️, Synthesis ◐, Uproar ◐, Voltaic ○, Apotheosis ○, Beat Down ○, Calamity ○, Catastrophe ○, Entrench ○, Equilibrium ◐, Eternal Armor ✅, Hand of Greed ✅, Jackpot ◐, Mayhem ○, Metamorphosis ○, Relax ⚠️, Rolling Boulder ◐, Shockwave ⚠️, Sovereign Blade ✅, The Bomb ○, Toric Toughness ◐, Whistle ◐.
- **"Strike" card** (tracked: yes). Payoffs: Strike Dummy ✅, Strike Dummy??? ✅. Fed by: Adaptive Strike ◐, Focused Strike ◐, Meteor Strike ✅, Momentum Strike ◐, Strike ✅, Minion Strike ✅, Seeker Strike ◐, Ultimate Strike ✅.
- **"Defend" card** (tracked: yes). Payoffs: Fasten ✅, Ghost Seed ○, Pandora's Box ·. Fed by: Defend ✅, Ultimate Defend ✅.
- **X-cost card** (tracked: yes). Payoffs: Chemical X ○. Fed by: Multi-Cast ○, Tempest ✅, Volley ✅.
- **Kill (Fatal)** (tracked: no). Payoffs: Sunder ⚠️, Hand of Greed ✅, Gremlin Horn ○, War Hammer ·. Fed by: Judgement ○.
- **Upgrade** (tracked: no). Payoffs: Miniature Cannon ✅. Fed by: Apotheosis ○, The Devil ◐, Bone Tea ○, Fishing Rod ·, Frozen Egg ·, Lava Lamp ·, Molten Egg ·, Razor Tooth ○, Stone Cracker ○, Toxic Egg ·, War Hammer ·.
- **Replay** (tracked: partly). Payoffs: Hidden Gem ◐. Fed by: Echo Form ✅, Signal Boost ◐, Beat Down ○, Catastrophe ○, Mayhem ○, The Devil ◐, The Fool ○.

### Defect: second-order chains

A converter turns one resource into another, so a producer reaches a payoff through it. Listed only where the far end has a payoff in the pool. "Visible" means the simulator reads all three links.

| From | Converter | To | Far payoffs | Visible |
|---|---|---|---|---|
| "Defend" card (2) | Fasten ✅ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| 0-cost card (76) | Scrape ◐ | Extra draw | Iteration ○, Automation ⚠️ | no |
| Card costing 2+ (40) | Intimidating Helmet ✅ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Dark (6) | Darkness ◐ | Orb channelled | Barrage ◐, Compile Driver ◐, Coolant ○, Synchronize ○, Voltaic ○, Metronome ○ | no |
| Exhaust (74) | Joss Paper ○ | Extra draw | Iteration ○, Scrape ◐, Automation ⚠️ | no |
| Extra draw (44) | Automation ⚠️ | Energy | Ice Cream ✅ | no |
| HP loss (yours) (2) | Centennial Puzzle ○ | Extra draw | Iteration ○, Scrape ◐, Automation ⚠️ | no |
| Kill (Fatal) (1) | Gremlin Horn ○ | Energy | Ice Cream ✅ | no |
| Kill (Fatal) (1) | Gremlin Horn ○ | Extra draw | Iteration ○, Scrape ◐, Automation ⚠️ | no |
| Kill (Fatal) (1) | Sunder ⚠️ | Energy | Ice Cream ✅ | no |
| Kill (Fatal) (1) | War Hammer · | Upgrade | Miniature Cannon ✅ | no |
| Lightning (9) | Infused Core ◐ | Orb channelled | Barrage ◐, Compile Driver ◐, Coolant ○, Synchronize ○, Voltaic ○, Metronome ○ | no |
| Lightning (9) | Voltaic ○ | Orb channelled | Barrage ◐, Compile Driver ◐, Coolant ○, Synchronize ○, Metronome ○ | no |
| Orb channelled (27) | Compile Driver ◐ | Extra draw | Iteration ○, Scrape ◐, Automation ⚠️ | no |
| Orb channelled (27) | Coolant ○ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Orb channelled (27) | Voltaic ○ | Lightning | Tesla Coil ◐, Thunder ⚠️, Infused Core ◐ | no |
| Status created (10) | Compact ◐ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Status created (10) | Flak Cannon ✅ | Exhaust | The Devil ◐, The Shot ○, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○ | yes |
| Status created (10) | Iteration ○ | Extra draw | Scrape ◐, Automation ⚠️ | no |
| Status created (10) | Rocket Punch ◐ | Extra draw | Iteration ○, Scrape ◐, Automation ⚠️ | no |
| Status created (10) | Trash to Treasure ○ | Orb channelled | Barrage ◐, Compile Driver ◐, Coolant ○, Synchronize ○, Voltaic ○, Metronome ○ | no |

Loops (a resource that comes back to itself through converters):

- Orb channelled → [Voltaic] → Lightning → [Infused Core] → Orb channelled

## Necrobinder

### Necrobinder: engines

**Osty.**

- Summon N summons Osty, or raises his Max HP for the combat.
- Summoners: Bodyguard ✅ (starter), Afterlife ✅, Pull Aggro ✅, Reanimate ✅, Spur ✅, Dirge ✅, Invoke ✅, Necro Mastery ◐, Bound Phylactery ✅ (1 a turn), Devour Life ⚠️ (per Soul played), Sic 'Em ⚠️ (per Osty hit this turn).
- Payoffs on Osty's HP:
  - Unleash ◐ (starter: 6 + current HP), Protector ◐ (+ Max HP)
  - Sacrifice ○ (Block = 3 × Max HP, Osty dies), Bone Shards ⚠️ (needs him alive, kills him)
  - Necro Mastery ◐ (enemies lose what Osty loses)
- Osty's attacks: Poke, Snap, Flatten, Fetch, Rattle, High Five, Right Hand Hand, Sic 'Em, Squeeze, Sweeping Gaze. All are ⚠️ or ◐ because of `OSTY_DOUBLE`.
- Osty-attack payoffs: Calcify ✅ (+4 each), Rattle (+1 hit per other Osty attack this turn), Flatten (free if Osty attacked), Squeeze (+5 per other Osty Attack in the deck), Bone Flute ○ (2 Block each).
- Strength: community guides (not the Codex data) say Strength doesn't apply to Osty, which makes Friendship ✅ (−2 Strength, +1 energy a turn) nearly free in an Osty deck.
- Second order:
  - Every Summon raises Unleash and Protector.
  - Sic 'Em turns Osty's hits into Summon, so Rattle (many hits) then Unleash hits much harder.
  - Sacrifice converts all Osty's HP into Block at 3×, the payoff for a long Summon build.
- Simulator: Summon is tallied but Osty's HP isn't tracked (`OSTY_SCALE`). Every Osty attack is counted twice and given your Strength (`OSTY_DOUBLE`). Osty decks are overcounted on chip damage and undercounted on Unleash and Protector. Fix `OSTY_DOUBLE` first; it is a data rule, not a model.

**Doom.**

- Doom kills a creature at the end of the enemy turn when its HP ≤ its Doom.
- Producers: Countdown ✅ (6 a turn), Negative Pulse ✅, Scourge ✅, Deathbringer ✅, End of Days ◐ (29 to all, then kills), No Escape ⚠️ (+5 per 10 already on), Blight Strike ◐ (Doom = damage), Reaper Form ○ (Doom = Attack damage), Oblivion ⚠️ (3 per card played this turn), Death ✅ (Tarot, 75).
- Payoffs: Time's Up ○ (damage = Doom), Shroud ⚠️ (3 Block per application), Death's Door ◐ (×3 Block if Doom applied this turn), Sleight of Flesh ○ (9 per debuff applied, if Doom counts as a debuff; the data doesn't say), Undying Sigil ○, Book Repair Knife.
- Second order:
  - Reaper Form turns every point of Attack damage into Doom, so damage counts twice toward a kill: once as HP lost, once as the threshold rising.
  - Oblivion with cheap cards applies Doom per card, and Shroud gives Block per application.
- Simulator: Doom is a tally worth half a damage point. It isn't enemy state, so no kill threshold, no Time's Up, no execute (`DOOM_DMG`, `DOOM_EXECUTE`). A Doom deck's kill turn can't be computed until Doom sits on the target next to its HP, which the turn view already estimates.

**Souls.**

- Souls are 0-cost, Exhaust: draw 2.
- Makers: Reave ✅, Severance ✅ (3, one per pile), Grave Warden ✅, Capture Spirit ✅ (3), Dirge ✅ (X), Seance ○, Funerary Mask ✅ (3 at combat start).
- Payoffs: Soul Storm ✅ (+4 per Soul in the Exhaust Pile), Devour Life ⚠️ (Summon per Soul played), Haunt ○ (7 HP loss to a random enemy per Soul played).
- Second order:
  - Souls draw, so Death March ◐ (+4 per card drawn this turn) gains from each.
  - Devour Life turns Souls into Summon, which feeds Unleash.
  - Every played Soul goes to the Exhaust Pile, which feeds Soul Storm.
- Simulator: Souls, their draw, their exhaust and Soul Storm are followed. Devour Life and Haunt need `ON_PLAY_NAMED`.

**Ethereal.**

- Ethereal cards: Defile, Fear, Defy, Parse, Enfeebling Touch, Demesne, Lethality, Seance, Sweeping Gaze. Sculpting Strike ◐ gives Ethereal, and Call of the Void ○ and Big Hat ○ make random Ethereal cards.
- Payoffs:
  - per Ethereal card played: Spirit of Ash ⚠️ (4 Block), Pull from Below ◐ (+5 each), Banshee's Cry ⚠️ (−2 cost each)
  - per Ethereal card drawn: Pagestorm ⚠️ (draw)
  - Veilpiercer ◐ (next Ethereal free), Eidolon ○ (replays every Ethereal card in the Exhaust Pile)
- Second order:
  - An Ethereal card left in the hand exhausts at end of turn, so the Exhaust Pile fills with them whether played or not. Eidolon replays the lot.
  - Banshee's Cry costs 9 and gets 2 cheaper per Ethereal played: about 5 Ethereal plays make it free.
- Simulator: Ethereal exhaust is followed. Every Ethereal payoff is misread or unread.

**Big costs.**

- Payoffs for cards costing 2 or more: Danse Macabre ✅ (4 Block), Right Hand Hand ⚠️ (returns to the hand), Intimidating Helmet ✅ (4 Block), Ivory Tile ✅ (3 or more: +1 energy).
- Second order: Borrowed Time ◐ (+4 energy, and every card costs 1 more this turn) pushes 1-cost cards to 2, which could trigger all four. Whether those payoffs read the raised cost isn't in the text.

**Anti-synergies.**

- Bone Shards and Sacrifice kill Osty, so the next Osty card does nothing until a new Summon.
- Friendship's Strength loss hurts the Strike and Reap side of the deck.
- Neurosurge ◐ puts 3 Doom on you each turn: a clock that ends the fight for you when your HP falls to your Doom.

### Necrobinder: first-order index

Every payoff in the pool, with what feeds it. Class cards come first, then Colorless, tokens, event cards and relics.

- **Exhaust** (tracked: yes). Payoffs: Eidolon ○, Soul Storm ✅, The Devil ◐, The Shot ○, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○. Fed by: 70 cards and relics, for example Afterlife ✅, Cleanse ◐, Dirge ✅, Dredge ○, Eidolon ○, Graveblast ◐, Putrefy ⚠️, Reanimate ✅ and 4 more.
- **Discard** (tracked: partly (whole-hand discards only)). Payoffs: Gambling Chip ○. Fed by: The Hermit ◐, Wheel of Fortune ✅, Runic Pyramid ✅.
- **Extra draw** (tracked: yes). Payoffs: Death March ◐, Pagestorm ⚠️, Automation ⚠️. Fed by: Demesne ✅, Fetch ⚠️, Neurosurge ◐, Pagestorm ⚠️, Parse ✅, Scourge ✅, Brightest Flame ✅, Finesse ✅, Flash of Steel ✅, Impatience ◐, Joy ✅, Master of Strategy ✅, Minion Strike ✅, Relax ⚠️, Restlessness ◐, Scrawl ○, Soul ✅, The Hermit ◐, The High Priestess ✅, The Star ⚠️, The Sun ✅, Thinking Ahead ◐, Wheel of Fortune ✅, Bag of Preparation ✅, Big Mushroom ○, Booming Conch ○, Centennial Puzzle ○, Fiddle ◐, Game Piece ✅, Gremlin Horn ○, Iron Club ✅, Joss Paper ○, Pael's Blood ✅, Pendulum ✅, Pocketwatch ○, Pollinous Core ✅, Snecko Eye ◐, Unceasing Top ○.
- **Shuffle** (tracked: yes). Payoffs: Stratagem ○, The Abacus ✅. Fed by: nothing in the pool.
- **HP loss (yours)** (tracked: no). Payoffs: Centennial Puzzle ○, Meat on the Bone ·. Fed by: Sleight of Flesh ○, The Hanged Man ○, Royal Poison ○.
- **Block gain** (tracked: yes). Payoffs: Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○. Fed by: 42 cards and relics, for example Bone Shards ⚠️, Danse Macabre ✅, Death's Door ◐, Defend ✅, Defy ✅, Delay ✅, Grave Warden ✅, Melancholy ⚠️ and 7 more.
- **Energy** (tracked: yes). Payoffs: Ice Cream ✅. Fed by: 42 cards and relics, for example Borrowed Time ◐, Delay ✅, Demesne ✅, Friendship ✅, Invoke ✅, Neurosurge ◐, Wisp ✅, Ivory Tile ✅.
- **Strength** (tracked: yes). Payoffs: Debilitate ◐, Eradicate ✅, Friendship ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅. Fed by: Feeding Frenzy ⚠️, Prowess ✅, Strength ✅, Ember Tea ○, Girya ·, Mighty Wings ○, Rainbow Ring ○, Reptile Trinket ○, Shuriken ✅, Sparkling Rouge ✅, Toasty Mittens ◐.
- **Vulnerable** (tracked: yes). Payoffs: Debilitate ◐. Fed by: Fear ✅, High Five ⚠️, Putrefy ⚠️, Shockwave ⚠️, Squash ✅, The Empress ⚠️, Bag of Marbles ✅, Hand Drill ○.
- **Weak** (tracked: no (applied, not tracked)). Payoffs: Debilitate ◐. Fed by: Deathbringer ✅, Defy ✅, Putrefy ⚠️, Shockwave ⚠️, The Empress ⚠️, Red Mask ○, Tormentous Collar ○.
- **Any debuff** (tracked: counted). Payoffs: Misery ✅, Sleight of Flesh ○, Rend ◐, Unsettling Lamp ○. Fed by: Blight Strike ◐, Countdown ✅, Deathbringer ✅, Defy ✅, End of Days ◐, Fear ✅, High Five ⚠️, Negative Pulse ✅, Neurosurge ◐, No Escape ⚠️, Oblivion ⚠️, Putrefy ⚠️, Reaper Form ○, Scourge ✅, Death ✅, Shockwave ⚠️, Squash ✅, The Empress ⚠️, Bag of Marbles ✅, Hand Drill ○, Red Mask ○, Tormentous Collar ○.
- **Doom** (tracked: tallied, not enemy state). Payoffs: Death's Door ◐, End of Days ◐, No Escape ⚠️, Shroud ⚠️, Time's Up ○, Book Repair Knife ·, Undying Sigil ○. Fed by: Blight Strike ◐, Countdown ✅, Deathbringer ✅, End of Days ◐, Negative Pulse ✅, Neurosurge ◐, No Escape ⚠️, Oblivion ⚠️, Reaper Form ○, Scourge ✅, Death ✅.
- **Soul** (tracked: yes). Payoffs: Devour Life ⚠️, Haunt ○, Soul Storm ✅. Fed by: Capture Spirit ✅, Dirge ✅, Grave Warden ✅, Reave ✅, Seance ○, Severance ✅, Funerary Mask ✅.
- **Summon (Osty HP)** (tracked: tallied (no Osty HP)). Payoffs: Bone Shards ⚠️, Necro Mastery ◐, Protector ◐, Sacrifice ○, Spur ✅, Unleash ◐. Fed by: Afterlife ✅, Bodyguard ✅, Cleanse ◐, Devour Life ⚠️, Dirge ✅, Invoke ✅, Necro Mastery ◐, Pull Aggro ✅, Reanimate ✅, Sic 'Em ⚠️, Spur ✅, Bound Phylactery ✅, Phylactery Unbound ✅.
- **Osty attack** (tracked: partly (counted twice)). Payoffs: Calcify ✅, Flatten ⚠️, Rattle ⚠️, Sic 'Em ⚠️, Squeeze ◐, Bone Flute ○. Fed by: Fetch ⚠️, Flatten ⚠️, High Five ⚠️, Poke ⚠️, Protector ◐, Rattle ⚠️, Right Hand Hand ⚠️, Sic 'Em ⚠️, Snap ⚠️, Squeeze ◐, Unleash ◐, Sweeping Gaze ⚠️.
- **Ethereal card** (tracked: yes). Payoffs: Banshee's Cry ⚠️, Eidolon ○, Pagestorm ⚠️, Pull from Below ◐, Spirit of Ash ⚠️, Veilpiercer ◐, Big Hat ○. Fed by: Defile ✅, Defy ✅, Demesne ✅, Enfeebling Touch ◐, Fear ✅, Lethality ✅, Parse ✅, Seance ○, Apparition ○, Sweeping Gaze ⚠️.
- **Retain** (tracked: yes). Payoffs: Bookmark ○. Fed by: Eradicate ✅, Reap ✅, Sacrifice ○, Snap ⚠️, Sow ✅, Spur ✅, Equilibrium ◐, Luminesce ✅, Purity ○, Restlessness ◐, Salvo ◐, Sovereign Blade ✅, The High Priestess ✅.
- **Multi-hit** (tracked: yes). Payoffs: Reaper Form ○, Sic 'Em ⚠️, Thumbscrew ✅. Fed by: Debilitate ◐, Eradicate ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅.
- **Card costing 2+** (tracked: yes). Payoffs: Danse Macabre ✅, Right Hand Hand ⚠️, Ivory Tile ✅, Intimidating Helmet ✅. Fed by: Banshee's Cry ⚠️, Bury ✅, Deathbringer ✅, Delay ✅, Demesne ✅, Eidolon ○, End of Days ◐, Flatten ⚠️, Forbidden Grimoire ○, High Five ⚠️, Melancholy ⚠️, Necro Mastery ◐, Pull Aggro ✅, Reanimate ✅, Reap ✅, Reaper Form ○, Sentry Mode ✅, Severance ✅, Sleight of Flesh ○, Squeeze ◐, The Scythe ◐, Time's Up ○, Apotheosis ○, Beat Down ○, Calamity ○, Catastrophe ○, Entrench ○, Equilibrium ◐, Eternal Armor ✅, Hand of Greed ✅, Jackpot ◐, Mayhem ○, Metamorphosis ○, Relax ⚠️, Rolling Boulder ◐, Shockwave ⚠️, Sovereign Blade ✅, The Bomb ○, Toric Toughness ◐, Whistle ◐.
- **"Strike" card** (tracked: yes). Payoffs: Strike Dummy ✅, Strike Dummy??? ✅. Fed by: Blight Strike ◐, Sculpting Strike ◐, Strike ✅, Minion Strike ✅, Seeker Strike ◐, Ultimate Strike ✅.
- **"Defend" card** (tracked: yes). Payoffs: Fasten ✅, Ghost Seed ○, Pandora's Box ·. Fed by: Defend ✅, Ultimate Defend ✅.
- **X-cost card** (tracked: yes). Payoffs: Chemical X ○. Fed by: Dirge ✅, Eradicate ✅, Volley ✅.
- **Kill (Fatal)** (tracked: no). Payoffs: Hand of Greed ✅, Book Repair Knife ·, Gremlin Horn ○, War Hammer ·. Fed by: End of Days ◐, Judgement ○.
- **Upgrade** (tracked: no). Payoffs: Miniature Cannon ✅. Fed by: Drain Power ◐, Apotheosis ○, The Devil ◐, Bone Tea ○, Fishing Rod ·, Frozen Egg ·, Lava Lamp ·, Molten Egg ·, Razor Tooth ○, Stone Cracker ○, Toxic Egg ·, War Hammer ·.
- **Replay** (tracked: partly). Payoffs: Hidden Gem ◐. Fed by: Eidolon ○, Transfigure ⚠️, Beat Down ○, Catastrophe ○, Mayhem ○, The Devil ◐, The Fool ○.

### Necrobinder: second-order chains

A converter turns one resource into another, so a producer reaches a payoff through it. Listed only where the far end has a payoff in the pool. "Visible" means the simulator reads all three links.

| From | Converter | To | Far payoffs | Visible |
|---|---|---|---|---|
| "Defend" card (2) | Fasten ✅ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Any debuff (22) | Sleight of Flesh ○ | HP loss (yours) | Centennial Puzzle ○, Meat on the Bone · | no |
| Card costing 2+ (40) | Danse Macabre ✅ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Card costing 2+ (40) | Intimidating Helmet ✅ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Card costing 2+ (40) | Ivory Tile ✅ | Energy | Ice Cream ✅ | yes |
| Card costing 2+ (40) | Right Hand Hand ⚠️ | Osty attack | Calcify ✅, Flatten ⚠️, Rattle ⚠️, Sic 'Em ⚠️, Squeeze ◐, Bone Flute ○ | no |
| Doom (11) | Death's Door ◐ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Doom (11) | End of Days ◐ | Kill (Fatal) | Hand of Greed ✅, Book Repair Knife ·, Gremlin Horn ○, War Hammer · | no |
| Doom (11) | End of Days ◐ | Any debuff | Misery ✅, Sleight of Flesh ○, Rend ◐, Unsettling Lamp ○ | no |
| Doom (11) | No Escape ⚠️ | Any debuff | Misery ✅, Sleight of Flesh ○, Rend ◐, Unsettling Lamp ○ | no |
| Doom (11) | Shroud ⚠️ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Ethereal card (10) | Eidolon ○ | Replay | Hidden Gem ◐ | no |
| Ethereal card (10) | Pagestorm ⚠️ | Extra draw | Death March ◐, Automation ⚠️ | no |
| Ethereal card (10) | Spirit of Ash ⚠️ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Exhaust (70) | Eidolon ○ | Replay | Hidden Gem ◐ | no |
| Exhaust (70) | Joss Paper ○ | Extra draw | Death March ◐, Pagestorm ⚠️, Automation ⚠️ | no |
| Extra draw (38) | Automation ⚠️ | Energy | Ice Cream ✅ | no |
| HP loss (yours) (3) | Centennial Puzzle ○ | Extra draw | Death March ◐, Pagestorm ⚠️, Automation ⚠️ | no |
| Kill (Fatal) (2) | Gremlin Horn ○ | Energy | Ice Cream ✅ | no |
| Kill (Fatal) (2) | Gremlin Horn ○ | Extra draw | Death March ◐, Pagestorm ⚠️, Automation ⚠️ | no |
| Kill (Fatal) (2) | War Hammer · | Upgrade | Miniature Cannon ✅ | no |
| Multi-hit (7) | Reaper Form ○ | Any debuff | Misery ✅, Sleight of Flesh ○, Rend ◐, Unsettling Lamp ○ | no |
| Multi-hit (7) | Reaper Form ○ | Doom | Death's Door ◐, End of Days ◐, No Escape ⚠️, Shroud ⚠️, Time's Up ○, Book Repair Knife ·, Undying Sigil ○ | no |
| Multi-hit (7) | Sic 'Em ⚠️ | Osty attack | Calcify ✅, Flatten ⚠️, Rattle ⚠️, Squeeze ◐, Bone Flute ○ | no |
| Multi-hit (7) | Sic 'Em ⚠️ | Summon (Osty HP) | Bone Shards ⚠️, Necro Mastery ◐, Protector ◐, Sacrifice ○, Spur ✅, Unleash ◐ | no |
| Osty attack (12) | Bone Flute ○ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Osty attack (12) | Sic 'Em ⚠️ | Summon (Osty HP) | Bone Shards ⚠️, Necro Mastery ◐, Protector ◐, Sacrifice ○, Spur ✅, Unleash ◐ | no |
| Soul (7) | Devour Life ⚠️ | Summon (Osty HP) | Bone Shards ⚠️, Necro Mastery ◐, Protector ◐, Sacrifice ○, Spur ✅, Unleash ◐ | no |
| Soul (7) | Soul ✅ | Extra draw | Death March ◐, Pagestorm ⚠️, Automation ⚠️ | no |
| Soul (7) | Soul ✅ | Skill played | Letter Opener ✅, Tuning Fork ✅ | yes |
| Soul (7) | Soul ✅ | Exhaust | Eidolon ○, Soul Storm ✅, The Devil ◐, The Shot ○, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○ | yes |
| Strength (11) | Friendship ✅ | Energy | Ice Cream ✅ | yes |
| Summon (Osty HP) (13) | Bone Shards ⚠️ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Summon (Osty HP) (13) | Protector ◐ | Osty attack | Calcify ✅, Flatten ⚠️, Rattle ⚠️, Sic 'Em ⚠️, Squeeze ◐, Bone Flute ○ | no |
| Summon (Osty HP) (13) | Sacrifice ○ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Summon (Osty HP) (13) | Unleash ◐ | Osty attack | Calcify ✅, Flatten ⚠️, Rattle ⚠️, Sic 'Em ⚠️, Squeeze ◐, Bone Flute ○ | no |

Loops (a resource that comes back to itself through converters):

- Osty attack → [Sic 'Em] → Summon (Osty HP) → [Protector] → Osty attack
- Osty attack → [Sic 'Em] → Summon (Osty HP) → [Unleash] → Osty attack

## Regent

### Regent: engines

**Stars.**

- Gainers:
  - Venerate ✅ (starter), Solar Strike ✅, Gather Light ✅, Glow ◐, Hidden Cache ◐, Shining Strike ◐, Big Bang ✅
  - per turn or per card: Genesis ✅ (2 a turn), The Sealed Throne ✅ (1 per card played)
  - Royal Gamble ⚠️ (9), Convergence ◐, Knockout Blow ◐ (5 on a kill)
  - relics: Lunar Pastry ✅, Divine Right ✅, Divine Destiny ✅
- Spenders (cost Stars):
  - Falling Star ✅ (starter), Meteor Shower ✅, Astral Pulse ✅, Comet ✅, Devastate ✅, Gamma Blast ✅, Seven Stars ✅
  - Stardust ⚠️ (X Stars), Cloak of Stars ✅, Particle Wall ◐ (returns to the hand), Reflect ◐, Resonance ⚠️, Alignment ✅, Quasar ○
  - Crescent Spear ◐, Neutron Aegis ✅, The Smith ✅, Decisions, Decisions ◐, Royal Gamble ⚠️
- Payoffs:
  - Black Hole ⚠️ (3 to all per Star gained or spent), Child of the Stars ○ (2 Block per Star spent)
  - Mini Regent ○ (+1 Strength on the first spend each turn), Galactic Dust ○ (10 Block per 10 spent)
  - Radiate ◐ (3 to all per Star gained this turn), Crescent Spear ◐ (+2 per Star-cost card in the deck)
- Second order:
  - Black Hole counts both sides: a Star gained and then spent triggers it twice. With The Sealed Throne (+1 Star per card), every card played is at least one trigger.
  - Particle Wall costs 2 Stars and returns to the hand, so it can be replayed as long as Stars last: 9 Block a time, plus Child of the Stars' Block and Black Hole's damage per Star.
- Simulator:
  - Star gain and spend are tracked.
  - Royal Gamble's 9 Stars read as 0 (`STARS_VAR`), and Stardust ignores its X (`X_STARS`).
  - Black Hole is misread as one Star and one hit on play. Child of the Stars, Mini Regent and Galactic Dust need an on-Stars trigger (`ON_STARS`); the spend hook exists.

**Forge and Sovereign Blade.**

- The first Forge each combat adds Sovereign Blade (2 energy, 10 damage, Retain); every Forge adds to its damage.
- Forgers: Wrought in War ✅, Refine Blade ✅, Spoils of Battle ✅, Bulwark ✅, Big Bang ✅, The Smith ✅ (30), Furnace ✅ (5 a turn), Seeking Edge ✅, Conqueror ◐, Summon Forth ✅, Beat into Shape ◐, Fencing Manual ✅.
- Blade payoffs: Sword Sage ✅ (Replay 1: the Blade plays twice), Parry ⚠️ (10 Block per Blade play), Seeking Edge ✅ (hits all), Conqueror ◐ (double damage this turn), Summon Forth ✅ (fetches it).
- Second order:
  - Every Forge is multiplied by the Blade's plays: with Sword Sage each Forge point is 2 damage a turn, and Conqueror doubles that again.
  - Strength adds per play, so per replay too.
  - Parry's Block repeats with each replay.
  - Pen Nib and Mighty Wings reward the Blade's single big hit.
- Simulator: Forge, the Blade and Sword Sage's Replay are followed. Conqueror's doubling (`NAMED_DMG_MULT`) and Parry (`ON_PLAY_NAMED`) are not.

**Card creation.**

- Makers: Collision Course ✅ (Debris), Crash Landing ◐ (fills the hand with Debris), Heirloom Hammer ◐, Bundle of Joy ○, Manifest Authority ◐, Spectrum Shift ○, Quasar ○, Orange Dough ○. BEGONE!, GUARDS!!! and CHARGE!! transform cards into Minion cards; whether a transform counts as "create" isn't stated.
- Payoffs: Arsenal ⚠️ (+1 Strength per card created), Pillar of Creation ⚠️ (2 Block per card created), Supermassive ◐ (+3 per card created this combat), Regalite ○ (4 Block on the first each turn).
- Second order:
  - Crash Landing fills the hand: with 5 cards left that is 5 Debris, so +5 Strength from Arsenal, 10 Block from Pillar, and +15 on Supermassive.
  - Debris costs 1 and exhausts, so it also clears itself and feeds exhaust payoffs.
- Simulator: making cards is tracked (`make` is one entry point), so the three payoffs need `ON_MAKE` and a made-cards counter. Crash Landing's fill isn't read (`FILL_HAND`).

**Minions.**

- BEGONE! ○ (Minion Strike: 0-cost, draw 1, counts as a Strike), GUARDS!!! ○ (Minion Sacrifice: 0-cost, 7 Block), CHARGE!! ○ (Minion Dive Bomb: 0-cost, 11 damage). All Minion cards exhaust.
- Payoff: Vitruvian Minion ○ (double damage and Block).
- Second order: Minion cards are 0-cost Exhaust cards, so they feed the attack counters, Strike Dummy (Minion Strike) and exhaust payoffs.
- Simulator: transforms aren't read, so none of this.

**Skill count.** Lunar Blast ◐ (4 per Skill this turn), Make It So ◐ (returns every 3 Skills), Letter Opener ✅ (5 to all per 3 Skills), Decisions, Decisions ◐ (plays a Skill 3 times).

**Anti-synergies.**

- Crash Landing's Debris clogs later draws if not played or exhausted.
- Stars spent on Royal Gamble's 5 aren't available to the Star cards that turn.

### Regent: first-order index

Every payoff in the pool, with what feeds it. Class cards come first, then Colorless, tokens, event cards and relics.

- **Exhaust** (tracked: yes). Payoffs: Bombardment ✅, The Devil ◐, The Shot ○, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○. Fed by: 66 cards and relics, for example Big Bang ✅, Bombardment ✅, Bundle of Joy ○, Decisions, Decisions ◐, GUARDS!!! ○, Know Thy Place ✅, Royal Gamble ⚠️, Tyranny ◐.
- **Discard** (tracked: partly (whole-hand discards only)). Payoffs: Gambling Chip ○. Fed by: The Hermit ◐, Wheel of Fortune ✅, Runic Pyramid ✅.
- **Extra draw** (tracked: yes). Payoffs: Kingly Kick ◐, Kingly Punch ◐, Automation ⚠️. Fed by: 42 cards and relics, for example Big Bang ✅, Decisions, Decisions ◐, Glimmer ◐, Glow ◐, Guiding Star ⚠️, Pale Blue Dot ○, Photon Cut ◐, Prophesize ✅ and 2 more.
- **Shuffle** (tracked: yes). Payoffs: Stratagem ○, The Abacus ✅. Fed by: nothing in the pool.
- **HP loss (yours)** (tracked: no). Payoffs: Centennial Puzzle ○, Meat on the Bone ·. Fed by: The Hanged Man ○, Royal Poison ○.
- **Block gain** (tracked: yes). Payoffs: Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○. Fed by: 42 cards and relics, for example Bulwark ✅, Child of the Stars ○, Cloak of Stars ✅, Cosmic Indifference ◐, Defend ✅, Gather Light ✅, Glitterstream ◐, I Am Invincible ◐ and 7 more.
- **Energy** (tracked: yes). Payoffs: Orbit ⚠️, Ice Cream ✅. Fed by: Alignment ✅, Big Bang ✅, Convergence ◐, Hegemony ✅, Orbit ⚠️, Refine Blade ✅, Automation ⚠️, Brightest Flame ✅, Fuel ✅, Joy ✅, Luminesce ✅, Outmaneuver ✅, Production ✅, Relax ⚠️, Restlessness ◐, The Emperor ✅, The Star ⚠️, The Sun ✅, Art of War ○, Blessed Antler ◐, Blood-Soaked Rose ✅, Booming Conch ○, Bread ⚠️, Candelabra ✅, Chandelier ✅, Ectoplasm ✅, Gremlin Horn ○, Happy Flower ✅, Happy Flower??? ✅, Nunchaku ✅, Pael's Flesh ✅, Pael's Tears ○, Philosopher's Stone ◐, Prismatic Gem ✅, Pumpkin Candle ✅, Seal of Gold ✅, Sozu ✅, Spiked Gauntlets ◐, Velvet Choker ◐, Whispering Earring ◐.
- **Strength** (tracked: yes). Payoffs: Astral Pulse ✅, Celestial Might ✅, Heavenly Drill ⚠️, Resonance ⚠️, Seven Stars ✅, Stardust ⚠️, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅. Fed by: Arsenal ⚠️, Monologue ○, Resonance ⚠️, Feeding Frenzy ⚠️, Prowess ✅, Strength ✅, Mini Regent ○, Ember Tea ○, Girya ·, Mighty Wings ○, Rainbow Ring ○, Reptile Trinket ○, Shuriken ✅, Sparkling Rouge ✅, Toasty Mittens ◐.
- **Any debuff** (tracked: counted). Payoffs: Rend ◐, Unsettling Lamp ○. Fed by: Comet ✅, Falling Star ✅, Gamma Blast ✅, Know Thy Place ✅, Meteor Shower ✅, Death ✅, Shockwave ⚠️, Squash ✅, The Empress ⚠️, Bag of Marbles ✅, Hand Drill ○, Red Mask ○, Tormentous Collar ○.
- **Card created** (tracked: yes). Payoffs: Arsenal ⚠️, Pillar of Creation ⚠️, Supermassive ◐, Regalite ○. Fed by: 49 cards and relics, for example Bundle of Joy ○, Collision Course ✅, Crash Landing ◐, Foregone Conclusion ○, Heirloom Hammer ◐, Make It So ◐, Manifest Authority ◐, Quasar ○ and 4 more.
- **Stars gained** (tracked: yes). Payoffs: Black Hole ⚠️, Radiate ◐. Fed by: Big Bang ✅, Gather Light ✅, Genesis ✅, Glow ◐, Hidden Cache ◐, Knockout Blow ◐, Royal Gamble ⚠️, Shining Strike ◐, Solar Strike ✅, The Sealed Throne ✅, Venerate ✅, Divine Destiny ✅, Divine Right ✅, Lunar Pastry ✅.
- **Stars spent** (tracked: yes). Payoffs: Child of the Stars ○, Galactic Dust ○, Mini Regent ○. Fed by: Alignment ✅, Astral Pulse ✅, Cloak of Stars ✅, Comet ✅, Crescent Spear ◐, Decisions, Decisions ◐, Devastate ✅, Dying Star ◐, Falling Star ✅, Gamma Blast ✅, Guiding Star ⚠️, Meteor Shower ✅, Neutron Aegis ✅, Particle Wall ◐, Quasar ○, Reflect ◐, Resonance ⚠️, Royal Gamble ⚠️, Seven Stars ✅, Stardust ⚠️, The Sealed Throne ✅, The Smith ✅.
- **Sovereign Blade** (tracked: yes). Payoffs: Conqueror ◐, Parry ⚠️, Seeking Edge ✅, Sword Sage ✅. Fed by: Beat into Shape ◐, Big Bang ✅, Bulwark ✅, Conqueror ◐, Furnace ✅, Refine Blade ✅, Seeking Edge ✅, Spoils of Battle ✅, Summon Forth ✅, The Smith ✅, Wrought in War ✅, Fencing Manual ✅.
- **Multi-hit** (tracked: yes). Payoffs: Beat into Shape ◐, Thumbscrew ✅. Fed by: Astral Pulse ✅, Celestial Might ✅, Heavenly Drill ⚠️, Seven Stars ✅, Stardust ⚠️, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅.
- **Card costing 2+** (tracked: yes). Payoffs: Intimidating Helmet ✅. Fed by: Bombardment ✅, Bulwark ✅, Celestial Might ✅, GUARDS!!! ○, Genesis ✅, Glitterstream ◐, Hegemony ✅, Heirloom Hammer ◐, Kingly Kick ◐, Knockout Blow ◐, Monarch's Gaze ○, Orbit ⚠️, Prophesize ✅, Seven Stars ✅, Spectrum Shift ○, Sword Sage ✅, Void Form ○, Apotheosis ○, Beat Down ○, Calamity ○, Catastrophe ○, Entrench ○, Equilibrium ◐, Eternal Armor ✅, Hand of Greed ✅, Jackpot ◐, Mayhem ○, Metamorphosis ○, Relax ⚠️, Rolling Boulder ◐, Shockwave ⚠️, Sovereign Blade ✅, The Bomb ○, Toric Toughness ◐, Whistle ◐.
- **"Strike" card** (tracked: yes). Payoffs: Strike Dummy ✅, Strike Dummy??? ✅. Fed by: Shining Strike ◐, Solar Strike ✅, Strike ✅, Minion Strike ✅, Seeker Strike ◐, Ultimate Strike ✅.
- **"Defend" card** (tracked: yes). Payoffs: Fasten ✅, Ghost Seed ○, Pandora's Box ·. Fed by: Defend ✅, Ultimate Defend ✅.
- **"Minion" card** (tracked: yes). Payoffs: Vitruvian Minion ○. Fed by: Minion Dive Bomb ✅, Minion Sacrifice ✅, Minion Strike ✅.
- **X-cost card** (tracked: yes). Payoffs: Chemical X ○. Fed by: Heavenly Drill ⚠️, Stardust ⚠️, Volley ✅.
- **Kill (Fatal)** (tracked: no). Payoffs: Knockout Blow ◐, Hand of Greed ✅, Gremlin Horn ○, War Hammer ·. Fed by: Judgement ○.
- **Upgrade** (tracked: no). Payoffs: Miniature Cannon ✅. Fed by: Apotheosis ○, The Devil ◐, Bone Tea ○, Fishing Rod ·, Frozen Egg ·, Lava Lamp ·, Molten Egg ·, Razor Tooth ○, Stone Cracker ○, Toxic Egg ·, War Hammer ·.
- **Replay** (tracked: partly). Payoffs: Hidden Gem ◐. Fed by: Decisions, Decisions ◐, Sword Sage ✅, Beat Down ○, Catastrophe ○, Mayhem ○, The Devil ◐, The Fool ○.

### Regent: second-order chains

A converter turns one resource into another, so a producer reaches a payoff through it. Listed only where the far end has a payoff in the pool. "Visible" means the simulator reads all three links.

| From | Converter | To | Far payoffs | Visible |
|---|---|---|---|---|
| "Defend" card (2) | Fasten ✅ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Card costing 2+ (35) | Intimidating Helmet ✅ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Card created (49) | Arsenal ⚠️ | Strength | Astral Pulse ✅, Celestial Might ✅, Heavenly Drill ⚠️, Resonance ⚠️, Seven Stars ✅, Stardust ⚠️, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅ | no |
| Card created (49) | Pillar of Creation ⚠️ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Card created (49) | Regalite ○ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Exhaust (66) | Burning Sticks ○ | Card created | Arsenal ⚠️, Pillar of Creation ⚠️, Supermassive ◐, Regalite ○ | no |
| Exhaust (66) | Joss Paper ○ | Extra draw | Kingly Kick ◐, Kingly Punch ◐, Automation ⚠️ | no |
| Extra draw (42) | Automation ⚠️ | Energy | Orbit ⚠️, Ice Cream ✅ | no |
| HP loss (yours) (2) | Centennial Puzzle ○ | Extra draw | Kingly Kick ◐, Kingly Punch ◐, Automation ⚠️ | no |
| Kill (Fatal) (1) | Gremlin Horn ○ | Energy | Orbit ⚠️, Ice Cream ✅ | no |
| Kill (Fatal) (1) | Gremlin Horn ○ | Extra draw | Kingly Kick ◐, Kingly Punch ◐, Automation ⚠️ | no |
| Kill (Fatal) (1) | Knockout Blow ◐ | Stars gained | Black Hole ⚠️, Radiate ◐ | no |
| Kill (Fatal) (1) | War Hammer · | Upgrade | Miniature Cannon ✅ | no |
| Multi-hit (10) | Beat into Shape ◐ | Sovereign Blade | Conqueror ◐, Parry ⚠️, Seeking Edge ✅, Sword Sage ✅ | no |
| Sovereign Blade (12) | Sovereign Blade ✅ | Card costing 2+ | Intimidating Helmet ✅ | yes |
| Sovereign Blade (12) | Sovereign Blade ✅ | Attack played | Monarch's Gaze ○, Calamity ○, Nostalgia ○, Daughter of the Wind ✅, Kunai ✅, Kusarigama ✅, Music Box ○, Nunchaku ✅, Ornamental Fan ✅, Pen Nib ○, Shuriken ✅ | yes |
| Sovereign Blade (12) | Sword Sage ✅ | Replay | Hidden Gem ◐ | no |
| Stars spent (22) | Child of the Stars ○ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Stars spent (22) | Galactic Dust ○ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Stars spent (22) | Mini Regent ○ | Strength | Astral Pulse ✅, Celestial Might ✅, Heavenly Drill ⚠️, Resonance ⚠️, Seven Stars ✅, Stardust ⚠️, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅ | no |

## The Fool

### The Fool: engines

**Arrows.**

- The Fool starts with 1 Arrow slot (Quiver ✅ adds more). A Shoot card spends an Arrow, which sticks in the enemy (Impaled). The Arrow comes back after 3 more hits on that enemy, or when a card Dislodges it.
- Shooters: Foolish Shot ✅ (starter), Quick Shot ✅, Heavy Shot ⚠️ (Aim counts twice), Rain Shot ✅, Sky Shot ✅, Divine Shot ✅, Melodic Shot ◐, Repeat Shot ◐ (X), Blossom Shot ◐, Hobbling Shot ⚠️, Rapid Shot ✅, Trick Shot ◐ (returns to the hand), The Shot ○.
- Aim (adds to the next Arrow): Song Bow ✅ (6 at combat start), Nock ✅, Cho Ready ✅, Archery ✅ (4 a turn), Brace ✅ (X × 8), Show Off ✅, Whittling Knife ✅ (5 per shuffle), Storyteller ✅.
- Payoffs:
  - Impaled enemies: Bow Smack ⚠️ (+1 energy), Ki Hop ⚠️ (draw 2), Wah Hammer ⚠️ (double damage), Nail ⚠️ (20 damage, Weak and Vulnerable), Uproot ⚠️, Wrest ⚠️, Recall ○, Serrated Arrows ○ (+25%), Explosive Arrows ○ (6 to all per Impale)
  - Dislodge: Yah ○ (+1 energy, first each turn), Twine ○ (4 Block), Epic Retrieval ⚠️ (+3 energy)
  - Shots: Honing ○ (+50%), Dislike You ◐ (+50% this turn), Quietus ○ (double, destroys the Arrow), Whistling Arrows ⚠️ (returns per shot)
- Second order:
  - Multi-hit cards (Rapid Shot, Wah Ki) bring the Arrow back faster (3 hits), so more Shots per turn. Each Shot is another Impale for Explosive Arrows.
  - Wrest dislodges for 0 and draws 2. With Yah that is +1 energy, with Twine 4 Block, so a Shot and Wrest form a cheap loop.
  - Archery's Aim each turn doubles on Heavy Shot.
- Simulator:
  - Arrows, stuck Arrows, returns after hits, dislodges and Aim are tracked.
  - Every "if the enemy is Impaled" card fires unconditionally (`IMPALE_COND`), though the stuck count needed to gate it is already there.
  - Heavy Shot's double Aim isn't read, and Whistling Arrows wrongly spends an Arrow.
  - Honing, Serrated Arrows, Explosive Arrows, Yah and Twine are unread.

**Melodies.**

- Wah, Cho and Ki cards (Wah Hammer, Wah Sweep, Wah Touch, Wah Blast, Wah Cho, Wah Ki, Wah Cho Ki, Ki Hop, Ki Whip, Ki Chant, Ki Rest, Ki Verse, Cho Barbs, Cho Firm, Cho Ready, Cho Ki, Cho Limbs, Cho Chorus).
  - Community guides (not the Codex data) say playing one gains that Melody.
  - Ocarina ○ adds a random one at combat start.
- Payoffs:
  - per Melody gained: Violent Chord ⚠️ (9 to all per Wah), Song Path ⚠️ (5 Block per Cho), Motif ⚠️ (draw per Melody), Lost Lute ○ (plays top cards)
  - on having every Melody: Lullaby ○ (20 Block), Conductor's Baton ○
  - per Melody held: Coda ◐ (−2 cost each), Melodic Shot ◐ (+1 hit each), Rhythm Strike ◐ (+1 hit each), Steady Cadence ◐ (draw each), Harmonic Shield ○ (5 Block each)
  - specific Melodies: Wah Blast ◐ (second hit with Cho or Ki), Cho Limbs ⚠️ (−2 with Wah or Ki), Ki Rest ◐ (Retain with Wah or Cho), Tune ○ (with Ki, next cards free)
  - Wah Symphony ◐ (+4 per Wah/Cho/Ki card in the deck)
  - Ki Verse ○ (next Wah/Cho/Ki card twice), Cho Chorus ○ (plays every Wah and Ki card in the hand)
- Second order:
  - Cho Chorus plays every Wah and Ki card in the hand at once, gaining Wah and Ki Melodies. That fires Violent Chord, Motif and Lost Lute per Melody, and with a Cho Melody from Cho Chorus itself, Lullaby.
  - Ki Verse doubles a Wah/Cho/Ki card; whether the second play gains the Melody again isn't stated.
  - Coda at 6 energy drops to 0 with three Melodies.
- Simulator: nothing. There is no Melody state, so all 15 payoffs are flat, misread or ignored. Melody state (three flags, set when a Wah/Cho/Ki card is played, plus a gained event) would make the whole archetype readable. Confirm the gain rule in game first.

**Sadness and Joy.**

- Sadness is a Retain, Unplayable Status. Joy is 0-cost, Exhaust: +1 energy, draw 1.
- Sadness makers: Sorrowful Strike ✅ (2), Mourning Blow ◐ (turns into one), Jest ✅, Rummage ✅, Vitriol ✅, Ennui ✅ (1 a turn), Brighten Up ✅ (2 Sadness, 2 Joy), Bizarre Lamp ⚠️.
- Payoffs:
  - per Sadness created: Anguish ◐ (+6 each this combat), All Is Well ⚠️ (4 Block), Fertilizer ○ (+1 Strength)
  - converters: Growth ✅ (exhausts hand Sadness for 4 Stretch each), Punch of Joy ◐ and Ecstasy ○ (turn Sadness into Joy)
- Second order:
  - Ennui makes a Sadness every turn: Fertilizer's Strength, All Is Well's Block and Anguish's damage all grow each turn.
  - Growth then clears the pile into Stretch, and the Block cards spend it.
  - Ecstasy turns one held Sadness into Joy each turn, which refunds the energy and the card.
  - Sadness retains, so without Growth or Ecstasy it fills the hand and shrinks every draw.
- Simulator: Sadness is made and retained (the clog is visible), Growth is read. The per-Sadness payoffs aren't (`ON_MAKE_NAMED`, `COUNTER_MADE`), and Bizarre Lamp makes nothing (`MAKE_PARSE`).

**Stretch.**

- Stretch adds to your next Block gain from a card.
- Sources: Hidden Roots ✅ (3 a turn), Deep Stretch ✅, Bounce ◐, Blossom Shot ◐ (Stretch = damage), Growth ✅, Cho Ki ◐ (regains spent Stretch), Exercise Mat ✅.
- Second order: Blossom Shot's Stretch scales with Aim, so Archery or Nock before Blossom Shot is also Block.
- Simulator: Stretch is tracked and spent by the next Block card.

**Anti-synergies.**

- Quietus destroys the Arrow it doubles, so the Fool has no Arrow until Quiver or a new turn's return.
- Sadness clogs the hand.
- Encore ◐ draws but then allows only one more card, which works against Cho Chorus and Melody chains.

### The Fool: first-order index

Every payoff in the pool, with what feeds it. Class cards come first, then Colorless, tokens, event cards and relics.

- **Exhaust** (tracked: yes). Payoffs: Growth ✅, The Devil ◐, The Shot ○, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○. Fed by: 65 cards and relics, for example Blot out the Sky ○, Brighten Up ✅, Florescence ○, Growth ✅, Quietus ○, Show Off ✅, Sky Shot ✅.
- **Discard** (tracked: partly (whole-hand discards only)). Payoffs: Gambling Chip ○. Fed by: The Hermit ◐, Wheel of Fortune ✅, Runic Pyramid ✅.
- **Extra draw** (tracked: yes). Payoffs: Automation ⚠️. Fed by: 42 cards and relics, for example Deep Stretch ✅, Encore ◐, Ennui ✅, Ki Hop ⚠️, Motif ⚠️, Rummage ✅, Steady Cadence ◐, Wah Cho Ki ◐ and 2 more.
- **Shuffle** (tracked: yes). Payoffs: Stratagem ○, Whittling Knife ✅, The Abacus ✅. Fed by: nothing in the pool.
- **HP loss (yours)** (tracked: no). Payoffs: Centennial Puzzle ○, Meat on the Bone ·. Fed by: The Hanged Man ○, Royal Poison ○.
- **Block gain** (tracked: yes). Payoffs: Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○. Fed by: 43 cards and relics, for example All Is Well ⚠️, Cho Barbs ◐, Cho Chorus ○, Cho Firm ◐, Cho Ki ◐, Cho Limbs ⚠️, Cho Ready ✅, Defend ✅ and 8 more.
- **Energy** (tracked: yes). Payoffs: Ice Cream ✅. Fed by: 41 cards and relics, for example Bow Smack ⚠️, Encore ◐, Epic Retrieval ⚠️, Ki Rest ◐, Ki Whip ◐, Wah Ki ✅, Yah! ○.
- **Strength** (tracked: yes). Payoffs: Heavy Shot ⚠️, Rapid Shot ✅, Repeat Shot ◐, Wah Blast ◐, Wah Ki ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅. Fed by: Feeding Frenzy ⚠️, Prowess ✅, Strength ✅, Fertilizer ○, Ember Tea ○, Girya ·, Mighty Wings ○, Rainbow Ring ○, Reptile Trinket ○, Shuriken ✅, Sparkling Rouge ✅, Toasty Mittens ◐.
- **Any debuff** (tracked: counted). Payoffs: Rend ◐, Unsettling Lamp ○. Fed by: Hobbling Shot ⚠️, Nail ⚠️, Uproot ⚠️, Vitriol ✅, Wah Touch ◐, Death ✅, Shockwave ⚠️, Squash ✅, The Empress ⚠️, Bag of Marbles ✅, Hand Drill ○, Red Mask ○, Tormentous Collar ○.
- **Sadness** (tracked: yes). Payoffs: All Is Well ⚠️, Anguish ◐, Ecstasy ○, Growth ✅, Punch of Joy ◐, Fertilizer ○. Fed by: Brighten Up ✅, Ennui ✅, Jest ✅, Mourning Blow ◐, Rummage ✅, Sorrowful Strike ✅, Vitriol ✅.
- **Shot (Arrow spent)** (tracked: yes). Payoffs: Arrow Stab ◐, Blot out the Sky ○, Dislike You ◐, Florescence ○, Honing ○, Locked In ○, Quietus ○, Whistling Arrows ⚠️. Fed by: Blossom Shot ◐, Divine Shot ✅, Foolish Shot ✅, Heavy Shot ⚠️, Hobbling Shot ⚠️, Melodic Shot ◐, Quick Shot ✅, Rain Shot ✅, Rapid Shot ✅, Repeat Shot ◐, Sky Shot ✅, Trick Shot ◐, The Shot ○.
- **Impaled enemy** (tracked: yes). Payoffs: Bow Smack ⚠️, Explosive Arrows ○, Ki Hop ⚠️, Nail ⚠️, Recall ○, Serrated Arrows ○, Uproot ⚠️, Wah Hammer ⚠️, Wrest ⚠️. Fed by: Blossom Shot ◐, Divine Shot ✅, Foolish Shot ✅, Heavy Shot ⚠️, Hobbling Shot ⚠️, Melodic Shot ◐, Quick Shot ✅, Rain Shot ✅, Rapid Shot ✅, Repeat Shot ◐, Sky Shot ✅, Trick Shot ◐, The Shot ○.
- **Dislodge** (tracked: yes). Payoffs: Epic Retrieval ⚠️, Yah! ○, Twine ○. Fed by: Recall ○, Uproot ⚠️, Wrest ⚠️.
- **Aim** (tracked: yes). Payoffs: Heavy Shot ⚠️. Fed by: Archery ✅, Brace ✅, Cho Ready ✅, Nock ✅, Show Off ✅, Song Bow ✅, Storyteller ✅, Whittling Knife ✅.
- **Stretch** (tracked: yes). Payoffs: Cho Ki ◐. Fed by: Blossom Shot ◐, Bounce ◐, Deep Stretch ✅, Growth ✅, Hidden Roots ✅, Exercise Mat ✅.
- **Melody (Wah/Cho/Ki card)** (tracked: no). Payoffs: Cho Limbs ⚠️, Coda ◐, Harmonic Shield ○, Ki Rest ◐, Lost Lute ○, Lullaby ○, Melodic Shot ◐, Motif ⚠️, Rhythm Strike ◐, Song Path ⚠️, Steady Cadence ◐, Tune ○, Violent Chord ⚠️, Wah Blast ◐, Conductor's Baton ○. Fed by: Cho Barbs ◐, Cho Chorus ○, Cho Firm ◐, Cho Ki ◐, Cho Limbs ⚠️, Cho Ready ✅, Ki Chant ○, Ki Hop ⚠️, Ki Rest ◐, Ki Verse ○, Ki Whip ◐, Wah Blast ◐, Wah Cho ◐, Wah Cho Ki ◐, Wah Hammer ⚠️, Wah Ki ✅, Wah Sweep ◐, Wah Symphony ◐, Wah Touch ◐.
- **Multi-hit** (tracked: yes). Payoffs: Thumbscrew ✅. Fed by: Heavy Shot ⚠️, Rapid Shot ✅, Repeat Shot ◐, Wah Blast ◐, Wah Ki ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅.
- **Card costing 2+** (tracked: yes). Payoffs: Intimidating Helmet ✅. Fed by: Blossom Shot ◐, Cho Chorus ○, Cho Limbs ⚠️, Coda ◐, Ecstasy ○, Epic Retrieval ⚠️, Heavy Shot ⚠️, Honing ○, Jest ✅, Ki Rest ◐, Lost Lute ○, Motif ⚠️, Mythic Form ○, Nail ⚠️, Punch of Joy ◐, Simpleton ○, Timber ◐, Tune ○, Wah Blast ◐, Wah Symphony ◐, Apotheosis ○, Beat Down ○, Calamity ○, Catastrophe ○, Entrench ○, Equilibrium ◐, Eternal Armor ✅, Hand of Greed ✅, Jackpot ◐, Mayhem ○, Metamorphosis ○, Relax ⚠️, Rolling Boulder ◐, Shockwave ⚠️, Sovereign Blade ✅, The Bomb ○, Toric Toughness ◐, Whistle ◐.
- **"Strike" card** (tracked: yes). Payoffs: Strike Dummy ✅, Strike Dummy??? ✅. Fed by: Rhythm Strike ◐, Sorrowful Strike ✅, Strike ✅, Minion Strike ✅, Seeker Strike ◐, Ultimate Strike ✅.
- **"Defend" card** (tracked: yes). Payoffs: Fasten ✅, Ghost Seed ○, Pandora's Box ·. Fed by: Defend ✅, Ultimate Defend ✅.
- **X-cost card** (tracked: yes). Payoffs: Chemical X ○. Fed by: Brace ✅, Repeat Shot ◐, Volley ✅.
- **Kill (Fatal)** (tracked: no). Payoffs: Show Off ✅, Timber ◐, Hand of Greed ✅, Gremlin Horn ○, War Hammer ·. Fed by: Judgement ○.
- **Upgrade** (tracked: no). Payoffs: Miniature Cannon ✅. Fed by: Apotheosis ○, The Devil ◐, Bone Tea ○, Fishing Rod ·, Frozen Egg ·, Lava Lamp ·, Molten Egg ·, Razor Tooth ○, Stone Cracker ○, Toxic Egg ·, War Hammer ·.
- **Replay** (tracked: partly). Payoffs: Hidden Gem ◐. Fed by: Cho Chorus ○, Ki Verse ○, Lost Lute ○, Beat Down ○, Catastrophe ○, Mayhem ○, The Devil ◐, The Fool ○.

### The Fool: second-order chains

A converter turns one resource into another, so a producer reaches a payoff through it. Listed only where the far end has a payoff in the pool. "Visible" means the simulator reads all three links.

| From | Converter | To | Far payoffs | Visible |
|---|---|---|---|---|
| "Defend" card (2) | Fasten ✅ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Aim (8) | Heavy Shot ⚠️ | Impaled enemy | Bow Smack ⚠️, Explosive Arrows ○, Ki Hop ⚠️, Nail ⚠️, Recall ○, Serrated Arrows ○, Uproot ⚠️, Wah Hammer ⚠️, Wrest ⚠️ | no |
| Aim (8) | Heavy Shot ⚠️ | Shot (Arrow spent) | Arrow Stab ◐, Blot out the Sky ○, Dislike You ◐, Florescence ○, Honing ○, Locked In ○, Quietus ○, Whistling Arrows ⚠️ | no |
| Card costing 2+ (38) | Intimidating Helmet ✅ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Dislodge (3) | Epic Retrieval ⚠️ | Energy | Ice Cream ✅ | no |
| Dislodge (3) | Twine ○ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Dislodge (3) | Yah! ○ | Energy | Ice Cream ✅ | no |
| Exhaust (65) | Growth ✅ | Stretch | Cho Ki ◐ | no |
| Exhaust (65) | Joss Paper ○ | Extra draw | Automation ⚠️ | no |
| Extra draw (42) | Automation ⚠️ | Energy | Ice Cream ✅ | no |
| HP loss (yours) (2) | Centennial Puzzle ○ | Extra draw | Automation ⚠️ | no |
| Impaled enemy (13) | Bow Smack ⚠️ | Energy | Ice Cream ✅ | no |
| Impaled enemy (13) | Ki Hop ⚠️ | Extra draw | Automation ⚠️ | no |
| Impaled enemy (13) | Nail ⚠️ | Any debuff | Rend ◐, Unsettling Lamp ○ | no |
| Impaled enemy (13) | Recall ○ | Dislodge | Epic Retrieval ⚠️, Yah! ○, Twine ○ | no |
| Impaled enemy (13) | Uproot ⚠️ | Dislodge | Epic Retrieval ⚠️, Yah! ○, Twine ○ | no |
| Impaled enemy (13) | Uproot ⚠️ | Any debuff | Rend ◐, Unsettling Lamp ○ | no |
| Impaled enemy (13) | Wrest ⚠️ | Dislodge | Epic Retrieval ⚠️, Yah! ○, Twine ○ | no |
| Impaled enemy (13) | Wrest ⚠️ | Extra draw | Automation ⚠️ | no |
| Joy (4) | Joy ✅ | Energy | Ice Cream ✅ | yes |
| Joy (4) | Joy ✅ | Extra draw | Automation ⚠️ | no |
| Joy (4) | Joy ✅ | Skill played | Letter Opener ✅, Tuning Fork ✅ | yes |
| Joy (4) | Joy ✅ | Exhaust | Growth ✅, The Devil ◐, The Shot ○, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○ | yes |
| Kill (Fatal) (1) | Gremlin Horn ○ | Energy | Ice Cream ✅ | no |
| Kill (Fatal) (1) | Gremlin Horn ○ | Extra draw | Automation ⚠️ | no |
| Kill (Fatal) (1) | Show Off ✅ | Aim | Heavy Shot ⚠️ | no |
| Kill (Fatal) (1) | War Hammer · | Upgrade | Miniature Cannon ✅ | no |
| Melody (Wah/Cho/Ki card) (19) | Cho Limbs ⚠️ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Melody (Wah/Cho/Ki card) (19) | Harmonic Shield ○ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Melody (Wah/Cho/Ki card) (19) | Ki Rest ◐ | Energy | Ice Cream ✅ | no |
| Melody (Wah/Cho/Ki card) (19) | Lost Lute ○ | Replay | Hidden Gem ◐ | no |
| Melody (Wah/Cho/Ki card) (19) | Lullaby ○ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Melody (Wah/Cho/Ki card) (19) | Melodic Shot ◐ | Impaled enemy | Bow Smack ⚠️, Explosive Arrows ○, Ki Hop ⚠️, Nail ⚠️, Recall ○, Serrated Arrows ○, Uproot ⚠️, Wah Hammer ⚠️, Wrest ⚠️ | no |
| Melody (Wah/Cho/Ki card) (19) | Melodic Shot ◐ | Shot (Arrow spent) | Arrow Stab ◐, Blot out the Sky ○, Dislike You ◐, Florescence ○, Honing ○, Locked In ○, Quietus ○, Whistling Arrows ⚠️ | no |
| Melody (Wah/Cho/Ki card) (19) | Motif ⚠️ | Extra draw | Automation ⚠️ | no |
| Melody (Wah/Cho/Ki card) (19) | Song Path ⚠️ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Melody (Wah/Cho/Ki card) (19) | Steady Cadence ◐ | Extra draw | Automation ⚠️ | no |
| Sadness (7) | All Is Well ⚠️ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Sadness (7) | Fertilizer ○ | Strength | Heavy Shot ⚠️, Rapid Shot ✅, Repeat Shot ◐, Wah Blast ◐, Wah Ki ✅, Exterminate ✅, Maul ◐, Peck ✅, Rip and Tear ✅, Volley ✅ | no |
| Sadness (7) | Growth ✅ | Exhaust | The Devil ◐, The Shot ○, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○ | yes |
| Sadness (7) | Growth ✅ | Stretch | Cho Ki ◐ | no |
| Shot (Arrow spent) (13) | Florescence ○ | Exhaust | Growth ✅, The Devil ◐, The Shot ○, Burning Sticks ○, Forgotten Soul ✅, Joss Paper ○ | no |
| Shot (Arrow spent) (13) | Whistling Arrows ⚠️ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
| Stretch (7) | Cho Ki ◐ | Block gain | Entrench ○, Prolong ○, Diamond Diadem ◐, Orichalcum ○, Orichalcum??? ○, Parrying Shield ○, Sturdy Clamp ○, Vambrace ○ | no |
