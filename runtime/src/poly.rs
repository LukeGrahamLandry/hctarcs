//! Wrapper types with more Scratch-like semantics.

use std::fmt::Debug;
use std::mem::size_of;
use std::ops::Index;

// TODO: avoid this whenever possible
// TODO: could probably make this smaller
/// Cloning is only expensive if it holds a dynamically computed string.
#[derive(Clone, Debug, Default)]
pub enum Poly {  // TODO: there are enough bits to pack this better, especially if you hoist the Str tag. Should fit in 3 words instead of 5.
    Num(f64),
    Str(Str),
    Bool(bool),
    #[default]
    Empty
}

#[derive(Clone, Debug)]
pub enum Str {
    Const(&'static str),
    Char(char),
    Owned(String),  // TODO: Rc
}

/// 1-indexed Vec with silently failing operations.
#[derive(Default, Clone, Debug)]
pub struct List<T: Clone + Debug>(Vec<T>);

impl Poly {
    #[must_use]
    pub fn as_num(&self) -> f64 {
        match self {
            Poly::Num(n) => *n,
            Poly::Str(s) => {
                if s.len() == 0.0 {
                    return 0.0;
                }
                s.as_ref().parse::<f64>().unwrap_or(0.0)
            }
            Poly::Empty | Poly::Bool(_) => 0.0,
        }
    }

    // TODO: you want to call the version with ownership when possible
    #[must_use]
    pub fn as_str(&self) -> Str {
        match self {
            Poly::Num(n) => Str::Owned(n.to_string()),  // TODO: this is a bit fishy
            Poly::Str(s) => s.clone(),
            Poly::Empty => Str::Const(""),
            // TODO: optimisation pass that makes sure you're not doing this because you're comparing to a string literal
            Poly::Bool(b) => if *b { Str::Const("true") } else { Str::Const("false") },
        }
    }

    #[must_use]
    pub fn as_bool(&self) -> bool {
        match self {
            Poly::Num(n) => todo!("Tried to convert {:?} to bool.", n),
            Poly::Str(s) => todo!("Tried to convert {:?} to bool.", s),
            Poly::Empty => false,
            Poly::Bool(b) => *b,
        }
    }

    pub fn is_num(&self) -> bool {
        match self {
            Poly::Num(_) => true,
            Poly::Str(s) => s.as_ref().parse::<f64>().is_ok(),
            Poly::Empty => true,
            Poly::Bool(_) => false,
        }
    }
}

impl Str {
    // TODO: what to do on OOB?
    /// Does not allocate.
    pub fn get_index(&self, index: f64) -> Str {
        assert!(index >= 1.0);
        let index = index as usize - 1;
        let c = match self {
            Str::Char(c) => if index == 0 {
                *c
            } else {
                panic!("Tried to get {index} in {self:?}")
            }
            // TODO: is scratch unicode aware? Could be faster if its just bytes.
            Str::Const(s)  => {
                s.chars().nth(index).unwrap()
            }
            Str::Owned(s) => {
                s.chars().nth(index).unwrap()
            }
        };
        Str::Char(c)
    }

    pub fn len(&self) -> f64 {
        (match self {
            Str::Const(s) => s.len(),
            Str::Owned(s) => s.len(),
            Str::Char(_) => 1,
        }) as f64
    }

    // TODO: its unfortunate that this doesnt take self by value. need to track ownership in the compiler.
    #[must_use = "Allocates a new string, does not mutate the original."]
    pub fn join(&self, other: Str) -> Str {
        let mut s = String::with_capacity((self.len() + other.len()) as usize);
        s.push_str(self.as_ref());
        s.push_str(other.as_ref());
        Str::Owned(s)
    }
}

impl<T: Clone + Debug> List<T> {
    pub fn len(&self) -> f64 {
        self.0.len() as f64
    }

