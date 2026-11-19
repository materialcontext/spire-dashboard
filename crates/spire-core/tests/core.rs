//! Behavior tests for the core, on small hand-built fixtures.

use spire_core::analysis::deck::{Coverage, coverage, deck_profile};
use spire_core::analysis::events::{needs, watchlist};
use spire_core::analysis::threats::next_threats;
use spire_core::analysis::{Dashboard, Severity, sim, timeline};
use spire_core::catalog::*;
use spire_core::playbook::*;
use spire_core::run::*;
use spire_core::*;

fn card(id: &str, t: CardType, rarity: Rarity, text: &str) -> Card {
    Card {
        id: CardId::new(id),
        name: id.to_string(),
        color: "hero".into(),
        card_type: t,
        rarity,
        cost: Some(1),
        x_cost: false,
        text: text.into(),
        upgrade_text: None,
        keywords: vec![],
        tags: vec![],
        target: Target::SingleEnemy,
        damage: None,
        hits: None,
        block: None,
        draw: None,
        energy: None,
        powers: vec![],
        vars: Default::default(),
        upgrade: Default::default(),
        star_cost: None,
        channels: vec![],
    }
}

fn monster(id: &str, hp: u32, hit: u32, intent: &str) -> Monster {
    Monster {
        id: MonsterId::new(id),
        name: id.into(),
        hp_min: hp,
        hp_max: None,
        moves: vec![MonsterMove {
            name: "Hit".into(),
            intent: intent.into(),
            damage: Some(hit),
            hits: None,
            block: None,
            damage_asc: Some(hit + 5),
            ..Default::default()
        }],
        innate_powers: vec![],
        hp_min_asc: Some(hp + 10),
        hp_max_asc: None,
        pattern: None,
        death_blow: None,
        death_summons: None,
        powers: Vec::new(),
        phases: Vec::new(),
    }
}

fn encounter(id: &str, room: RoomType, weak: bool, monsters: &[&str]) -> Encounter {
    Encounter {
        id: EncounterId::new(id),
        name: id.into(),
        room,
        weak,
        act: Some(ActId::new("ACT1")),
        monsters: monsters.iter().map(|m| MonsterId::new(*m)).collect(),
        tags: vec![],
        lineups: vec![],
        spawns: vec![],
        stats: None,
    }
}

fn catalog() -> Catalog {
    let mut strike = card("STRIKE", CardType::Attack, Rarity::Basic, "Deal 6 damage.");
    strike.damage = Some(6);
    strike.tags = vec!["Strike".into()];
    let mut defend = card("DEFEND", CardType::Skill, Rarity::Basic, "Gain 5 Block.");
    defend.block = Some(5);
    defend.tags = vec!["Defend".into()];
    let mut cleave = card("CLEAVE", CardType::Attack, Rarity::Common, "Deal 8 damage to ALL enemies.");
    cleave.damage = Some(8);
    cleave.target = Target::AllEnemies;
    let mut inflame = card("INFLAME", CardType::Power, Rarity::Uncommon, "Gain 2 Strength.");
    inflame.powers = vec![PowerAmount { power: "Strength".into(), amount: 2 }];
    let curse = card("REGRET", CardType::Curse, Rarity::Curse, "Unplayable.");

    let pack = DataPack {
        format: 1,
        source: "test".into(),
        characters: vec![Character {
            id: CharacterId::new("HERO"),
            name: "Hero".into(),
            max_hp: 80,
            gold: 99,
            energy: 3,
            starting_deck: vec!["STRIKE".into(), "STRIKE".into(), "DEFEND".into()],
            starting_relics: vec!["BLOOD".into()],
            description: String::new(),
            orb_slots: None,
            arrow_slots: None,
        }],
        cards: vec![strike, defend, cleave, inflame, curse],
        monsters: vec![
            monster("SLIME", 20, 5, "Attack"),
            monster("CULTIST", 50, 6, "Buff"),
            monster("BRUTE", 120, 30, "Attack"),
            monster("BOSS", 300, 20, "Attack + Buff"),
        ],
        encounters: vec![
            encounter("SLIMES_WEAK", RoomType::Monster, true, &["SLIME"]),
            encounter("CULTIST_PAIR", RoomType::Monster, false, &["CULTIST", "SLIME"]),
            encounter("BRUTE_ELITE", RoomType::Elite, false, &["BRUTE"]),
            encounter("BIG_BOSS", RoomType::Boss, false, &["BOSS"]),
        ],
        events: vec![
            Event {
                id: EventId::new("SHRINE"),
                name: "Shrine".into(),
                kind: EventKind::Act,
                acts: vec![ActId::new("ACT1")],
                options: vec![EventOption { title: "Pray".into(), text: "Remove a card from your deck.".into() }],
            },
            Event {
                id: EventId::new("FOUNTAIN"),
                name: "Fountain".into(),
                kind: EventKind::Shared,
                acts: vec![],
                options: vec![EventOption { title: "Drink".into(), text: "Heal 10 HP.".into() }],
            },
        ],
        acts: vec![Act {
            id: ActId::new("ACT1"),
            name: "Act One".into(),
            index: 0,
            rooms: 15,
            bosses: vec![EncounterId::new("BIG_BOSS")],
            ancients: vec![],
            events: vec![EventId::new("SHRINE")],
            encounters: vec![],
            weak_fights: None,
            length: None,
            fixed_rooms: Vec::new(),
        }],
        ..Default::default()
    };
    Catalog::from_layers([pack])
}

fn playbook() -> Playbook {
    let json = r#"{
      "version": 1,
      "role_rules": [
        { "role": "fast_damage", "when": "has_damage" },
        { "role": "frontload_defense", "when": "has_block" },
        { "role": "scaling_damage", "when": { "applies_power": "Strength" } },
        { "role": "aoe", "when": { "all": ["has_damage", { "target": "all_enemies" }] } }
      ],
      "multi_enemy_names": ["pair"],
      "opportunity_rules": [
        { "opportunity": "card_removal", "any_text": ["remove"] },
        { "opportunity": "heal", "any_text": ["heal"] }
      ],
      "acts": [{
        "index": 0, "weak_fights": 1,
        "targets": [
          { "role": "fast_damage", "min": 2 }, { "role": "scaling_damage", "min": 1 }, { "role": "aoe", "min": 1 }
        ],
        "big_hit": 25,
        "long_fight_hp": { "monster": 100, "elite": 150, "boss": 250 }
      }],
      "needs": { "low_hp": 0.5, "starter_cards": 2, "low_upgrades": 0.25 }
    }"#;
    serde_json::from_str(json).expect("test playbook parses")
}

fn started(cat: &Catalog) -> Run {
    let mut run = Run::default();
    run.push(RunEvent::start(cat, &CharacterId::new("HERO"), 0).unwrap());
    run.push(RunEvent::EnterAct { act: ActId::new("ACT1"), index: 0, boss: Some(EncounterId::new("BIG_BOSS")) });
    run
}

#[test]
fn start_resolves_kit_from_catalog() {
    let cat = catalog();
    let s = started(&cat).state();
    assert_eq!(s.hp, 80);
    assert_eq!(s.gold, 99);
    assert_eq!(s.deck.len(), 3);
    assert_eq!(s.relics, vec![RelicId::new("BLOOD")]);
    assert_eq!(s.act_index, 0);
}

