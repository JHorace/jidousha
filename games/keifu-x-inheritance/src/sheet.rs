//! The hero sheet and hero card as information (SPEC §19.1, §5.4).
//!
//! `hero_sheet` is the one function that says what a sheet shows: the screen
//! draws its lines and the verify run reads the same lines, so the picture and
//! the oracle cannot disagree. Layout is the screen's; words come from content.

use crate::blessing::blessing_effect;
use crate::constants::{
    BONDS_SHOWN, CONQUERED_FEAR_BONUS, COURAGE_TO_CONQUER, DREAD_LIMIT, LEGACY_HEIRLOOM_BONUS,
    MARRY_IN_RENOWN, TALE_YEARLY_RENOWN, WOUND_PENALTY, fear_penalty, phase_adjustment,
};
use crate::content::Content;
use crate::dream::{Dream, StageMark, told_title};
use crate::hero::{Fate, Hero, HeroId};
use crate::ids::{Aptitude, BondKind, Destiny, LegacyKind, Phase};
use crate::text::{fmt, lowered, signed, year_telling};
use crate::words::W;

/// How a sheet line is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    /// The hero's full name.
    Name,
    /// A section heading: DREAM, FEAR, ...
    Heading,
    /// Ordinary text.
    Body,
    /// Secondary text: notes, promises, provenance.
    Note,
    /// A warning.
    Warning,
    /// A stage already done.
    Done,
    /// The current stage.
    Current,
    /// A stage still to come.
    Upcoming,
    /// A bond to someone gone.
    Gone,
}

/// One line of a sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// How it is set.
    pub ink: Ink,
    /// What it says.
    pub text: String,
    /// Pips drawn after the text: (filled, of).
    pub pips: Option<(i32, i32)>,
}

fn line(ink: Ink, text: impl Into<String>) -> Line {
    Line {
        ink,
        text: text.into(),
        pips: None,
    }
}

/// The station: "Child of house H, aged A" or "<Vocation>, <phase>, aged A".
pub fn station(content: &Content, hero: &Hero) -> String {
    let words = &content.words;
    let age = if hero.fate == Fate::Living {
        hero.age
    } else {
        hero.fate_age
    };
    if hero.phase() == Phase::Child {
        fmt(
            &words[W::SheetChildStation],
            &[&hero.house, &age.to_string()],
        )
    } else {
        let vocation = &content.lore.vocations[hero.vocation.index()];
        let phase = lowered(&content.lore.phases[hero.phase().index()].title);
        fmt(
            &words[W::SheetStation],
            &[vocation, &phase, &age.to_string()],
        )
    }
}

/// The aptitude row: "Might 5 (-2)", or "Spirit 4" when the phase changes nothing.
pub fn aptitude_row(content: &Content, hero: &Hero, aptitude: Aptitude) -> String {
    let title = &content.lore.aptitudes[aptitude.index()];
    let adjustment = phase_adjustment(hero.phase(), aptitude);
    if adjustment == 0 {
        format!("{title} {}", hero.effective(aptitude))
    } else {
        format!(
            "{title} {} ({})",
            hero.effective(aptitude),
            signed(adjustment)
        )
    }
}

/// A sheet: its lines in order, and where the second column starts.
pub struct Sheet {
    /// Every line, in reading order.
    pub lines: Vec<Line>,
    /// The index of the first line of the second column (the DESTINY heading).
    pub second_column: usize,
}

