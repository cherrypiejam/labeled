use super::Label;

pub mod clause;
pub mod component;

pub use clause::*;
pub use component::*;

pub type Principal = alloc::string::String;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct DCLabel {
    pub secrecy: Component,
    pub integrity: Component,
}

impl DCLabel {
    pub fn new<S: Into<Component>, I: Into<Component>>(secrecy: S, integrity: I) -> DCLabel {
        let mut secrecy = secrecy.into();
        let mut integrity = integrity.into();
        secrecy.reduce();
        integrity.reduce();
        DCLabel { secrecy, integrity }
    }

    pub fn public() -> DCLabel {
        Self::new(Component::dc_true(), Component::dc_true())
    }

    pub fn top() -> DCLabel {
        Self::new(Component::dc_false(), Component::dc_true())
    }

    pub fn bottom() -> DCLabel {
        Self::new(Component::dc_true(), Component::dc_false())
    }

    pub fn reduce(&mut self) {
        self.secrecy.reduce();
        self.integrity.reduce();
    }
}

impl Label for DCLabel {
    fn lub(self, rhs: Self) -> Self {
        let mut res = DCLabel {
            secrecy: self.secrecy & rhs.secrecy,
            integrity: self.integrity | rhs.integrity,
        };
        res.reduce();
        res
    }

    fn glb(self, rhs: Self) -> Self {
        let mut res = DCLabel {
            secrecy: self.secrecy | rhs.secrecy,
            integrity: self.integrity & rhs.integrity,
        };
        res.reduce();
        res
    }

    fn can_flow_to(&self, rhs: &Self) -> bool {
        rhs.secrecy.implies(&self.secrecy) && self.integrity.implies(&rhs.integrity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extreme_can_flow_to() {
        assert_eq!(true, DCLabel::bottom().can_flow_to(&DCLabel::top()));
        assert_eq!(true, DCLabel::bottom().can_flow_to(&DCLabel::public()));
        assert_eq!(true, DCLabel::public().can_flow_to(&DCLabel::top()));

        assert_eq!(false, DCLabel::top().can_flow_to(&DCLabel::bottom()));
        assert_eq!(false, DCLabel::top().can_flow_to(&DCLabel::public()));
        assert_eq!(false, DCLabel::public().can_flow_to(&DCLabel::bottom()));
    }

    #[test]
    fn test_basic_can_flow_to_integrity() {
        assert_eq!(
            true,
            DCLabel::new(true, [["Amit"]])
                .can_flow_to(&DCLabel::public())
        );

        assert_eq!(
            true,
            DCLabel::new(true, [["Amit", "Yue"]])
                .can_flow_to(&DCLabel::public())
        );

        assert_eq!(
            true,
            DCLabel::new(true, [["Amit"], ["Yue"]])
                .can_flow_to(&DCLabel::new(true, [["Amit"]]))
        );

        assert_eq!(
            true,
            DCLabel::new(true, [["Amit"], ["Yue"]])
                .can_flow_to(&DCLabel::new(true, [["Amit", "Yue"]]))
        );

        assert_eq!(
            false,
            DCLabel::new(true, [["Amit", "Yue"]])
                .can_flow_to(&DCLabel::new(true, [["Amit"], ["Yue"]]))
        );
    }

    #[test]
    fn test_basic_can_flow_to_secrecy() {
        assert_eq!(
            false,
            DCLabel::new([["Amit"]], true)
                .can_flow_to(&DCLabel::public())
        );

        assert_eq!(
            false,
            DCLabel::new([["Amit", "Yue"]], true)
                .can_flow_to(&DCLabel::public())
        );

        assert_eq!(
            false,
            DCLabel::new([["Amit"], ["Yue"]], true)
                .can_flow_to(&DCLabel::new([["Amit"]], true))
        );

        assert_eq!(
            false,
            DCLabel::new([["Amit"], ["Yue"]], true)
                .can_flow_to(&DCLabel::new([["Amit"]], true))
        );

        assert_eq!(
            false,
            DCLabel::new([["Amit"], ["Yue"]], true)
                .can_flow_to(&DCLabel::new([["Amit", "Yue"]], true))
        );

        assert_eq!(
            true,
            DCLabel::new([["Amit", "Yue"]], true)
                .can_flow_to(&DCLabel::new([["Amit"], ["Yue"]], true))
        );
    }

    #[test]
    fn test_lub() {
        assert_eq!(DCLabel::top(), DCLabel::public().lub(DCLabel::top()));
        assert_eq!(DCLabel::top(), DCLabel::top().lub(DCLabel::public()));
        assert_eq!(DCLabel::top(), DCLabel::bottom().lub(DCLabel::top()));
        assert_eq!(DCLabel::public(), DCLabel::bottom().lub(DCLabel::public()));

        assert_eq!(DCLabel::new([["Amit"], ["Yue"]], true),
            DCLabel::new([["Amit"]], true).lub(DCLabel::new([["Yue"]], true)));

        assert_eq!(DCLabel::new(true, [["Amit", "Yue"]]),
            DCLabel::new(true, [["Amit"]]).lub(DCLabel::new(true, [["Yue"]])));
    }

    #[test]
    fn test_glb() {
        assert_eq!(DCLabel::public(), DCLabel::public().glb(DCLabel::top()));
        assert_eq!(DCLabel::public(), DCLabel::top().glb(DCLabel::public()));
        assert_eq!(DCLabel::bottom(), DCLabel::bottom().glb(DCLabel::top()));
        assert_eq!(DCLabel::bottom(), DCLabel::bottom().glb(DCLabel::public()));

        assert_eq!(DCLabel::new([["Amit", "Yue"]], true),
            DCLabel::new([["Amit"]], true).glb(DCLabel::new([["Yue"]], true)));

        assert_eq!(DCLabel::new(true, [["Amit"], ["Yue"]]),
            DCLabel::new(true, [["Amit"]]).glb(DCLabel::new(true, [["Yue"]])));
    }
}
