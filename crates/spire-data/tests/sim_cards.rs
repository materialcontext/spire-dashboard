//! What the simulator reads from real cards and relics (the bundled
//! catalog), one case per misread found in `docs/research/CARDS.md`.

use spire_core::Catalog;
use spire_core::analysis::sim::{Effects, Reading, reading};
use spire_core::ids::CardId;
use spire_core::mechanics::{Effect, When, read};

fn cat() -> &'static Catalog {
    static CAT: std::sync::OnceLock<Catalog> = std::sync::OnceLock::new();
    CAT.get_or_init(spire_data::bundled_catalog)
}

fn of(id: &str) -> Reading {
    of_at(id, false)
}

fn of_at(id: &str, upgraded: bool) -> Reading {
    let card = cat().cards.get(&CardId::new(id)).unwrap_or_else(|| panic!("{id} in the catalog"));
    reading(card, upgraded, cat())
}

fn now(id: &str) -> Effects {
    of(id).now
}

fn power(id: &str) -> Vec<(When, Vec<Effect>)> {
    of(id).mech.triggers
}

#[test]
fn powers_fire_per_event_not_on_play() {
    // Each of these fires per event; read as "when played" they credited a
    // whole fight's worth to one play.
    for id in ["RUPTURE", "INFERNO", "VICIOUS", "ENVENOM", "SHROUD", "SPIRIT_OF_ASH", "PAGESTORM", "CALTROPS"] {
        let m = of(id).mech;
        assert!(m.triggers.iter().all(|(w, _)| *w != When::Played), "{id}: {:?}", m.triggers);
    }
    use spire_core::mechanics::{CardFilter, Counted, Debuff};
    let has = |id: &str, w: When| power(id).iter().any(|(x, _)| *x == w);
    assert!(has("RUPTURE", When::OnHpLoss));
    assert!(has("INFERNO", When::OnHpLoss));
    assert!(has("INFERNO", When::TurnStart), "loses 1 HP each turn");
    assert!(has("VICIOUS", When::OnApply(Debuff::Vulnerable)));
    assert!(has("SHROUD", When::OnApply(Debuff::Doom)));
    assert!(has("ENVENOM", When::OnAttackHit));
    assert!(has("SPIRIT_OF_ASH", When::OnPlay(CardFilter::Keyword("ethereal".into()))));
    assert!(has("PAGESTORM", When::OnDrawOf(CardFilter::Keyword("ethereal".into()))));
    assert!(has("SMOKESTACK", When::OnMake(CardFilter::Type(spire_core::catalog::CardType::Status))));
    assert!(has("ARSENAL", When::OnMake(CardFilter::Any)));
    assert!(has("DEVOUR_LIFE", When::OnPlay(CardFilter::Named("soul".into()))));
    assert!(has("HAUNT", When::OnPlay(CardFilter::Named("soul".into()))));
    assert!(has("THUNDER", When::OnEvoke(Some(spire_core::ids::OrbId::new("LIGHTNING_ORB")))));
    assert!(has("ORBIT", When::Every { what: Counted::EnergySpent, n: 4 }));
    assert!(has("PARRY", When::OnPlay(CardFilter::Named("sovereign blade".into()))));
    assert!(has("BLACK_HOLE", When::OnStars { gain: true, spend: true, each: false }));
    assert!(has("CHILD_OF_THE_STARS", When::OnStars { gain: false, spend: true, each: true }));
    // Plain on-play powers still read.
    assert_eq!(power("INFLAME"), vec![(When::Played, vec![Effect::Strength(2)])]);
}

#[test]
fn osty_attacks_are_osty_damage_only() {
    for (id, osty) in [("POKE", 6), ("FLATTEN", 12), ("UNLEASH", 6), ("BONE_SHARDS", 9)] {
        let e = now(id);
        assert_eq!((e.damage, e.osty), (0, osty), "{id}");
    }
    assert!(of("BONE_SHARDS").all_enemies);
    assert_eq!(now("STRIKE_NECROBINDER").osty, 0);
}

#[test]
fn energy_only_from_gaining_it() {
    for id in ["BANSHEES_CRY", "EXPECT_A_FIGHT", "HEAVENLY_DRILL", "MELANCHOLY", "RIGHT_HAND_HAND", "TRANSFIGURE"] {
        assert_eq!(now(id).energy, 0, "{id} names energy as a cost or threshold");
    }
    assert_eq!(now("SUNDER").energy, 0, "only if it kills");
    assert_eq!(now("BLOODLETTING").energy, 2);
    assert_eq!(now("ADRENALINE").energy, 1);
}