#[test]
fn node_reports_fold_into_state_and_undo() {
    let cat = catalog();
    let mut run = started(&cat);
    run.push(RunEvent::Node(NodeReport {
        kind: NodeKind::Monster,
        encounter: Some(EncounterId::new("SLIMES_WEAK")),
        event: None,
        changes: vec![
            Change::AddCard(DeckCard::new("CLEAVE".into())),
            Change::UpgradeCard(DeckCard::new("STRIKE".into())),
            Change::SetHp(70),
            Change::SetGold(120),
            Change::AddPotion("FIRE".into()),
        ],
    }));
    let s = run.state();
    assert_eq!(s.floor, 1);
    assert_eq!(s.hp, 70);
    assert_eq!(s.gold, 120);
    assert_eq!(s.deck.len(), 4);
    assert_eq!(s.deck.iter().filter(|c| c.upgraded).count(), 1);
    assert_eq!(s.hallway_fights_in_act, 1);
    assert_eq!(s.encounters_seen, vec![EncounterId::new("SLIMES_WEAK")]);

    run.undo();
    assert_eq!(run.state().floor, 0);
    assert_eq!(run.state().deck.len(), 3);
}

#[test]
fn removing_a_missing_card_is_ignored() {
    let cat = catalog();
    let mut run = started(&cat);
    run.push(RunEvent::Adjust(vec![Change::RemoveCard(DeckCard::new("NOPE".into()))]));
    assert_eq!(run.state().deck.len(), 3);
}

#[test]
fn undo_never_removes_start() {
    let cat = catalog();
    let mut run = Run::default();
    run.push(RunEvent::start(&cat, &CharacterId::new("HERO"), 0).unwrap());
    assert!(run.undo().is_none());
    assert!(run.state().started());
}

#[test]
fn profile_splits_starters_from_added_cards() {
    let cat = catalog();
    let pb = playbook();
    let deck = vec![
        DeckCard::new("STRIKE".into()),
        DeckCard::new("DEFEND".into()),
        DeckCard::new("CLEAVE".into()),
        DeckCard::new("INFLAME".into()),
        DeckCard::new("REGRET".into()),
    ];
    let p = deck_profile(&deck, &cat, &pb);
    assert_eq!(p.total, 5);
    assert_eq!(p.of_type(CardType::Attack), 2);
    assert_eq!(p.of_type(CardType::Curse), 1);
    assert_eq!(p.role(Role::FastDamage), 1, "Strike is a starter, Cleave is added");
    assert_eq!(p.starter_role(Role::FastDamage), 1);
    assert_eq!(p.role(Role::ScalingDamage), 1);
    assert_eq!(p.role(Role::Aoe), 1);
    // "frontload_defense" in the rules reads as Mitigation (old names load).
    assert_eq!(p.starter_role(Role::Mitigation), 1);
    assert_eq!(p.starter_basics, 2);
    let plan = pb.act(0);
    assert_eq!(coverage(&p, Role::FastDamage, plan), Coverage::Thin);
    assert_eq!(coverage(&p, Role::Aoe, plan), Coverage::Covered);
    assert_eq!(coverage(&p, Role::Survival, plan), Coverage::Missing);
}

#[test]
fn card_role_overrides_replace_rules() {
    let cat = catalog();
    let mut pb = playbook();
    pb.card_roles.insert(CardId::new("STRIKE"), vec![Role::Chip]);
    assert_eq!(pb.roles_of(&cat.cards[&CardId::new("STRIKE")]), vec![Role::Chip]);
}

#[test]
fn hallway_threats_follow_weak_then_normal_pool() {
    let cat = catalog();
    let pb = playbook();
    let mut run = started(&cat);
    let deck = deck_profile(&run.state().deck, &cat, &pb);

    let first = next_threats(&run.state(), NodeKind::Monster, &cat, &pb, &deck);
    assert_eq!(first.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["SLIMES_WEAK"]);

    run.push(RunEvent::Node(NodeReport {
        kind: NodeKind::Monster,
        encounter: Some(EncounterId::new("SLIMES_WEAK")),
        event: None,
        changes: vec![],
    }));
    let later = next_threats(&run.state(), NodeKind::Monster, &cat, &pb, &deck);
    assert_eq!(later.len(), 1);
    let pair = &later[0];
    assert_eq!(pair.name, "CULTIST_PAIR");
    let roles: Vec<Role> = pair.checks.iter().map(|c| c.demand.role).collect();
    assert!(roles.contains(&Role::Aoe), "two enemies want area damage");
    assert!(roles.contains(&Role::FastDamage), "a buffer wants a fast kill");
}

#[test]
fn elite_and_boss_threats_read_damage_and_hp() {
    let cat = catalog();
    let pb = playbook();
    let s = started(&cat).state();
    let deck = deck_profile(&s.deck, &cat, &pb);

    let elite = &next_threats(&s, NodeKind::Elite, &cat, &pb, &deck)[0];
    assert_eq!(elite.profile.peak_turn, 30);
    assert!(elite.checks.iter().any(|c| c.demand.role == Role::Survival), "a big hit wants Survival");

    let boss = &next_threats(&s, NodeKind::Boss, &cat, &pb, &deck)[0];
    assert_eq!(boss.name, "BIG_BOSS");
    assert!(boss.checks.iter().any(|c| c.demand.role == Role::ScalingDamage));
}

#[test]
fn watchlist_ranks_events_by_needs() {
    let cat = catalog();
    let pb = playbook();
    let mut run = started(&cat);
    run.push(RunEvent::Adjust(vec![Change::AddCard(DeckCard::new("REGRET".into())), Change::SetHp(20)]));
    let s = run.state();
    let deck = deck_profile(&s.deck, &cat, &pb);
    let wants = needs(&s, &deck, &pb);
    assert!(wants.iter().any(|n| n.opportunity == Opportunity::CardRemoval));
    assert!(wants.iter().any(|n| n.opportunity == Opportunity::Heal));

    let list = watchlist(&s, &cat, &pb, &wants);
    assert_eq!(list.len(), 2, "act event plus shared event");
    assert!(list.iter().all(|e| e.score > 0));
}

#[test]
fn dashboard_advice_flags_gaps_and_boss() {
    let cat = catalog();
    let pb = playbook();
    let s = started(&cat).state();
    let d = Dashboard::compute(&s, &cat, &pb);
    assert_eq!(d.bosses.len(), 1);
    assert!(d.advice.iter().any(|a| a.severity == Severity::Urgent));
    assert_eq!(d.threats.len(), 3);
}

#[test]
fn later_layers_override_earlier() {
    let base = catalog();
    let mut card = base.cards[&CardId::new("STRIKE")].clone();
    card.name = "Better Strike".into();
    let pack = DataPack { format: 1, source: "user".into(), cards: vec![card], ..Default::default() };
    let first = DataPack {
        format: 1,
        source: "base".into(),
        cards: base.cards.values().cloned().collect(),
        ..Default::default()
    };
    let merged = Catalog::from_layers([first, pack]);
    assert_eq!(merged.card_name(&CardId::new("STRIKE")), "Better Strike");
    assert_eq!(merged.cards.len(), base.cards.len());
}

#[test]
fn run_log_round_trips_through_json() {
    let cat = catalog();
    let run = started(&cat);
    let json = serde_json::to_string(&run).unwrap();
    let back: Run = serde_json::from_str(&json).unwrap();
    assert_eq!(back, run);
}

