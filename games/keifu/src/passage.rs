//! The turning's pages (SPEC §3.1 `passage`, §18 step 1): what the winter's passing
//! wrote.
//!
//! W8 SCAFFOLD: the turning of the year is W8's. Until it lands a passage holds only
//! the winter page — "The winter of year N" with the winter's lines, or "A quiet
//! winter." — and the turning screen (`turning_view.rs`) shows it with "Summer comes".
//! Nobody ages, dies of age, is born, comes of age or arrives; `House::mourned` keeps
//! the summer's dead for W8's death pages. W8 adds its pages after the winter's.

use crate::winter::WinterPlan;

/// This turning's pages, from the winter's passing until summer comes.
#[derive(Clone, Debug, PartialEq)]
pub struct Passage {
    /// The year whose winter it tells.
    pub year: i32,
    /// The winter page's lines (SPEC §11.3), in the order the winter wrote them.
    pub winter: Vec<String>,
    /// What each winter seat did, for the checks (the previews were drawn from the same).
    pub done: WinterPlan,
}
