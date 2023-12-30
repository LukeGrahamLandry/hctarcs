//! Wrapper types with more Scratch-like semantics.

use std::fmt::Debug;
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
    Owned(String),
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
        assert!(index > 0.0);
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

    // TODO: this is kinda ass
    #[must_use = "Allocates a new string, does not mutate the original."]
    pub fn join(&self, other: Str) -> Str {
        // TODO: I was trying so hard to not just clone it but gave up
        match self.clone() {
            Str::Const(s) => match other {
                Str::Const(s2) => {
                    let mut s3 = String::with_capacity(s.len() + s2.len());
                    s3.push_str(s);
                    s3.push_str(s2);
                    Str::Owned(s3)
                },
                Str::Char(c)  => {
                    let mut s3 = String::with_capacity(s.len() + 1);
                    s3.push(c);
                    s3.push_str(s);
                    Str::Owned(s3)
                },
                Str::Owned(s2) => {
                    // TODO: reuse allocation
                    let mut s3 = String::with_capacity(s.len() + s2.len());
                    s3.push_str(s);
                    s3.push_str(s2.as_str());
                    Str::Owned(s3)
                },
            }
            Str::Char(c) => match other {
                Str::Const(s2) => {
                    let mut s = String::with_capacity(s2.len() + 1);
                    s.push(c);
                    s.push_str(s2);
                    Str::Owned(s)
                },
                Str::Char(c2)  => {
                    let mut s = String::with_capacity(2);
                    s.push(c);
                    s.push(c2);
                    Str::Owned(s)
                },
                Str::Owned(mut s2) => {
                    s2.insert(0, c);
                    Str::Owned(s2)
                },
            }
            Str::Owned(mut s) => match other {
                Str::Const(s2) => Str::Owned(s + s2),
                Str::Char(c)  => {
                    s.push(c);
                    Str::Owned(s)
                },
                Str::Owned(s2) => Str::Owned(s + s2.as_str()),
            }
        }
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
}

impl<T: Clone + Debug> Index<f64> for List<T> {
    type Output = T;

    fn index(&self, index: f64) -> &Self::Output {
        let index = index - 1.0;
        if index >= 0.0 && index < self.len() {  // Rounding
            &self.0[index as usize]
        } else {
            // TODO: its hard to silently fail here cause of the references thing.
            todo!("List[OOB] index {} in len {}", index, self.0.len())
        }
    }
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
            Str::Char(_) => {
                todo!("as ref str for Str::Char is non-trivial")
            }
        }
    }
}

// TODO: ugly because idk how to as_ref a char
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
