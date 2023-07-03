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
// use serde::{Deserialize, Serialize};

use core::alloc::Allocator;
use alloc::alloc::Global;

use super::{HasPrivilege, Label};

pub mod clause;
pub mod component;

pub use clause::*;
pub use component::*;

pub type Principal<A> = Vec<u8, A>;

#[derive(Debug, Clone)]
pub struct Buckle2<A: Allocator + Clone = Global> {
    pub secrecy: Component<A>,
    pub integrity: Component<A>,
    alloc: A,
}

impl<A: Allocator + Clone> PartialEq for Buckle2<A> {
    fn eq(&self, other: &Self) -> bool {
        self.secrecy.eq(&other.secrecy) && self.integrity.eq(&other.integrity)
    }
}

impl Buckle2 {
    pub fn parse(input: &str) -> Result<Buckle2, ()> {
        Self::parse_in(input, Global)
    }
}

impl<A: Allocator + Clone> Buckle2<A> {
    /// Parses a string into a DCLabel.
    ///
    /// principles with '/'. The backslash character ('\') allows escaping these
    /// special characters (including itself).
    pub fn parse_in(input: &str, alloc: A) -> Result<Buckle2<A>, ()> {
        let mut s = input.split(',');
        match (s.next(), s.next(), s.next()) {
            (Some(s), Some(i), None) => Ok(Buckle2 {
                    secrecy: Self::parse_component(s, alloc.clone()),
                    integrity: Self::parse_component(i, alloc.clone()),
                    alloc,
            }),
            _ => Err(()),
        }
    }

    fn parse_component(input: &str, alloc: A) -> Component<A> {
        use alloc::collections::BTreeSet;

        if let Some(_) = input.find('T') {
            Component::dc_true_in(alloc)
        } else if let Some(_) =  input.find('F') {
            Component::dc_false()
        } else {
            let mut formula = BTreeSet::new_in(alloc.clone());
            let alloc_dup = alloc.clone();
            input.split('&')
                .for_each(|t| {
                    let mut clause_vec = Vec::new_in(alloc_dup.clone());
                    t.split('|').for_each(|t| {
                        let mut clause_inner = Vec::new_in(alloc_dup.clone());
                        t.split('/').for_each(|t| {
                            clause_inner.push(t.as_bytes().to_vec_in(alloc_dup.clone()))
                        });
                        clause_vec.push(clause_inner)
                    });
                    formula.insert(Clause::new_from_vec_in(clause_vec, alloc_dup.clone()));
                });
            Component::DCFormula(formula, alloc)
        }
    }
}

#[cfg(test)]
impl Arbitrary for Buckle2 {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Buckle2 {
            secrecy: Component::arbitrary(g),
            integrity: Component::arbitrary(g),
            alloc: Global,
        }
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(
            (self.secrecy.clone(), self.integrity.clone())
                .shrink()
                .map(|(secrecy, integrity)| Buckle2 { secrecy, integrity, alloc: Global }),
        )
    }
}

impl Buckle2 {
    pub fn new<S: Into<Component>, I: Into<Component>>(secrecy: S, integrity: I) -> Buckle2 {
        Self::new_in(secrecy, integrity, Global)
    }

    pub fn public() -> Buckle2 {
        Self::public_in(Global)
    }

    pub fn top() -> Buckle2 {
        Self::top_in(Global)
    }

    pub fn bottom() -> Buckle2 {
        Self::bottom_in(Global)
    }
}

impl<A: Allocator + Clone> Buckle2<A> {
    pub fn new_in<S: Into<Component<A>>, I: Into<Component<A>>>(secrecy: S, integrity: I, alloc: A) -> Buckle2<A> {
        let mut secrecy = secrecy.into();
        let mut integrity = integrity.into();
        secrecy.reduce();
        integrity.reduce();
        Buckle2 { secrecy, integrity, alloc }
    }

    pub fn public_in(alloc: A) -> Buckle2<A> {
        Self::new_in(Component::dc_true_in(alloc.clone()), Component::dc_true_in(alloc.clone()), alloc)
    }

    pub fn top_in(alloc: A) -> Buckle2<A> {
        Self::new_in(Component::dc_false(), Component::dc_true_in(alloc.clone()), alloc)
    }

    pub fn bottom_in(alloc: A) -> Buckle2<A> {
        Self::new_in(Component::dc_true_in(alloc.clone()), Component::dc_false(), alloc)
    }