#[test]
fn known_lineups_count_every_monster() {
    use spire_core::analysis::threats::threat_profile;
    let cat = catalog();
    let pb = playbook();
    let mut enc = cat.encounters[&EncounterId::new("SLIMES_WEAK")].clone();
    let before = threat_profile(&enc, &cat, &pb, Default::default());
    assert_eq!(before.enemies, 1);

    enc.lineups = vec![
        Lineup { monsters: vec!["SLIME".into(), "SLIME".into(), "SLIME".into()], share: 0.6 },
        Lineup { monsters: vec!["SLIME".into(), "CULTIST".into()], share: 0.4 },
    ];
    enc.spawns = vec!["CULTIST".into()];
    let t = threat_profile(&enc, &cat, &pb, Default::default());
    assert_eq!(t.enemies, 3);
    assert_eq!((t.hp_min, t.hp_max), (60, 70), "3 slimes = 60, slime + cultist = 70");
    assert_eq!(t.peak_turn, 15);
    assert!(t.summons);
    assert_eq!(cat.lineup_label(&enc.lineups[0]), "3× SLIME");
}

#[test]
fn deck_rows_group_identical_cards() {
    use spire_core::analysis::deck::{classify, deck_rows};
    let cat = catalog();
    let pb = playbook();
    let mut up = DeckCard::new("STRIKE".into());
    up.upgraded = true;
    let deck = vec![DeckCard::new("STRIKE".into()), DeckCard::new("STRIKE".into()), up, DeckCard::new("DEFEND".into())];
    let rows = deck_rows(&classify(&deck, &cat, &pb));
    let view: Vec<(&str, u32)> = rows.iter().map(|r| (r.name.as_str(), r.count)).collect();
    assert_eq!(view, [("STRIKE", 2), ("STRIKE+", 1), ("DEFEND", 1)]);
}

#[test]
fn case_insensitive_contains_matches_without_allocating() {
    use spire_core::playbook::contains_ci;
    assert!(contains_ci("Gain 5 Block.", "block"));
    assert!(contains_ci("A Pair of Nibbits", "PAIR"));
    assert!(!contains_ci("Deal 6 damage.", "block"));
    assert!(contains_ci("anything", ""));
}

#[test]
fn ascension_rules_shape_the_start() {
    let mut cat = catalog();
    cat.rules.start_curse = Some(CardId::new("REGRET"));
    let start = |asc| {
        let mut run = Run::default();
        run.push(RunEvent::start(&cat, &CharacterId::new("HERO"), asc).unwrap());
        run.state()
    };
    let a0 = start(0);
    assert_eq!((a0.potion_slots, a0.deck.len()), (3, 3));
    let a4 = start(4);
    assert_eq!((a4.potion_slots, a4.deck.len()), (2, 3), "A4: one fewer potion slot");
    let a5 = start(5);
    assert_eq!(a5.deck.len(), 4, "A5: starts cursed");
    assert!(a5.deck.iter().any(|c| c.card == CardId::new("REGRET")));
    assert_eq!(start(20).ascension, 10, "StS2 ascension tops out at 10");
}

#[test]
fn high_ascension_uses_ascension_stats() {
    use spire_core::analysis::threats::{Scaling, threat_profile};
    let cat = catalog();
    let pb = playbook();
    let enc = &cat.encounters[&EncounterId::new("BRUTE_ELITE")];
    let base = threat_profile(enc, &cat, &pb, Scaling::default());
    let tough = threat_profile(enc, &cat, &pb, Scaling { tough: true, deadly: false });
    let deadly = threat_profile(enc, &cat, &pb, Scaling { tough: true, deadly: true });
    assert_eq!((base.hp_min, base.peak_turn), (120, 30));
    assert_eq!((tough.hp_min, tough.peak_turn), (130, 30), "A8: tougher enemies");
    assert_eq!(deadly.peak_turn, 35, "A9: deadlier enemies");
}

#[test]
fn double_boss_act_tracks_both_bosses() {
    let cat = catalog();
    let mut run = started(&cat);
    run.push(RunEvent::SetSecondBoss(EncounterId::new("BRUTE_ELITE")));
    let s = run.state();
    assert_eq!(s.bosses().count(), 2);
    assert!(cat.rules.double_boss(10, 2));
    assert!(!cat.rules.double_boss(9, 2));
    // Entering a new act clears both.
    run.push(RunEvent::EnterAct { act: ActId::new("ACT1"), index: 1, boss: None });
    assert_eq!(run.state().bosses().count(), 0);
}

fn with_eternal_curse(mut cat: Catalog) -> Catalog {
    let mut bane = card("BANE", CardType::Curse, Rarity::Curse, "Unplayable. Eternal.");
    bane.cost = None;
    bane.keywords = vec!["Unplayable".into(), "Eternal".into()];
    cat.cards.insert(bane.id.clone(), bane);
    cat
}

#[test]
fn eternal_curses_never_drive_removal_or_transform() {
    use spire_core::analysis::events::needs;
    let cat = with_eternal_curse(catalog());
    let mut pb = playbook();
    pb.needs.starter_cards = 99; // isolate curses from the starter-card rule
    let mut run = started(&cat);
    run.push(RunEvent::Adjust(vec![Change::AddCard(DeckCard::new("BANE".into()))]));
    let s = run.state();
    let deck = deck_profile(&s.deck, &cat, &pb);
    assert_eq!((deck.eternal, deck.removable_curses, deck.dead), (1, 0, 1));
    let wants = needs(&s, &deck, &pb);
    assert!(
        !wants.iter().any(|n| matches!(n.opportunity, Opportunity::CardRemoval | Opportunity::Transform)),
        "an eternal curse alone must not ask for removal: {wants:?}"
    );

    // A removable curse still does.
    run.push(RunEvent::Adjust(vec![Change::AddCard(DeckCard::new("REGRET".into()))]));
    let s = run.state();
    let deck = deck_profile(&s.deck, &cat, &pb);
    assert_eq!(deck.removable_curses, 1);
    let wants = needs(&s, &deck, &pb);
    let removal = wants.iter().find(|n| n.opportunity == Opportunity::CardRemoval).unwrap();
    assert!(removal.reason.starts_with("1 removable"), "{}", removal.reason);
}

#[test]
fn eternal_curses_get_no_general_advice() {
    let cat = with_eternal_curse(catalog());
    let pb = playbook();
    let mut run = started(&cat);
    run.push(RunEvent::Adjust(vec![Change::AddCard(DeckCard::new("BANE".into()))]));
    let d = Dashboard::compute(&run.state(), &cat, &pb);
    // No general "can't be removed" reminder, and Eternal curses don't count as removable.
    assert!(!d.advice.iter().any(|a| a.text.contains("can't be removed")));
    assert!(!d.advice.iter().any(|a| a.text.contains("removable curse")));
}

#[test]
fn a_card_takes_one_enchantment_ever() {
    let cat = catalog();
    let mut run = started(&cat);
    let strike = DeckCard::new("STRIKE".into());
    run.push(RunEvent::Adjust(vec![
        Change::Enchant { card: strike.clone(), enchantment: "SHARP".into() },
        Change::Enchant { card: strike.clone(), enchantment: "SHARP".into() },
    ]));
    let s = run.state();
    let enchanted: Vec<_> = s.deck.iter().filter(|c| c.enchantment.is_some()).collect();
    // Two Strikes: each enchant lands on a different, unenchanted Strike.
    assert_eq!(enchanted.len(), 2);
    run.push(RunEvent::Adjust(vec![Change::Enchant { card: strike, enchantment: "OTHER".into() }]));
    let s = run.state();
    assert!(
        !s.deck.iter().any(|c| c.enchantment == Some("OTHER".into())),
        "no unenchanted Strike left, so nothing changes"
    );
}

