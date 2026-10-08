//! Epitaphs (SPEC §20, `lineage/epitaph.jai`): a wording rolled once per hero, nine parts
//! chosen by their rules, a priority walk under a six-sentence budget, the frame's order,
//! and the subject named in the first part emitted.
//!
//! A wording is a frame (0..2, never the previous frame rolled anywhere in the run — the
//! house's `WritingMemory` keeps it) and nine fair coins, one per part in `Part` order:
//! a part with two variants takes `_1` on a 1 and `_0` on a 0. The wording is kept on
//! the hero, so every recomposition reads the same one (§15.2 "with the same wording");
//! only a new roll changes it.
//!
//! The nine parts' rules are `epitaph_parts.rs` (ORIGIN to DREAM) and `epitaph_ends.rs`
//! (PROPHECY, LOVE, END, LEFT). Every word comes from `epitaph.json` by key
//! (`epitaph_lore::E`), or from the content the template's `args` name: the lore's
//! pronouns, places, tags and text conventions, a dream's told title, a hero's fate
//! telling, an heirloom's or a legacy's name.
//!
//! The composition points (§20 "When composed") are the callers': the founding's two dead
//! (`House::found`), a crowning (`harm::crown`), the death page (`death_page`), the heir
//! choice (`heirs::choose`), a ghost laid (`ghost::lay_ghost`) and a ghost's dream taken
//! up at a coming of age (`coming_of_age`). The Ending's are W10's.

use jidousha::prelude::Rng;

use crate::chance::{fresh_index, index};
use crate::constants::{EPITAPH_FRAME_COUNT, EPITAPH_SENTENCES};
use crate::content::Content;
use crate::epitaph_parts::parts;
use crate::hero::{Hero, HeroId};
use crate::ids::Part;
use crate::text::{WritingMemory, capitalized};

/// An epitaph's wording (SPEC §20 "Wording"): its frame and one coin per part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wording {
    /// 0..=2: which of `epitaph.json`'s frames orders the parts.
    pub frame: usize,
    /// One fair coin per part, by `Part::index`: `true` takes a part's `_1` variant.
    pub coins: [bool; 9],
}

impl Wording {
    /// A part's coin.
    pub fn coin(&self, part: Part) -> bool {
        self.coins[part.index()]
    }
}

/// Roll a wording (SPEC §20, CONSTANTS §13): the frame uniform over the three, never the
/// frame rolled last anywhere in the run (the first is uniform over all), then nine coins.
/// SPEC-GAPS KG-58: the frame first, then one fair coin per part in `parts` order.
pub fn roll_wording(writing: &mut WritingMemory, rng: &mut Rng) -> Wording {
    let frame = match writing.last_frame {
        None => index(rng, EPITAPH_FRAME_COUNT),
        Some(previous) => fresh_index(rng, EPITAPH_FRAME_COUNT, previous),
    };
    writing.last_frame = Some(frame);
    let mut coins = [false; 9];
    for coin in &mut coins {
        *coin = index(rng, 2) == 1;
    }
    Wording { frame, coins }
}

/// Roll `id`'s wording and compose their epitaph: the founding's, a crowning's, a death
/// page's (SPEC §20 "When composed").
pub fn remember(
    content: &Content,
    heroes: &mut [Hero],
    writing: &mut WritingMemory,
    id: HeroId,
    rng: &mut Rng,
) {
    heroes[id].wording = Some(roll_wording(writing, rng));
    recompose(content, heroes, id);
}

/// Compose `id`'s epitaph again with the wording they already have (SPEC §15.2, §14.4,
/// §9.5): after the heir choice, a ghost laid, a ghost's dream taken up.
pub fn recompose(content: &Content, heroes: &mut [Hero], id: HeroId) {
    let Some(wording) = heroes[id].wording else {
        panic!(
            "[keifu_x_inheritance_r2] {}'s epitaph is recomposed and no wording was ever rolled for them\n  \
             likely cause: a recomposition point reached before the hero's death page or \
             crowning\n  fix: SPEC §20 rolls a wording before the first composition",
            heroes[id].name
        );
    };
    let epitaph = compose(content, heroes, id, wording);
    heroes[id].epitaph = Some(epitaph);
}

/// The number of sentences in a part: its "." characters (CONSTANTS §13).
pub fn sentences(part: &str) -> usize {
    part.matches('.').count()
}

/// The epitaph for `id` under `wording` (SPEC §20 "Composition"): walk the priority
/// order, choosing each non-empty part that still fits the budget (a part that does not
/// fit is skipped and later ones are still considered); emit the chosen in the frame's
/// order, joined by single spaces; name the subject in the first.
pub fn compose(content: &Content, heroes: &[Hero], id: HeroId, wording: Wording) -> String {
    let lore = &content.epitaph;
    let parts = parts(content, heroes, id, wording);
    let mut chosen = [false; 9];
    let mut total = 0;
    for part in lore.priorities {
        let text = &parts[part.index()];
        if text.is_empty() {
            continue;
        }
        let count = sentences(text);
        if total + count <= EPITAPH_SENTENCES {
            chosen[part.index()] = true;
            total += count;
        }
    }
    let mut emitted: Vec<String> = lore.frames[wording.frame]
        .iter()
        .filter(|part| chosen[part.index()])
        .map(|part| parts[part.index()].clone())
        .collect();
    if let Some(first) = emitted.first_mut() {
        *first = name_the_subject(content, &heroes[id], first);
    }
    emitted.join(" ")
}

/// Name the subject (SPEC §20): a leading "He "/"She " becomes the full name; a leading
/// "His "/"Her " becomes "<full name>'s"; otherwise the first " he "/" she " inside
/// becomes " <full name> "; otherwise the part is unchanged. The forms are the hero's
/// own pronoun's, from the lore — the only pronoun of the hero a part can carry.
/// SPEC-GAPS KG-60: the inside search is the lower-case form, case-sensitive, once.
pub fn name_the_subject(content: &Content, hero: &Hero, part: &str) -> String {
    let forms = &content.lore.pronouns[hero.pronoun.index()];
    let full = hero.full_name();
    let he = format!("{} ", capitalized(&forms.subject));
    let his = format!("{} ", capitalized(&forms.possessive));
    let inside = format!(" {} ", forms.subject);
    if let Some(rest) = part.strip_prefix(&he) {
        format!("{full} {rest}")
    } else if let Some(rest) = part.strip_prefix(&his) {
        format!("{full}'s {rest}")
    } else if part.contains(&inside) {
        part.replacen(&inside, &format!(" {full} "), 1)
    } else {
        part.to_owned()
    }
}
