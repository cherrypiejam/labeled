//! Buckle is a hierarchical version of DCLabels
//!
//! Similar to DCLabels, Buckle labels are composed of a secrecy and integrity
//! components which are conjunctions of disjunctions of principals. However,
//! unlike DCLabels, Buckle principals are not strings, but rather ordered
//! lists, where prefixes imply longer lists.

#[cfg(test)]
use alloc::boxed::Box;
use alloc::vec::Vec;
#[cfg(test)]
use quickcheck::Arbitrary;
use serde::{Deserialize, Serialize};

use super::{HasPrivilege, Label};

pub mod clause;
pub mod component;

pub use clause::*;
pub use component::*;

pub type Principal = alloc::string::String;

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Buckle {
    pub secrecy: Component,
    pub integrity: Component,
}

impl Buckle {
    /// Parses a string into a DCLabel.
    ///
    /// The string separates secrecy and integrity with a comma, clauses
    /// separated with a '&' and principle vectors with a '|', and delegated
    /// principles with '/'. The backslash character ('\') allows escaping these
    /// special characters (including itself).
    pub fn parse(input: &str) -> Result<Buckle, nom::Err<nom::error::Error<&str>>> {
        Self::parser(input).map(|r| r.1)
    }

    pub fn parser(input: &str) -> nom::IResult<&str, Buckle> {
        use alloc::collections::BTreeSet;
        use nom::{
            bytes::complete::{escaped_transform, tag},
            character::complete::{alphanumeric1, one_of},
            multi::separated_list1,
            sequence::tuple,
            Parser,
        };

        fn component(input: &str) -> nom::IResult<&str, Component> {
            tag("T")
                .map(|_| Component::dc_true())
                .or(tag("F").map(|_| Component::dc_false()))
                .or(nom::combinator::map(
                    separated_list1(
                        tag("&"),
                        separated_list1(
                            tag("|"),
                            separated_list1(
                                tag("/"),
                                escaped_transform(alphanumeric1, '\\', one_of(r#",|&/\"#)),
                            ),
                        ),
                    ),
                    |mut c| {
                        Component::DCFormula(
                            c.iter_mut()
                                .map(|c| c.drain(..).collect::<BTreeSet<Vec<Principal>>>().into())
                                .collect::<BTreeSet<Clause>>(),
                        )
                    },
                ))
                .parse(input)
        }

        let (input, (secrecy, _, integrity)) =
            tuple((component, tag(","), component)).parse(input)?;

        Ok((input, Buckle::new(secrecy, integrity)))
    }
}

#[cfg(test)]
impl Arbitrary for Buckle {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Buckle {
            secrecy: Component::arbitrary(g),
            integrity: Component::arbitrary(g),
        }
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(
            (self.secrecy.clone(), self.integrity.clone())
                .shrink()
                .map(|(secrecy, integrity)| Buckle { secrecy, integrity }),
        )
    }
}

impl Buckle {
    pub fn new<S: Into<Component>, I: Into<Component>>(secrecy: S, integrity: I) -> Buckle {
        let mut secrecy = secrecy.into();
        let mut integrity = integrity.into();
        secrecy.reduce();
        integrity.reduce();
        Buckle { secrecy, integrity }
    }

    pub fn public() -> Buckle {
        Self::new(Component::dc_true(), Component::dc_true())
    }

    pub fn top() -> Buckle {
        Self::new(Component::dc_false(), Component::dc_true())
    }

    pub fn bottom() -> Buckle {
        Self::new(Component::dc_true(), Component::dc_false())
    }

    pub fn reduce(&mut self) {
        self.secrecy.reduce();
        self.integrity.reduce();
    }

    pub fn endorse(mut self, privilege: &Component) -> Buckle {
        self.integrity = privilege.clone() & self.integrity;
        self
    }
}

impl Label for Buckle {
    fn lub(self, rhs: Self) -> Self {
        let mut res = Buckle {
            secrecy: self.secrecy & rhs.secrecy,
            integrity: self.integrity | rhs.integrity,
        };
        res.reduce();
        res
    }

