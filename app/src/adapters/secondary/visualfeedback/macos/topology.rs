use objc2_foundation::NSRect;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ScreenTopology {
    pub(super) frames: Vec<NSRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScreenChange {
    Identical,
    Differs,
}

impl ScreenTopology {
    pub(super) fn diff(&self, other: &Self) -> ScreenChange {
        if self.frames.len() != other.frames.len() {
            return ScreenChange::Differs;
        }
        for (a, b) in self.frames.iter().zip(other.frames.iter()) {
            if !rect_eq(a, b) {
                return ScreenChange::Differs;
            }
        }
        ScreenChange::Identical
    }
}

fn rect_eq(a: &NSRect, b: &NSRect) -> bool {
    a.origin.x.to_bits() == b.origin.x.to_bits()
        && a.origin.y.to_bits() == b.origin.y.to_bits()
        && a.size.width.to_bits() == b.size.width.to_bits()
        && a.size.height.to_bits() == b.size.height.to_bits()
}

pub(super) fn diff_screens(old: &ScreenTopology, new: &ScreenTopology) -> ScreenChange {
    old.diff(new)
}

#[cfg(test)]
mod tests {
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    use super::{rect_eq, ScreenChange, ScreenTopology};

    fn make_rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
        NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
    }

    fn topology(rects: Vec<NSRect>) -> ScreenTopology {
        ScreenTopology { frames: rects }
    }

    #[test]
    fn should_report_identical_for_two_empty_topologies() {
        let old = topology(vec![]);
        let new = topology(vec![]);
        assert_eq!(old.diff(&new), ScreenChange::Identical);
    }

    #[test]
    fn should_report_identical_for_same_single_screen() {
        let rect = make_rect(0.0, 0.0, 1920.0, 1080.0);
        let old = topology(vec![rect]);
        let new = topology(vec![make_rect(0.0, 0.0, 1920.0, 1080.0)]);
        assert_eq!(old.diff(&new), ScreenChange::Identical);
    }

    #[test]
    fn should_report_identical_for_two_matching_multi_screen_topologies() {
        let old = topology(vec![
            make_rect(0.0, 0.0, 1920.0, 1080.0),
            make_rect(1920.0, 0.0, 2560.0, 1440.0),
        ]);
        let new = topology(vec![
            make_rect(0.0, 0.0, 1920.0, 1080.0),
            make_rect(1920.0, 0.0, 2560.0, 1440.0),
        ]);
        assert_eq!(old.diff(&new), ScreenChange::Identical);
    }

    #[test]
    fn should_report_differs_when_lengths_mismatch() {
        let old = topology(vec![make_rect(0.0, 0.0, 1920.0, 1080.0)]);
        let new = topology(vec![
            make_rect(0.0, 0.0, 1920.0, 1080.0),
            make_rect(1920.0, 0.0, 2560.0, 1440.0),
        ]);
        assert_eq!(old.diff(&new), ScreenChange::Differs);
    }

    #[test]
    fn should_report_differs_when_one_frame_moved() {
        let old = topology(vec![make_rect(0.0, 0.0, 1920.0, 1080.0)]);
        let new = topology(vec![make_rect(10.0, 0.0, 1920.0, 1080.0)]);
        assert_eq!(old.diff(&new), ScreenChange::Differs);
    }

    #[test]
    fn should_report_differs_when_one_frame_resized() {
        let old = topology(vec![make_rect(0.0, 0.0, 1920.0, 1080.0)]);
        let new = topology(vec![make_rect(0.0, 0.0, 2560.0, 1440.0)]);
        assert_eq!(old.diff(&new), ScreenChange::Differs);
    }

    #[test]
    fn should_report_differs_for_empty_vs_nonempty() {
        let old = topology(vec![]);
        let new = topology(vec![make_rect(0.0, 0.0, 1920.0, 1080.0)]);
        assert_eq!(old.diff(&new), ScreenChange::Differs);
    }

    #[test]
    fn should_report_identical_for_nan_with_same_bit_pattern() {
        let nan = f64::NAN;
        let a = NSRect::new(NSPoint::new(nan, 0.0), NSSize::new(1920.0, 1080.0));
        let b = NSRect::new(NSPoint::new(nan, 0.0), NSSize::new(1920.0, 1080.0));
        assert!(rect_eq(&a, &b));
        let old = topology(vec![a]);
        let new = topology(vec![b]);
        assert_eq!(old.diff(&new), ScreenChange::Identical);
    }

    #[test]
    fn should_report_differs_for_nan_with_different_bit_patterns() {
        let nan1 = f64::from_bits(0x7ff8_0000_0000_0001);
        let nan2 = f64::from_bits(0x7ff8_0000_0000_0002);
        let a = NSRect::new(NSPoint::new(nan1, 0.0), NSSize::new(1920.0, 1080.0));
        let b = NSRect::new(NSPoint::new(nan2, 0.0), NSSize::new(1920.0, 1080.0));
        assert!(!rect_eq(&a, &b));
        let old = topology(vec![a]);
        let new = topology(vec![b]);
        assert_eq!(old.diff(&new), ScreenChange::Differs);
    }
}