#[test]
fn debuffs_under_a_shared_number_are_applied() {
    for (id, weak, vulnerable) in [("UPPERCUT", 1, 1), ("PUTREFY", 2, 2), ("SHOCKWAVE", 3, 3), ("EXPOSE", 0, 2)] {
        let e = now(id);
        assert_eq!((e.weak, e.vulnerable), (weak, vulnerable), "{id}");
    }
    assert_eq!(now("BOUNCING_FLASK").poison, 9, "3 Poison, 3 times");
}

#[test]
fn stars_and_x_stars() {
    assert_eq!(now("ROYAL_GAMBLE").stars, 9);
    assert_eq!(now("HIDDEN_CACHE").stars, 1, "the 3 next turn wait");
    assert!(of("STARDUST").x_stars);
}

#[test]
fn powers_channel_through_their_triggers_only() {
    for id in ["STORM", "SPINNER", "CONSUMING_SHADOW"] {
        assert!(of(id).channels.is_empty(), "{id}");
    }
    // Lightning Rod's Lightning comes on the next two turns, not now.
    let rod = of("LIGHTNING_ROD");
    assert!(rod.channels.is_empty());
    assert!(matches!(rod.text.later.as_slice(), [(2, es)] if matches!(es.as_slice(), [Effect::Channel(_, 1)])));
}

#[test]
fn next_turn_effects_wait() {
    for (id, n) in [("PREDATOR", 2), ("GUIDING_STAR", 2), ("THE_STAR", 3), ("RELAX", 2)] {
        assert_eq!(now(id).draw, 0, "{id} draws next turn");
        let later = &of(id).text.later;
        assert!(later.iter().any(|(t, es)| *t == 1 && es.contains(&Effect::Draw(n))), "{id}: {later:?}");
    }
    // Glow draws now and next turn.
    assert_eq!(now("GLOW").draw, 1);
    assert_eq!(of("TORIC_TOUGHNESS").text.later, vec![(2, vec![Effect::Block(5)])]);
    assert_eq!(of("DODGE_AND_ROLL").text.later, vec![(1, vec![Effect::Block(4)])]);
    assert_eq!(now("INVOKE").summon, 0);
    assert_eq!(now("INVOKE").energy_next, 2);
    assert!(of("GOD_BYRD").text.later.iter().any(|(_, es)| es.contains(&Effect::DamageAll(30))));
}

#[test]
fn this_turn_stats_and_self_buffs_on_attacks() {
    assert_eq!(now("ANTICIPATE").temp, [0, 2, 0]);
    assert_eq!(now("ANTICIPATE").dexterity, 0);
    assert_eq!(now("HOTFIX").temp, [0, 0, 2]);
    assert_eq!(now("FEEDING_FRENZY").temp, [5, 0, 0]);
    assert_eq!(now("SETUP_STRIKE").temp, [3, 0, 0]);
    assert_eq!(now("FOCUSED_STRIKE").temp, [0, 0, 1]);
    assert_eq!(now("HYPERBEAM").temp, [0, 0, -3]);
    // Your Strength, not the enemy's.
    assert_eq!(now("FIGHT_ME").strength, 3);
    assert_eq!(now("RESONANCE").strength, 1);
    assert_eq!(now("MANGLE").strength, 0);
    assert_eq!(now("BRAND").strength, 1);
}

#[test]
fn block_twice_and_cards_for_other_players() {
    assert_eq!(now("CHO_LIMBS").block_times, 2);
    for id in ["LIFT", "BELIEVE_IN_YOU", "CONSTELLATION"] {
        assert_eq!(now(id), Effects::default(), "{id} helps another player");
    }
}

#[test]
fn shooting_and_playable_statuses() {
    assert!(!of("WHISTLING_ARROWS").shoots);
    assert!(of("QUICK_SHOT").shoots);
    assert_eq!(of("SLIMED").cost, Some(1), "played to clear it");
    assert_eq!(of("WOUND").cost, None);
}

#[test]
fn relic_text_fixes() {
    let orbs: Vec<_> = cat().orbs.values().cloned().collect();
    let relic = |id: &str| read(&cat().relics[&spire_core::ids::RelicId::new(id)].text, &orbs, false);
    let bread = relic("BREAD");
    assert!(bread.triggers.iter().any(|(w, es)| *w == When::TurnsFrom(2) && es == &vec![Effect::Energy(1)]));
    let lamp = relic("BIZARRE_LAMP");
    let made: Vec<&str> = lamp
        .triggers
        .iter()
        .flat_map(|(_, es)| es)
        .filter_map(|e| if let Effect::Make { name, .. } = e { Some(name.as_str()) } else { None })
        .collect();
    assert_eq!(made, vec!["joy", "sadness"]);
    assert!(relic("RED_MASK").triggers.iter().any(|(_, es)| es.contains(&Effect::Weak(1))));
    assert!(relic("BLESSED_ANTLER").triggers.iter().any(|(w, es)| *w == When::CombatStart
        && es.iter().any(|e| matches!(e, Effect::Make { name, n: 3, .. } if name == "dazed"))));
}

