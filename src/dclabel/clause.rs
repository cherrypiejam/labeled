use serde::{Serialize, Deserialize};

use super::Principal;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Clone, Serialize, Deserialize)]
pub struct Clause(pub BTreeSet<Principal>);

impl Clause {
    pub fn empty() -> Self {
        Self::new([] as [Principal; 0])
    }

    pub fn new<P: Into<Principal> + Clone, const N: usize>(principals: [P; N]) -> Clause {
        let mut result = BTreeSet::new();
        for p in principals.iter() {
            result.insert(p.clone().into());
        }
        Self(result)
    }

    pub fn new_from_vec<P: Into<Principal> + Clone>(principals: Vec<P>) -> Clause {
        let mut result = BTreeSet::new();
        for p in principals.iter() {
            result.insert(p.clone().into());
        }
        Self(result)
    }

    pub fn implies(&self, other: &Self) -> bool {
        // self is subset of other
        self.0.is_subset(&other.0)
    }
}

impl<P: Into<Principal> + Clone, const N: usize> From<[P; N]> for Clause {
    fn from(principals: [P; N]) -> Clause {
        Clause::new(principals)
    }
}

impl<P: Into<Principal> + Clone> From<Vec<P>> for Clause {
    fn from(principals: Vec<P>) -> Clause {
        Clause::new_from_vec(principals)
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
}