/// Everything a hero's sheet shows, in order (SPEC §19.1).
pub fn hero_sheet(content: &Content, heroes: &[Hero], id: HeroId) -> Sheet {
    let hero = &heroes[id];
    let words = &content.words;
    let mut out = vec![
        line(Ink::Name, hero.full_name()),
        line(Ink::Body, station(content, hero)),
    ];
    out.push(match hero.fate {
        Fate::Dead => line(
            Ink::Body,
            fmt(
                &words[W::SheetDied],
                &[
                    &year_telling(content, hero.fate_year),
                    &hero.fate_age.to_string(),
                ],
            ),
        ),
        Fate::Departed => line(
            Ink::Body,
            fmt(
                &words[W::SheetLeft],
                &[
                    &year_telling(content, hero.fate_year),
                    &hero.fate_age.to_string(),
                ],
            ),
        ),
        Fate::Living if hero.wounded => line(
            Ink::Warning,
            fmt(
                &words[W::SheetRenownWounded],
                &[&hero.renown.to_string(), &WOUND_PENALTY.to_string()],
            ),
        ),
        Fate::Living if hero.settled => line(
            Ink::Body,
            fmt(&words[W::SheetRenownSettled], &[&hero.renown.to_string()]),
        ),
        Fate::Living => line(
            Ink::Body,
            fmt(&words[W::SheetRenown], &[&hero.renown.to_string()]),
        ),
    });
    if !hero.family && hero.is_living() {
        out.push(line(
            Ink::Note,
            fmt(&words[W::SheetOutsider], &[&MARRY_IN_RENOWN.to_string()]),
        ));
    }
    for aptitude in Aptitude::ALL {
        out.push(line(Ink::Body, aptitude_row(content, hero, *aptitude)));
    }
    for aptitude in Aptitude::ALL {
        out.push(line(Ink::Note, aptitude_note(content, hero, *aptitude)));
    }
    if hero.wounded {
        out.push(line(Ink::Warning, &words[W::SheetWoundedWarning]));
    }
    dream_section(content, heroes, id, &mut out);
    fear_section(content, hero, &mut out);
    let second_column = out.len();
    destiny_section(content, hero, &mut out);
    bonds_section(content, heroes, id, &mut out);
    if let Some(heirloom) = &hero.heirloom {
        out.push(line(Ink::Heading, &words[W::SheetHeirloom]));
        out.push(line(Ink::Body, heirloom.name.clone()));
        let effect = fmt(
            &content.legacies.heirloom_effect,
            &[
                &heirloom.bonus.to_string(),
                &content.lore.aptitudes[heirloom.aptitude.index()],
            ],
        );
        out.push(line(Ink::Body, effect));
        out.push(line(Ink::Note, heirloom.provenance.clone()));
    }
    for blessing in &hero.blessings {
        out.push(line(Ink::Heading, &words[W::SheetBlessed]));
        out.push(line(Ink::Body, blessing.title.clone()));
        out.push(line(Ink::Note, blessing_effect(content, blessing)));
    }
    if hero.legacy.0 != LegacyKind::None {
        out.push(line(Ink::Heading, &words[W::SheetLeaves]));
        out.push(line(Ink::Body, hero.legacy.1.clone()));
    }
    for scar in &hero.scars {
        out.push(line(Ink::Heading, &words[W::SheetScar]));
        out.push(line(Ink::Body, scar.clone()));
    }
    Sheet {
        lines: out,
        second_column,
    }
}

/// What the original shows on hover over an aptitude: base "of 9", and the
/// phase's adjustment when it has one.
fn aptitude_note(content: &Content, hero: &Hero, aptitude: Aptitude) -> String {
    let words = &content.words;
    let title = &content.lore.aptitudes[aptitude.index()];
    let base = hero.base(aptitude).to_string();
    let limit = crate::constants::APTITUDE_LIMIT.to_string();
    let adjustment = phase_adjustment(hero.phase(), aptitude);
    if adjustment == 0 {
        fmt(&words[W::SheetAptitudeNote], &[title, &base, &limit])
    } else {
        let sign = if adjustment > 0 { "+" } else { "-" };
        let telling = &content.lore.phases[hero.phase().index()].telling;
        fmt(
            &words[W::SheetAptitudeNoteAdjusted],
            &[
                title,
                &base,
                &limit,
                sign,
                &adjustment.abs().to_string(),
                telling,
            ],
        )
    }
}