#[test]
fn tags_read_what_the_card_does_for_you() {
    let pb = spire_data::bundled_playbook();
    let tags = |id: &str| -> Vec<String> {
        let card = &cat().cards[&CardId::new(id)];
        spire_core::analysis::tags::read(card, false, &pb).iter().map(|t| t.label()).collect()
    };
    let has = |id: &str, tag: &str| tags(id).iter().any(|t| t.starts_with(tag));
    // Your own Strength loss isn't Weak; the enemy's is.
    for id in ["BRAND", "BULK_UP", "FRIENDSHIP", "RUPTURE"] {
        assert!(!has(id, "Weak"), "{id}: {:?}", tags(id));
    }
    for id in ["MANGLE", "PIERCING_WAIL", "DARK_SHACKLES", "MONARCHS_GAZE"] {
        assert!(has(id, "Weak"), "{id}: {:?}", tags(id));
    }
    for id in ["BANSHEES_CRY", "DANSE_MACABRE", "EXPECT_A_FIGHT", "MELANCHOLY", "TRANSFIGURE", "BELIEVE_IN_YOU"] {
        assert!(!has(id, "Energy"), "{id}: {:?}", tags(id));
    }
    assert!(has("BLOODLETTING", "Energy"));
    for id in ["AMNESIA", "VOID", "SLIMED", "DOUBT"] {
        assert!(tags(id).is_empty(), "{id}: {:?}", tags(id));
    }
}

// ---------------------------------------------------------------- engines in the fight

fn fight(character: &str, deck: &[(&str, usize)], relics: &[&str]) -> spire_core::analysis::sim::DeckSim {
    use spire_core::ids::{CharacterId, RelicId};
    use spire_core::run::{DeckCard, RunEvent, RunState, apply};
    let start = RunEvent::start(cat(), &CharacterId::new(character), 0).expect("character starts");
    let mut s: RunState = apply(RunState::default(), &start);
    s.deck = deck.iter().flat_map(|(id, n)| std::iter::repeat_n(DeckCard::new(CardId::new(*id)), *n)).collect();
    s.relics = relics.iter().map(|r| RelicId::new(*r)).collect();
    let mut rules = spire_data::bundled_playbook().simulation;
    rules.trials = 200;
    spire_core::analysis::sim::simulate(&s, cat(), &rules)
}

fn total(sim: &spire_core::analysis::sim::DeckSim, damage: bool) -> f32 {
    sim.turns.iter().map(|t| if damage { t.damage.mean } else { t.block.mean }).sum()
}

#[test]
fn dualcast_evokes_on_demand() {
    let with = fight("DEFECT", &[("ZAP", 5), ("DUALCAST", 5)], &[]);
    // Against dead cards, so the difference is what Dualcast itself adds.
    let without = fight("DEFECT", &[("ZAP", 5), ("WOUND", 5)], &[]);
    // Evoking trades the orb's later passives for its evoke twice: damage
    // comes sooner (and the passives it gives up are missed later).
    let early = |s: &spire_core::analysis::sim::DeckSim| s.turns[..2].iter().map(|t| t.damage.mean).sum::<f32>();
    assert!(early(&with) > early(&without) + 5.0, "{} vs {}", early(&with), early(&without));
}

#[test]
fn self_damage_feeds_rupture() {
    let base = [("STRIKE_IRONCLAD", 6), ("BLOODLETTING", 3)];
    let with = fight("IRONCLAD", &[base[0], base[1], ("RUPTURE", 1)], &[]);
    let without = fight("IRONCLAD", &[base[0], base[1], ("DEFEND_IRONCLAD", 1)], &[]);
    let late = |s: &spire_core::analysis::sim::DeckSim| s.turns[4].damage.mean;
    assert!(late(&with) > late(&without) + 2.0, "{} vs {}", late(&with), late(&without));
}

#[test]
fn this_turn_triggers_fire_and_end() {
    // Rage: Block per Attack played this turn.
    let rage = fight("IRONCLAD", &[("RAGE", 3), ("ANGER", 7)], &[]);
    assert!(total(&rage, false) > 10.0, "{}", total(&rage, false));
}

