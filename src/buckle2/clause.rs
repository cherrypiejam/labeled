#[cfg(test)]
use alloc::boxed::Box;
#[cfg(test)]
use quickcheck::Arbitrary;

// use serde::{Deserialize, Serialize};

use super::Principal;
use alloc::{collections::BTreeSet, vec::Vec};

use core::alloc::Allocator;
use alloc::alloc::Global;

#[derive(Debug, Clone)]
pub struct Clause<A: Allocator + Clone = Global>(pub BTreeSet<Vec<Principal<A>, A>, A>);

impl<A: Allocator + Clone> PartialEq for Clause<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<A: Allocator + Clone> Eq for Clause<A> {}

impl<A: Allocator + Clone> PartialOrd for Clause<A> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<A: Allocator + Clone> Ord for Clause<A> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

#[cfg(test)]
impl Arbitrary for Clause {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Clause(BTreeSet::arbitrary(g))
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(self.0.shrink().map(|x| Clause(x)))
    }
}


impl<P: Into<Principal<Global>> + Clone, const N: usize> From<[P; N]> for Clause {
    fn from(principals: [P; N]) -> Clause {
        Clause::new(principals)
    }
}

impl<P: Into<Principal<Global>> + Clone> From<Vec<P>> for Clause {
    fn from(mut principals: Vec<P>) -> Clause {
        use alloc::vec;
        Clause::new_from_vec(principals.drain(..).map(|p| vec![p]).collect())
    }
}

impl<A: Allocator + Clone, P: Into<Principal<A>> + Clone, const N: usize> From<([P; N], A)> for Clause<A> {
    fn from((principals, alloc): ([P; N], A)) -> Clause<A> {
        Clause::new_in(principals, alloc)
    }
}

impl Clause {
    pub fn empty() -> Clause {
        Self::empty_in(Global)
    }

    pub fn new<P: Into<Principal<Global>> + Clone, const N: usize>(principals: [P; N]) -> Clause {
        Self::new_in(principals, Global)
    }

    pub fn new_from_vec<P: Into<Principal<Global>> + Clone>(principals: Vec<Vec<P>>) -> Clause {
        Self::new_from_vec_in(principals, Global)
    }
}

impl<A: Allocator + Clone, P: Into<Principal<A>> + Clone> From<(Vec<P, A>, A)> for Clause<A> {
    fn from((mut principals, alloc): (Vec<P, A>, A)) -> Clause<A> {
        let mut v = Vec::new_in(alloc.clone());
        principals.drain(..).for_each(|p| {
            let mut vv = Vec::new_in(alloc.clone());
            vv.push(p);
            v.push(vv);
        });
        Clause::new_from_vec_in(v, alloc)
        // Clause::new_from_vec_in(principals.drain(..).map(|p| vec![p]).collect(), alloc)
    }
}

impl<A: Allocator + Clone> From<BTreeSet<Vec<Principal<A>, A>, A>> for Clause<A> {
    fn from(principals: BTreeSet<Vec<Principal<A>, A>, A>) -> Clause<A> {
        Clause(principals)
    }
}

impl<A: Allocator + Clone> Clause<A> {
    pub fn empty_in(alloc: A) -> Clause<A> {
        Self::new_in([] as [Principal<A>; 0], alloc)
    }

    pub fn new_in<P: Into<Principal<A>> + Clone, const N: usize>(principals: [P; N], alloc: A) -> Clause<A>
    {
        let mut result = BTreeSet::new_in(alloc.clone());
        for p in principals.iter() {
            let mut v = Vec::new_in(alloc.clone());
            v.push(p.clone().into());
            result.insert(v);
        }
        Self(result)
    }

    pub fn new_from_vec_in<P: Into<Principal<A>> + Clone>(principals: Vec<Vec<P, A>, A>, alloc: A) -> Clause<A> {
        let mut result = BTreeSet::new_in(alloc.clone());
        for p in principals.iter() {

            let mut v = Vec::new_in(alloc.clone());
            p.clone().drain(..).for_each(|e| v.push(e.into()));
            result.insert(v);

            // let b = p.clone().into_iter().map(Into::<Principal<A>>::into).collect::<Vec<Principal<A>, A>>();
            // let a = p.clone().drain(..).map(Into::into).collect();// .map(Into::into).collect();
            // result.insert(p.clone().drain(..).map(Into::into).collect());
        }
        Self(result)
    }

    pub fn implies(&self, other: &Self) -> bool {
        // self is subset of other
        if self.0.is_empty() {
            true
        } else if other.0.is_empty() {
            false
        } else {
            //self.0.is_subset(&other.0)
            self.0.iter()
                .all(|svec| other.0.iter().any(|ovec| {
                    ovec.starts_with(svec)
                }))
            //other.0.iter()
            //    .any(|ovec| self.0.iter().any(|svec| {
            //    ovec.starts_with(svec)
            //    }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::alloc::Global;

    #[test]
    fn test_x_implies_x() {
        // False implies False
        assert!(Clause::empty_in(Global).implies(&Clause::empty_in(Global)));

        // "Amit" implies "Amit"
        assert!(Clause::from((["Amit"], Global)).implies(&Clause::from((["Amit"], Global))));
        assert!(Clause::new_in(["Amit"], Global).implies(&Clause::new_in(["Amit"], Global)));
    }

    #[test]
    fn test_subset_implies_superset() {
        // False implies "Amit"
        assert!(Clause::empty().implies(&Clause::from((["Amit"], Global))));

        // "Amit" implies "Amit" \/ "Yue"
        assert!(Clause::from((["Amit"], Global)).implies(&Clause::from((["Amit", "Yue"], Global))));
    }

    #[test]
    fn test_superset_not_implies_subset() {
        // "Amit" not-implies False
        assert_eq!(false, Clause::from((["Amit"], Global)).implies(&Clause::empty()));

        // "Amit" \/ "Yue" not-implies "Amit"
        assert_eq!(
            false,
            Clause::from((["Amit", "Yue"], Global)).implies(&Clause::from((["Amit"], Global)))
        );
    }

    quickcheck! {
        fn empty_clause_implies_all(clause: Clause) -> bool {
            let empty = Clause::empty();
            empty.implies(&clause)
        }

        fn subset_implies_superset(clause1: Clause, clause2: Clause) -> bool {
            let mut clause1 = clause1.clone();
            clause1.0.append(&mut clause2.0.clone());
            clause2.implies(&clause1)
        }
    }
}
