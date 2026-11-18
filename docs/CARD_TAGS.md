# Card tags, setup value and deck composition: plan

Status: **proposal**, waiting on the decisions at the end. Nothing here is
built yet.

Three asks:

1. Card categories that say what a card actually does, and that answer the
   problems the major enemies pose. One simple system for both.
2. Value Aim, Stretch and the other "boost the next card" effects properly.
3. Deck status as composition by category, compared with decks that
   succeeded, saying what we're short of.

## Where we are

**Roles.** Five macro roles: frontloaded or scaling damage, frontloaded or
scaling defense, and "Consistency / acceleration". They come from about 50
rules in `data/playbook.json`, mostly `text_contains` (for example "cost",
"into your hand", "orb slot"), plus 34 hand-assigned cards.

- **Consistency lumps unlike things together.** Draw, energy, Retain, cost
  cuts, Stars and Plasma all land there, so "you lack Consistency" doesn't
  say what to pick.
- **Text matching misreads cards.**
  - "Lose Strength" counts as defense, but anything mentioning "cost" counts
    as consistency.
  - Yoga ("ALL players gain 6 Stretch") is hand-labelled consistency.
  - Nock, Show Off and Brace (Aim) are hand-labelled scaling damage.
  - Archery (4 Aim every turn) and most Stretch cards get no role at all.
- **Roles don't map to enemies.** The enemy view shows demands ("Area damage
  missing", "Consistency / acceleration thin"), but only Area is specific.

**Aim and Stretch.**

- The game data says:
  - Aim: "Your next Arrow deals more damage."
  - Stretch: "The next time you gain Block from a card, gain additional
    Block."
- The simulator already spends them correctly. When a Shoot card is played
  it adds Aim (doubled for Heavy Shot) and clears it. Stretch adds to the
  next Block gain and clears.
- What it gets wrong is choosing when to play them:
  - **Scoring.** An Aim card scores 1 point per Aim whether or not a Shoot
    card is in reach; Stretch scores 1 per point as Block.
  - **Playing the setup card.** Nothing weighs whether a consumer is in
    hand, or whether the setup will be wasted at end of turn.
  - **The deck view.** Aim cards show up as "scaling damage" however few
    Shoot cards the deck has.

**Deck status.** It counts cards per role against fixed act targets in the
playbook, for example 4 frontload damage in Act 2. Those targets are our
guesses, not community data.

**Data we have but don't use.** Community runs (Spire Codex, refreshed by
the user) record each player's final deck, with `floor_added_to_deck` on
every card, plus per-floor removals and transforms.

- That's enough to rebuild the deck at the end of each act, so we can
  compare against decks that cleared that act's boss, or won.
- One refresh holds about 3,000 runs: 1,142 solo, about 580 past Act 1,
  and 178 wins.

## Proposal

### 1. Tags: what a card does

A card gets one or more tags. They are **read from the card's numbers**
where the data has them: damage, hits, target, block, draw, energy, and the
powers it applies with their targets. Power and relic text goes through the
`mechanics` reader we already use for the simulator. The text rules in the
playbook become the fallback for what the data leaves out, and playbook
overrides stay.

| Tag | Means | Read from |
|---|---|---|
| **Damage** | Deals damage to one enemy this turn | `damage × hits`, single target |
| **Area** | Hits every enemy | target all enemies, `DamageAll` |
| **Block** | Gains Block this turn | `block` |
| **Draw** | Draws cards | `draw`, draw effects |
| **Energy** | Gains energy or makes cards cheaper | `energy`, cost effects |
| **Weak** | Cuts what enemies deal (Weak, lowering their Strength) | powers on enemies |
| **Vulnerable** | Raises what enemies take | powers on enemies |
| **Scaling** | Grows over the fight: Strength, Dexterity, Focus, Plating, Poison, Doom, and powers that act every turn | powers on you, recurring triggers |
| **Setup** | Boosts the next card of a kind: Aim (next Arrow), Stretch (next Block), Vigor (next Attack) | powers on you |
| **Cleanup** | Exhausts or removes junk from the deck (Status, Curse) | exhaust effects |

- **Ten tags, each one sentence.**
- **Tags carry their number where there is one**, so a chip reads "Damage
  9×2", "Block 8", "Draw 2" or "Setup: Aim 5 → Arrow".
- **Scaling is one tag with a side**, damage or block, shown as an arrow
  on the chip. Splitting it into two tags would add complexity without
  making the advice more useful.
- **No catch-all.** A card that gets no tag is listed in the audit tool,
  to be read or overridden, instead of falling into "Consistency".

### 2. Problems: what an enemy asks of the deck

Each major fight (elites and bosses, and normal fights where they stand
out) gets a few **problems**. They're read from data we already compute,
and each points at the tags that solve it.