#[test]
fn soul_and_star_engines() {
    // Devour Life: Summon per Soul played; Grave Warden adds Souls.
    let devour = fight("NECROBINDER", &[("DEVOUR_LIFE", 1), ("GRAVE_WARDEN", 9)], &[]);
    let summon = |s: &spire_core::analysis::sim::DeckSim| {
        s.resources.iter().find(|(r, _)| *r == spire_core::analysis::sim::Resource::Summon).map_or(0.0, |r| r.1)
    };
    let plain = fight("NECROBINDER", &[("DEFEND_NECROBINDER", 1), ("GRAVE_WARDEN", 9)], &[]);
    assert!(summon(&devour) > summon(&plain), "{} vs {}", summon(&devour), summon(&plain));
    // Black Hole: damage per Star gained or spent.
    let hole = fight("REGENT", &[("BLACK_HOLE", 1), ("VENERATE", 5), ("FALLING_STAR", 4)], &[]);
    let none = fight("REGENT", &[("DEFEND_REGENT", 1), ("VENERATE", 5), ("FALLING_STAR", 4)], &[]);
    assert!(total(&hole, true) > total(&none, true), "{} vs {}", total(&hole, true), total(&none, true));
}

#[test]
fn enchantments_change_the_play() {
    use spire_core::ids::{CharacterId, EnchantmentId};
    use spire_core::run::{DeckCard, RunEvent, RunState, apply};
    let run = |enchant: Option<(&str, u32)>| {
        let start = RunEvent::start(cat(), &CharacterId::new("IRONCLAD"), 0).expect("starts");
        let mut s: RunState = apply(RunState::default(), &start);
        s.deck = (0..10)
            .map(|_| DeckCard {
                enchantment: enchant.map(|(e, _)| EnchantmentId::new(e)),
                enchant_amount: enchant.map(|(_, n)| n),
                ..DeckCard::new(CardId::new("STRIKE_IRONCLAD"))
            })
            .collect();
        s.relics.clear();
        let mut rules = spire_data::bundled_playbook().simulation;
        rules.trials = 100;
        spire_core::analysis::sim::simulate(&s, cat(), &rules).turns[1].damage.mean
    };
    let plain = run(None);
    assert_eq!(plain, 18.0, "three Strikes a turn");
    assert_eq!(run(Some(("SHARP", 3))), 27.0);
    // Tezcatara's Ember: free and +3, so all five in hand are played.
    assert_eq!(run(Some(("TEZCATARAS_EMBER", 1))), 45.0);
    assert!(run(Some(("SPIRAL", 1))) > 30.0, "each Strike plays twice");
}

#[test]
fn conditions_and_counts_are_read() {
    use spire_core::mechanics::{Bonus, Cond, Count, Scaled};
    let t = |id: &str| of(id).text;
    assert_eq!(t("GRAND_FINALE").play_if, Some(Cond::DrawPileEmpty));
    assert_eq!(t("CLASH").play_if, Some(Cond::HandAllAttacks));
    assert_eq!(t("NAIL").works_if, Some(Cond::EnemyImpaled));
    assert_eq!(t("BUBBLE_BUBBLE").works_if, Some(Cond::EnemyPoisoned));
    assert!(t("WREST").dislodges && t("WREST").works_if == Some(Cond::EnemyImpaled));
    assert_eq!(t("BOW_SMACK").bonus_if, vec![(Cond::EnemyImpaled, Bonus::Energy)]);
    assert_eq!(t("DISMANTLE").bonus_if, vec![(Cond::EnemyVulnerable, Bonus::Hits)]);
    assert_eq!(t("SPITE").bonus_if, vec![(Cond::LostHpThisTurn, Bonus::Hits)]);
    assert_eq!(t("CHO_FIRM").bonus_if, vec![(Cond::HaveArrow, Bonus::BlockTimes(1))]);
    assert_eq!(t("DEATHS_DOOR").bonus_if, vec![(Cond::AppliedDoomThisTurn, Bonus::BlockTimes(2))]);
    assert_eq!(t("WAH_HAMMER").bonus_if, vec![(Cond::EnemyImpaled, Bonus::DoubleDamage)]);
    let scale = |id: &str| t(id).scale.into_iter().map(|s| (s.count, s.per, s.into)).collect::<Vec<_>>();
    assert_eq!(scale("PERFECTED_STRIKE"), vec![(Count::DeckNamed(vec!["strike".into()]), 2, Scaled::DamageAdd)]);
    assert_eq!(scale("FINISHER"), vec![(Count::AttacksThisTurn, 6, Scaled::DamagePer)]);
    assert_eq!(scale("BODY_SLAM"), vec![(Count::Block, 1, Scaled::DamagePer)]);
    assert_eq!(scale("TIMES_UP"), vec![(Count::EnemyDoom, 1, Scaled::DamagePer)]);
    assert_eq!(scale("PRECISE_CUT"), vec![(Count::OtherCardsInHand, -2, Scaled::DamageAdd)]);
    assert_eq!(scale("TEAR_ASUNDER"), vec![(Count::HpLosses, 1, Scaled::Hits)]);
    assert_eq!(scale("STACK"), vec![(Count::DiscardPile, 1, Scaled::Block)]);
    assert_eq!(scale("COMPILE_DRIVER"), vec![(Count::UniqueOrbs, 1, Scaled::Draw)]);
    assert_eq!(
        scale("WAH_SYMPHONY"),
        vec![(Count::DeckNamed(vec!["wah".into(), "cho".into(), "ki".into()]), 4, Scaled::DamageAdd)]
    );
    assert_eq!(scale("ANGUISH"), vec![(Count::Made(Some("sadness".into())), 6, Scaled::DamageAdd)]);
    assert_eq!(now("FINISHER").damage, 0, "per Attack, not a base");
}