#[test]
fn enchantment_fit_follows_type_and_text_rules() {
    let cat = catalog();
    let ench = |types: Vec<CardType>, rule: Option<&str>| Enchantment {
        id: "E".into(),
        name: "E".into(),
        text: String::new(),
        card_types: types,
        applicable_to: rule.map(String::from),
        stackable: false,
    };
    let strike = &cat.cards[&CardId::new("STRIKE")];
    let defend = &cat.cards[&CardId::new("DEFEND")];
    let cleave = &cat.cards[&CardId::new("CLEAVE")];
    let regret = &cat.cards[&CardId::new("REGRET")];
    assert!(ench(vec![CardType::Attack], None).fits(cleave));
    assert!(!ench(vec![CardType::Attack], None).fits(defend));
    assert!(ench(vec![CardType::Attack, CardType::Skill], None).fits(defend));
    assert!(ench(vec![], Some("Defend cards")).fits(defend));
    assert!(!ench(vec![], Some("Defend cards")).fits(strike));
    assert!(ench(vec![], Some("Basic Strike or Defend cards")).fits(strike));
    assert!(!ench(vec![], Some("Basic Strike or Defend cards")).fits(cleave));
    assert!(ench(vec![], Some("cards that gain Block")).fits(defend));
    assert!(!ench(vec![], None).fits(regret), "curses can't be enchanted");
}

#[test]
fn fixed_rooms_are_predicted_from_the_act_floor() {
    use spire_core::analysis::floors::expected_room;
    let mut cat = catalog();
    {
        let act = cat.acts.get_mut(&ActId::new("ACT1")).unwrap();
        act.length = Some(6);
        act.fixed_rooms = vec![
            FixedRoom { at: 0, room: NodeKind::Ancient },
            FixedRoom { at: 1, room: NodeKind::Monster },
            FixedRoom { at: 3, room: NodeKind::Treasure },
            FixedRoom { at: -2, room: NodeKind::Rest },
            FixedRoom { at: -1, room: NodeKind::Boss },
        ];
    }
    let mut run = started(&cat);
    let mut rooms = Vec::new();
    for _ in 0..6 {
        rooms.push(expected_room(&run.state(), &cat));
        run.push(RunEvent::Node(NodeReport { kind: NodeKind::Unknown, encounter: None, event: None, changes: vec![] }));
    }
    use NodeKind::*;
    assert_eq!(rooms, [Some(Ancient), Some(Monster), None, Some(Treasure), Some(Rest), Some(Boss)]);
    assert_eq!(run.state().act_floor, 6);

    // Double-boss act (A10, act index 2): one extra boss floor before the last.
    cat.rules.double_boss_act = 0;
    let mut run = Run::default();
    run.push(RunEvent::start(&cat, &CharacterId::new("HERO"), 10).unwrap());
    run.push(RunEvent::EnterAct { act: ActId::new("ACT1"), index: 0, boss: None });
    let mut rooms = Vec::new();
    for _ in 0..7 {
        rooms.push(expected_room(&run.state(), &cat));
        run.push(RunEvent::Node(NodeReport { kind: NodeKind::Unknown, encounter: None, event: None, changes: vec![] }));
    }
    assert_eq!(rooms, [Some(Ancient), Some(Monster), None, Some(Treasure), Some(Rest), Some(Boss), Some(Boss)]);
}

#[test]
fn ending_a_run_records_outcome_and_summary() {
    use spire_core::run::Outcome;
    let cat = catalog();
    let mut run = started(&cat);
    run.started_at = Some("2026-11-17T08:00:00Z".into());
    assert!(!run.ended());
    let death = Outcome::Died { encounter: Some(EncounterId::new("BIG_BOSS")) };
    run.push(RunEvent::End { outcome: death.clone(), at: Some("2026-11-17T09:00:00Z".into()) });
    assert!(run.ended());
    let sum = run.summary();
    assert_eq!(sum.outcome, Some(death));
    assert_eq!(sum.character, Some(CharacterId::new("HERO")));
    assert_eq!(sum.ended_at.as_deref(), Some("2026-11-17T09:00:00Z"));
    assert_eq!(run.state_at(1).act, None, "scrubbing to the start is before entering an act");
}

#[test]
fn role_source_explains_rules_and_overrides() {
    use spire_core::playbook::RoleSource;
    let cat = catalog();
    let mut pb = playbook();
    let inflame = &cat.cards[&CardId::new("INFLAME")];
    match pb.role_source(inflame) {
        RoleSource::Rules(m) => assert_eq!(m, vec![(Role::ScalingDamage, "applies Strength".to_string())]),
        other => panic!("expected rules, got {other:?}"),
    }
    pb.card_roles.insert(inflame.id.clone(), vec![Role::Chip]);
    assert!(matches!(pb.role_source(inflame), RoleSource::Playbook(_)));
    pb.personal_roles.insert(inflame.id.clone(), vec![Role::FastDamage]);
    assert_eq!(pb.roles_of(inflame), vec![Role::FastDamage], "personal overrides win");
    let rule = Condition::All(vec![Condition::CardType(CardType::Power), Condition::TextContains("block".into())]);
    assert_eq!(rule.describe(), "is a Power and mentions \"block\"");
}

// ---------------------------------------------------------------- deck simulation

fn sim_state(cat: &Catalog, deck: &[(&str, bool)]) -> RunState {
    let start = RunEvent::start(cat, &CharacterId::new("HERO"), 0).expect("hero starts");
    let mut s = run::apply(RunState::default(), &start);
    s.deck =
        deck.iter().map(|(id, up)| DeckCard { card: CardId::new(*id), upgraded: *up, enchantment: None }).collect();
    s
}

fn quick() -> SimAssumptions {
    SimAssumptions { trials: 200, ..SimAssumptions::default() }
}

#[test]
fn sim_is_deterministic_and_bounded_by_energy() {
    let cat = catalog();
    let deck = [("STRIKE", false); 5].into_iter().chain([("DEFEND", false); 5]).collect::<Vec<_>>();
    let s = sim_state(&cat, &deck);
    let a = sim::simulate(&s, &cat, &quick());
    assert_eq!(a, sim::simulate(&s, &cat, &quick()), "same deck, same numbers");
    assert_eq!(a.turns.len(), 5);
    // Three energy, one-cost cards: at most three Strikes or three Defends a turn.
    for t in &a.turns {
        assert!(t.damage.mean > 6.0 && t.damage.mean <= 18.0, "damage {:?}", t.damage);
        assert!(t.block.mean > 5.0 && t.block.mean <= 15.0, "block {:?}", t.block);
        assert!(t.damage.low <= t.damage.mean && t.damage.mean <= t.damage.high + 0.01);
    }
    // Ten cards, five a turn: the deck cycles in two turns.
    assert!((a.turnover - 2.0).abs() < 0.01, "turnover {}", a.turnover);
    assert_eq!(a.dead_draws, 0.0);
    // Five cards in hand, three played: two energy's worth left behind.
    assert!((a.stranded_energy - 2.0).abs() < 0.01, "stranded {}", a.stranded_energy);
}

