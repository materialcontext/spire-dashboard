//! Fight rules: monster powers that change what kind of damage or play
//! works, read from the powers' text in the data.
//!
//! Raw numbers aren't enough against some enemies. Vantom's Slippery turns
//! its next 2 HP losses into 1 each, so it wants many hits; Entomancer's
//! Personal Hive adds a Dazed per hit, so it wants few, big ones. Each rule
//! says what it asks of the deck ([`Ask`]); the lines and roles use that.
//! Text the reader doesn't recognise is kept as [`Kind::Other`], so the
//! power still shows with its words.

use crate::analysis::tags::Tag;
use crate::catalog::{Monster, MonsterPower};
use crate::ids::MonsterId;

/// What a rule does, as read from its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// "The next N times this creature loses HP, it only loses 1 HP" (Slippery).
    HitsLoseOne,
    /// "Reduce all damage taken and HP loss to 1" (Intangible), or gaining it
    /// on a schedule (Nemesis).
    Intangible,
    /// "The first time this creature is hit each turn, it gains Block" (Skittish).
    BlockOnFirstHit,
    /// "Cannot lose more than N HP each turn" (Hardened Shell).
    HpCapPerTurn,
    /// Each Attack hit on it adds a junk card to your deck (Personal Hive).
    JunkPerHit,
    /// It takes more damage for each card you play this turn (Slow).
    MorePerCard,
    /// Your Skills give it Strength (Enrage).
    PunishesSkills,
    /// Unblocked attack damage adds junk to your deck (Painful Stabs).
    PunishesUnblocked,
    /// "Negates debuffs" (Artifact).
    NegatesDebuffs,
    /// Stunned the first time its HP reaches a line (Shriek, Plow).
    StunAtHp,
    /// Gains Block every turn (Plating).
    BlockEachTurn,
    /// Your cards suffer while it lives (Hex: all your cards are Ethereal).
    KillFirst,
    /// Minions leave without their leader.
    Minion,
    /// Grows stronger when an ally dies (Crab Rage).
    AllyDeathRage,
    /// Revives instead of dying (Adaptable): the fight has phases.
    Revives,
    /// Gains Strength every turn (Territorial).
    Ramps,
    /// Deals damage after it dies (Steam Eruption).
    BlastAfterDeath,
    /// Summons more monsters when it dies (Infested, Stock).
    SummonsOnDeath,
    /// Asleep: wakes after some turns or when hurt.
    Asleep,
    /// Not recognised: shown with its text.
    Other,
}

/// What a rule asks of the deck.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ask {
    ManyHits,
    FewBigHits,
    SpreadDamage,
    ManyCards,
    FewSkills,
    BlockFully,
    RepeatDebuffs,
    CrossHpLine,
    KillFirst,
    CleanUp,
}

impl Ask {
    pub fn label(self) -> &'static str {
        match self {
            Ask::ManyHits => "Many hits",
            Ask::FewBigHits => "Few, big hits",
            Ask::SpreadDamage => "Damage spread over turns",
            Ask::ManyCards => "Many cards a turn",
            Ask::FewSkills => "Few Skills",
            Ask::BlockFully => "Block its attacks fully",
            Ask::RepeatDebuffs => "Debuffs applied more than once",
            Ask::CrossHpLine => "Damage to cross its HP line",
            Ask::KillFirst => "Kill it first",
            Ask::CleanUp => "Clean-Up for the junk",
        }
    }
}

impl Ask {
    /// The tags that answer it: what to look for in cards.
    pub fn tags(self) -> &'static [Tag] {
        match self {
            Ask::ManyHits => &[Tag::MultiHit, Tag::ChipDamage],
            Ask::FewBigHits | Ask::CrossHpLine | Ask::KillFirst => &[Tag::Damage],
            Ask::SpreadDamage => &[Tag::ScalingDamage, Tag::ChipDamage],
            Ask::ManyCards => &[Tag::ChipDamage, Tag::Draw, Tag::Energy],
            Ask::FewSkills => &[],
            Ask::BlockFully => &[Tag::Block, Tag::Weak],
            Ask::RepeatDebuffs => &[Tag::Weak, Tag::Vulnerable],
            Ask::CleanUp => &[Tag::CleanUp],
        }
    }
}

impl Kind {
    /// What the rule asks of the deck (nothing for rules that only shape
    /// the fight: phases, ramps, sleep, after-death effects).
    pub fn asks(self) -> &'static [Ask] {
        match self {
            Kind::HitsLoseOne | Kind::Intangible | Kind::BlockOnFirstHit => &[Ask::ManyHits],
            Kind::HpCapPerTurn => &[Ask::SpreadDamage],
            Kind::JunkPerHit => &[Ask::FewBigHits, Ask::CleanUp],
            Kind::BlockEachTurn => &[Ask::FewBigHits],
            Kind::MorePerCard => &[Ask::ManyCards],
            Kind::PunishesSkills => &[Ask::FewSkills],
            Kind::PunishesUnblocked => &[Ask::BlockFully, Ask::CleanUp],
            Kind::NegatesDebuffs => &[Ask::RepeatDebuffs],
            Kind::StunAtHp => &[Ask::CrossHpLine],
            Kind::KillFirst | Kind::Minion | Kind::AllyDeathRage => &[Ask::KillFirst],
            Kind::Revives | Kind::Ramps | Kind::BlastAfterDeath | Kind::SummonsOnDeath | Kind::Asleep | Kind::Other => {
                &[]
            }
        }
    }
}