#[test]
fn conditions_and_counts_play_out() {
    // Perfected Strike counts every Strike in the deck: with 6 more it hits far harder.
    let strikes = fight("IRONCLAD", &[("PERFECTED_STRIKE", 4), ("STRIKE_IRONCLAD", 6)], &[]);
    let plain = fight("IRONCLAD", &[("PERFECTED_STRIKE", 4), ("DEFEND_IRONCLAD", 6)], &[]);
    assert!(total(&strikes, true) > total(&plain, true) + 20.0);
    // Body Slam hits for the Block gained first.
    let slam = fight("IRONCLAD", &[("BODY_SLAM", 4), ("SHRUG_IT_OFF", 6)], &[]);
    assert!(total(&slam, true) > 20.0, "{}", total(&slam, true));
    // Grand Finale waits for an empty Draw Pile, so it isn't played on turn 1 of a full deck.
    let finale = fight("SILENT", &[("GRAND_FINALE", 1), ("DEFEND_SILENT", 14)], &[]);
    assert!(finale.turns[0].damage.mean < 30.0, "{}", finale.turns[0].damage.mean);
    // Dismantle hits twice only on a Vulnerable enemy.
    let alone = fight("IRONCLAD", &[("DISMANTLE", 10)], &[]);
    let with_bash = fight("IRONCLAD", &[("DISMANTLE", 7), ("TREMBLE", 3)], &[]);
    assert!(alone.turns[0].damage.mean <= 24.0 + 0.1, "one hit each: {}", alone.turns[0].damage.mean);
    assert!(with_bash.turns[2].damage.mean > alone.turns[2].damage.mean);
}

