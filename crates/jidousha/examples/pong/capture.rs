//! placeholder
use crate::checks::Checks;
use jidousha::testing::{BackendTextureId, FrameRecord};

/// placeholder
pub(crate) fn capture_a_frame(
    _checks: &mut Checks,
    _frame: &FrameRecord,
    _font: BackendTextureId,
) -> String {
    "not yet".to_owned()
}