| Problem | Read from | Solved by |
|---|---|---|
| **Big hits** | a turn at or over the act's big-hit line, from move patterns | Block, Weak |
| **Groups** | more than one enemy, or summons (Phrog's 4 Wrigglers) | Area |
| **Long fight** | high HP, or a boss | Scaling |
| **Ramping** | the enemy's Strength grows every cycle | Damage (kill fast), Weak |
| **Race** | a blast after death, or an enrage | Damage, Vulnerable |
| **Junk** | moves that add Status cards | Draw, Energy, Cleanup |

The enemy view would then read, for example:

> Phrog Parasite asks for **Area** (4 Wrigglers join) and **Draw/Cleanup**
> (Infect adds statuses). You have 1 Area card, 2 Draw.

The survival check keeps doing the arithmetic. Tags say what to pick to
improve it.

### 3. Deck composition against decks that succeeded

- **The deck panel** shows each tag as a share of the deck.
- **A benchmark** sits beside each share: what decks of the same character
  held at the same point, when they went on to clear that act's boss. That
  is the 25th to 75th percentile and the median.
- **The headline** is the tags below the 25th percentile, the deck's
  shortfalls: "Short of Area: 4% (decks that cleared Act 2: 9–17%)".
- **Built at data refresh.** `refresh-bundle`, or Update data, reads the
  community runs:
  - It rebuilds each deck at each act's end: final deck cards added by
    that floor, plus cards removed or transformed later.
  - It tags them with the same functions the app uses, and stores the
    percentiles in the catalog as community data, like encounter stats.
  - When a character × act has fewer than 30 decks, it falls back to all
    characters, and says so.
- **The act targets** in the playbook then become a fallback, for when
  there's no community data. The playbook still holds beliefs, such as how
  far below the benchmark counts as "short".

### 4. Setup cards: Aim, Stretch, Vigor

- **Simulator scoring.** Value setup by what can consume it:
  - Aim is worth `Aim × multiplier` of the best Shoot card in hand, or the
    chance of drawing one this turn. It's worth 0 with no Arrow to shoot.
  - Stretch works the same way against Block cards, and Vigor against
    Attacks.
  - The play order already spends them. Only the score that decides when
    to play the setup card changes.
- **The deck view.**
  - The Setup chip names its consumer ("Aim → Arrow").
  - The composition flags setup without consumers: "3 Aim cards, 2 Shoot
    cards: Aim will often be wasted".
  - Recurring setup also counts as Scaling: Archery (4 Aim each turn) and
    Hidden Roots (3 Stretch each turn).
- **Checking it.** A simulator test: the Fool's starter deck plus Nock,
  with and without extra Shoot cards. Nock's value should follow the Shoot
  count.

## Milestones

Each one ships as its own PR.

| | What | Done when |
|---|---|---|
| **T1** | Tag model in `spire-core`, derived from card data and the mechanics reader; playbook fallback rules and overrides; `audit --tags` lists untagged and doubtful cards | Every class card is tagged or listed; tests on known cards (Yoga = Setup Block, Nock = Setup Aim, Bash = Damage + Vulnerable) |
| **T2** | UI: tags replace roles on cards, the deck panel, filters and personal overrides (`my_roles.json` migrates) | The Cards and Run tabs show tags; old overrides carry over |
| **T3** | Problems for each encounter, shown with your deck's count of the tags that solve them | Phrog Parasite reads Groups + Junk; Waterfall Giant reads Race |
| **T4** | Benchmark from community runs at refresh: decks rebuilt per act, tagged, percentiles stored | The bundle carries percentiles per character × act, with a sample size |
| **T5** | Composition panel: share per tag, the benchmark band, shortfalls first | "Short of X" lines match the band |
| **T6** | Setup value in the simulator, and setup-without-consumers in the deck view | The Fool's Nock test passes; the warning shows on a deck with few Shoot cards |

## Decisions for you

- **D1 Tag set.** The ten above, or fewer or more?
  - **Recommended:** the ten. Each answers one question the enemy table
    asks.
- **D2 Roles.** Replace the five roles with tags, or keep roles as a
  summary on top?
  - **Recommended:** replace. Two systems is what we're trying to leave.
- **D3 "Succeeded".** Which decks count as the benchmark?
  - **Cleared the act's boss:** about 580 decks after Act 1.
  - **Won the run:** 178 decks per refresh, spread over 6 characters and
    many ascensions.
  - **Recommended:** cleared the act's boss, by character, with an
    all-characters fallback.
- **D4 Starter cards.** Count Strikes and Defends in the shares?
  - **Recommended:** count them, since they are your actual draws, but
    show starter and added cards in different shades.
- **D5 Aim and Stretch between turns.** The data's text doesn't say
  whether unspent Aim or Stretch carries into the next turn. Can you check
  in game? Gain Aim, end the turn without shooting, and see whether it's
  still there next turn. Same for Stretch. Until we know, the simulator
  treats them as lasting until spent, as it does now.
- **D6 "Etc."** Besides Aim, Stretch and Vigor, are there effects you think
  are undervalued, such as Fool's Arrows (slots and returns), or Stars?
