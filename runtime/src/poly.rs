use std::borrow::Cow;

// TODO: this name is misleading now
// TODO: avoid this whenever possible
// TODO: could probably make this smaller
/// Cloning is only expensive if it holds a dynamically computed string.
#[derive(Clone, PartialEq, Debug, Default)]
pub enum NumOrStr {
    Num(f64),
    Str(Str),
    Bool(bool),
    #[default]
    Empty
}

#[derive(Clone, PartialEq, Debug)]
pub enum Str {
    Const(&'static str),
    Char(char),
    Owned(String),
}

impl NumOrStr {
    #[must_use]
    pub fn as_num(&self) -> f64 {
        match self {
            NumOrStr::Num(n) => *n,
            NumOrStr::Str(_) | NumOrStr::Empty | NumOrStr::Bool(_) => 0.0,
        }
    }

    // TODO: you want to call the version with ownership when possible
    #[must_use]
    pub fn as_str(&self) -> Str {
        match self {
            NumOrStr::Num(n) => Str::Owned(n.to_string()),  // TODO: this is a bit fishy
            NumOrStr::Str(s) => s.clone(),
            NumOrStr::Empty => Str::Const(""),
            // TODO: optimisation pass that makes sure you're not doing this because you're comparing to a string literal
            NumOrStr::Bool(b) => if *b { Str::Const("true") } else { Str::Const("false") },
        }
    }

    #[must_use]
    pub fn as_bool(&self) -> bool {
        match self {
            NumOrStr::Num(n) => todo!("Tried to convert {:?} to bool.", n),
            NumOrStr::Str(s) => todo!("Tried to convert {:?} to bool.", s),
            NumOrStr::Empty => false,
            NumOrStr::Bool(b) => *b,
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

impl From<f64> for NumOrStr {
    fn from(value: f64) -> Self {
        NumOrStr::Num(value)
    }
}

impl From<Str> for NumOrStr {
    /// Does not reallocate.
    fn from(value: Str) -> Self {
        NumOrStr::Str(value)
    }
}

impl From<bool> for NumOrStr {
    fn from(value: bool) -> Self {
        NumOrStr::Bool(value)
    }
}

impl From<NumOrStr> for f64 {
    fn from(value: NumOrStr) -> Self {
        value.as_num()
    }
}

impl From<&'static str> for Str {
    /// Create a string constant. Does not allocate.
    fn from(value: &'static str) -> Self {
        Str::Const(value)
    }
}

impl From<NumOrStr> for Str {
    /// If it was already a string, does not reallocate. Numbers do allocate tho...
    fn from(value: NumOrStr) -> Self {
        match value {
            NumOrStr::Str(s) => s,  // We have ownership so don't call the cloning version.
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