#[test]
fn sim_counts_dead_draws_and_upgrades() {
    let cat = catalog();
    let plain = sim_state(&cat, &[("STRIKE", false), ("STRIKE", false), ("REGRET", false), ("REGRET", false)]);
    let sim = sim::simulate(&plain, &cat, &quick());
    assert!((sim.dead_draws - 0.5).abs() < 0.01, "half the deck is dead: {}", sim.dead_draws);
    assert!(sim.odds.iter().all(|o| o.by_turn[0] > 0.99), "a four-card deck is drawn on turn one");

    let mut upgraded = cat.clone();
    let strike = upgraded.cards.get_mut(&CardId::new("STRIKE")).unwrap();
    strike.upgrade.deltas.insert("damage".into(), 3);
    let up = sim_state(&upgraded, &[("STRIKE", true), ("STRIKE", true)]);
    let base = sim_state(&upgraded, &[("STRIKE", false), ("STRIKE", false)]);
    let (up, base) = (sim::simulate(&up, &upgraded, &quick()), sim::simulate(&base, &upgraded, &quick()));
    assert_eq!(base.turns[0].damage.mean, 12.0);
    assert_eq!(up.turns[0].damage.mean, 18.0);
}

#[test]
fn sim_carries_strength_and_orbs_between_turns() {
    let mut cat = catalog();
    // Inflame: a power that gains 2 Strength, played before attacking.
    cat.cards.get_mut(&CardId::new("INFLAME")).unwrap().vars.insert("strengthpower".into(), 2);
    let deck = [("INFLAME", false)].into_iter().chain([("STRIKE", false); 9]).collect::<Vec<_>>();
    let sim = sim::simulate(&sim_state(&cat, &deck), &cat, &quick());
    let (first, last) = (sim.turns[0].damage.mean, sim.turns[4].damage.mean);
    assert!(last > first + 2.0, "Strength scales later turns: {first} then {last}");

    // A zero-cost Zap that channels Lightning, with three orb slots.
    let mut zap = cat.cards[&CardId::new("STRIKE")].clone();
    zap.id = CardId::new("ZAP");
    zap.damage = None;
    zap.card_type = CardType::Skill;
    zap.text = "Channel 1 Lightning.".into();
    zap.channels = vec![OrbChannel { orb: OrbId::new("LIGHTNING"), count: 1, x: false }];
    cat.cards.insert(zap.id.clone(), zap);
    cat.orbs.insert(
        OrbId::new("LIGHTNING"),
        Orb {
            id: OrbId::new("LIGHTNING"),
            name: "Lightning".into(),
            text: String::new(),
            passive: 3,
            passive_effect: OrbEffect::Damage,
            evoke: 8,
            evoke_effect: OrbEffect::Damage,
            focus: true,
            decay: 0,
        },
    );
    cat.characters.get_mut(&CharacterId::new("HERO")).unwrap().orb_slots = Some(3);
    let sim = sim::simulate(&sim_state(&cat, &[("ZAP", false); 6]), &cat, &quick());
    assert!(sim.resources.iter().any(|(r, n)| *r == sim::Resource::Orbs && *n > 2.0), "{:?}", sim.resources);
    // Turn one: three Zaps, three passives (9). Later turns evoke as well.
    assert_eq!(sim.turns[0].damage.mean, 9.0);
    assert!(sim.turns[1].damage.mean > 9.0);
}

#[test]
fn timeline_tracks_floors_fights_and_elites() {
    let cat = catalog();
    let pb = playbook();
    let mut run = Run::default();
    run.push(RunEvent::start(&cat, &CharacterId::new("HERO"), 0).unwrap());
    run.push(RunEvent::EnterAct { act: ActId::new("ACT1"), index: 0, boss: None });
    let node = |kind, enc: &str, hp| {
        RunEvent::Node(NodeReport {
            kind,
            encounter: Some(EncounterId::new(enc)),
            event: None,
            changes: vec![Change::SetHp(hp)],
        })
    };
    run.push(node(NodeKind::Monster, "SLIMES_WEAK", 70));
    run.push(node(NodeKind::Elite, "BRUTE_ELITE", 40));
    run.push(RunEvent::Node(NodeReport {
        kind: NodeKind::Rest,
        encounter: None,
        event: None,
        changes: vec![Change::SetHp(64)],
    }));
    let t = timeline::timeline(&run, &cat, &pb);
    assert_eq!(t.points.len(), 3);
    assert_eq!(t.points.iter().map(|p| p.hp_lost).collect::<Vec<_>>(), vec![10, 30, -24]);
    assert_eq!(t.fights().count(), 2);
    assert_eq!(t.elites_by_act, vec![1]);
    assert_eq!(t.points[1].floor, 2);
    // Replaying to a point's event index gives that floor's state.
    assert_eq!(run.state_at(t.points[1].event_index).hp, 40);
}

#[test]
fn upgrades_change_cost_text_and_keywords() {
    let mut zap = card("ZAP", CardType::Skill, Rarity::Basic, "Channel 1 Lightning.");
    zap.upgrade = Upgrade { cost: Some(0), ..Default::default() };
    zap.upgrade_text = Some(String::new());
    assert_eq!((zap.cost_label_at(false), zap.cost_label_at(true)), ("1".to_string(), "0".to_string()));
    assert_eq!(zap.text_at(true), "Channel 1 Lightning.", "an empty upgrade text keeps the base text");
    assert_eq!(zap.upgrade_changes(), vec!["Cost 1 to 0".to_string()]);

    let mut bash = card("BASH", CardType::Attack, Rarity::Basic, "Deal 8 damage.");
    bash.upgrade_text = Some("Deal 10 damage.".into());
    bash.vars.insert("damage".into(), 8);
    bash.keywords = vec!["Exhaust".into()];
    bash.upgrade = Upgrade {
        deltas: [("damage".to_string(), 2)].into_iter().collect(),
        adds: vec!["Innate".into()],
        removes: vec!["Exhaust".into()],
        ..Default::default()
    };
    assert_eq!(bash.text_at(true), "Deal 10 damage.");
    assert_eq!(bash.keywords_at(false), vec!["Exhaust"]);
    assert_eq!(bash.keywords_at(true), vec!["Innate"]);
    assert_eq!(bash.upgrade_changes(), vec!["Damage 8 to 10", "Gains Innate", "Loses Exhaust"]);
}

// ---------------------------------------------------------------- relics and ordering

fn with_relic(cat: &mut Catalog, id: &str, text: &str) -> RelicId {
    let relic = Relic {
        id: RelicId::new(id),
        name: id.into(),
        rarity: Rarity::Common,
        pool: "shared".into(),
        text: text.into(),
    };
    cat.relics.insert(relic.id.clone(), relic);
    RelicId::new(id)
}

fn strikes(cat: &Catalog, n: usize, relics: &[RelicId]) -> RunState {
    let mut s = sim_state(cat, &vec![("STRIKE", false); n]);
    s.relics = relics.to_vec();
    s
}

#[test]
fn vulnerable_multiplies_after_flat_bonuses_and_rounds_per_hit() {
    let mut cat = catalog();
    cat.rules.vulnerable_pct = Some(50);
    let marbles = with_relic(&mut cat, "MARBLES", "At the start of each combat, apply 1 Vulnerable to ALL enemies.");
    let base = sim::simulate(&strikes(&cat, 10, &[]), &cat, &quick());
    assert_eq!(base.turns[0].damage.mean, 18.0, "three Strikes");
    let vuln = sim::simulate(&strikes(&cat, 10, std::slice::from_ref(&marbles)), &cat, &quick());
    assert_eq!(vuln.turns[0].damage.mean, 27.0, "6 x 1.5 = 9 per Strike");
    assert_eq!(vuln.turns[1].damage.mean, 18.0, "one turn of Vulnerable wears off");

    // A flat bonus lands before the multiplier: (6 + 1) x 1.5 = 10.5, rounded down to 10.
    let dummy = with_relic(&mut cat, "DUMMY", "Cards containing “Strike” deal 1 additional damage.");
    let both = sim::simulate(&strikes(&cat, 10, &[marbles, dummy]), &cat, &quick());
    assert_eq!(both.turns[0].damage.mean, 30.0);
}