    fn glb(self, rhs: Self) -> Self {
        let mut res = Buckle {
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

impl HasPrivilege for Buckle {
    type Privilege = Component;

    fn downgrade(mut self, privilege: &Component) -> Buckle {
        self.secrecy = match (self.secrecy, privilege) {
            //not real (DCTrue, _) => DCTrue, // can't go lower than true
            (_, Component::DCFalse) => Component::dc_true(), // false can downgrade _anything_ to true
            (Component::DCFalse, _) => Component::dc_false(), // only false can downgrade false
            (Component::DCFormula(mut sec), Component::DCFormula(p)) => {
                sec.retain(|c| !p.iter().any(|pclause| pclause.implies(c)));
                Component::DCFormula(sec)
            }
        };
        self.integrity = privilege.clone() & self.integrity;
        self
    }

    fn downgrade_to(self, target: Self, privilege: &Self::Privilege) -> Self {
        if self.can_flow_to_with_privilege(&target, privilege) {
            return target;
        } else {
            return self;
        }
    }

    fn can_flow_to_with_privilege(&self, rhs: &Self, privilege: &Component) -> bool {
        (rhs.secrecy.clone() & privilege.clone()).implies(&self.secrecy)
            && (self.integrity.clone() & privilege.clone()).implies(&rhs.integrity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_can_flow_to_with_privilege() {
        let privilege = &Component::formula([["go_grader"]]);
        // declassification
        assert_eq!(
            true,
            Buckle::new([["go_grader"]], [["go_grader"]])
                .can_flow_to_with_privilege(&Buckle::new(true, [["go_grader"]]), privilege)
        );

        assert_eq!(
            true,
            Buckle::new([["go_grader"], ["bob"]], [["go_grader"]])
                .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        );

        assert_eq!(
            true,
            Buckle::new([vec!["go_grader", "staff"], vec!["bob"]], [["go_grader"]])
                .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        );

        assert_eq!(
            true,
            Buckle::new([vec!["go_grader", "staff"], vec!["bob"]], [["go_grader"]])
                .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        );

        assert_eq!(
            true,
            Buckle::new(
                [
                    vec!["go_grader", "staff"],
                    vec!["go_grader", "alice"],
                    vec!["bob"]
                ],
                [["go_grader"]]
            )
            .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        );

        assert_eq!(
            true,
            Buckle::new(
                [
                    vec!["go_grader", "staff"],
                    vec!["go_grader", "alice"],
                    vec!["bob"]
                ],
                [["go_grader"]]
            )
            .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        );

        // banned declassification
        assert_eq!(
            false,
            Buckle::new([["go_grader"], ["staff"], ["bob"]], [["go_grader"]])
                .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        );

        // endorse
        assert_eq!(
            true,
            Buckle::new([["bob"]], true)
                .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        );
    }

    #[test]
    fn test_downgrade() {
        // True can't downgrade anything
        assert_eq!(
            Buckle::new(true, true),
            Buckle::new(true, true).downgrade(&true.into())
        );
        assert_eq!(
            Buckle::new(false, true),
            Buckle::new(false, true).downgrade(&true.into())
        );
        assert_eq!(
            Buckle::new(true, false),
            Buckle::new(true, false).downgrade(&true.into())
        );
        assert_eq!(
            Buckle::new([["amit"]], false),
            Buckle::new([["amit"]], false).downgrade(&true.into())
        );
        assert_eq!(
            Buckle::new(false, [["amit"]]),
            Buckle::new(false, [["amit"]]).downgrade(&true.into())
        );

        // False downgrades everything
        assert_eq!(
            Buckle::new(true, false),
            Buckle::new(true, true).downgrade(&false.into())
        );
        assert_eq!(
            Buckle::new(true, false),
            Buckle::new(false, true).downgrade(&false.into())
        );
        assert_eq!(
            Buckle::new(true, false),
            Buckle::new(true, false).downgrade(&false.into())
        );
        assert_eq!(
            Buckle::new(true, false),
            Buckle::new([["amit"]], false).downgrade(&false.into())
        );
        assert_eq!(
            Buckle::new(true, false),
            Buckle::new(false, [["amit"]]).downgrade(&false.into())
        );
    }

    #[test]
    fn test_extreme_can_flow_to() {
        assert_eq!(true, Buckle::bottom().can_flow_to(&Buckle::top()));
        assert_eq!(true, Buckle::bottom().can_flow_to(&Buckle::public()));
        assert_eq!(true, Buckle::public().can_flow_to(&Buckle::top()));

        assert_eq!(false, Buckle::top().can_flow_to(&Buckle::bottom()));
        assert_eq!(false, Buckle::top().can_flow_to(&Buckle::public()));
        assert_eq!(false, Buckle::public().can_flow_to(&Buckle::bottom()));
    }

    #[test]
    fn test_basic_can_flow_to_integrity() {
        assert_eq!(
            true,
            Buckle::new(true, [["Amit"]]).can_flow_to(&Buckle::public())
        );

        assert_eq!(
            true,
            Buckle::new(true, [["Amit", "Yue"]]).can_flow_to(&Buckle::public())
        );

        assert_eq!(
            true,
            Buckle::new(true, [["Amit"], ["Yue"]]).can_flow_to(&Buckle::new(true, [["Amit"]]))
        );

        assert_eq!(
            true,
            Buckle::new(true, [["Amit"], ["Yue"]])
                .can_flow_to(&Buckle::new(true, [["Amit", "Yue"]]))
        );

        assert_eq!(
            false,
            Buckle::new(true, [["Amit", "Yue"]])
                .can_flow_to(&Buckle::new(true, [["Amit"], ["Yue"]]))
        );
    }

    #[test]
    fn test_basic_can_flow_to_secrecy() {
        assert_eq!(
            false,
            Buckle::new([["Amit"]], true).can_flow_to(&Buckle::public())
        );

        assert_eq!(
            false,
            Buckle::new([["Amit", "Yue"]], true).can_flow_to(&Buckle::public())
        );

        assert_eq!(
            false,
            Buckle::new([["Amit"], ["Yue"]], true).can_flow_to(&Buckle::new([["Amit"]], true))
        );

        assert_eq!(
            false,
            Buckle::new([["Amit"], ["Yue"]], true).can_flow_to(&Buckle::new([["Amit"]], true))
        );

        assert_eq!(
            false,
            Buckle::new([["Amit"], ["Yue"]], true)
                .can_flow_to(&Buckle::new([["Amit", "Yue"]], true))
        );

        assert_eq!(
            true,
            Buckle::new([["Amit", "Yue"]], true)
                .can_flow_to(&Buckle::new([["Amit"], ["Yue"]], true))
        );
    }

    #[test]
    fn test_lub() {
        assert_eq!(Buckle::top(), Buckle::public().lub(Buckle::top()));
        assert_eq!(Buckle::top(), Buckle::top().lub(Buckle::public()));
        assert_eq!(Buckle::top(), Buckle::bottom().lub(Buckle::top()));
        assert_eq!(Buckle::public(), Buckle::bottom().lub(Buckle::public()));

        assert_eq!(
            Buckle::new([["Amit"], ["Yue"]], true),
            Buckle::new([["Amit"]], true).lub(Buckle::new([["Yue"]], true))
        );

        assert_eq!(
            Buckle::new(true, [["Amit", "Yue"]]),
            Buckle::new(true, [["Amit"]]).lub(Buckle::new(true, [["Yue"]]))
        );
    }

    #[test]
    fn test_glb() {
        assert_eq!(Buckle::public(), Buckle::public().glb(Buckle::top()));
        assert_eq!(Buckle::public(), Buckle::top().glb(Buckle::public()));
        assert_eq!(Buckle::bottom(), Buckle::bottom().glb(Buckle::top()));
        assert_eq!(Buckle::bottom(), Buckle::bottom().glb(Buckle::public()));

        assert_eq!(
            Buckle::new([["Amit", "Yue"]], true),
            Buckle::new([["Amit"]], true).glb(Buckle::new([["Yue"]], true))
        );

        assert_eq!(
            Buckle::new(true, [["Amit"], ["Yue"]]),
            Buckle::new(true, [["Amit"]]).glb(Buckle::new(true, [["Yue"]]))
        );
    }

    #[test]
    fn test_parse() {
        assert_eq!(Buckle::parse("T,T"), Ok(Buckle::public()));
        assert_eq!(Buckle::parse("T,F"), Ok(Buckle::bottom()));
        assert_eq!(Buckle::parse("F,T"), Ok(Buckle::top()));
        assert_eq!(
            Buckle::parse("Amit,Yue"),
            Ok(Buckle::new([["Amit"]], [["Yue"]]))
        );
        assert_eq!(
            Buckle::parse("Amit|Yue,Yue"),
            Ok(Buckle::new([["Amit", "Yue"]], [["Yue"]]))
        );
        assert_eq!(
            Buckle::parse("Amit&Yue,Yue"),
            Ok(Buckle::new([["Amit"], ["Yue"]], [["Yue"]]))
        );
        assert_eq!(
            Buckle::parse("Amit&Yue|Natalie|Gongqi&Deian,Yue"),
            Ok(Buckle::new(
                [
                    Clause::from(["Amit"]),
                    Clause::from(["Yue", "Natalie", "Gongqi"]),
                    Clause::from(["Deian"])
                ],
                [["Yue"]]
            ))
        );
        assert_eq!(
            Buckle::parse(r#"Am\&it&Yue,Y\|ue"#),
            Ok(Buckle::new([["Am&it"], ["Yue"]], [["Y|ue"]]))
        );

        assert_eq!(
            Buckle::parse("Amit/test,Amit"),
            Ok(Buckle::new(
                Component::from([Clause::new_from_vec(vec![vec!["Amit", "test"]])]),
                [["Amit"]]
            ))
        )
    }

    quickcheck! {
        fn everything_can_flow_to_top(lbl: Buckle) -> bool {
            let top = Buckle::top();
            lbl.can_flow_to(&top)
        }

        fn bottom_can_flow_to_everything(lbl: Buckle) -> bool {
            let bottom = Buckle::bottom();
            bottom.can_flow_to(&lbl)
        }

        fn both_can_flow_to_lub(lbl1: Buckle, lbl2: Buckle) -> bool {
            let result = lbl1.clone().lub(lbl2.clone());
            lbl1.can_flow_to(&result) && lbl2.can_flow_to(&result)
        }

        fn glb_can_flow_to_both(lbl1: Buckle, lbl2: Buckle) -> bool {
            let result = lbl1.clone().glb(lbl2.clone());
            result.can_flow_to(&lbl1) && result.can_flow_to(&lbl2)
        }

        fn endorse_equiv_downgrade_to(lbl: Buckle, privilege: Component) -> bool {
            let target = Buckle { secrecy: lbl.secrecy.clone(), integrity: lbl.integrity.clone() & privilege.clone() };
            lbl.clone().downgrade_to(target, &privilege) == lbl.endorse(&privilege)
        }
    }
}
