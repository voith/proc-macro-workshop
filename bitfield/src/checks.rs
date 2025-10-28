pub enum ZeroMod8 {}
pub enum OneMod8 {}
pub enum TwoMod8 {}
pub enum ThreeMod8 {}
pub enum FourMod8 {}
pub enum FiveMod8 {}
pub enum SixMod8 {}
pub enum SevenMod8 {}

pub trait TotalSizeIsMultipleOfEightBits {
    type Check;
}

pub trait Markers {
    type Marker;
}

impl Markers for [(); 0] {
    type Marker = ZeroMod8;
}

impl Markers for [(); 1] {
    type Marker = OneMod8;
}

impl Markers for [(); 2] {
    type Marker = TwoMod8;
}

impl Markers for [(); 3] {
    type Marker = ThreeMod8;
}

impl Markers for [(); 4] {
    type Marker = FourMod8;
}

impl Markers for [(); 5] {
    type Marker = FiveMod8;
}

impl Markers for [(); 6] {
    type Marker = SixMod8;
}

impl Markers for [(); 7] {
    type Marker = SevenMod8;
}

impl TotalSizeIsMultipleOfEightBits for ZeroMod8 {
    type Check = ();
}

pub type MultipleOfEight<T> = <<T as Markers>::Marker as TotalSizeIsMultipleOfEightBits>::Check;
