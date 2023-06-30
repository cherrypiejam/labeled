//! Buckle is a hierarchical version of DCLabels
//!
//! Similar to DCLabels, Buckle labels are composed of a secrecy and integrity
//! components which are conjunctions of disjunctions of principals. However,
//! unlike DCLabels, Buckle principals are not strings, but rather ordered
//! lists, where prefixes imply longer lists.

// #[cfg(test)]
// use alloc::boxed::Box;
use alloc::vec::Vec;
// #[cfg(test)]
// use quickcheck::Arbitrary;
// use serde::{Deserialize, Serialize};

use core::alloc::Allocator;
use alloc::alloc::Global;

use super::{HasPrivilege, Label};

pub mod clause;
pub mod component;

pub use clause::*;
pub use component::*;

pub type Principal<A> = Vec<u8, A>;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Buckle2<A: Allocator + Clone = Global> {
    pub secrecy: Component<A>,
    pub integrity: Component<A>,
    alloc: A,
}

impl<A: Allocator + Clone> Buckle2<A> {
    /// Parses a string into a DCLabel.
    ///
    /// principles with '/'. The backslash character ('\') allows escaping these
    /// special characters (including itself).
    pub fn parse_in(input: &str, alloc: A) -> Option<Buckle2<A>> {
        let mut s = input.split(',');
        match (s.next(), s.next(), s.next()) {
            (Some(s), Some(i), None) => Some(Buckle2 {
                    secrecy: Self::parse_component(s, alloc.clone()),
                    integrity: Self::parse_component(i, alloc.clone()),
                    alloc,
            }),
            _ => None,
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

    // pub fn parser_in(input: &str, alloc: A) -> nom::IResult<&str, Buckle2<A>> {
        // use alloc::collections::BTreeSet;
        // use nom::{
            // bytes::complete::{escaped_transform, tag},
            // character::complete::{alphanumeric1, one_of},
            // multi::separated_list1,
            // sequence::tuple,
            // Parser,
        // };

        // let a = if let Some(_) = input.find('T') {
            // Component::dc_true_in(alloc)
        // } else if let Some(_) =  input.find('F') {
            // Component::dc_false()
        // } else {

        // };

        // // fn component(input: &str, alloc: A) -> nom::IResult<&str, Component<A>> {
            // // tag("T")
                // // .map(|_| Component::dc_true_in(alloc))
                // // .or(tag("F").map(|_| Component::dc_false()))
                // // .or(nom::combinator::map(
                    // // separated_list1(
                        // // tag("&"),
                        // // separated_list1(
                            // // tag("|"),
                            // // separated_list1(
                                // // tag("/"),
                                // // escaped_transform(alphanumeric1, '\\', one_of(r#",|&/\"#)),
                            // // ),
                        // // ),
                    // // ),
                    // // |mut c| {
                        // // Component::DCFormula(
                            // // c.iter_mut()
                                // // .map(|c| c.drain(..).collect::<BTreeSet<Vec<Principal<A>>>>().into())
                                // // .collect::<BTreeSet<Clause>>(),
                            // // alloc,
                        // // )
                    // // },
                // // ))
                // // .parse(input)
        // // }

        // let (input, (secrecy, _, integrity)) =
            // tuple((component, tag(","), component)).parse(input)?;

        // Ok((input, Buckle::new(secrecy, integrity)))
    // }
}

// #[cfg(test)]
// impl Arbitrary for Buckle {
    // fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        // Buckle {
            // secrecy: Component::arbitrary(g),
            // integrity: Component::arbitrary(g),
        // }
    // }

    // fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        // Box::new(
            // (self.secrecy.clone(), self.integrity.clone())
                // .shrink()
                // .map(|(secrecy, integrity)| Buckle { secrecy, integrity }),
        // )
    // }
// }

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

// #[cfg(test)]
// mod tests {
    // use super::*;
    // use alloc::vec;

    // #[test]
    // fn test_can_flow_to_with_privilege() {
        // let privilege = &Component::formula([["go_grader"]]);
        // // declassification
        // assert_eq!(
            // true,
            // Buckle::new([["go_grader"]], [["go_grader"]])
                // .can_flow_to_with_privilege(&Buckle::new(true, [["go_grader"]]), privilege)
        // );

        // assert_eq!(
            // true,
            // Buckle::new([["go_grader"], ["bob"]], [["go_grader"]])
                // .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        // );

        // assert_eq!(
            // true,
            // Buckle::new([vec!["go_grader", "staff"], vec!["bob"]], [["go_grader"]])
                // .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        // );

        // assert_eq!(
            // true,
            // Buckle::new([vec!["go_grader", "staff"], vec!["bob"]], [["go_grader"]])
                // .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        // );

        // assert_eq!(
            // true,
            // Buckle::new(
                // [
                    // vec!["go_grader", "staff"],
                    // vec!["go_grader", "alice"],
                    // vec!["bob"]
                // ],
                // [["go_grader"]]
            // )
            // .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        // );

        // assert_eq!(
            // true,
            // Buckle::new(
                // [
                    // vec!["go_grader", "staff"],
                    // vec!["go_grader", "alice"],
                    // vec!["bob"]
                // ],
                // [["go_grader"]]
            // )
            // .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        // );

        // // banned declassification
        // assert_eq!(
            // false,
            // Buckle::new([["go_grader"], ["staff"], ["bob"]], [["go_grader"]])
                // .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        // );

        // // endorse
        // assert_eq!(
            // true,
            // Buckle::new([["bob"]], true)
                // .can_flow_to_with_privilege(&Buckle::new([["bob"]], [["go_grader"]]), privilege)
        // );
    // }

    // #[test]
    // fn test_downgrade() {
        // // True can't downgrade anything
        // assert_eq!(
            // Buckle::new(true, true),
            // Buckle::new(true, true).downgrade(&true.into())
        // );
        // assert_eq!(
            // Buckle::new(false, true),
            // Buckle::new(false, true).downgrade(&true.into())
        // );
        // assert_eq!(
            // Buckle::new(true, false),
            // Buckle::new(true, false).downgrade(&true.into())
        // );
        // assert_eq!(
            // Buckle::new([["amit"]], false),
            // Buckle::new([["amit"]], false).downgrade(&true.into())
        // );
        // assert_eq!(
            // Buckle::new(false, [["amit"]]),
            // Buckle::new(false, [["amit"]]).downgrade(&true.into())
        // );

        // // False downgrades everything
        // assert_eq!(
            // Buckle::new(true, false),
            // Buckle::new(true, true).downgrade(&false.into())
        // );
        // assert_eq!(
            // Buckle::new(true, false),
            // Buckle::new(false, true).downgrade(&false.into())
        // );
        // assert_eq!(
            // Buckle::new(true, false),
            // Buckle::new(true, false).downgrade(&false.into())
        // );
        // assert_eq!(
            // Buckle::new(true, false),
            // Buckle::new([["amit"]], false).downgrade(&false.into())
        // );
        // assert_eq!(
            // Buckle::new(true, false),
            // Buckle::new(false, [["amit"]]).downgrade(&false.into())
        // );
    // }

    // #[test]
    // fn test_extreme_can_flow_to() {
        // assert_eq!(true, Buckle::bottom().can_flow_to(&Buckle::top()));
        // assert_eq!(true, Buckle::bottom().can_flow_to(&Buckle::public()));
        // assert_eq!(true, Buckle::public().can_flow_to(&Buckle::top()));

        // assert_eq!(false, Buckle::top().can_flow_to(&Buckle::bottom()));
        // assert_eq!(false, Buckle::top().can_flow_to(&Buckle::public()));
        // assert_eq!(false, Buckle::public().can_flow_to(&Buckle::bottom()));
    // }

    // #[test]
    // fn test_basic_can_flow_to_integrity() {
        // assert_eq!(
            // true,
            // Buckle::new(true, [["Amit"]]).can_flow_to(&Buckle::public())
        // );

        // assert_eq!(
            // true,
            // Buckle::new(true, [["Amit", "Yue"]]).can_flow_to(&Buckle::public())
        // );

        // assert_eq!(
            // true,
            // Buckle::new(true, [["Amit"], ["Yue"]]).can_flow_to(&Buckle::new(true, [["Amit"]]))
        // );

        // assert_eq!(
            // true,
            // Buckle::new(true, [["Amit"], ["Yue"]])
                // .can_flow_to(&Buckle::new(true, [["Amit", "Yue"]]))
        // );

        // assert_eq!(
            // false,
            // Buckle::new(true, [["Amit", "Yue"]])
                // .can_flow_to(&Buckle::new(true, [["Amit"], ["Yue"]]))
        // );
    // }

    // #[test]
    // fn test_basic_can_flow_to_secrecy() {
        // assert_eq!(
            // false,
            // Buckle::new([["Amit"]], true).can_flow_to(&Buckle::public())
        // );

        // assert_eq!(
            // false,
            // Buckle::new([["Amit", "Yue"]], true).can_flow_to(&Buckle::public())
        // );

        // assert_eq!(
            // false,
            // Buckle::new([["Amit"], ["Yue"]], true).can_flow_to(&Buckle::new([["Amit"]], true))
        // );

        // assert_eq!(
            // false,
            // Buckle::new([["Amit"], ["Yue"]], true).can_flow_to(&Buckle::new([["Amit"]], true))
        // );

        // assert_eq!(
            // false,
            // Buckle::new([["Amit"], ["Yue"]], true)
                // .can_flow_to(&Buckle::new([["Amit", "Yue"]], true))
        // );

        // assert_eq!(
            // true,
            // Buckle::new([["Amit", "Yue"]], true)
                // .can_flow_to(&Buckle::new([["Amit"], ["Yue"]], true))
        // );
    // }

    // #[test]
    // fn test_lub() {
        // assert_eq!(Buckle::top(), Buckle::public().lub(Buckle::top()));
        // assert_eq!(Buckle::top(), Buckle::top().lub(Buckle::public()));
        // assert_eq!(Buckle::top(), Buckle::bottom().lub(Buckle::top()));
        // assert_eq!(Buckle::public(), Buckle::bottom().lub(Buckle::public()));

        // assert_eq!(
            // Buckle::new([["Amit"], ["Yue"]], true),
            // Buckle::new([["Amit"]], true).lub(Buckle::new([["Yue"]], true))
        // );

        // assert_eq!(
            // Buckle::new(true, [["Amit", "Yue"]]),
            // Buckle::new(true, [["Amit"]]).lub(Buckle::new(true, [["Yue"]]))
        // );
    // }

    // #[test]
    // fn test_glb() {
        // assert_eq!(Buckle::public(), Buckle::public().glb(Buckle::top()));
        // assert_eq!(Buckle::public(), Buckle::top().glb(Buckle::public()));
        // assert_eq!(Buckle::bottom(), Buckle::bottom().glb(Buckle::top()));
        // assert_eq!(Buckle::bottom(), Buckle::bottom().glb(Buckle::public()));

        // assert_eq!(
            // Buckle::new([["Amit", "Yue"]], true),
            // Buckle::new([["Amit"]], true).glb(Buckle::new([["Yue"]], true))
        // );

        // assert_eq!(
            // Buckle::new(true, [["Amit"], ["Yue"]]),
            // Buckle::new(true, [["Amit"]]).glb(Buckle::new(true, [["Yue"]]))
        // );
    // }

    // #[test]
    // fn test_parse() {
        // assert_eq!(Buckle::parse("T,T"), Ok(Buckle::public()));
        // assert_eq!(Buckle::parse("T,F"), Ok(Buckle::bottom()));
        // assert_eq!(Buckle::parse("F,T"), Ok(Buckle::top()));
        // assert_eq!(
            // Buckle::parse("Amit,Yue"),
            // Ok(Buckle::new([["Amit"]], [["Yue"]]))
        // );
        // assert_eq!(
            // Buckle::parse("Amit|Yue,Yue"),
            // Ok(Buckle::new([["Amit", "Yue"]], [["Yue"]]))
        // );
        // assert_eq!(
            // Buckle::parse("Amit&Yue,Yue"),
            // Ok(Buckle::new([["Amit"], ["Yue"]], [["Yue"]]))
        // );
        // assert_eq!(
            // Buckle::parse("Amit&Yue|Natalie|Gongqi&Deian,Yue"),
            // Ok(Buckle::new(
                // [
                    // Clause::from(["Amit"]),
                    // Clause::from(["Yue", "Natalie", "Gongqi"]),
                    // Clause::from(["Deian"])
                // ],
                // [["Yue"]]
            // ))
        // );
        // assert_eq!(
            // Buckle::parse(r#"Am\&it&Yue,Y\|ue"#),
            // Ok(Buckle::new([["Am&it"], ["Yue"]], [["Y|ue"]]))
        // );

        // assert_eq!(
            // Buckle::parse("Amit/test,Amit"),
            // Ok(Buckle::new(
                // Component::from([Clause::new_from_vec(vec![vec!["Amit", "test"]])]),
                // [["Amit"]]
            // ))
        // )
    // }

    // quickcheck! {
        // fn everything_can_flow_to_top(lbl: Buckle) -> bool {
            // let top = Buckle::top();
            // lbl.can_flow_to(&top)
        // }

        // fn bottom_can_flow_to_everything(lbl: Buckle) -> bool {
            // let bottom = Buckle::bottom();
            // bottom.can_flow_to(&lbl)
        // }

        // fn both_can_flow_to_lub(lbl1: Buckle, lbl2: Buckle) -> bool {
            // let result = lbl1.clone().lub(lbl2.clone());
            // lbl1.can_flow_to(&result) && lbl2.can_flow_to(&result)
        // }

        // fn glb_can_flow_to_both(lbl1: Buckle, lbl2: Buckle) -> bool {
            // let result = lbl1.clone().glb(lbl2.clone());
            // result.can_flow_to(&lbl1) && result.can_flow_to(&lbl2)
        // }

        // fn endorse_equiv_downgrade_to(lbl: Buckle, privilege: Component) -> bool {
            // let target = Buckle { secrecy: lbl.secrecy.clone(), integrity: lbl.integrity.clone() & privilege.clone() };
            // lbl.clone().downgrade_to(target, &privilege) == lbl.endorse(&privilege)
        // }
    // }
// }
