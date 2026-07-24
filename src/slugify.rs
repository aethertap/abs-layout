use slugify::slugify;

pub trait Slugify:AsRef<str> {
    fn slugify(&self)->String {
        slugify::slugify!(self.as_ref()).replace("-s-", "s-")
    }
}

impl Slugify for String{}



