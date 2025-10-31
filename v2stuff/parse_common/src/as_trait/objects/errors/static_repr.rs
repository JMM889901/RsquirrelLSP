use std::fmt::Display;

use sqparse::TokenItem;
///Why do i even have this still?
/// basically, the goal was like, have a thing that could go from the source stuff (lifetimed) to a static thing (hence, staticrepr) for use in errors
/// But then i also decided i want to be able to turn back so its basically just a comically overcomplicated clone
pub trait StaticReprReq : Display + 'static + PartialEq {}
pub trait StaticRepr<Target> : StaticReprReq + From<Target> + PartialEq<Target> {
}

impl<T> StaticReprReq for T where
    T: Display + 'static + PartialEq,
{
}
impl<T, Target> StaticRepr<Target> for T where
    T: StaticReprReq + From<Target> + PartialEq<Target>,
{
}