#[test]
fn cost_growth_and_turn_rules_are_read() {
    use spire_core::catalog::CardType;
    use spire_core::mechanics::{CardFilter, CardMod, Cond, Count};
    let mods = |id: &str| of(id).text.mods;
    assert_eq!(mods("STOMP"), vec![CardMod::CostPer(Count::AttacksThisTurn, 1)]);
    assert_eq!(mods("PINPOINT"), vec![CardMod::CostPer(Count::SkillsThisTurn, 1)]);
    assert_eq!(mods("BANSHEES_CRY"), vec![CardMod::CostPer(Count::EtherealPlayed, 2)]);
    assert_eq!(mods("UNRELENTING"), vec![CardMod::NextFree(CardFilter::Type(CardType::Attack))]);
    assert_eq!(mods("VEILPIERCER"), vec![CardMod::NextFree(CardFilter::Keyword("ethereal".into()))]);
    assert_eq!(mods("EXPECT_A_FIGHT"), vec![CardMod::HandCheaper(CardFilter::Type(CardType::Attack), 1)]);
    assert_eq!(mods("BULLET_TIME"), vec![CardMod::DrawLock, CardMod::HandFree]);
    assert_eq!(mods("ENLIGHTENMENT"), vec![CardMod::HandCost(1)]);
    assert_eq!(mods("BORROWED_TIME"), vec![CardMod::CostUp(1)]);
    assert_eq!(mods("MOMENTUM_STRIKE"), vec![CardMod::SelfCostZero]);
    assert_eq!(mods("UP_MY_SLEEVE"), vec![CardMod::SelfCost(-1)]);
    assert_eq!(mods("MODDED"), vec![CardMod::SelfCost(1)]);
    assert_eq!(mods("KINGLY_KICK"), vec![CardMod::CostOnDraw(-1)]);
    assert_eq!(mods("KINGLY_PUNCH"), vec![CardMod::GrowOnDraw(4)]);
    assert_eq!(mods("RAMPAGE"), vec![CardMod::Grow(5)]);
    assert_eq!(mods("CLAW"), vec![CardMod::GrowNamed("claw".into(), 2)]);
    assert_eq!(mods("FLATTEN"), vec![CardMod::FreeIf(Cond::OstyAttackedThisTurn)]);
    assert_eq!(mods("EXPERTISE"), vec![CardMod::RetainHand]);
    assert_eq!(mods("BATTLE_TRANCE"), vec![CardMod::DrawLock]);
    assert_eq!(mods("ENCORE"), vec![CardMod::PlayLimit(1)]);
    assert_eq!(mods("DOUBLE_ENERGY"), vec![CardMod::DoubleEnergy]);
    assert_eq!(mods("ENTRENCH"), vec![CardMod::DoubleBlock]);
    assert_eq!(mods("SHADOWMELD"), vec![CardMod::DoubleBlockGains]);
    assert_eq!(mods("PANIC_BUTTON"), vec![CardMod::BlockLock(2)]);
    assert_eq!(mods("SHADOW_STEP"), vec![CardMod::AttacksDoubleNext]);
    assert_eq!(mods("HANG"), vec![CardMod::NamedDouble("hang".into())]);
    assert_eq!(mods("CONQUEROR"), vec![CardMod::NamedDoubleTurn("sovereign blade".into())]);
    assert_eq!(of("SIGNAL_BOOST").text.next_extra, Some(CardType::Power));
}

#[test]
fn growth_and_costs_play_out() {
    // Claw grows every Claw; Rampage grows itself.
    let claws = fight("DEFECT", &[("CLAW", 10)], &[]);
    assert!(claws.turns[4].damage.mean > claws.turns[0].damage.mean + 10.0);
    // Stomp gets cheaper with each Attack played first.
    let stomp = fight("IRONCLAD", &[("STOMP", 3), ("ANGER", 7)], &[]);
    let plain = fight("IRONCLAD", &[("STOMP", 3), ("WOUND", 7)], &[]);
    assert!(total(&stomp, true) > total(&plain, true) + 20.0);
}

#[test]
fn osty_melody_sly_and_exhaust_choices() {
    use spire_core::mechanics::{Cond, Count, Effect, Pile, Scaled};
    // Readings.
    assert!(of("UNLEASH").text.scale.iter().any(|s| s.count == Count::OstyHp && s.into == Scaled::DamageAdd));
    assert_eq!(of("SACRIFICE").text.works_if, Some(Cond::OstyAlive));
    assert!(of("BONE_SHARDS").text.actions.contains(&Effect::OstyDies));
    assert!(of("HARMONIC_SHIELD").text.scale.iter().any(|s| s.count == Count::Melodies && s.per == 5));
    assert_eq!(of("WAH_BLAST").text.bonus_if.first().map(|b| b.0), Some(Cond::HaveMelody(false, true, true)));
    assert_eq!(of("SURVIVOR").text.actions, vec![Effect::Discard(1)]);
    assert_eq!(of("BURNING_PACT").text.actions, vec![Effect::Exhaust { n: 1, random: false, from: Pile::Hand }]);
    assert_eq!(of("CINDER").text.actions, vec![Effect::Exhaust { n: 1, random: true, from: Pile::Hand }]);
    assert!(of("CALCULATED_GAMBLE").text.draw_per_discard);

    // Unleash grows with Osty's HP: Bodyguard's Summons raise it.
    let summoned = fight("NECROBINDER", &[("UNLEASH", 5), ("BODYGUARD", 5)], &[]);
    let bare = fight("NECROBINDER", &[("UNLEASH", 5), ("WOUND", 5)], &[]);
    assert!(total(&summoned, true) > total(&bare, true) + 10.0, "{} vs {}", total(&summoned, true), total(&bare, true));
    // Violent Chord: 9 to all per Wah Melody gained.
    let chord = fight("FOOL", &[("VIOLENT_CHORD", 1), ("WAH_SWEEP", 9)], &[]);
    let plain = fight("FOOL", &[("WOUND", 1), ("WAH_SWEEP", 9)], &[]);
    assert!(total(&chord, true) > total(&plain, true) + 20.0, "{} vs {}", total(&chord, true), total(&plain, true));
    // Sly: Tactician discarded by Survivor plays for free (+1 energy).
    let sly = fight("SILENT", &[("SURVIVOR", 3), ("TACTICIAN", 3), ("LEG_SWEEP", 4)], &[]);
    let dull = fight("SILENT", &[("SURVIVOR", 3), ("WOUND", 3), ("LEG_SWEEP", 4)], &[]);
    // The free Tactician's energy plays more Survivors.
    assert!(total(&sly, false) > total(&dull, false) + 3.0, "{} vs {}", total(&sly, false), total(&dull, false));
    // Feel No Pain pays off Burning Pact's chosen exhaust (of the Wounds).
    let pact = fight("IRONCLAD", &[("FEEL_NO_PAIN", 1), ("BURNING_PACT", 4), ("WOUND", 5)], &[]);
    let none = fight("IRONCLAD", &[("WOUND", 1), ("BURNING_PACT", 4), ("WOUND", 5)], &[]);
    assert!(total(&pact, false) > total(&none, false) + 5.0, "{} vs {}", total(&pact, false), total(&none, false));
}

