//! Meters: a chip is a count of people, and every count opens into the
//! people it counted, each with the reason they count.
//!
//! Key types: `MeterSpec`. Key functions: `faces`, `count`, `meter_faults`.
//! Depends on: `floors` (for `Breach`).
//! INVARIANT: a count is the length of the list it opens into, by
//! construction — `count` is `faces(..).len()` and there is no second
//! derivation that could disagree. The question each row asks is the game's,
//! and the game owes the other half of the claim: that the set a chip names
//! is the set its simulation acts on (ADR-0046).

use crate::floors::Breach;

/// One registered aggregate: what it is called, what the chip says, which
/// picture it carries, and the question it asks of one person.
///
/// `Ask` is the game's own question type — in practice a function pointer
/// over the game's read-only view of its world, returning `Some(reason)` for
/// a person who counts. The kit does not call it; the game's one `faces`
/// wrapper does, so the kit never names the game's world.
///
/// ```
/// use jidousha_ui::{MeterSpec, count, faces};
///
/// struct Camp { wallets: Vec<i64> }
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Art { Coin, Heart }
///
/// const METERS: &[MeterSpec<Art, fn(&Camp, usize) -> Option<String>>] = &[
///     MeterSpec { id: "short", label: "short", icon: Art::Coin, asks: short },
///     MeterSpec { id: "idle", label: "idle", icon: Art::Heart, asks: idle },
/// ];
/// fn short(camp: &Camp, who: usize) -> Option<String> {
///     (camp.wallets[who] < 4).then(|| format!("holds {}g of the 4g due", camp.wallets[who]))
/// }
/// fn idle(_camp: &Camp, _who: usize) -> Option<String> {
///     Some("at home".to_owned())
/// }
///
/// let camp = Camp { wallets: vec![10, 2, 0] };
/// let roll = 0..camp.wallets.len();
/// let listed = faces(roll.clone(), |who| (METERS[0].asks)(&camp, who));
/// assert_eq!(listed, vec![(1, "holds 2g of the 4g due".to_owned()), (2, "holds 0g of the 4g due".to_owned())]);
/// assert_eq!(count(roll, |who| (METERS[0].asks)(&camp, who)), 2);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct MeterSpec<Icon, Ask> {
    /// The id a stamp and a report name it by. ASCII, lowercase, dashes.
    pub id: &'static str,
    /// What the chip says beside its count.
    pub label: &'static str,
    /// The chip's picture — a second channel beside the colour.
    pub icon: Icon,
    /// The question the chip asks of one person: `Some(reason)` when they
    /// count, and the reason is what the faces list shows beside them.
    pub asks: Ask,
}

/// The faces behind one chip: who counts, and why — everybody on `roll`
/// for whom `asks` has a reason, in roll order.
///
/// Over the game's roll rather than its registry, so a chip counts the
/// people who are *here* and not everybody who could ever be.
pub fn faces(
    roll: impl IntoIterator<Item = usize>,
    asks: impl Fn(usize) -> Option<String>,
) -> Vec<(usize, String)> {
    roll.into_iter()
        .filter_map(|who| asks(who).map(|reason| (who, reason)))
        .collect()
}

/// How many people a chip counts — the length of the list it opens into,
/// and nothing else.
pub fn count(
    roll: impl IntoIterator<Item = usize>,
    asks: impl Fn(usize) -> Option<String>,
) -> usize {
    faces(roll, asks).len()
}

/// What a meter table can be wrong about before any world is built: an id
/// that is not stamp-shaped, two rows sharing an id, two chips wearing one
/// picture.
///
/// ```
/// use jidousha_ui::{MeterSpec, meter_faults};
///
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Art { Coin }
/// let twice: &[MeterSpec<Art, ()>] = &[
///     MeterSpec { id: "short", label: "short", icon: Art::Coin, asks: () },
///     MeterSpec { id: "Short", label: "short", icon: Art::Coin, asks: () },
/// ];
/// let faults = meter_faults(twice);
/// assert_eq!(faults.len(), 3, "{faults:?}");
/// ```
pub fn meter_faults<I: PartialEq + std::fmt::Debug, A>(table: &[MeterSpec<I, A>]) -> Vec<Breach> {
    let mut out = Vec::new();
    for (index, spec) in table.iter().enumerate() {
        if spec.id.is_empty()
            || !spec
                .id
                .chars()
                .all(|glyph| glyph.is_ascii_lowercase() || glyph == '-')
        {
            out.push(Breach {
                what: "a meter id is not stamp-shaped ASCII",
                detail: format!("row {index} is named {:?}", spec.id),
            });
        }
        if table.iter().filter(|other| other.id == spec.id).count() != 1 {
            out.push(Breach {
                what: "two meters share an id",
                detail: format!("{:?} appears more than once", spec.id),
            });
        }
        if table.iter().filter(|other| other.icon == spec.icon).count() != 1 {
            out.push(Breach {
                what: "two meter chips carry the same icon",
                detail: format!(
                    "{:?} draws {:?}, and a chip's picture is how it is told from its neighbour",
                    spec.id, spec.icon
                ),
            });
        }
    }
    out
}
