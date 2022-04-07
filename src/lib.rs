#![no_std]

extern crate alloc;

#[cfg(feature = "dclabel")]
pub mod dclabel;

pub trait Label {
    fn lub(self, rhs: Self) -> Self;
    fn glb(self, rhs: Self) -> Self;
    fn can_flow_to(&self, rhs: &Self) -> bool;
    #[cfg(feature = "dclabel")]
    fn can_flow_to_with_privilege(&self, rhs: &Self, privilege: &dclabel::Component) -> bool;
}

