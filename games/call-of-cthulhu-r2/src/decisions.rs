//! The handoff's two decision rows, each asserted the way its table says.
//!
//! Row 1 — which answer to give: the question, the options, sanity, the
//! being's temper and the known lore are on the call screen before answering;
//! then an answer on a fixed seed moves sanity and temper by exactly what
//! `rules::answer_outcome` returns — and by the shipped literal, so a check
//! that read the function back could not walk with a mutated constant.
//!
//! Row 2 — how to spend the morning: two runs that differ in one morning
//! choice; the stated effect is on the morning screen at choosing, and the
//! night's calls differ exactly as it said.

use jidousha::prelude::*;
use jidousha::ui::wrap;

use crate::checks::Checks;
use crate::conductor::{Conductor, Photo};
use crate::lore::{AnswerKind, Being};
use crate::play::Phase;
use crate::players::key_for;
use crate::rules::{self, MorningAction};
use crate::screens::{self, Pick};

/// The seed the decision rows are asserted on.
pub const SEED: u64 = 1890;

/// Base drains, as shipped: Dagon, Nyarlathotep, Yog-Sothoth. Literal, so a
/// mutated drain cannot carry the expectation with it.
const SHIPPED_DRAIN: [i32; 3] = [2, 3, 4];
/// What an insult costs over the drain, as shipped.
const SHIPPED_SHOCK: i32 = 6;
/// A call's opening interest, and what lore and wrong answers do to it.
const SHIPPED_START: i32 = 5;
const SHIPPED_LORE: i32 = -3;
const SHIPPED_WRONG: i32 = 1;

/// The screen position of `kind` among a question's answers.
fn position_of(order: [AnswerKind; 3], kind: AnswerKind) -> usize {
    order.iter().position(|each| *each == kind).unwrap_or(0)
}

/// Row 1. Returns the photo of the call screen, for the capture.
pub fn answer_row(checks: &mut Checks) -> Option<Photo> {
    let mut run = Conductor::new(SEED);
    let first = rules::tonight(&run.game().run).callers()[0];
    let options = rules::morning_options(&run.game().run);
    let Some(study) = options
        .iter()
        .position(|each| *each == MorningAction::Study(first))
    else {
        checks.require(
            false,
            "decision row 1: the first caller cannot be studied on day 1",
            format!("{options:?}"),
        );
        return None;
    };
    run.press(key_for(Pick::Morning(study)));
    let photo = run.photo();
    let game = &photo.game;
    let Phase::Call(call) = &game.phase else {
        checks.require(
            false,
            "decision row 1: choosing a morning did not ring the phone",
            format!("{:?}", game.phase),
        );
        return None;
    };
    let Some(asking) = game.asking() else {
        checks.require(
            false,
            "decision row 1: a call with no question",
            format!("{call:?}"),
        );
        return None;
    };
    let lore = call.being.lore();
    let fact = &lore.facts[asking.fact];
    // On screen before answering: the question, every option, sanity, temper,
    // and the known lore for this being.
    let voice = TextStyle {
        size: 22.0,
        ..TextStyle::default()
    };
    let question_rows: Vec<String> = wrap(
        fact.question,
        voice.columns_in(screens::DESIGN_W - 2.0 * screens::MARGIN),
    )
    .lines()
    .map(str::to_owned)
    .collect();
    let mut missing = Vec::new();
    for row in &question_rows {
        if !photo.shows(row) {
            missing.push(format!("question row {row:?}"));
        }
    }
    for (index, kind) in asking.order.iter().enumerate() {
        let text = fact.answers[position_of(AnswerKind::ALL, *kind)];
        let want = format!("{}  {text}", index + 1);
        if !photo.shows(&want) {
            missing.push(format!("option {want:?}"));
        }
    }
    let sanity = format!("SANITY {}", game.run.sanity);
    if !photo.shows(&sanity) {
        missing.push(sanity);
    }
    if !photo.mentions("temper 0") {
        missing.push("temper 0".to_owned());
    }
    if !photo.shows(fact.known) {
        missing.push(format!("known lore {:?}", fact.known));
    }
    checks.require(
        missing.is_empty() && asking.fact == 0 && game.run.knows(call.being, 0),
        "decision row 1: the call screen does not show what the answer turns on",
        format!(
            "{} is asking about fact {} (known: {}); missing from the screen: {missing:?}; screen: {:?}",
            lore.name,
            asking.fact,
            game.run.knows(call.being, asking.fact),
            photo.strings()
        ),
    );
    // Every option is hinted, from the one function, because the fact is known.
    for index in 0..3 {
        let Some((_, outcome)) = game.preview(index) else {
            continue;
        };
        let hint = screens::hint_line(outcome);
        checks.require(
            photo.shows(&hint),
            "decision row 1: a known question's option carries no hint",
            format!(
                "option {} should read {hint:?}; screen: {:?}",
                index + 1,
                photo.strings()
            ),
        );
    }
    // Answer with the insult: the dearest answer, and the one that moves temper.
    let being = call.being;
    let insult = position_of(asking.order, AnswerKind::Insult);
    let before_sanity = game.run.sanity;
    let before_temper = game.run.temper_of(being);
    let interest = call.interest;
    let outcome =
        rules::answer_outcome(being, AnswerKind::Insult, before_temper, game.run.composure);
    run.press(key_for(Pick::Answer(insult)));
    let after = run.game();
    let spent = before_sanity - after.run.sanity;
    let angered = after.run.temper_of(being) - before_temper;
    let shipped = SHIPPED_DRAIN[being.index()] + SHIPPED_SHOCK;
    let now_interest = match &after.phase {
        Phase::Call(call) => call.interest,
        _ => -99,
    };
    checks.require(
        spent == outcome.sanity_cost && angered == outcome.temper_change && spent == shipped && angered == 1
            && interest == SHIPPED_START && now_interest == SHIPPED_START,
        "decision row 1: an insult did not cost what answer_outcome says",
        format!(
            "{} at temper {before_temper}: sanity {before_sanity} -> {} (spent {spent}; the function says {}, \
             shipped {shipped}); temper +{angered} (function +{}, shipped +1); interest {interest} -> {now_interest} \
             (shipped {SHIPPED_START} -> {SHIPPED_START})",
            lore.name, after.run.sanity, outcome.sanity_cost, outcome.temper_change
        ),
    );
    // Then lore and wrong, against the shipped literals, at the new temper.
    for (kind, change) in [
        (AnswerKind::Lore, SHIPPED_LORE),
        (AnswerKind::Wrong, SHIPPED_WRONG),
    ] {
        let outcome = rules::answer_outcome(being, kind, 1, 0);
        checks.require(
            outcome.interest_change == change
                && outcome.sanity_cost == SHIPPED_DRAIN[being.index()] + 1
                && outcome.temper_change == 0,
            "decision row 1: an answer's outcome is not the shipped rule",
            format!(
                "{kind:?} to {} at temper 1: {outcome:?}; shipped interest {change:+}, cost {}",
                lore.name,
                SHIPPED_DRAIN[being.index()] + 1
            ),
        );
    }
    Some(photo)
}