fn dream_section(content: &Content, heroes: &[Hero], id: HeroId, out: &mut Vec<Line>) {
    let hero = &heroes[id];
    let words = &content.words;
    match &hero.dream {
        Some(dream) => {
            out.push(line(Ink::Heading, &words[W::SheetDream]));
            dream_lines(content, heroes, dream, out);
        }
        // SPEC-GAPS KG-4: an undreamt adult shows no DREAM section.
        None if hero.phase() == Phase::Child => {
            out.push(line(Ink::Heading, &words[W::SheetDream]));
            out.push(line(Ink::Body, &words[W::SheetTooYoung]));
        }
        None => {}
    }
    if let Some(burden) = &hero.burden {
        out.push(line(Ink::Heading, &words[W::SheetBurden]));
        dream_lines(content, heroes, burden, out);
    }
}

fn dream_lines(content: &Content, heroes: &[Hero], dream: &Dream, out: &mut Vec<Line>) {
    let words = &content.words;
    match dream.owner {
        Some(owner) => {
            out.push(line(
                Ink::Body,
                told_title(content, dream, heroes[owner].pronoun),
            ));
            out.push(line(
                Ink::Note,
                fmt(&words[W::SheetTakenUp], &[&heroes[owner].full_name()]),
            ));
        }
        None => out.push(line(Ink::Body, dream.title.clone())),
    }
    if dream.is_fulfilled() {
        out.push(line(Ink::Body, &words[W::SheetFulfilled]));
    } else {
        for (index, stage) in dream.stages.iter().enumerate() {
            let mark = dream.mark(index);
            // An owned (carried) dream shows only its current stage (SPEC §19.1).
            if dream.owner.is_some() && mark != StageMark::Current {
                continue;
            }
            let mut task = stage.task.clone();
            // SPEC-GAPS KG-5: raw tasks, and the count on a current stage with goal > 1.
            if mark == StageMark::Current && stage.goal > 1 {
                task = fmt(
                    &content.dream_formats.progress,
                    &[
                        &task,
                        &stage.count.min(stage.goal).to_string(),
                        &stage.goal.to_string(),
                    ],
                );
            }
            let ink = match mark {
                StageMark::Done => Ink::Done,
                StageMark::Current => Ink::Current,
                StageMark::Upcoming => Ink::Upcoming,
            };
            out.push(line(ink, task));
        }
    }
    // SPEC-GAPS KG-23: the promise is shown under a fulfilled dream too.
    let legacy = content.dreams[dream.kind.index()].legacy;
    let promise = &content.legacies.promises[legacy.index()];
    let promise = match legacy {
        LegacyKind::Heirloom => fmt(promise, &[&LEGACY_HEIRLOOM_BONUS.to_string()]),
        LegacyKind::Tale => fmt(promise, &[&TALE_YEARLY_RENOWN.to_string()]),
        LegacyKind::Blessing | LegacyKind::None => promise.clone(),
    };
    if !promise.is_empty() {
        out.push(line(Ink::Note, promise));
    }
}

fn fear_section(content: &Content, hero: &Hero, out: &mut Vec<Line>) {
    let words = &content.words;
    let fear = &hero.fear;
    let tag = &content.lore.tags[fear.tag.index()];
    let label = if fear.born_brave {
        W::FearStateBornBrave
    } else if fear.conquered {
        W::FearStateConquered
    } else if fear.broken {
        W::FearStateBroken
    } else {
        W::FearState
    };
    out.push(line(Ink::Heading, &words[label]));
    out.push(line(Ink::Body, format!("{} ({})", tag.title, tag.noun)));
    if fear.born_brave || fear.conquered {
        let bonus = CONQUERED_FEAR_BONUS.to_string();
        out.push(line(
            Ink::Note,
            fmt(&words[W::FearEffectConquered], &[&bonus, &tag.title]),
        ));
    } else if fear.broken {
        out.push(line(
            Ink::Note,
            fmt(&words[W::FearEffectBroken], &[&tag.title]),
        ));
    } else if hero.settled {
        out.push(line(Ink::Body, &words[W::SheetSettled]));
    } else {
        let penalty = fear_penalty(fear.dread);
        out.push(Line {
            ink: Ink::Body,
            text: words[W::SheetDread].to_owned(),
            pips: Some((fear.dread, DREAD_LIMIT)),
        });
        out.push(Line {
            ink: Ink::Body,
            text: words[W::SheetCourage].to_owned(),
            pips: Some((fear.courage, COURAGE_TO_CONQUER)),
        });
        out.push(line(
            Ink::Warning,
            fmt(&words[W::SheetPenalty], &[&penalty.to_string()]),
        ));
        out.push(line(
            Ink::Note,
            fmt(&words[W::FearEffect], &[&penalty.to_string(), &tag.title]),
        ));
    }
}