#[test]
fn piles_autoplay_and_random_cards_are_read() {
    use spire_core::catalog::CardType;
    use spire_core::mechanics::{CardFilter, Effect, Pile, Pool, Source};
    let a = |id: &str| of(id).text.actions;
    assert_eq!(
        a("HOLOGRAM"),
        vec![Effect::Fetch { from: Pile::Discard, to: Pile::Hand, n: 1, filter: CardFilter::Any }]
    );
    assert_eq!(
        a("HEADBUTT"),
        vec![Effect::Fetch { from: Pile::Discard, to: Pile::Draw, n: 1, filter: CardFilter::Any }]
    );
    assert_eq!(
        a("SECRET_WEAPON"),
        vec![Effect::Fetch { from: Pile::Draw, to: Pile::Hand, n: 1, filter: CardFilter::Type(CardType::Attack) }]
    );
    assert_eq!(
        a("ALL_FOR_ONE"),
        vec![Effect::Fetch { from: Pile::Discard, to: Pile::Hand, n: -1, filter: CardFilter::ZeroCost }]
    );
    assert_eq!(
        a("HAVOC"),
        vec![Effect::AutoPlay { from: Source::DrawTop, n: 1, filter: CardFilter::Any, exhaust: true }]
    );
    assert_eq!(
        a("CASCADE"),
        vec![Effect::AutoPlay { from: Source::DrawTop, n: -1, filter: CardFilter::Any, exhaust: false }]
    );
    assert_eq!(
        a("BEAT_DOWN"),
        vec![Effect::AutoPlay {
            from: Source::Discard,
            n: 3,
            filter: CardFilter::Type(CardType::Attack),
            exhaust: false
        }]
    );
    assert_eq!(
        a("KNIFE_TRAP"),
        vec![Effect::AutoPlay {
            from: Source::Exhaust,
            n: -1,
            filter: CardFilter::Named("shiv".into()),
            exhaust: false
        }]
    );
    assert_eq!(
        a("INFERNAL_BLADE"),
        vec![Effect::MakeRandom { pool: Pool::Class(Some(CardType::Attack)), n: 1, pile: Pile::Hand, free: true }]
    );
    assert_eq!(
        a("BUNDLE_OF_JOY"),
        vec![Effect::MakeRandom { pool: Pool::Colorless, n: 3, pile: Pile::Hand, free: false }]
    );
    assert_eq!(of("TRICK_SHOT").text.returns, Some(Pile::Hand));
    assert_eq!(of("BOLAS").text.returns, Some(Pile::Draw));
    assert!(power("CREATIVE_AI").iter().any(|(_, es)| es.iter().any(|e| matches!(e, Effect::MakeRandom { .. }))));
    assert!(power("STAMPEDE").iter().any(|(_, es)| es.iter().any(|e| matches!(e, Effect::AutoPlay { .. }))));
}

