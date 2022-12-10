#[cfg(test)]
use alloc::boxed::Box;
#[cfg(test)]
use quickcheck::Arbitrary;

use serde::{Deserialize, Serialize};

use super::Principal;
use alloc::vec;
use alloc::{collections::BTreeSet, vec::Vec};

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Clause(pub BTreeSet<Vec<Principal>>);

#[cfg(test)]
impl Arbitrary for Clause {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Clause(BTreeSet::arbitrary(g))
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(self.0.shrink().map(|x| Clause(x)))
    }
}

impl Clause {
    pub fn empty() -> Self {
        Self::new([] as [Principal; 0])
    }

    pub fn new<P: Into<Principal> + Clone, const N: usize>(principals: [P; N]) -> Clause {
        let mut result = BTreeSet::new();
        for p in principals.iter() {
            result.insert(vec![p.clone().into()]);
        }
        Self(result)
    }

    pub fn new_from_vec<P: Into<Principal> + Clone>(principals: Vec<Vec<P>>) -> Clause {
        let mut result = BTreeSet::new();
        for p in principals.iter() {
            result.insert(p.clone().drain(..).map(Into::into).collect());
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

impl<P: Into<Principal> + Clone, const N: usize> From<[P; N]> for Clause {
    fn from(principals: [P; N]) -> Clause {
        Clause::new(principals)
    }
}

impl<P: Into<Principal> + Clone> From<Vec<P>> for Clause {
    fn from(mut principals: Vec<P>) -> Clause {
        Clause::new_from_vec(principals.drain(..).map(|p| vec![p]).collect())
    }
}

impl From<BTreeSet<Vec<Principal>>> for Clause {
    fn from(principals: BTreeSet<Vec<Principal>>) -> Clause {
        Clause(principals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x_implies_x() {
        // False implies False
        assert!(Clause::empty().implies(&Clause::empty()));

        // "Amit" implies "Amit"
        assert!(Clause::from(["Amit"]).implies(&Clause::from(["Amit"])));
    }

    #[test]
    fn test_subset_implies_superset() {
        // False implies "Amit"
        assert!(Clause::empty().implies(&Clause::from(["Amit"])));

        // "Amit" implies "Amit" \/ "Yue"
        assert!(Clause::from(["Amit"]).implies(&Clause::from(["Amit", "Yue"])));
    }

    #[test]
    fn test_superset_not_implies_subset() {
        // "Amit" not-implies False
        assert_eq!(false, Clause::from(["Amit"]).implies(&Clause::empty()));

        // "Amit" \/ "Yue" not-implies "Amit"
        assert_eq!(
            false,
            Clause::from(["Amit", "Yue"]).implies(&Clause::from(["Amit"]))
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
