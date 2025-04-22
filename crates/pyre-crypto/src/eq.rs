pub trait ConstantTimeEq {
    fn eq(&self, other: &Self) -> bool;
}

impl ConstantTimeEq for &[u8] {
    fn eq(&self, other: &Self) -> bool {
        subtle::ConstantTimeEq::ct_eq(*self, *other).into()
    }
}