    pub fn reduce(&mut self) {
        self.secrecy.reduce();
        self.integrity.reduce();
    }

    pub fn endorse(mut self, privilege: &Component<A>) -> Buckle2<A> {
        self.integrity = self.integrity & privilege.clone();
        self
    }
}

impl<A: Allocator + Clone> Label for Buckle2<A> {
    fn lub(self, rhs: Self) -> Self {
        let mut res = Buckle2 {
            secrecy: self.secrecy & rhs.secrecy,
            integrity: self.integrity | rhs.integrity,
            alloc: self.alloc,
        };
        res.reduce();
        res
    }

    fn glb(self, rhs: Self) -> Self {
        let mut res = Buckle2 {
            secrecy: self.secrecy | rhs.secrecy,
            integrity: self.integrity & rhs.integrity,
            alloc: self.alloc
        };
        res.reduce();
        res
    }

    fn can_flow_to(&self, rhs: &Self) -> bool {
        rhs.secrecy.implies(&self.secrecy) && self.integrity.implies(&rhs.integrity)
    }
}

impl<A: Allocator + Clone> HasPrivilege for Buckle2<A> {
    type Privilege = Component<A>;

    fn downgrade(mut self, privilege: &Component<A>) -> Buckle2<A> {
        self.secrecy = match (self.secrecy, privilege) {
            //not real (DCTrue, _) => DCTrue, // can't go lower than true
            (_, Component::DCFalse) => Component::dc_true_in(self.alloc.clone()), // false can downgrade _anything_ to true
            (Component::DCFalse, _) => Component::dc_false(), // only false can downgrade false
            (Component::DCFormula(mut sec, a), Component::DCFormula(p, _)) => {
                sec.retain(|c| !p.iter().any(|pclause| pclause.implies(c)));
                Component::DCFormula(sec, a)
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

    fn can_flow_to_with_privilege(&self, rhs: &Self, privilege: &Component<A>) -> bool {
        (rhs.secrecy.clone() & privilege.clone()).implies(&self.secrecy)
            && (self.integrity.clone() & privilege.clone()).implies(&rhs.integrity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::alloc::Global;

    #[test]
    fn test_can_flow_to_with_privilege() {
        let privilege = &Component::formula([["go_grader"]], Global);
        // declassification
        assert_eq!(
            true,
            Buckle2::new([["go_grader"]], [["go_grader"]])
                .can_flow_to_with_privilege(&Buckle2::new(true, [["go_grader"]]), privilege)
        );

        assert_eq!(
            true,
            Buckle2::new([["go_grader"], ["bob"]], [["go_grader"]])
                .can_flow_to_with_privilege(&Buckle2::new([["bob"]], [["go_grader"]]), privilege)
        );

        assert_eq!(
            true,
            Buckle2::new([vec!["go_grader", "staff"], vec!["bob"]], [["go_grader"]])
                .can_flow_to_with_privilege(&Buckle2::new([["bob"]], [["go_grader"]]), privilege)
        );

        assert_eq!(
            true,
            Buckle2::new([vec!["go_grader", "staff"], vec!["bob"]], [["go_grader"]])
                .can_flow_to_with_privilege(&Buckle2::new([["bob"]], [["go_grader"]]), privilege)
        );

        assert_eq!(
            true,
            Buckle2::new(
                [
                    vec!["go_grader", "staff"],
                    vec!["go_grader", "alice"],
                    vec!["bob"]
                ],
                [["go_grader"]]
            )
            .can_flow_to_with_privilege(&Buckle2::new([["bob"]], [["go_grader"]]), privilege)
        );

        assert_eq!(
            true,
            Buckle2::new(
                [
                    vec!["go_grader", "staff"],
                    vec!["go_grader", "alice"],
                    vec!["bob"]
                ],
                [["go_grader"]]
            )
            .can_flow_to_with_privilege(&Buckle2::new([["bob"]], [["go_grader"]]), privilege)
        );

        // banned declassification
        assert_eq!(
            false,
            Buckle2::new([["go_grader"], ["staff"], ["bob"]], [["go_grader"]])
                .can_flow_to_with_privilege(&Buckle2::new([["bob"]], [["go_grader"]]), privilege)
        );

        // endorse
        assert_eq!(
            true,
            Buckle2::new([["bob"]], true)
                .can_flow_to_with_privilege(&Buckle2::new([["bob"]], [["go_grader"]]), privilege)
        );
    }

    #[test]
    fn test_downgrade() {
        // True can't downgrade anything
        assert_eq!(
            Buckle2::new(true, true),
            Buckle2::new(true, true).downgrade(&true.into())
        );
        assert_eq!(
            Buckle2::new(false, true),
            Buckle2::new(false, true).downgrade(&true.into())
        );
        assert_eq!(
            Buckle2::new(true, false),
            Buckle2::new(true, false).downgrade(&true.into())
        );
        assert_eq!(
            Buckle2::new([["amit"]], false),
            Buckle2::new([["amit"]], false).downgrade(&true.into())
        );
        assert_eq!(
            Buckle2::new(false, [["amit"]]),
            Buckle2::new(false, [["amit"]]).downgrade(&true.into())
        );

        // False downgrades everything
        assert_eq!(
            Buckle2::new(true, false),
            Buckle2::new(true, true).downgrade(&false.into())
        );
        assert_eq!(
            Buckle2::new(true, false),
            Buckle2::new(false, true).downgrade(&false.into())
        );
        assert_eq!(
            Buckle2::new(true, false),
            Buckle2::new(true, false).downgrade(&false.into())
        );
        assert_eq!(
            Buckle2::new(true, false),
            Buckle2::new([["amit"]], false).downgrade(&false.into())
        );
        assert_eq!(
            Buckle2::new(true, false),
            Buckle2::new(false, [["amit"]]).downgrade(&false.into())
        );
    }

    #[test]
    fn test_extreme_can_flow_to() {
        assert_eq!(true, Buckle2::bottom().can_flow_to(&Buckle2::top()));
        assert_eq!(true, Buckle2::bottom().can_flow_to(&Buckle2::public()));
        assert_eq!(true, Buckle2::public().can_flow_to(&Buckle2::top()));

        assert_eq!(false, Buckle2::top().can_flow_to(&Buckle2::bottom()));
        assert_eq!(false, Buckle2::top().can_flow_to(&Buckle2::public()));
        assert_eq!(false, Buckle2::public().can_flow_to(&Buckle2::bottom()));
    }

    #[test]
    fn test_basic_can_flow_to_integrity() {
        assert_eq!(
            true,
            Buckle2::new(true, [["Amit"]]).can_flow_to(&Buckle2::public())
        );

        assert_eq!(
            true,
            Buckle2::new(true, [["Amit", "Yue"]]).can_flow_to(&Buckle2::public())
        );

        assert_eq!(
            true,
            Buckle2::new(true, [["Amit"], ["Yue"]]).can_flow_to(&Buckle2::new(true, [["Amit"]]))
        );

        assert_eq!(
            true,
            Buckle2::new(true, [["Amit"], ["Yue"]])
                .can_flow_to(&Buckle2::new(true, [["Amit", "Yue"]]))
        );

        assert_eq!(
            false,
            Buckle2::new(true, [["Amit", "Yue"]])
                .can_flow_to(&Buckle2::new(true, [["Amit"], ["Yue"]]))
        );
    }

    #[test]
    fn test_basic_can_flow_to_secrecy() {
        assert_eq!(
            false,
            Buckle2::new([["Amit"]], true).can_flow_to(&Buckle2::public())
        );

        assert_eq!(
            false,
            Buckle2::new([["Amit", "Yue"]], true).can_flow_to(&Buckle2::public())
        );

        assert_eq!(
            false,
            Buckle2::new([["Amit"], ["Yue"]], true).can_flow_to(&Buckle2::new([["Amit"]], true))
        );

        assert_eq!(
            false,
            Buckle2::new([["Amit"], ["Yue"]], true).can_flow_to(&Buckle2::new([["Amit"]], true))
        );

        assert_eq!(
            false,
            Buckle2::new([["Amit"], ["Yue"]], true)
                .can_flow_to(&Buckle2::new([["Amit", "Yue"]], true))
        );

        assert_eq!(
            true,
            Buckle2::new([["Amit", "Yue"]], true)
                .can_flow_to(&Buckle2::new([["Amit"], ["Yue"]], true))
        );
    }

    #[test]
    fn test_lub() {
        assert_eq!(Buckle2::top(), Buckle2::public().lub(Buckle2::top()));
        assert_eq!(Buckle2::top(), Buckle2::top().lub(Buckle2::public()));
        assert_eq!(Buckle2::top(), Buckle2::bottom().lub(Buckle2::top()));
        assert_eq!(Buckle2::public(), Buckle2::bottom().lub(Buckle2::public()));

        assert_eq!(
            Buckle2::new([["Amit"], ["Yue"]], true),
            Buckle2::new([["Amit"]], true).lub(Buckle2::new([["Yue"]], true))
        );

        assert_eq!(
            Buckle2::new(true, [["Amit", "Yue"]]),
            Buckle2::new(true, [["Amit"]]).lub(Buckle2::new(true, [["Yue"]]))
        );
    }

    #[test]
    fn test_glb() {
        assert_eq!(Buckle2::public(), Buckle2::public().glb(Buckle2::top()));
        assert_eq!(Buckle2::public(), Buckle2::top().glb(Buckle2::public()));
        assert_eq!(Buckle2::bottom(), Buckle2::bottom().glb(Buckle2::top()));
        assert_eq!(Buckle2::bottom(), Buckle2::bottom().glb(Buckle2::public()));

        assert_eq!(
            Buckle2::new([["Amit", "Yue"]], true),
            Buckle2::new([["Amit"]], true).glb(Buckle2::new([["Yue"]], true))
        );

        assert_eq!(
            Buckle2::new(true, [["Amit"], ["Yue"]]),
            Buckle2::new(true, [["Amit"]]).glb(Buckle2::new(true, [["Yue"]]))
        );
    }

    #[test]
    fn test_parse() {
        assert_eq!(Buckle2::parse("T,T"), Ok(Buckle2::public()));
        assert_eq!(Buckle2::parse("T,F"), Ok(Buckle2::bottom()));
        assert_eq!(Buckle2::parse("F,T"), Ok(Buckle2::top()));
        assert_eq!(
            Buckle2::parse("Amit,Yue"),
            Ok(Buckle2::new([["Amit"]], [["Yue"]]))
        );
        assert_eq!(
            Buckle2::parse("Amit|Yue,Yue"),
            Ok(Buckle2::new([["Amit", "Yue"]], [["Yue"]]))
        );
        assert_eq!(
            Buckle2::parse("Amit&Yue,Yue"),
            Ok(Buckle2::new([["Amit"], ["Yue"]], [["Yue"]]))
        );
        assert_eq!(
            Buckle2::parse("Amit&Yue|Natalie|Gongqi&Deian,Yue"),
            Ok(Buckle2::new(
                [
                    Clause::from(["Amit"]),
                    Clause::from(["Yue", "Natalie", "Gongqi"]),
                    Clause::from(["Deian"])
                ],
                [["Yue"]]
            ))
        );
        // assert_eq!(
            // Buckle2::parse(r#"Am\&it&Yue,Y\|ue"#),
            // Ok(Buckle2::new([["Am&it"], ["Yue"]], [["Y|ue"]]))
        // );

        assert_eq!(
            Buckle2::parse("Amit/test,Amit"),
            Ok(Buckle2::new(
                Component::from([Clause::new_from_vec(vec![vec!["Amit", "test"]])]),
                [["Amit"]]
            ))
        )
    }

    quickcheck! {
        fn everything_can_flow_to_top(lbl: Buckle2) -> bool {
            let top = Buckle2::top();
            lbl.can_flow_to(&top)
        }

        fn bottom_can_flow_to_everything(lbl: Buckle2) -> bool {
            let bottom = Buckle2::bottom();
            bottom.can_flow_to(&lbl)
        }

        fn both_can_flow_to_lub(lbl1: Buckle2, lbl2: Buckle2) -> bool {
            let result = lbl1.clone().lub(lbl2.clone());
            lbl1.can_flow_to(&result) && lbl2.can_flow_to(&result)
        }

        fn glb_can_flow_to_both(lbl1: Buckle2, lbl2: Buckle2) -> bool {
            let result = lbl1.clone().glb(lbl2.clone());
            result.can_flow_to(&lbl1) && result.can_flow_to(&lbl2)
        }

        fn endorse_equiv_downgrade_to(lbl: Buckle2, privilege: Component) -> bool {
            let target = Buckle2 { secrecy: lbl.secrecy.clone(), integrity: lbl.integrity.clone() & privilege.clone(), alloc: Global };
            lbl.clone().downgrade_to(target, &privilege) == lbl.endorse(&privilege)
        }
    }
}
