// Crates that have the "proc-macro" crate type are only allowed to export
// procedural macros. So we cannot have one crate that defines procedural macros
// alongside other types of public APIs like traits and structs.
//
// For this project we are going to need a #[bitfield] macro but also a trait
// and some structs. We solve this by defining the trait and structs in this
// crate, defining the attribute macro in a separate bitfield-impl crate, and
// then re-exporting the macro from this crate so that users only have one crate
// that they need to import.
//
// From the perspective of a user of this crate, they get all the necessary APIs
// (macro, trait, struct) through the one bitfield crate.
pub use bitfield_impl::bitfield;
pub mod checks;

pub trait Specifier {
    const BITS: usize;
    type Ty;
}

pub struct FixedBits<const N: usize>();

macro_rules! fixed_bytes_aliases {
    ($($name:ident<$N:literal, $ty:ty>),* $(,)?) => {
        $(
            pub type $name = FixedBits<$N>;
            impl Specifier for FixedBits<$N> {
                const BITS: usize = $N;
                type Ty = $ty;
            }
        )*
    };
}

fixed_bytes_aliases! {
    B1<1, u8>,
    B2<2, u8>,
    B3<3, u8>,
    B4<4, u8>,
    B5<5, u8>,
    B6<6, u8>,
    B7<7, u8>,
    B8<8, u8>,
    B9<9, u16>,
    B10<10, u16>,
    B11<11, u16>,
    B12<12, u16>,
    B13<13, u16>,
    B14<14, u16>,
    B15<15, u16>,
    B16<16, u16>,
    B17<17, u32>,
    B18<18, u32>,
    B19<19, u32>,
    B20<20, u32>,
    B21<21, u32>,
    B22<22, u32>,
    B23<23, u32>,
    B24<24, u32>,
}

pub fn create_get_bit_mask(start: u8, end: u8) -> u8 {
    let mut mask: u8 = 0b00000000;
    for i in start..=end {
        mask |= 1 << i;
    }
    mask
}

pub fn create_set_width_bit_mask(start: u8, end: u8) -> u64 {
    debug_assert!(end >= start && (end - start) < 64);
    let mut mask: u64 = 0b0;
    for i in 0..=end - start {
        mask |= 1 << i;
    }
    mask
}
