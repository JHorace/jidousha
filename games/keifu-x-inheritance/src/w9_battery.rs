//! W9's property battery: the W8 whole-year battery's houses, watched for their epitaphs.
//!
//! The W8 battery (`w8_battery.rs`) plays 160 houses up to 25 years; this watcher is
//! handed each house at the founding, after each summer, after each turning and after its
//! heirs are chosen, with the heroes whose wordings that step rolled, in the order it
//! rolled them (the founding's Elsbeth then Aud; a summer's crowned in page and party order;
//! a turning's death pages in order). Over every hero it holds:
//!
//! - **the budget**: no epitaph has more than six "."s;
//! - **the naming rule**: no epitaph begins with the hero's own "He "/"She "/"His "/"Her ";
//!   the full name appears at most once, and no " he "/" she " of theirs comes before it;
//! - **the no-repeat frame**: in the order wordings are rolled across a house's whole run,
//!   no frame repeats the one before it, and the house's memory holds the last;
//! - **the roll points**: a wording appears exactly where the step says one was rolled —
//!   never on the living, never twice, never changed once rolled;
//! - **the recomposition triggers**: an epitaph changes only in a step that rolled its
//!   wording or moved its hero's bequest (the heir choice, a ghost laid, a ghost's dream
//!   taken up), and a move of an undone own dream's fate always shows in it — its DREAM
//!   sentence, which always fits the budget, is the one the fate's template words.
//!
//! The text the house holds is also asked to be the composition of the hero as they now
//! are, except where a TEACH_A_GREATER teacher's student has grown since: the one input a
//! non-trigger moves, which the spec does not recompose for (counted, and printed).
//!
//! INVARIANT: the fragments the triggers are read by are shipped literals copied by hand
//! from `content/epitaph.json`; the budget's 6 is a literal, not `EPITAPH_SENTENCES`.

use crate::content::Content;
use crate::epitaph::{Wording, compose};
use crate::hero::{DreamFate, Fate, Hero, HeroId};
use crate::house::House;
use crate::ids::{Destiny, Pronoun};

/// What one hero looked like at the last observation.
#[derive(Clone)]
struct Seen {
    full: String,
    wording: Option<Wording>,
    epitaph: Option<String>,
    bequest: (DreamFate, Option<HeroId>, bool, Option<i32>),
}

fn seen(hero: &Hero) -> Seen {
    Seen {
        full: hero.full_name(),
        wording: hero.wording,
        epitaph: hero.epitaph.clone(),
        bequest: (
            hero.dream_fate,
            hero.bequest_heir,
            hero.bequest_decided,
            hero.laid_year,
        ),
    }
}

/// The watcher: per house, what it saw last; over the battery, what it counted.
#[derive(Default)]
pub struct Watch {
    seed: u64,
    last: Vec<Seen>,
    frames: Vec<usize>,
    composed: usize,
    rolled: usize,
    rolled_frames: [usize; 3],
    recomposed: usize,
    by_trigger: [usize; 3],
    unnamed: usize,
    longest: usize,
    sentences: [usize; 7],
    greater_grown: usize,
    broken: Vec<String>,
}

impl Watch {
    fn require(&mut self, ok: bool, what: impl FnOnce() -> String) {
        if !ok && self.broken.len() < 12 {
            self.broken
                .push(format!("seed {:#x}: {}", self.seed, what()));
        }
    }

    /// A house just founded: its two dead founders rolled first, Elsbeth then Aud.
    pub fn found(&mut self, content: &Content, seed: u64, house: &House) {
        self.seed = seed;
        self.last = Vec::new();
        self.frames = Vec::new();
        let founders: Vec<HeroId> = ["Elsbeth", "Aud"]
            .iter()
            .filter_map(|name| house.heroes.iter().position(|h| h.name == *name))
            .collect();
        self.observe(content, house, &founders, "the founding");
    }

