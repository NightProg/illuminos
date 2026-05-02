use alloc::vec::Vec;
use core::fmt::Debug;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

pub fn ceil(val: f64) -> i64 {
    libm::ceil(val) as i64
}

pub trait Arithmetic:
    Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + AddAssign
    + SubAssign
    + MulAssign
    + DivAssign
    + Sized
    + Copy
    + Clone
    + PartialEq
    + PartialOrd
    + Debug
{
    fn pi() -> Self {
        Self::new(core::f64::consts::PI)
    }
    fn new(f: f64) -> Self;

    fn range(start: Self, end: Self) -> Vec<Self> {
        let mut result = Vec::new();
        let step = Self::new(1.0);
        let mut current = start;

        while current <= end {
            result.push(current);
            current += step;
        }

        result
    }

    fn neg(self) -> Self {
        Self::new(-self.as_f64())
    }
    fn sqrt(self) -> Self {
        Self::new(libm::sqrt(self.as_f64()))
    }
    fn powf(self, exp: Self) -> Self {
        Self::new(libm::pow(self.as_f64(), exp.as_f64()))
    }
    fn abs(self) -> Self {
        Self::new(libm::fabs(self.as_f64()))
    }
    fn floor(self) -> Self {
        Self::new(libm::floor(self.as_f64()))
    }

    fn ceil(self) -> Self {
        Self::new(libm::ceil(self.as_f64()))
    }

    fn as_f64(self) -> f64;
    fn as_f32(self) -> f32;
    fn as_i64(self) -> i64;
    fn as_u64(self) -> u64;
    fn as_isize(self) -> isize;
    fn as_usize(self) -> usize;
    fn as_u32(self) -> u32;
    fn as_u16(self) -> u16;
    fn as_u8(self) -> u8;
    fn as_i32(self) -> i32;
    fn as_i16(self) -> i16;
    fn as_i8(self) -> i8;
}

pub trait Vector<T: Arithmetic>:
    Sized + Div<T, Output = Self> + Mul<T, Output = Self> + Copy + Clone
{
    fn new(vecs: &[T]) -> Option<Self>;
    fn magnitude(&self) -> T {
        let mut sum = T::new(0.0);
        for val in self.vecs().iter().copied() {
            sum = sum + val.powf(T::new(2.0));
        }
        sum.sqrt()
    }
    fn normalize(&self) -> Self {
        let mag = self.magnitude();
        *self / mag
    }
    fn dot(&self, other: &Self) -> T {
        let mut sum = T::new(0.0);
        for (a, b) in self.vecs().iter().zip(other.vecs().iter()) {
            sum = sum + *a * *b;
        }
        sum
    }
    fn cross(&self, other: &Self) -> T {
        let mut sum = T::new(0.0);
        for (a, b) in self.vecs().iter().zip(other.vecs().iter()) {
            sum = sum + *a * *b;
        }
        sum
    }

    fn vecs(&self) -> Vec<T>;

    fn add(self, other: Self) -> Self {
        let mut v = Vec::new();
        for (a, b) in self.vecs().iter().zip(other.vecs().iter()) {
            v.push(*a + *b);
        }
        Self::new(&v).unwrap()
    }

    fn sub(self, other: Self) -> Self {
        let mut v = Vec::new();
        for (a, b) in self.vecs().iter().zip(other.vecs().iter()) {
            v.push(*a - *b);
        }
        Self::new(&v).unwrap()
    }

    fn mul(self, other: Self) -> Self {
        let mut v = Vec::new();
        for (a, b) in self.vecs().iter().zip(other.vecs().iter()) {
            v.push(*a * *b);
        }
        Self::new(&v).unwrap()
    }

    fn div(self, other: Self) -> Self {
        let mut v = Vec::new();
        for (a, b) in self.vecs().iter().zip(other.vecs().iter()) {
            v.push(*a / *b);
        }
        Self::new(&v).unwrap()
    }
}