/// The prophecy as it is read out: "Blood of X: you will ..." for an inherited Door
/// destiny, else the destiny's own (SPEC §19.1; the Seer's lines at a coming of age and an
/// arrival read the same, §17.2, §17.5).
pub fn prophecy(content: &Content, hero: &Hero) -> String {
    let lore = &content.destinies[hero.destiny.kind.index()];
    match &hero.destiny.blood_of {
        Some(blood) => fmt(
            &content.blood_of_prophecy,
            &[blood, &lowered(&lore.prophecy)],
        ),
        None => lore.prophecy.clone(),
    }
}

fn destiny_section(content: &Content, hero: &Hero, out: &mut Vec<Line>) {
    let words = &content.words;
    let destiny = &hero.destiny;
    let lore = &content.destinies[destiny.kind.index()];
    let heading = if destiny.fulfilled {
        W::SheetDestinyCome
    } else {
        W::SheetDestiny
    };
    out.push(line(Ink::Heading, &words[heading]));
    out.push(line(Ink::Body, prophecy(content, hero)));
    // SPEC-GAPS KG-6: UNSPOKEN shows its prophecy only.
    if destiny.kind != Destiny::Unspoken {
        out.push(line(Ink::Note, lore.doom.clone()));
        out.push(line(Ink::Note, lore.gift.clone()));
    }
}

/// The bonds the sheet shows: shown kinds only, living first, formation order within.
pub fn shown_bonds(content: &Content, heroes: &[Hero], id: HeroId) -> Vec<(BondKind, HeroId)> {
    let shown: Vec<(BondKind, HeroId)> = heroes[id]
        .bonds
        .iter()
        .filter(|bond| content.bonds.shown[bond.kind.index()])
        .map(|bond| (bond.kind, bond.other))
        .collect();
    // SPEC-GAPS KG-1: "living first" with formation order inside each group.
    let living = shown.iter().filter(|(_, other)| heroes[*other].is_living());
    let gone = shown
        .iter()
        .filter(|(_, other)| !heroes[*other].is_living());
    living.chain(gone).copied().collect()
}

/// A bond's gendered title, by the other hero's pronoun where the kind has one.
pub fn bond_title(content: &Content, kind: BondKind, other: &Hero) -> String {
    match &content.bonds.gendered[kind.index()] {
        Some(forms) => forms[other.pronoun.index()].clone(),
        None => content.bonds.titles[kind.index()].clone(),
    }
}

fn bonds_section(content: &Content, heroes: &[Hero], id: HeroId, out: &mut Vec<Line>) {
    let words = &content.words;
    out.push(line(Ink::Heading, &words[W::SheetBonds]));
    let bonds = shown_bonds(content, heroes, id);
    if bonds.is_empty() {
        out.push(line(Ink::Body, &words[W::SheetNoBonds]));
    }
    for (kind, other) in bonds.iter().take(BONDS_SHOWN) {
        let other = &heroes[*other];
        let title = bond_title(content, *kind, other);
        out.push(if other.is_living() {
            let power = signed(crate::constants::bond_power(*kind));
            line(Ink::Body, format!("{title} {} {power}", other.name))
        } else {
            line(
                Ink::Gone,
                fmt(&words[W::SheetBondGone], &[&title, &other.name]),
            )
        });
    }
    if bonds.len() > BONDS_SHOWN {
        let more = (bonds.len() - BONDS_SHOWN).to_string();
        out.push(line(Ink::Note, fmt(&words[W::SheetMoreBonds], &[&more])));
    }
}
