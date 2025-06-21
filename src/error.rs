#[must_use]
pub struct TypeError {
    pub range: text_size::TextRange,
    pub message: String,
}

impl TypeError {
    pub fn new(range: text_size::TextRange, message: impl Into<String>) -> Self {
        Self {
            range,
            message: message.into(),
        }
    }
}

#[must_use]
#[derive(Default)]
pub enum Errors {
    #[default]
    AllGood,
    Single(Box<TypeError>),
    Many(Vec<Errors>), // None of the elements is AllGood.
}

impl Errors {
    pub const ALL_GOOD: Self = Self::AllGood;

    pub fn single(range: text_size::TextRange, message: impl Into<String>) -> Self {
        Self::Single(Box::new(TypeError {
            range,
            message: message.into(),
        }))
    }
}

impl From<TypeError> for Errors {
    fn from(value: TypeError) -> Self {
        Self::Single(Box::new(value))
    }
}

impl From<ErrorsBuilder> for Errors {
    fn from(value: ErrorsBuilder) -> Self {
        value.build()
    }
}

impl From<()> for Errors {
    fn from(_: ()) -> Self {
        Errors::AllGood
    }
}

impl<E1: Into<Errors>, E2: Into<Errors>> From<(E1, E2)> for Errors {
    fn from(value: (E1, E2)) -> Self {
        let mut builder = ErrorsBuilder::new();
        let (v1, v2) = value;
        builder.add(v1);
        builder.add(v2);
        builder.build()
    }
}

pub struct ErrorsBuilder {
    errors: Vec<Errors>,
}

impl ErrorsBuilder {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn add(&mut self, error: impl Into<Errors>) {
        let error = error.into();
        if !matches!(error, Errors::AllGood) {
            self.errors.push(error);
        }
    }

    pub fn build(self) -> Errors {
        match self.errors.len() {
            0 => Errors::AllGood,
            1 => self.errors.into_iter().next().unwrap(), // We just checked the length.
            _ => Errors::Many(self.errors),
        }
    }
}

#[must_use]
pub struct WithErrors<T> {
    pub inner: T,
    pub errors: Errors,
}

impl<T> WithErrors<T> {
    pub fn new(inner: T, errors: impl Into<Errors>) -> Self {
        WithErrors {
            inner,
            errors: errors.into(),
        }
    }
}

pub trait WithErrorsExt: Sized {
    fn with_errors(self, errors: impl Into<Errors>) -> WithErrors<Self>;
}

impl<T> WithErrorsExt for T {
    fn with_errors(self, errors: impl Into<Errors>) -> WithErrors<Self> {
        WithErrors::new(self, errors)
    }
}

pub struct ErrorsIter {
    stack: Vec<Errors>,
}

impl IntoIterator for Errors {
    type Item = TypeError;

    type IntoIter = ErrorsIter;

    fn into_iter(self) -> Self::IntoIter {
        let stack = match self {
            Errors::AllGood => Vec::new(),
            Errors::Single(error) => Vec::from([Errors::Single(error)]),
            Errors::Many(vec) => vec,
        };
        ErrorsIter { stack }
    }
}

impl Iterator for ErrorsIter {
    type Item = TypeError;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(top) = self.stack.pop() {
            match top {
                Errors::AllGood => {}
                Errors::Single(error) => {
                    return Some(*error);
                }
                Errors::Many(vec) => {
                    self.stack.extend(vec.into_iter().rev());
                }
            }
        }
        None
    }
}