macro_rules! vector {
    ($f:ident, $name:ident, $n:expr, {$($fields:ident),*}) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
        pub struct $name<T: Arithmetic> {
            $(pub $fields: T),*
        }

        impl<T: Arithmetic> $name<T> {
            pub fn new($($fields: T),*) -> Self {
                Self { $($fields),* }
            }

            pub fn as_usize(self) -> $name<usize> {
                $name { $($fields: self.$fields.as_usize()),* }
            }

            pub fn into<K: From<T> + Arithmetic>(self) -> $name<K> {
                let mut v = Vec::new();
                for val in self.vecs().iter().copied() {
                    v.push(K::from(val));
                }
                Vector::new(&v).unwrap()
            }

            pub fn to_slice(&self) -> [T; $n] {
                [$(self.$fields),*]
            }
        }

        impl<T: Arithmetic> Add<T> for $name<T> {
            type Output = Self;

            fn add(self, other: T) -> Self::Output {
                Self::new($(self.$fields + other),*)
            }
        }

        impl<T: Arithmetic> Add<Self> for $name<T> {
            type Output = Self;

            fn add(self, other: Self) -> Self::Output {
                Self::new($(self.$fields + other.$fields),*)
            }
        }

        impl<T: Arithmetic> Sub<T> for $name<T> {
            type Output = Self;

            fn sub(self, other: T) -> Self::Output {
                Self::new($(self.$fields - other),*)
            }
        }

        impl<T: Arithmetic> Sub<Self> for $name<T> {
            type Output = Self;

            fn sub(self, other: Self) -> Self::Output {
                Self::new($(self.$fields - other.$fields),*)
            }
        }

        impl<T: Arithmetic> Mul<T> for $name<T> {
            type Output = Self;

            fn mul(self, other: T) -> Self::Output {
                Self::new($(self.$fields * other),*)
            }
        }

        impl<T: Arithmetic> Div<T> for $name<T> {
            type Output = Self;

            fn div(self, other: T) -> Self::Output {
                Self::new($(self.$fields / other),*)
            }
        }

        impl<T: Arithmetic> Vector<T> for $name<T> {
            fn new(vec: &[T]) -> Option<Self> {
                let [$($fields),*] = *vec else {
                    return None
                };
                Some(Self { $($fields),* })
            }

            fn vecs(&self) -> Vec<T> {
                alloc::vec![ $(self.$fields),* ]
            }

        }

        pub fn $f<T: Arithmetic>($($fields: T),*) -> $name<T> {
            $name { $($fields),* }
        }
    }
}

vector!(vec2, Vec2, 2, {x, y});
vector!(vec3, Vec3, 3, {x, y, z});
vector!(vec4, Vec4, 4, {x, y, z, w});

macro_rules! component {
    ($i:ty) => {
        impl Arithmetic for $i {
            fn new(f: f64) -> Self {
                f as $i
            }

            fn as_f64(self) -> f64 {
                self as f64
            }

            fn as_f32(self) -> f32 {
                self as f32
            }

            fn as_i64(self) -> i64 {
                self as i64
            }

            fn as_u64(self) -> u64 {
                self as u64
            }

            fn as_isize(self) -> isize {
                self as isize
            }

            fn as_usize(self) -> usize {
                self as usize
            }

            fn as_u32(self) -> u32 {
                self as u32
            }

            fn as_u16(self) -> u16 {
                self as u16
            }

            fn as_u8(self) -> u8 {
                self as u8
            }

            fn as_i32(self) -> i32 {
                self as i32
            }

            fn as_i16(self) -> i16 {
                self as i16
            }

            fn as_i8(self) -> i8 {
                self as i8
            }
        }
    };

    ($($i:ty),*) => {
        $(component!($i);)*
    }
}

component!(i8, i16, i32, i64, isize);

component!(u8, u16, u32, u64, usize);

component!(f32, f64);