#[test]
fn combat_start_draws_join_the_opening_hand() {
    let mut cat = catalog();
    let snake = with_relic(&mut cat, "SNAKE", "At the start of each combat, draw 2 additional cards.");
    // Ten one-cost Strikes and 3 energy: drawing 7 instead of 5 doesn't add damage,
    // but the deck cycles faster.
    let plain = sim::simulate(&strikes(&cat, 12, &[]), &cat, &quick());
    let drawn = sim::simulate(&strikes(&cat, 12, &[snake]), &cat, &quick());
    assert!(drawn.turnover < plain.turnover, "{} vs {}", drawn.turnover, plain.turnover);
    let energy = with_relic(&mut cat, "LANTERN", "Start each combat with an additional 1 Energy.");
    let more = sim::simulate(&strikes(&cat, 12, &[energy]), &cat, &quick());
    assert_eq!((more.turns[0].damage.mean, more.turns[1].damage.mean), (24.0, 18.0));
}

#[test]
fn powers_trigger_on_later_cards_not_themselves() {
    let mut cat = catalog();
    let mut image = cat.cards[&CardId::new("INFLAME")].clone();
    image.id = CardId::new("AFTERIMAGE");
    image.cost = Some(0);
    image.text = "Whenever you play a card, gain 1 Block.".into();
    image.keywords = vec!["Innate".into()];
    cat.cards.insert(image.id.clone(), image);
    // Afterimage (innate, free) then three Strikes: 3 Block on turn 1, not 4.
    let mut deck = vec![("AFTERIMAGE", false)];
    deck.extend([("STRIKE", false); 9]);
    let sim = sim::simulate(&sim_state(&cat, &deck), &cat, &quick());
    assert_eq!(sim.turns[0].block.mean, 3.0, "{:?}", sim.turns[0]);
}

#[test]
fn per_turn_counters_reset_each_turn() {
    let mut cat = catalog();
    let fan = with_relic(&mut cat, "FAN", "Every time you play 2 Attacks in a single turn, gain 4 Block.");
    // Three Strikes a turn: one trigger a turn (the third Strike doesn't carry over).
    let sim = sim::simulate(&strikes(&cat, 10, &[fan]), &cat, &quick());
    assert_eq!((sim.turns[0].block.mean, sim.turns[1].block.mean), (4.0, 4.0));
}

#[test]
fn glass_decays_each_turn() {
    let mut cat = catalog();
    cat.characters.get_mut(&CharacterId::new("HERO")).unwrap().orb_slots = Some(3);
    let glass = Orb {
        id: OrbId::new("GLASS"),
        name: "Glass".into(),
        text: String::new(),
        passive: 4,
        passive_effect: OrbEffect::DamageAll,
        evoke: 8,
        evoke_effect: OrbEffect::DamageAll,
        focus: true,
        decay: 1,
    };
    cat.orbs.insert(glass.id.clone(), glass);
    let core = with_relic(&mut cat, "CORE", "At the start of each combat, Channel 1 Glass.");
    let mut s = sim_state(&cat, &[("REGRET", false); 6]);
    s.relics = vec![core];
    let sim = sim::simulate(&s, &cat, &quick());
    let by_turn: Vec<f32> = sim.turns.iter().map(|t| t.damage.mean).collect();
    assert_eq!(by_turn, vec![4.0, 3.0, 2.0, 1.0, 0.0]);
    assert_eq!(sim.relics_simulated, vec!["CORE".to_string()]);
}

#[test]
fn vigor_counts_on_every_hit_of_the_attack_that_spends_it() {
    let mut cat = catalog();
    let mut twin = cat.cards[&CardId::new("STRIKE")].clone();
    twin.id = CardId::new("TWIN");
    twin.damage = Some(2);
    twin.hits = Some(3);
    twin.keywords = vec!["Innate".into()];
    twin.tags = vec![];
    cat.cards.insert(twin.id.clone(), twin);
    let beko = with_relic(&mut cat, "BEKO", "At the start of each combat, gain 8 Vigor.");
    let mut s = sim_state(&cat, &[("TWIN", false)]);
    s.relics = vec![beko];
    let sim = sim::simulate(&s, &cat, &quick());
    // (2 + 8) x 3 on turn 1; Vigor is spent, so 2 x 3 afterwards.
    assert_eq!((sim.turns[0].damage.mean, sim.turns[1].damage.mean), (30.0, 6.0));
}

// ---------------------------------------------------------------- healing and tags

#[test]
fn weary_traveler_lowers_starting_hp_and_ancient_heals() {
    let cat = catalog();
    let rules = &cat.rules;
    assert_eq!(rules.ancient_heal_pct(1), 100);
    assert_eq!(rules.ancient_heal_pct(2), 80);
    // Community runs: Necrobinder 66 max starts at 52 on A2+ (52.8 rounded down).
    assert_eq!(rules.start_hp(66, 2), 52);
    assert_eq!(rules.start_hp(80, 1), 80);
    let a0 = run::apply(RunState::default(), &RunEvent::start(&cat, &CharacterId::new("HERO"), 0).unwrap());
    let a2 = run::apply(RunState::default(), &RunEvent::start(&cat, &CharacterId::new("HERO"), 2).unwrap());
    assert_eq!((a0.hp, a2.hp, a2.max_hp), (80, 64, 80));

    // Act 2: the heal is due once the boss is known, and only once.
    let mut s = run::apply(a2, &RunEvent::Adjust(vec![Change::SetHp(30)]));
    s = run::apply(s, &RunEvent::EnterAct { act: ActId::new("ACT1"), index: 1, boss: None });
    assert_eq!(s.ancient_heal_due(rules), None, "boss not chosen yet");
    s = run::apply(s, &RunEvent::SetBoss(EncounterId::new("BIG_BOSS")));
    let pct = s.ancient_heal_due(rules).expect("heal due");
    s = run::apply(s, &RunEvent::AncientHeal { percent: pct });
    // 50 missing x 80% = 40.
    assert_eq!(s.hp, 70);
    assert_eq!(s.ancient_heal_due(rules), None, "already healed");
}

#[test]
fn rest_heals_thirty_percent_plus_relic_bonus_capped_at_missing() {
    let mut cat = catalog();
    let mut s = run::apply(RunState::default(), &RunEvent::start(&cat, &CharacterId::new("HERO"), 0).unwrap());
    s.hp = 20;
    assert_eq!(analysis::healing::rest_heal(&s, &cat), 24, "30% of 80");
    let pillow = with_relic(&mut cat, "PILLOW", "Whenever you Rest, heal an additional 15 HP.");
    s.relics.push(pillow);
    assert_eq!(analysis::healing::rest_heal(&s, &cat), 39);
    s.hp = 70;
    assert_eq!(analysis::healing::rest_heal(&s, &cat), 10, "never more than missing");
}