/// Row 2. Two runs on one seed, differing only in the first morning.
pub fn morning_row(checks: &mut Checks) {
    let base = Conductor::new(SEED);
    let run = base.game().run.clone();
    let callers = rules::tonight(&run).callers();
    let first = callers[0];
    let name = rules::short_name(first);
    let options = rules::morning_options(&run);
    let index_of = |action| options.iter().position(|each| *each == action);
    let (Some(train), Some(disrupt), Some(study)) = (
        index_of(MorningAction::Train),
        index_of(MorningAction::Disrupt(first)),
        index_of(MorningAction::Study(first)),
    ) else {
        checks.require(
            false,
            "decision row 2: day 1 lacks the options it should have",
            format!("{options:?}"),
        );
        return;
    };
    // The stated effects, as literal sentences, on the screen at choosing.
    let mut looking = Conductor::new(SEED);
    let morning = looking.photo();
    let learned = first.lore().facts[0].title;
    let want_disrupt = format!("{name} will not call tonight; temper 0->1");
    let want_study = format!("learn {learned}; it calls tonight, asks this first");
    let tonight_line = format!(
        "TONIGHT, AS THINGS STAND: {}",
        crate::play::callers_line(&callers)
    );
    for (what, want) in [
        ("disrupt", &want_disrupt),
        ("study", &want_study),
        ("tonight", &tonight_line),
    ] {
        checks.require(
            morning.shows(want),
            "decision row 2: a morning option's effect on tonight is not on the screen",
            format!(
                "the {what} row should read {want:?}; the screen reads {:?}",
                morning.strings()
            ),
        );
    }
    // Run A disrupts the first caller, run B trains, run C studies it.
    let mut a = Conductor::new(SEED);
    a.press(key_for(Pick::Morning(disrupt)));
    let mut b = Conductor::new(SEED);
    b.press(key_for(Pick::Morning(train)));
    let mut c = Conductor::new(SEED);
    c.press(key_for(Pick::Morning(study)));
    let night_a = a.game().night.callers();
    let night_b = b.game().night.callers();
    let without: Vec<Being> = night_b
        .iter()
        .copied()
        .filter(|each| *each != first)
        .collect();
    let ringing = |conductor: &Conductor| match &conductor.game().phase {
        Phase::Call(call) => Some(call.being),
        _ => None,
    };
    checks.require(
        night_a == without && night_b == callers && ringing(&a) != Some(first) && ringing(&b) == Some(first)
            && a.game().run.temper_of(first) == 1 && b.game().run.temper_of(first) == 0,
        "decision row 2: disrupting a cult did not change tonight as the morning said",
        format!(
            "the screen said {want_disrupt:?}; after disrupting, tonight is {night_a:?} and {:?} is ringing \
             (temper {}); after training, {night_b:?} with {:?} ringing (temper {})",
            ringing(&a), a.game().run.temper_of(first), ringing(&b), b.game().run.temper_of(first)
        ),
    );
    let first_question = c.game().asking().map(|asking| asking.fact);
    let photo = c.photo();
    let question = first.lore().facts[0].question;
    checks.require(
        ringing(&c) == Some(first) && first_question == Some(0) && photo.mentions(&question[..question.len().min(12)]),
        "decision row 2: studying a being did not make it ask that fact first",
        format!(
            "the screen said {want_study:?}; {:?} is ringing and asking fact {first_question:?}; screen {:?}",
            ringing(&c), photo.strings()
        ),
    );
}
