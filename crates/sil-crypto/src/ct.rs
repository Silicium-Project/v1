use subtle::ConstantTimeEq;

/// Égalité en temps constant (tokens, MAC). La longueur peut fuiter, pas le contenu.
pub fn ct_eq(left: &[u8], right: &[u8]) -> bool {
    left.ct_eq(right).into()
}