    /// After a step: `rolled` are the heroes whose wordings it rolled, in roll order.
    pub fn observe(&mut self, content: &Content, house: &House, rolled: &[HeroId], step: &str) {
        for &id in rolled {
            let fresh = self.last.get(id).is_none_or(|s| s.wording.is_none());
            match house.heroes[id].wording {
                Some(wording) if fresh => {
                    self.frames.push(wording.frame);
                    self.rolled += 1;
                    self.rolled_frames[wording.frame] += 1;
                }
                _ => self.require(false, || {
                    format!(
                        "{step}: {} was to roll a wording and did not",
                        house.heroes[id].name
                    )
                }),
            }
        }
        let repeats = self.frames.windows(2).filter(|w| w[0] == w[1]).count();
        let last = self.frames.last().copied();
        let frames = self.frames.clone();
        self.require(repeats == 0 && house.writing.last_frame == last, || {
            format!(
                "{step}: frames rolled {frames:?}, memory {:?}",
                house.writing.last_frame
            )
        });
        for id in 0..house.heroes.len() {
            let before = self.last.get(id).cloned();
            self.hero(content, house, id, before, rolled.contains(&id), step);
        }
        self.last = house.heroes.iter().map(seen).collect();
    }

    fn hero(
        &mut self,
        content: &Content,
        house: &House,
        id: HeroId,
        before: Option<Seen>,
        rolled: bool,
        step: &str,
    ) {
        let hero = &house.heroes[id];
        let name = &hero.name;
        let was = before.as_ref().and_then(|s| s.wording);
        if hero.is_living() {
            self.require(hero.wording.is_none() && hero.epitaph.is_none(), || {
                format!("{step}: {name}, living, has an epitaph")
            });
            return;
        }
        if hero.wording.is_some() && was.is_none() && !rolled {
            self.require(false, || {
                format!("{step}: {name} rolled a wording off a roll point")
            });
        }
        if was.is_some() && hero.wording != was {
            self.require(false, || format!("{step}: {name}'s wording changed"));
        }
        let (Some(wording), Some(epitaph)) = (hero.wording, &hero.epitaph) else {
            self.require(hero.wording.is_none() && hero.epitaph.is_none(), || {
                format!("{step}: {name} has a wording or an epitaph without the other")
            });
            return;
        };
        self.composed += 1;
        let count = epitaph.matches('.').count();
        self.longest = self.longest.max(count);
        self.sentences[count.min(6)] += 1;
        self.require(count <= 6, || {
            format!("{step}: {name}'s epitaph has {count} sentences")
        });
        self.naming(hero, epitaph, step);
        self.dream(house, hero, epitaph, step);
        // What the house holds is the composition of the hero as they are — but for a greater
        // student's growth since, which nothing recomposes for.
        let now = compose(content, &house.heroes, id, wording);
        if now != *epitaph {
            let grown = hero.destiny.kind == Destiny::TeachAGreater;
            self.greater_grown += usize::from(grown);
            self.require(grown, || {
                format!("{step}: {name}'s epitaph is not their composition: {epitaph:?} / {now:?}")
            });
        }
        // Recomposed exactly at a trigger.
        let Some(before) = before else {
            return;
        };
        let changed = before.epitaph.as_ref() != Some(epitaph);
        // Variant: an outsider who marries in takes the family name, and the dead's
        // epitaphs that name them are composed again (`winter::court`).
        let renamed = house
            .heroes
            .iter()
            .enumerate()
            .any(|(i, h)| self.last.get(i).is_some_and(|s| s.full != h.full_name()));
        let moved = before.bequest != seen(hero).bequest || renamed;
        if changed && !rolled {
            self.recomposed += 1;
            self.require(moved, || {
                format!("{step}: {name}'s epitaph changed with nothing of theirs moved")
            });
            let trigger = match step {
                "the heirs chosen" => 0,
                "a summer" => 1,
                _ => 2,
            };
            self.by_trigger[trigger] += 1;
        }
        let undone = hero.dream.as_ref().is_some_and(|d| !d.is_fulfilled());
        if before.bequest.0 != hero.dream_fate && undone && before.epitaph.is_some() {
            self.require(changed, || {
                format!("{step}: {name}'s dream fate moved and the epitaph did not follow")
            });
        }
    }

