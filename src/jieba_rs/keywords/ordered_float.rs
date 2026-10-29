//! Minimal private f64 wrapper for TF-IDF and TextRank heap comparisons, avoiding
//! an external `ordered-float` dependency for this small use case.
//!
//! Supports Debug, Clone, Copy, Eq/PartialEq, Ord/PartialOrd, tuple construction,
//! and `into_inner`. All NaNs compare equal and above every non-NaN; signed zeros
//! compare equal; other values use numeric order, including infinities. This is
//! not the bitwise ordering of `f64::total_cmp`.
//!
//! This is not a general replacement for upstream `ordered-float`: no generic
//! float support, Hash, arithmetic, conversions, NotNan, or optional integrations.
//! Upstream attribution: <https://github.com/reem/rust-ordered-float> (MIT;
//! see its LICENSE-MIT). The local source's exact version/commit and whether it
//! was copied or independently implemented are unverified. Preserve upstream
//! copyright/permission notices if copied code is identified; see VENDOR_NOTES.md.
use std::cmp::Ordering;
#[derive(Debug, Clone, Copy)]
pub(super) struct OrderedFloat(pub f64);
impl PartialEq for OrderedFloat { fn eq(&self, rhs: &Self) -> bool { self.cmp(rhs) == Ordering::Equal } }
impl Eq for OrderedFloat {}
impl PartialOrd for OrderedFloat { fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> { Some(self.cmp(rhs)) } }
impl Ord for OrderedFloat {
    fn cmp(&self, rhs: &Self) -> Ordering {
        match (self.0.is_nan(), rhs.0.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => self.0.partial_cmp(&rhs.0).expect("non-NaN f64 values are comparable"),
        }
    }
}

impl OrderedFloat {
    #[inline]
    pub const fn into_inner(self) -> f64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::OrderedFloat as F;
    #[test] fn semantics() {
        assert_eq!(F(-0.0), F(0.0));
        assert_eq!(F(f64::NAN), F(-f64::NAN));
        assert!(F(f64::NAN) > F(f64::INFINITY));
        assert!(F(f64::NEG_INFINITY) < F(-1.0));
    }
}