    pub fn push(&mut self, value: T) {
        self.0.push(value)
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn remove(&mut self, index: f64) {
        let index = index - 1.0;
        if index >= 0.0 && index < self.len() {  // Rounding
            self.0.remove(index as usize);
        }
    }

    // This can't use IndexMut because it has to fail silently.
    pub fn replace(&mut self, index: f64, value: T) {
        let index = index - 1.0;
        if index >= 0.0 && index < self.len() {  // Rounding
            self.0[index as usize] = value;
        }
    }

    // UNUSED thus far
    pub fn insert(&mut self, index: f64, value: T) {
        let index = index - 1.0;
        if index >= 0.0 && index < self.len() + 1.0 { // Rounding, allow one off the end
            self.0.insert(index as usize, value);
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item=&T> {
        self.0.iter()
    }
}

impl<T: Clone + Debug + ConstEmpty> Index<f64> for List<T> {
    type Output = T;

    fn index(&self, index: f64) -> &Self::Output {
        let index = index - 1.0;
        if index >= 0.0 && index < self.len() {  // Rounding
            &self.0[index as usize]
        } else {  // Fail silently
            T::EMPTY
        }
    }
}

// Different from default because there's a const instance so you can return an immutable reference to it.
trait ConstEmpty: 'static {
    const EMPTY: &'static Self;
}

impl ConstEmpty for f64 {
    const EMPTY: &'static Self = &0.0;
}

impl ConstEmpty for Poly {
    const EMPTY: &'static Self = &Poly::Empty;
}

impl ConstEmpty for Str {
    const EMPTY: &'static Self = &Str::Const("");
}

impl From<f64> for Poly {
    fn from(value: f64) -> Self {
        Poly::Num(value)
    }
}

impl From<Str> for Poly {
    /// Does not reallocate.
    fn from(value: Str) -> Self {
        Poly::Str(value)
    }
}

impl From<bool> for Poly {
    fn from(value: bool) -> Self {
        Poly::Bool(value)
    }
}

impl From<Poly> for f64 {
    fn from(value: Poly) -> Self {
        value.as_num()
    }
}

impl From<&'static str> for Str {
    /// Create a string constant. Does not allocate.
    fn from(value: &'static str) -> Self {
        Str::Const(value)
    }
}

impl From<Poly> for Str {
    /// If it was already a string, does not reallocate. Numbers do allocate tho...
    fn from(value: Poly) -> Self {
        match value {
            Poly::Str(s) => s,  // We have ownership so don't call the cloning version.
            _ => value.as_str()
        }
    }
}

impl Default for Str {
    fn default() -> Self {
        Str::Const("")
    }
}

impl AsRef<str> for Str {
    fn as_ref(&self) -> &str {
        match self {
            Str::Const(s) => s,
            Str::Owned(s) => s.as_str(),
            Str::Char(c) => {
                assert!(size_of::<char>() <= size_of::<usize>()); // if the compiler cant figure out this is constant, there are larger problems in the world.
                let mut i = *c as usize;  // if you're somehow on a 16 bit machine you deserve whatever you get
                if i >= 128usize { // TOD0: non-ascii char->str is not supported yet
                    i = 0;
                }

                &ASCII[i..i + 1]
            }
        }
    }
}

impl From<Vec<f64>> for List<Poly> {
    fn from(value: Vec<f64>) -> Self {
        List(value.into_iter().map(Poly::from).collect())
    }
}

impl From<Vec<Poly>> for List<Poly> {
    fn from(value: Vec<Poly>) -> Self {
        List(value)
    }
}


// TODO: include_bytes! and pack it
pub fn str_to_poly_list(s: &str) -> List<Poly> {
    List::<Poly>::from(s.split(',').map(|v| v.parse().unwrap()).collect::<Vec<f64>>())
}

// str needs to be a reference so its not obvious how to convert a char into one without allocating.
// print('const ASCII: &str = "' + "".join([("\\x" + hex(i).replace("0x", "").zfill(2)) if (i != 0) else "?" for i in range(128)]) + '";')
const ASCII: &str = "?\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\x20\x21\x22\x23\x24\x25\x26\x27\x28\x29\x2a\x2b\x2c\x2d\x2e\x2f\x30\x31\x32\x33\x34\x35\x36\x37\x38\x39\x3a\x3b\x3c\x3d\x3e\x3f\x40\x41\x42\x43\x44\x45\x46\x47\x48\x49\x4a\x4b\x4c\x4d\x4e\x4f\x50\x51\x52\x53\x54\x55\x56\x57\x58\x59\x5a\x5b\x5c\x5d\x5e\x5f\x60\x61\x62\x63\x64\x65\x66\x67\x68\x69\x6a\x6b\x6c\x6d\x6e\x6f\x70\x71\x72\x73\x74\x75\x76\x77\x78\x79\x7a\x7b\x7c\x7d\x7e\x7f";

impl PartialEq for Str {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Str::Char(a) => return match other {
                Str::Char(b) => a == b,
                _ => other.as_ref().len() == 1 && other.as_ref().chars().next().unwrap() == *a,
            },
            _ => match other {
                Str::Char(a) => return self.as_ref().len() == 1 && self.as_ref().chars().next().unwrap() == *a,
                _ => {}
            },
        }
        self.as_ref() == other.as_ref()
    }
}

impl PartialEq for Poly {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Poly::Num(n) => match other {
                Poly::Num(n2) => *n == *n2,
                Poly::Str(_) => *n == other.as_num(),
                _ => false,
            },
            Poly::Str(s) => match other {
                Poly::Str(s2) => s == s2,
                Poly::Num(n2) => self.as_num() == *n2,
                _ => false,
            },
            Poly::Bool(b) => match other {
                Poly::Bool(b2) => *b == *b2,
                _ => false,
            },
            Poly::Empty => matches!(other, Poly::Empty)
        }
    }
}