    /// The naming rule, read off the text.
    fn naming(&mut self, hero: &Hero, epitaph: &str, step: &str) {
        let (he, his, inside) = match hero.pronoun {
            Pronoun::He => ("He ", "His ", " he "),
            Pronoun::She => ("She ", "Her ", " she "),
        };
        let full = hero.full_name();
        let times = epitaph.matches(full.as_str()).count();
        let leading = epitaph.starts_with(he) || epitaph.starts_with(his);
        let ahead = epitaph
            .find(full.as_str())
            .is_some_and(|at| epitaph[..at].contains(inside));
        self.unnamed += usize::from(times == 0);
        self.require(!leading && times <= 1 && !ahead, || {
            format!(
                "{step}: {} is not named by the rule: {epitaph:?}",
                hero.name
            )
        });
    }

    /// The DREAM sentence the own dream's fate words (`epitaph.json`'s dream templates).
    fn dream(&mut self, house: &House, hero: &Hero, epitaph: &str, step: &str) {
        let Some(dream) = &hero.dream else {
            return;
        };
        let want: Vec<String> = if dream.is_fulfilled() {
            vec!["lived to see it done.".into(), ", and did it.".into()]
        } else {
            match (hero.dream_fate, hero.bequest_heir, hero.fate) {
                (DreamFate::PassedOn, Some(heir), _) => {
                    let heir = &house.heroes[heir].name;
                    vec![
                        format!(", was left to {heir}."),
                        format!(" {heir} carries it now."),
                    ]
                }
                (DreamFate::LeftToNoOne, ..) => {
                    vec![". It was left to no one, and it has not left the house.".into()]
                }
                (DreamFate::LaidToRest, ..) => vec!["when the house laid it to rest.".into()],
                (_, _, Fate::Departed) => vec![", and left it undone for a crown.".into()],
                _ => vec![", and died before it was done.".into()],
            }
        };
        self.require(want.iter().any(|w| epitaph.contains(w.as_str())), || {
            format!(
                "{step}: {}'s DREAM is not its fate's ({want:?}): {epitaph:?}",
                hero.name
            )
        });
    }

    /// The summary, and the checks' verdict.
    pub fn summary(&self, checks: &mut crate::checks::Checks) -> Vec<String> {
        checks.require(
            self.broken.is_empty(),
            "an epitaph broke the budget, the naming rule, the no-repeat frame or a trigger",
            format!("{:?}", self.broken),
        );
        checks.require(
            // 160 houses' two founders are 320; every death page and crowning adds one.
            self.rolled > 320 + 100 && self.by_trigger.iter().all(|&n| n > 0),
            "the epitaph watch met too few wordings or a trigger never fired",
            format!(
                "rolled {}, recomposed by trigger {:?}",
                self.rolled, self.by_trigger
            ),
        );
        vec![
            format!(
                "W9 epitaphs over the W8 battery: {} wordings rolled (frames {:?}), consecutive frames never equal, the memory always the last; {} epitaph readings, none over six sentences (by count 0..6: {:?}), every one named by the rule ({} with no pronoun to name)",
                self.rolled, self.rolled_frames, self.composed, self.sentences, self.unnamed
            ),
            format!(
                "W9 epitaphs, recomposition: {} recompositions, each at a trigger — the heir chosen {}, a ghost laid {}, a ghost's dream taken up {}; no wording ever re-rolled; a greater student grown since the epitaph {} times — {} broken",
                self.recomposed,
                self.by_trigger[0],
                self.by_trigger[1],
                self.by_trigger[2],
                self.greater_grown,
                self.broken.len()
            ),
        ]
    }
}
