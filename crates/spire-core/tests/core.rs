//! Behavior tests for the core, on small hand-built fixtures.

use spire_core::analysis::deck::{Coverage, coverage, deck_profile};
use spire_core::analysis::events::{needs, watchlist};
use spire_core::analysis::threats::next_threats;
use spire_core::analysis::{Dashboard, Severity};
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
        }],
        innate_powers: vec![],
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
        }],
        ..Default::default()
    };
    Catalog::from_layers([pack])
}

fn playbook() -> Playbook {
    let json = r#"{
      "version": 1,
      "role_rules": [
        { "role": "frontload_damage", "when": "has_damage" },
        { "role": "frontload_defense", "when": "has_block" },
        { "role": "scaling_damage", "when": { "applies_power": "Strength" } }
      ],
      "aoe": { "all": ["has_damage", { "target": "all_enemies" }] },
      "multi_enemy_names": ["pair"],
      "opportunity_rules": [
        { "opportunity": "card_removal", "any_text": ["remove"] },
        { "opportunity": "heal", "any_text": ["heal"] }
      ],
      "acts": [{
        "index": 0, "weak_fights": 1,
        "targets": [{ "role": "frontload_damage", "min": 2 }, { "role": "scaling_damage", "min": 1 }],
        "needs_aoe": 1, "big_hit": 25,
        "long_fight_hp": { "monster": 100, "elite": 150, "boss": 250 },
        "notes": ["Take fights."]
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
    assert_eq!(p.role(Role::FrontloadDamage), 1, "Strike is a starter, Cleave is added");
    assert_eq!(p.starter_role(Role::FrontloadDamage), 1);
    assert_eq!(p.role(Role::ScalingDamage), 1);
    assert_eq!(p.aoe, 1);
    assert_eq!(p.starter_basics, 2);
    let plan = pb.act(0);
    assert_eq!(coverage(&p, Capability::Role(Role::FrontloadDamage), plan), Coverage::Thin);
    assert_eq!(coverage(&p, Capability::Aoe, plan), Coverage::Covered);
    assert_eq!(coverage(&p, Capability::Role(Role::ScalingDefense), plan), Coverage::Missing);
}

#[test]
fn card_role_overrides_replace_rules() {
    let cat = catalog();
    let mut pb = playbook();
    pb.card_roles.insert(CardId::new("STRIKE"), vec![Role::Consistency]);
    assert_eq!(pb.roles_of(&cat.cards[&CardId::new("STRIKE")]), vec![Role::Consistency]);
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
    let caps: Vec<Capability> = pair.checks.iter().map(|c| c.demand.capability).collect();
    assert!(caps.contains(&Capability::Aoe), "two enemies want area damage");
    assert!(caps.contains(&Capability::Role(Role::FrontloadDamage)), "a buffer wants a fast kill");
}

#[test]
fn elite_and_boss_threats_read_damage_and_hp() {
    let cat = catalog();
    let pb = playbook();
    let s = started(&cat).state();
    let deck = deck_profile(&s.deck, &cat, &pb);

    let elite = &next_threats(&s, NodeKind::Elite, &cat, &pb, &deck)[0];
    assert_eq!(elite.profile.worst_turn, 30);
    assert!(elite.checks.iter().any(|c| c.demand.capability == Capability::Role(Role::FrontloadDefense)));

    let boss = &next_threats(&s, NodeKind::Boss, &cat, &pb, &deck)[0];
    assert_eq!(boss.name, "BIG_BOSS");
    assert!(boss.checks.iter().any(|c| c.demand.capability == Capability::Role(Role::ScalingDamage)));
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
    assert!(d.boss.is_some());
    assert!(d.advice.iter().any(|a| a.severity == Severity::Urgent));
    assert!(d.advice.iter().any(|a| a.text == "Take fights."));
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
    let before = threat_profile(&enc, &cat, &pb);
    assert_eq!(before.enemies, 1);

    enc.lineups = vec![
        Lineup { monsters: vec!["SLIME".into(), "SLIME".into(), "SLIME".into()], share: 0.6 },
        Lineup { monsters: vec!["SLIME".into(), "CULTIST".into()], share: 0.4 },
    ];
    enc.spawns = vec!["CULTIST".into()];
    let t = threat_profile(&enc, &cat, &pb);
    assert_eq!(t.enemies, 3);
    assert_eq!((t.hp_min, t.hp_max), (60, 70), "3 slimes = 60, slime + cultist = 70");
    assert_eq!(t.worst_turn, 15);
    assert!(t.summons);
    assert_eq!(cat.lineup_label(&enc.lineups[0]), "3× SLIME");
}
