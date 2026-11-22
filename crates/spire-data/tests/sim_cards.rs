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
    // Evoking trades the orb's passive for its evoke twice: a modest gain
    // over five turns, where it used to add nothing.
    assert!(total(&with, true) > total(&without, true) + 2.0, "{} vs {}", total(&with, true), total(&without, true));
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