#[test]
fn piles_autoplay_and_random_cards_play_out() {
    // Knife Trap replays every Shiv in the Exhaust Pile.
    let trap = fight("SILENT", &[("BLADE_DANCE", 6), ("KNIFE_TRAP", 4)], &[]);
    let none = fight("SILENT", &[("BLADE_DANCE", 6), ("WOUND", 4)], &[]);
    assert!(total(&trap, true) > total(&none, true) + 20.0, "{} vs {}", total(&trap, true), total(&none, true));
    // Havoc plays the top card for free.
    let havoc = fight("IRONCLAD", &[("HAVOC", 4), ("STRIKE_IRONCLAD", 3), ("WOUND", 3)], &[]);
    let dead = fight("IRONCLAD", &[("WOUND", 4), ("STRIKE_IRONCLAD", 3), ("WOUND", 3)], &[]);
    // On turn 1 it plays an extra card; later it has exhausted the Strikes
    // it played, which the fight remembers.
    let first = |s: &spire_core::analysis::sim::DeckSim| s.turns[0].damage.mean;
    assert!(first(&havoc) > first(&dead) + 1.0, "{} vs {}", first(&havoc), first(&dead));
    // Infernal Blade makes a free random Attack.
    let blade = fight("IRONCLAD", &[("INFERNAL_BLADE", 5), ("WOUND", 5)], &[]);
    assert!(total(&blade, true) > 20.0, "{}", total(&blade, true));
    // Thrumming Hatchet comes back every turn.
    let hatchet = fight("IRONCLAD", &[("THRUMMING_HATCHET", 1), ("WOUND", 9)], &[]);
    assert!(hatchet.turns[3].damage.mean > 8.0, "{}", hatchet.turns[3].damage.mean);
}

#[test]
fn survival_reads_weak_and_self_damage() {
    assert_eq!(cat().rules.weak_pct, Some(25), "Weak's percentage comes from the power data");
    // Neutralize keeps the enemy Weak: a share of its attacks is taken off.
    let weak = fight("SILENT", &[("NEUTRALIZE", 5), ("DEFEND_SILENT", 5)], &[]);
    assert!(weak.turns[2].mitigation > 0.05, "{}", weak.turns[2].mitigation);
    let none = fight("SILENT", &[("STRIKE_SILENT", 5), ("DEFEND_SILENT", 5)], &[]);
    assert_eq!(none.turns[2].mitigation, 0.0);
    // Bloodletting costs 3 HP each play.
    let blood = fight("IRONCLAD", &[("BLOODLETTING", 5), ("STRIKE_IRONCLAD", 5)], &[]);
    assert!(blood.turns[0].self_damage > 2.0, "{}", blood.turns[0].self_damage);
}

#[test]
fn conditions_and_hand_choices_are_not_actions() {
    // "If you play 5 or more cards", "the first time you play…": conditions.
    assert!(power("PALE_BLUE_DOT").is_empty());
    assert!(power("FERAL").is_empty());
    // "Choose a card in your Hand…" (copy, transform, replay) isn't a fetch.
    for id in ["DUAL_WIELD", "NIGHTMARE", "BEGONE", "DECISIONS_DECISIONS", "HEIRLOOM_HAMMER"] {
        assert!(of(id).text.actions.is_empty(), "{id}: {:?}", of(id).text.actions);
    }
    // Choosing from a pile still is.
    let fetch = |id: &str| matches!(power(id).first().map(|r| &r.1[..]), Some([Effect::Fetch { .. }]));
    assert!(fetch("STRATAGEM"));
    assert!(matches!(of("KI_CHANT").text.actions[..], [Effect::Fetch { .. }]));
    // Rainbow Ring needs one of each type in a turn: left unread.
    let ring = read(&cat().relics[&spire_core::ids::RelicId::new("RAINBOW_RING")].text, &[], false);
    assert!(ring.triggers.is_empty() && ring.unread.len() == 1);
}

#[test]
fn every_card_fetches_end_with_a_full_hand() {
    // All for One returns every 0-cost card from the discard pile; with a
    // full hand they go back there and must not be taken again.
    let sim = fight("DEFECT", &[("ALL_FOR_ONE", 4), ("CLAW", 12), ("GO_FOR_THE_EYES", 6), ("BEAM_CELL", 6)], &[]);
    assert!(sim.turns[3].damage.mean > 0.0);
}

#[test]
fn scaling_powers_are_played_for_what_they_build() {
    // Demon Form does nothing the turn it's played; its Strength each turn
    // is worth the turns left, so it's played early and the deck ramps.
    let starter = [("STRIKE_IRONCLAD", 5), ("DEFEND_IRONCLAD", 4), ("BASH", 1)];
    let demon = fight("IRONCLAD", &[starter[0], starter[1], starter[2], ("DEMON_FORM", 1)], &[]);
    let strike = fight("IRONCLAD", &[starter[0], starter[1], starter[2], ("STRIKE_IRONCLAD", 1)], &[]);
    let last = |s: &spire_core::analysis::sim::DeckSim| s.turns.last().unwrap().damage.mean;
    assert!(last(&demon) > last(&strike) * 1.5, "{} vs {}", last(&demon), last(&strike));
}