/// One rule in a fight: which monster, which power, and how it reads.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub monster: MonsterId,
    pub power: MonsterPower,
    pub kind: Kind,
}

/// Reads a power's text. Lower-cased phrases, as the data words them.
pub fn read(text: &str) -> Kind {
    let t = text.to_lowercase();
    let has = |p: &str| t.contains(p);
    if has("only loses 1 hp") {
        Kind::HitsLoseOne
    } else if has("damage taken and hp loss to 1") || has("gains intangible") {
        Kind::Intangible
    } else if has("first time this creature is hit each turn") && has("block") {
        Kind::BlockOnFirstHit
    } else if has("cannot lose more than") && has("each turn") {
        Kind::HpCapPerTurn
    } else if has("hit by an attack") && (has("dazed") || has("wound") || has("into your")) {
        Kind::JunkPerHit
    } else if has("whenever you play a card") && has("more damage") {
        Kind::MorePerCard
    } else if has("whenever you play a skill") && has("strength") {
        Kind::PunishesSkills
    } else if has("unblocked attack damage") {
        Kind::PunishesUnblocked
    } else if has("negates debuffs") {
        Kind::NegatesDebuffs
    } else if has("hp reaches") && has("stunned") {
        Kind::StunAtHp
    } else if has("ethereal") && has("while") {
        Kind::KillFirst
    } else if has("without their leader") {
        Kind::Minion
    } else if has("when an ally dies") {
        Kind::AllyDeathRage
    } else if has("revives") {
        Kind::Revives
    } else if (has("when killed") || has("upon dying")) && has("summon") || has("summoned in its place") {
        Kind::SummonsOnDeath
    } else if has("when killed") && has("deals damage") {
        Kind::BlastAfterDeath
    } else if has("awakens") {
        Kind::Asleep
    } else if has("gain") && has("block") && has("end of") {
        Kind::BlockEachTurn
    } else if has("gains") && has("strength") && (has("end of") || has("each turn")) {
        Kind::Ramps
    } else {
        Kind::Other
    }
}

/// A monster's rules, in the order the data lists its powers.
pub fn of(monster: &Monster) -> Vec<Rule> {
    monster.powers.iter().map(|p| Rule { monster: monster.id.clone(), power: p.clone(), kind: read(&p.text) }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_rules_the_data_words() {
        let cases = [
            ("The next 2 times this creature loses HP, it only loses 1 HP instead.", Kind::HitsLoseOne),
            ("Reduce all damage taken and HP loss to 1 this turn.", Kind::Intangible),
            ("At the end of every other turn, gains Intangible 1.", Kind::Intangible),
            ("The first time this creature is hit each turn, it gains  Block.", Kind::BlockOnFirstHit),
            ("This creature cannot lose more than  HP each turn.", Kind::HpCapPerTurn),
            ("Whenever this enemy is hit by an Attack, add Dazed into your Draw Pile.", Kind::JunkPerHit),
            (
                "Whenever you play a card, this enemy receives 10% more damage from Attacks this turn.",
                Kind::MorePerCard,
            ),
            ("Whenever you play a Skill, gains 2 Strength.", Kind::PunishesSkills),
            (
                "Shuffle 1 Wound into your Discard Pile each time you receive unblocked attack damage.",
                Kind::PunishesUnblocked,
            ),
            ("Negates debuffs.", Kind::NegatesDebuffs),
            ("The first time this creature's HP reaches  or below, it becomes Stunned.", Kind::StunAtHp),
            (
                "At the end of your turn, gain  Block. Plating is reduced by 1 at the start of your turn.",
                Kind::BlockEachTurn,
            ),
            ("While Spectral Knight is alive, ALL your cards are Ethereal.", Kind::KillFirst),
            ("Minions abandon combat without their leader.", Kind::Minion),
            ("When an ally dies, this creature gains 6 Strength and 99 Block.", Kind::AllyDeathRage),
            ("When this creature would be defeated, it instead revives even stronger.", Kind::Revives),
            ("At the end of this creature's turn, it gains  Strength.", Kind::Ramps),
            ("When killed, deals damage at the end of your next turn.", Kind::BlastAfterDeath),
            ("Upon dying, summons... something.", Kind::SummonsOnDeath),
            ("When killed, a new Axebot is summoned in its place.", Kind::SummonsOnDeath),
            ("Awakens upon losing HP or after 2 turns.", Kind::Asleep),
            ("Deals 50% more damage when it is attacking you from behind.", Kind::Other),
        ];
        for (text, kind) in cases {
            assert_eq!(read(text), kind, "{text}");
        }
        assert_eq!(Kind::HitsLoseOne.asks(), &[Ask::ManyHits]);
        assert_eq!(Kind::JunkPerHit.asks(), &[Ask::FewBigHits, Ask::CleanUp]);
        assert_eq!(Ask::ManyHits.tags(), &[Tag::MultiHit, Tag::ChipDamage]);
    }
}