#[test]
fn tags_come_from_numbers_and_text() {
    use spire_core::analysis::tags::{Tag, Tagged, tags};
    let pb = playbook();
    let names = |c: &Card| tags(c, false, &pb).iter().map(|t| t.tag).collect::<Vec<_>>();
    let mut pommel = card("POMMEL", CardType::Attack, Rarity::Common, "Deal 9 damage. Draw 1 card.");
    (pommel.damage, pommel.draw) = (Some(9), Some(1));
    assert_eq!(names(&pommel), vec![Tag::Damage, Tag::Draw]);
    let surge = card("SURGE", CardType::Skill, Rarity::Common, "Gain 2 Energy.");
    assert_eq!(names(&surge), vec![Tag::Energy]);
    let demesne = card(
        "DEMESNE",
        CardType::Power,
        Rarity::Rare,
        "At the start of your turn, gain 1 Energy and draw 1 additional card.",
    );
    assert_eq!(names(&demesne), vec![Tag::Draw, Tag::Energy]);
    let mut twin = card("TWIN", CardType::Attack, Rarity::Common, "Deal 5 damage twice.");
    (twin.damage, twin.hits, twin.cost) = (Some(5), Some(2), Some(1));
    let t = tags(&twin, false, &pb);
    assert!(t.contains(&Tagged { tag: Tag::MultiHit, value: 5, hits: 2 }));
    assert_eq!(t.iter().find(|t| t.tag == Tag::MultiHit).unwrap().label(), "Multi-Hit 2×5");
    let mut shiv = card("SHIV_ISH", CardType::Attack, Rarity::Common, "Deal 4 damage.");
    (shiv.damage, shiv.cost) = (Some(4), Some(0));
    assert_eq!(names(&shiv), vec![Tag::Damage, Tag::ChipDamage]);
    let mut cleave = card("CLEAVE_ISH", CardType::Attack, Rarity::Common, "Deal 8 damage to ALL enemies.");
    (cleave.damage, cleave.target) = (Some(8), spire_core::catalog::Target::AllEnemies);
    assert_eq!(names(&cleave), vec![Tag::Area]);
}

#[test]
fn roles_come_from_tags_and_overrides_replace_them() {
    use spire_core::analysis::tags::{Tag, TagSource, of};
    use spire_core::playbook::{RoleSource, TagRole};
    let mut pb = playbook();
    pb.role_rules.clear();
    pb.tag_roles = vec![
        TagRole { role: Role::Mitigation, tag: Tag::Block, min: 0 },
        TagRole { role: Role::Survival, tag: Tag::Block, min: 10 },
    ];
    let mut wall = card("WALL", CardType::Skill, Rarity::Common, "Gain 12 Block.");
    wall.block = Some(12);
    match pb.role_source(&wall) {
        RoleSource::Rules(m) => {
            assert_eq!(m, vec![(Role::Mitigation, "Block 12".to_string()), (Role::Survival, "Block 12".to_string())])
        }
        other => panic!("expected rules, got {other:?}"),
    }
    let mut defend = card("DEFEND_ISH", CardType::Skill, Rarity::Common, "Gain 5 Block.");
    defend.block = Some(5);
    assert_eq!(pb.roles_of(&defend), vec![Role::Mitigation], "5 Block is under the Survival line");
    assert_eq!(pb.tags_for(Role::Survival), vec![Tag::Block]);
    // Costing more than 1 rules Mitigation out, whatever gives it.
    pb.role_max_cost.insert(Role::Mitigation, 1);
    let mut wall2 = card("WALL2", CardType::Skill, Rarity::Common, "Gain 12 Block.");
    (wall2.block, wall2.cost) = (Some(12), Some(2));
    assert_eq!(pb.roles_of(&wall2), vec![Role::Survival]);
    wall2.x_cost = true;
    assert_eq!(pb.roles_of(&wall2), vec![Role::Survival], "X costs count as over the cap");
    assert_eq!(pb.roles_of(&defend), vec![Role::Mitigation], "1 energy is within it");
    pb.personal_tags.insert(wall.id.clone(), vec![Tag::Weak]);
    let (t, source) = of(&wall, false, &pb);
    assert_eq!((t.iter().map(|t| t.tag).collect::<Vec<_>>(), source), (vec![Tag::Weak], TagSource::Personal));
    assert!(pb.roles_of(&wall).is_empty(), "roles follow the overridden tags");
}

#[test]
fn history_edits_rewrite_the_log() {
    let mut cat = catalog();
    cat.rules.start_curse = Some(CardId::new("REGRET"));
    let mut run = Run::default();
    run.push(RunEvent::start(&cat, &CharacterId::new("HERO"), 0).unwrap());
    let fight = |e: &str| {
        RunEvent::Node(NodeReport {
            kind: NodeKind::Monster,
            encounter: Some(EncounterId::new(e)),
            event: None,
            changes: vec![],
        })
    };
    run.push(fight("A"));
    run.push(fight("B"));
    run.push(RunEvent::End { outcome: Outcome::Abandoned, at: Some("t".into()) });

    // Ascension: the start is resolved again (A5 adds the curse).
    let a5 = run.with_ascension(&cat, 5).unwrap();
    assert_eq!(a5.state().ascension, 5);
    assert_eq!(a5.state().deck.len(), 4);
    assert_eq!(a5.events.len(), run.events.len());

    // Outcome: replaced, keeping the end time.
    let died = run.with_outcome(Some(Outcome::Died { encounter: Some(EncounterId::new("B")) }));
    assert!(matches!(died.events.last(), Some(RunEvent::End { outcome: Outcome::Died { .. }, at: Some(_) })));
    assert_eq!(died.events.iter().filter(|e| matches!(e, RunEvent::End { .. })).count(), 1);

    // Floors: numbered, removable, and their fight can change.
    let floors: Vec<u32> = run.floors().iter().map(|(_, f, _)| *f).collect();
    assert_eq!(floors, vec![1, 2]);
    let (first, _, _) = run.floors()[0];
    assert_eq!(run.without_floor(first).floors().len(), 1);
    assert_eq!(run.without_floor(0).events.len(), run.events.len(), "only floors can be removed");
    let changed = run.with_floor_encounter(first, Some(EncounterId::new("C")));
    assert_eq!(changed.floors()[0].2.encounter, Some(EncounterId::new("C")));
}

#[test]
fn next_fights_leave_out_what_cannot_come_next() {
    let mut cat = catalog();
    for (id, room) in [("CULTIST_TRIO", RoomType::Monster), ("BRUTE_TWO", RoomType::Elite)] {
        cat.encounters.insert(EncounterId::new(id), encounter(id, room, false, &["CULTIST"]));
    }
    cat.acts.get_mut(&ActId::new("ACT1")).unwrap().weak_fights = Some(0);
    let pb = playbook();
    let mut run = started(&cat);
    let fight = |kind, e: &str| {
        RunEvent::Node(NodeReport { kind, encounter: Some(EncounterId::new(e)), event: None, changes: vec![] })
    };
    run.push(fight(NodeKind::Monster, "CULTIST_PAIR"));
    run.push(fight(NodeKind::Elite, "BRUTE_ELITE"));
    let s = run.state();
    let deck = deck_profile(&s.deck, &cat, &pb);
    let names = |kind, cat: &Catalog| -> Vec<String> {
        next_threats(&s, kind, cat, &pb, &deck).into_iter().map(|t| t.name).collect()
    };

    // Unmeasured rules: everything stays.
    assert_eq!(names(NodeKind::Monster, &cat).len(), 2);
    assert_eq!(names(NodeKind::Elite, &cat).len(), 2);

    cat.rules.unique_hallway_fights = true;
    cat.rules.no_elite_twice_in_a_row = true;
    assert_eq!(names(NodeKind::Monster, &cat), ["CULTIST_TRIO"]);
    assert_eq!(names(NodeKind::Elite, &cat), ["BRUTE_TWO"]);
}

