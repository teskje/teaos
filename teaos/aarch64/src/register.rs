use core::arch::asm;
use core::fmt;

use paste::paste;

macro_rules! system_register {
    (
        $name:ident,
        $( $field:ident [ $a:literal : $b:literal ], )*
    ) => {
        #[derive(Clone, Copy)]
        #[allow(non_camel_case_types)]
        pub struct $name(u64);

        impl $name {
            pub fn read() -> Self {
                let value: u64;
                unsafe {
                    asm!(
                        concat!("mrs {x}, ", stringify!($name)),
                        x = out(reg) value,
                        options(nomem, preserves_flags, nostack),
                    );
                }
                Self(value)
            }

            pub unsafe fn write<V: Into<u64>>(value: V) {
                unsafe {
                    asm!(
                        concat!("msr ", stringify!($name), ", {x}"),
                        x = in(reg) value.into(),
                        options(nomem, preserves_flags, nostack),
                    );
                }
            }

            paste! {
                $(
                    #[allow(non_snake_case)]
                    #[allow(clippy::eq_op, clippy::identity_op)]
                    pub fn $field(&self) -> u64 {
                        let mask = (2u64 << $b - $a).wrapping_sub(1);
                        (self.0 >> $a) & mask
                    }

                    #[allow(non_snake_case)]
                    #[allow(clippy::eq_op, clippy::identity_op)]
                    pub fn [<set_ $field>](&mut self, x: u64) {
                        let mask = (2u64 << $b - $a).wrapping_sub(1);
                        assert!(x <= mask);
                        self.0 &= !(mask << $a);
                        self.0 |= x << $a;
                    }
                )*
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
                write!(f, "{}({:#018x})", stringify!($name), self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
                f.debug_struct(stringify!($name))
                    .field("_", &format_args!("{:#x}", self.0))
                $(
                    .field(stringify!($field), &format_args!("{:#x}", self.$field()))
                )*
                    .finish()
            }
        }

        impl From<$name> for u64 {
            fn from(x: $name) -> u64 {
                x.0
            }
        }
    };
}

system_register!(CNTFRQ_EL0,
    ClockFreq[0:31],
);

system_register!(CNTVCT_EL0,
    VirtualCount[0:63],
);

system_register!(ESR_EL1,
    ISS[0:24],
    IL[25:25],
    EC[26:31],
    ISS2[32:55],
);

system_register!(FAR_EL1,
    VA[0:63],
);

system_register!(PAR_EL1,
    F[0:0],
    PA[12:47],
);

system_register!(TCR_EL1,
    T0SZ[0:5],
    EPD0[7:7],
    IRGN0[8:9],
    ORGN0[10:11],
    SH0[12:13],
    TG0[14:15],
    T1SZ[16:21],
    A1[22:22],
    EPD1[23:23],
    IRGN1[24:25],
    ORGN1[26:27],
    SH1[28:29],
    TG1[30:31],
    IPS[32:34],
    AS[36:36],
    TBI0[37:37],
    TBI1[38:38],
);

system_register!(TTBR1_EL1,
    BADDR[1:47],
    ASID[48:63],
);

system_register!(VBAR_EL1,
    VBA[11:63],
);
