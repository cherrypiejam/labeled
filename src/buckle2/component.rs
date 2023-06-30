// #[cfg(test)]
// use alloc::boxed::Box;
// #[cfg(test)]
// use quickcheck::{empty_shrinker, Arbitrary};
// use serde::{Deserialize, Serialize};

use super::clause::Clause;
use alloc::collections::BTreeSet;

use core::alloc::Allocator;
use alloc::alloc::Global;

// #[derive(Debug, Clone)]
// pub enum Component<A: Allocator + Clone = Global> {
    // DCFalse,
    // DCFormula(BTreeSet<Clause<A>, A>),
// }

// TODO: Alternatively
#[derive(Debug, Clone)]
pub enum Component<A: Allocator + Clone = Global> {
    DCFalse,
    DCFormula(BTreeSet<Clause<A>, A>, A),
}
//

impl<A: Allocator + Clone> PartialEq for Component<A> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Component::DCFormula(e1, _), Component::DCFormula(e2, _)) => e1.eq(&e2),
            (Component::DCFalse, Component::DCFalse) => true,
            _ => false,
        }
    }
}

impl<A: Allocator + Clone> Eq for Component<A> {}


// #[cfg(test)]
// impl Arbitrary for Component {
    // fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        // if !bool::arbitrary(g) {
            // Component::DCFalse
        // } else {
            // Component::DCFormula(BTreeSet::arbitrary(g))
        // }
    // }

    // fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        // match self {
            // Component::DCFalse => empty_shrinker(),
            // Component::DCFormula(clauses) => Box::new(clauses.shrink().map(Component::DCFormula)),
        // }
    // }
// }

impl Component {
    pub fn dc_true() -> Self {
        Component::dc_true_in(Global)
    }
}

impl<A: Allocator + Clone> Component<A> {
    pub fn formula<C: Into<Clause<A>> + Clone, const N: usize>(clauses: [C; N], alloc: A) -> Component<A> {
        let mut result = BTreeSet::new_in(alloc.clone());
        for c in clauses.iter() {
            result.insert(c.clone().into());
        }
        Component::DCFormula(result, alloc)
    }

    pub fn dc_false() -> Self {
        Component::DCFalse
    }

    pub fn dc_true_in(alloc: A) -> Self {
        Component::DCFormula(BTreeSet::new_in(alloc.clone()), alloc)
    }

    pub fn is_false(&self) -> bool {
        match self {
            Component::DCFalse => true,
            _ => false,
        }
    }

    pub fn is_true(&self) -> bool {
        match self {
            Component::DCFalse => false,
            Component::DCFormula(o, _) => o.is_empty(),
        }
    }

    pub fn implies(&self, other: &Self) -> bool {
        match (self, other) {
            (Component::DCFalse, _) => true,
            (_, Component::DCFalse) => false,
            (_, o) if o.is_true() => true,
            (s, _) if s.is_true() => false,
            (Component::DCFormula(s, _), Component::DCFormula(o, _)) => {
                // for all clauses in other there must be at least one in self that implies it
                o.iter()
                    .all(|oclause| s.iter().any(|sclause| sclause.implies(oclause)))
            }
        }
    }

    pub fn reduce(&mut self) {
        match self {
            Component::DCFalse => {}
            Component::DCFormula(clauses, a) => {
                let mut rmlist = BTreeSet::new_in(a.clone());
                for (i, clausef) in clauses.iter().enumerate() {
                    for clauser in clauses.iter().skip(i + 1) {
                        if clausef.implies(clauser) {
                            rmlist.insert(clauser.clone());
                        } else if clauser.implies(clausef) {
                            rmlist.insert(clausef.clone());
                        }
                    }
                }
                for rmclause in rmlist.iter() {
                    clauses.remove(rmclause);
                }
            }
        }
    }
}

// impl<C: Into<Clause> + Clone, const N: usize> From<[C; N]> for Component {
    // fn from(clauses: [C; N]) -> Component {
        // Component::formula(clauses)
    // }
// }

// impl From<bool> for Component {
    // fn from(clause: bool) -> Component {
        // if clause {
            // Component::dc_true()
        // } else {
            // Component::dc_false()
        // }
    // }
// }

// impl From<BTreeSet<Clause>> for Component {
    // fn from(clauses: BTreeSet<Clause>) -> Component {
        // Component::DCFormula(clauses)
    // }
// }

impl<A: Allocator + Clone> core::ops::BitAnd for Component<A> {
    type Output = Component<A>;
    fn bitand(self, rhs: Self) -> Component<A> {
        match (self, rhs) {
            (Component::DCFalse, _) => Component::DCFalse,
            (_, Component::DCFalse) => Component::DCFalse,
            (Component::DCFormula(mut s, a), Component::DCFormula(mut o, _)) => {
                s.append(&mut o);
                Component::DCFormula(s, a)
            }
        }
    }
}

