//! Length units - 长度单位
//! Length units - SI length units (meter, kilometer, etc.)
//!
//! 提供长度量纲的 SI 单位定义，包括米、千米、厘米、毫米、微米、纳米等。
//! Provides SI unit definitions for length dimension, including meter, kilometer, centimeter, millimeter, micrometer, nanometer, etc.

use crate::dimension::derived::Length;
use crate::scale::{CENTI, DECA, DECI, HECTO, KILO, MICRO, MILLI, NANO, PICO, Scale};
use crate::unit::CTUnit;

// ============================================================================
// SI 前缀单位 / SI prefix units
// ============================================================================

define_unit!(Meter, "meter", "m", Length);
define_unit!(Kilometer, "kilometer", "km", Length, KILO.clone());
define_unit!(Hectometer, "hectometer", "hm", Length, HECTO.clone());
define_unit!(Decameter, "decameter", "dam", Length, DECA.clone());
define_unit!(Decimeter, "decimeter", "dm", Length, DECI.clone());
define_unit!(Cetimeter, "centimeter", "cm", Length, CENTI.clone());
define_unit_by!(Centimeter, "centimeter", "cm", Cetimeter);
define_unit!(Millimeter, "millimeter", "mm", Length, MILLI.clone());
define_unit!(Micrometer, "micrometer", "μm", Length, MICRO.clone());
define_unit!(Nanometer, "nanometer", "nm", Length, NANO.clone());
define_unit!(Picometer, "picometer", "pm", Length, PICO.clone());

// ============================================================================
// 海里单位 / Nautical mile units
// ============================================================================

// 国际标准海里 / International nautical mile
define_unit!(
    NauticalMile,
    "nautical mile",
    "nmi",
    Length,
    Scale::from_f64(1852.0)
);

// 法国海里 / French nautical mile
define_unit!(
    FRNauticalMile,
    "french nautical mile",
    "fr.nmi",
    Length,
    Scale::from_f64(1853.27)
);

// 英国海里 / UK nautical mile
define_unit!(
    UKNauticalMile,
    "uk nautical mile",
    "uk.nmi",
    Length,
    Scale::from_f64(1854.55)
);

// 俄罗斯海里 / Russian nautical mile
define_unit!(
    RUNauticalMile,
    "russian nautical mile",
    "ru.nmi",
    Length,
    Scale::from_f64(1855.78)
);

// 美国海里 / US nautical mile
define_unit!(
    USNauticalMile,
    "us nautical mile",
    "us.nmi",
    Length,
    Scale::from_f64(1851.01)
);

// ============================================================================
// 英制单位 / Imperial units
// ============================================================================

// 英寸 / Inch
define_unit!(Inch, "inch", "in", Length, Scale::from_f64(0.0254));

// 英尺 / Foot
define_unit!(Foot, "foot", "ft", Length, Scale::from_f64(0.3048));

// 码 / Yard
define_unit!(Yard, "yard", "yd", Length, Scale::from_f64(0.9144));

// 链 / Chain
define_unit!(Chain, "chain", "ch", Length, Scale::from_f64(20.1168));

// 杆 / Rod
define_unit!(Rod, "rod", "rd", Length, Scale::from_f64(5.0292));

// 英里 / Mile
define_unit!(Mile, "mile", "mi", Length, Scale::from_f64(1609.344));

// 英寻 / Fathom
define_unit!(Fathom, "fathom", "fm", Length, Scale::from_f64(1.852));

// 链（海里单位）/ Cable
define_unit!(Cable, "cable", "cab", Length, Scale::from_f64(185.2));

// ============================================================================
// 天文单位 / Astronomical units
// ============================================================================

// 天文单位 / Astronomical unit
define_unit!(
    AstronomicalUnit,
    "astronomical unit",
    "au",
    Length,
    Scale::from_f64(149597870700.0)
);

// 光秒 / Light second
define_unit!(
    LightSecond,
    "light second",
    "lsc",
    Length,
    Scale::from_f64(299792458.0)
);

// 光分 / Light minute
define_unit!(
    LightMinute,
    "light minute",
    "lmn",
    Length,
    Scale::from_f64(17987547480.0)
);

// 光时 / Light hour
define_unit!(
    LightHour,
    "light hour",
    "lhr",
    Length,
    Scale::from_f64(1079252848800.0)
);

// 光日 / Light day
define_unit!(
    LightDay,
    "light day",
    "ldy",
    Length,
    Scale::from_f64(25902068371200.0)
);

// 光年 / Light year
define_unit!(
    LightYear,
    "light year",
    "ly",
    Length,
    Scale::from_f64(9460730472580800.0)
);

// 秒差距 / Parsec
define_unit!(
    Parsec,
    "parsec",
    "pc",
    Length,
    Scale::from_f64(30856775814913673.0)
);

// 千秒差距 / Kiloparsec
define_unit!(
    Kiloparsec,
    "kiloparsec",
    "kpc",
    Length,
    Scale::from_f64(3.0856775814913673e19)
);

// 百万秒差距 / Megaparsec
define_unit!(
    Megaparsec,
    "megaparsec",
    "Mpc",
    Length,
    Scale::from_f64(3.0856775814913673e22)
);

// 十亿秒差距 / Gigaparsec
define_unit!(
    Gigaparsec,
    "gigaparsec",
    "Gpc",
    Length,
    Scale::from_f64(3.0856775814913673e25)
);