#[test]
fn relics_that_add_potion_slots_say_so() {
    let relic = |text: &str| spire_core::catalog::Relic {
        id: RelicId::new("R"),
        name: "R".into(),
        rarity: Rarity::Common,
        pool: "shared".into(),
        text: text.into(),
    };
    assert_eq!(relic("Upon pickup, gain 2 potion slots.").potion_slots(), 2);
    assert_eq!(relic("Upon pickup, gain 1 potion slot and procure 2 random Potions.").potion_slots(), 1);
    assert_eq!(relic("At the start of each combat, fill all empty potion slots.").potion_slots(), 0);
    let mut run = started(&catalog());
    let before = run.state().potion_slots;
    run.push(RunEvent::Adjust(vec![spire_core::run::Change::AddPotionSlots(2)]));
    assert_eq!(run.state().potion_slots, before + 2);
}

// ---------------------------------------------------------------- benchmarks

#[test]
fn benchmarks_group_by_character_and_fall_back_to_everyone() {
    use spire_core::analysis::benchmark::{MIN_DECKS, Sample, for_run, measure};
    let mut cat = catalog();
    let mut pb = playbook();
    pb.simulation = SimAssumptions { trials: 20, ..SimAssumptions::default() };
    let hero = CharacterId::new("HERO");
    let deck = sim_state(&cat, &[("STRIKE", false), ("STRIKE", true), ("DEFEND", false), ("INFLAME", false)]);
    let sample = |character: &CharacterId, act| Sample { character: character.clone(), act, state: deck.clone() };
    // The hero cleared act 1 enough times; a second character only a few.
    let rare = CharacterId::new("RARE");
    let mut samples: Vec<Sample> = (0..MIN_DECKS).map(|_| sample(&hero, 0)).collect();
    samples.extend((0..3).map(|_| sample(&rare, 0)));
    let benchmarks = measure(&samples, &cat, &pb);
    assert_eq!(benchmarks.len(), 2, "the hero's, and everyone's");
    let mine = &benchmarks[0];
    assert_eq!((mine.character.as_ref(), mine.decks, mine.size.p50), (Some(&hero), MIN_DECKS as u32, 4.0));
    assert_eq!(benchmarks[1].decks, MIN_DECKS as u32 + 3);
    cat.benchmarks = benchmarks;
    assert_eq!(for_run(&cat, &hero, 0).and_then(|b| b.character.clone()), Some(hero));
    assert_eq!(for_run(&cat, &rare, 0).map(|b| b.character.is_none()), Some(true), "too few: everyone's decks");
    assert!(for_run(&cat, &rare, 1).is_none());
}

#[test]
fn an_endless_combo_ends_and_next_turn_energy_waits() {
    let mut cat = catalog();
    // Free, draws itself back: the game would let it loop.
    let mut spin = card("SPIN", CardType::Skill, Rarity::Common, "Deal 1 damage. Draw 1 card.");
    (spin.cost, spin.damage, spin.draw) = (Some(0), Some(1), Some(1));
    cat.cards.insert(spin.id.clone(), spin);
    let s = sim_state(&cat, &[("SPIN", false), ("SPIN", false)]);
    let sim = sim::simulate(&s, &cat, &quick());
    assert!(sim.turns[0].damage.mean > 0.0, "it plays, and stops");
    // "Next turn, gain 2 Energy" is spent next turn, not now.
    let mut invoke = card("INVOKE_ISH", CardType::Skill, Rarity::Common, "Next turn, gain 2 Energy.");
    invoke.energy = Some(2);
    invoke.vars.insert("energy".into(), 2);
    cat.cards.insert(invoke.id.clone(), invoke);
    let mut big = card("BIG", CardType::Attack, Rarity::Common, "Deal 20 damage.");
    (big.cost, big.damage) = (Some(3), Some(20));
    cat.cards.insert(big.id.clone(), big);
    let s = sim_state(&cat, &[("INVOKE_ISH", false), ("BIG", false)]);
    let sim = sim::simulate(&s, &cat, &quick());
    assert_eq!(sim.turns[0].damage.mean, 0.0, "3 energy: Invoke, then Big is out of reach this turn");
}

#[test]
fn strikes_and_weak_attacks_are_excluded_from_roles() {
    use spire_core::analysis::tags::Tag;
    use spire_core::playbook::{RoleExclusion, TagRole};
    let mut pb = playbook();
    pb.role_rules.clear();
    pb.tag_roles = vec![TagRole { role: Role::FastDamage, tag: Tag::Damage, min: 0 }];
    pb.role_exclusions = vec![
        RoleExclusion {
            role: None,
            when: Condition::All(vec![Condition::Rarity(Rarity::Basic), Condition::SourceTag("Strike".into())]),
            why: None,
        },
        RoleExclusion {
            role: Some(Role::FastDamage),
            when: Condition::Any(vec![Condition::TotalDamageAtMost(6), Condition::DamagePerEnergyAtMost(6)]),
            why: None,
        },
    ];
    let attack = |id: &str, cost: u8, damage: u32, hits: u32| {
        let mut c = card(id, CardType::Attack, Rarity::Common, "Deal damage.");
        (c.cost, c.damage, c.hits) = (Some(cost), Some(damage), Some(hits));
        c
    };
    let mut strike = attack("STRIKE_HERO", 1, 9, 1);
    (strike.rarity, strike.tags) = (Rarity::Basic, vec!["Strike".into()]);
    assert!(pb.roles_of(&strike).is_empty(), "starter Strikes fill no role");
    assert!(pb.roles_of(&attack("POKE", 0, 6, 1)).is_empty(), "6 damage or less");
    assert!(pb.roles_of(&attack("HEAVY", 2, 12, 1)).is_empty(), "6 per energy or less");
    assert_eq!(pb.roles_of(&attack("TWIN", 1, 4, 2)), vec![Role::FastDamage], "8 in all for 1 energy");
    assert_eq!(pb.roles_of(&attack("CLEAVE", 2, 13, 1)), vec![Role::FastDamage]);
}

#[test]
fn the_current_floor_is_the_one_being_played() {
    let cat = catalog();
    let mut run = started(&cat);
    assert_eq!(run.state().current_floor(), 1, "a new run is on floor 1");
    run.push(RunEvent::End { outcome: Outcome::Died { encounter: None }, at: None });
    assert_eq!(run.state().current_floor(), 0, "an ended run shows the floors it reached");
}

#[test]
fn the_next_act_is_offered_only_after_this_acts_boss() {
    let cat = catalog();
    let mut run = started(&cat);
    let floor = |kind| RunEvent::Node(NodeReport { kind, encounter: None, event: None, changes: vec![] });
    run.push(floor(NodeKind::Monster));
    assert!(!run.state().act_boss_beaten());
    run.push(floor(NodeKind::Boss));
    assert!(run.state().act_boss_beaten(), "the boss is beaten: offer the next act");
    // Entered the next act, nothing logged in it yet: the last act's boss
    // must not offer the act after (Act 3 straight after Act 1's boss).
    run.push(RunEvent::EnterAct { act: ActId::new("ACT1"), index: 1, boss: None });
    assert!(!run.state().act_boss_beaten());
    run.push(floor(NodeKind::Ancient));
    assert!(!run.state().act_boss_beaten());
}