impl<A: Allocator + Clone> core::ops::BitOr for Component<A> {
    type Output = Component<A>;
    fn bitor(self, rhs: Self) -> Component<A> {
        match (self, rhs) {
            (s, Component::DCFalse) => s,
            (Component::DCFalse, o) => o,
            (Component::DCFormula(s, a), Component::DCFormula(o, _)) if s.is_empty() || o.is_empty() => {
                Component::dc_true_in(a)
            }
            (Component::DCFormula(s, a), Component::DCFormula(o, _)) => {
                let mut result = BTreeSet::new_in(a.clone());
                for mut clauses in s.iter().cloned() {
                    for mut clauseo in o.iter().cloned() {
                        clauses.0.append(&mut clauseo.0);
                    }
                    result.insert(clauses);
                }
                Component::DCFormula(result, a)
            }
        }
    }
}

// #[cfg(test)]
// mod tests {
    // use super::*;

    // #[test]
    // fn test_x_implies_x() {
        // assert!(Component::from(false).implies(&Component::from(false)));
        // assert!(Component::from(true).implies(&Component::from(true)));
        // assert!(Component::from([["Amit"]]).implies(&Component::from([["Amit"]])));
    // }

    // #[test]
    // fn test_true_not_implies_not_true() {
        // assert_eq!(
            // false,
            // Component::dc_true().implies(&Component::from([["Amit"]]))
        // );
    // }

    // #[test]
    // fn test_nothing_implies_false() {
        // assert_eq!(false, Component::dc_true().implies(&Component::dc_false()));
    // }

    // #[test]
    // fn test_false_implies_everything() {
        // assert!(Component::dc_false().implies(&Component::dc_false()));
        // assert!(Component::dc_false().implies(&Component::dc_true()));
        // assert!(Component::dc_false().implies(&Component::from([["Amit"]])));
    // }

    // #[test]
    // fn test_everything_implies_true() {
        // assert!(Component::dc_false().implies(&Component::dc_true()));
        // assert!(Component::from([["Amit"]]).implies(&Component::dc_true()));
    // }

    // #[test]
    // fn test_superset_implies_subset() {
        // assert!(Component::from([["Amit"], ["Yue"]]).implies(&Component::from([["Amit"]])));
    // }

    // #[test]
    // fn test_reduce_simplifies() {
        // {
            // let mut component = Component::from([["Amit", "Yue"]]) & Component::from([["Yue"]]);
            // component.reduce();
            // assert_eq!(Component::from([["Yue"]]), component);
        // }
        // {
            // let mut component = Component::from([["Amit", "Yue"]]) & Component::from([["Amit"]]);
            // component.reduce();
            // assert_eq!(Component::from([["Amit"]]), component);
        // }
    // }

    // #[test]
    // fn test_yue_implies_yue_sub_hello() {
        // use alloc::{vec, string::String};
        // let clause_sup = Clause::new_from_vec(vec![vec![String::from("Yue")]]);
        // let clause_sub = Clause::new_from_vec(vec![vec!["Yue", "hello"]]);

        // assert_eq!(true, clause_sup.implies(&clause_sub));

        // let component_sup = Component::formula([clause_sup]);
        // let component_sub = Component::formula([clause_sub]);

        // assert_eq!(true, component_sup.implies(&component_sub));
    // }

    // #[test]
    // fn test_or() {
        // assert_eq!(
            // Component::from([["Amit", "Yue"], ["David", "Yue"]]),
            // Component::from([["Amit"], ["David"]]) | Component::from([["Yue"]])
        // );
    // }

    // quickcheck! {
        // fn x_implies_x(component: Component) -> bool {
            // let other = component.clone();
            // component.implies(&other) && other.implies(&component)
        // }

        // fn true_not_implies_not_true(component: Component) -> bool {
            // if component.is_true() {
                // true
            // } else {
                // !Component::dc_true().implies(&component)
            // }
        // }

        // fn nothing_implies_false(component: Component) -> bool {
            // if component.is_false() {
                // true
            // } else {
                // !component.implies(&Component::dc_false())
            // }
        // }

        // fn false_implies_everything(component: Component) -> bool {
            // Component::dc_false().implies(&component)
        // }

        // fn everything_implies_true(component: Component) -> bool {
            // component.implies(&Component::dc_true())
        // }

        // fn superset_implies_subset(component1: Component, component2: Component) -> bool {
            // let component1 = component1 & component2.clone();
            // component1.implies(&component2)
        // }

        // fn reduce_simplifies(component: Component) -> bool {
            // let mut component = component.clone();
            // component.reduce();
            // if let Component::DCFormula(clauses) =  component {
                // for (i, clausef) in clauses.iter().enumerate() {
                    // for clauser in clauses.iter().skip(i + 1) {
                        // if clausef.implies(clauser) || clauser.implies(clausef) {
                            // return false
                        // }
                    // }
                // }
            // }
            // true
        // }
    // }
// }
