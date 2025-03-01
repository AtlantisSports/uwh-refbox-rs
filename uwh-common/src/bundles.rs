use crate::color::Color;
use core::ops::{Index, IndexMut};
use derivative::Derivative;
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use std::fmt::{Display, Formatter};

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlackWhiteBundle<T> {
    pub black: T,
    pub white: T,
}

impl<T> BlackWhiteBundle<T> {
    pub fn iter(&self) -> impl Iterator<Item = (Color, &T)> {
        self.into_iter()
    }
}

impl<T: Eq> BlackWhiteBundle<T> {
    pub fn are_not_equal(&self) -> bool {
        self.black != self.white
    }
}

impl<T> Index<Color> for BlackWhiteBundle<T> {
    type Output = T;

    fn index(&self, color: Color) -> &Self::Output {
        match color {
            Color::Black => &self.black,
            Color::White => &self.white,
        }
    }
}

impl<T> IndexMut<Color> for BlackWhiteBundle<T> {
    fn index_mut(&mut self, color: Color) -> &mut Self::Output {
        match color {
            Color::Black => &mut self.black,
            Color::White => &mut self.white,
        }
    }
}

#[cfg(feature = "std")]
impl<T: Display> Display for BlackWhiteBundle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Black: {}, White: {}", self.black, self.white)
    }
}

pub struct BlackWhiteBundleIterator<'a, T> {
    bundle: &'a BlackWhiteBundle<T>,
    index: usize,
}

impl<'a, T> Iterator for BlackWhiteBundleIterator<'a, T> {
    type Item = (Color, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let value = match self.index {
            0 => (Color::Black, &self.bundle.black),
            1 => (Color::White, &self.bundle.white),
            _ => return None,
        };

        self.index += 1;
        Some(value)
    }
}

impl<'a, T> IntoIterator for &'a BlackWhiteBundle<T> {
    type Item = (Color, &'a T);
    type IntoIter = BlackWhiteBundleIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        BlackWhiteBundleIterator {
            bundle: self,
            index: 0,
        }
    }
}

impl<T> IntoIterator for BlackWhiteBundle<T> {
    type Item = (Color, T);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        vec![(Color::Black, self.black), (Color::White, self.white)].into_iter()
    }
}

impl<T: Default> FromIterator<(Color, T)> for BlackWhiteBundle<T> {
    fn from_iter<I: IntoIterator<Item = (Color, T)>>(iter: I) -> Self {
        let mut bundle = BlackWhiteBundle::default();
        for (color, value) in iter {
            match color {
                Color::Black => bundle.black = value,
                Color::White => bundle.white = value,
            }
        }
        bundle
    }
}

impl<T> BlackWhiteBundle<Option<T>> {
    pub fn complete(self) -> Option<BlackWhiteBundle<T>> {
        Some(BlackWhiteBundle {
            black: self.black?,
            white: self.white?,
        })
    }
}

impl<T, E> BlackWhiteBundle<Result<T, E>> {
    pub fn complete(self) -> Result<BlackWhiteBundle<T>, E> {
        Ok(BlackWhiteBundle {
            black: self.black?,
            white: self.white?,
        })
    }
}

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptColorBundle<T> {
    pub black: T,
    pub equal: T,
    pub white: T,
}

impl<T> OptColorBundle<T> {
    pub fn iter(&self) -> impl Iterator<Item = (Option<Color>, &T)> {
        self.into_iter()
    }
}

impl<T> Index<Option<Color>> for OptColorBundle<T> {
    type Output = T;

    fn index(&self, color: Option<Color>) -> &Self::Output {
        match color {
            Some(Color::Black) => &self.black,
            None => &self.equal,
            Some(Color::White) => &self.white,
        }
    }
}

impl<T> IndexMut<Option<Color>> for OptColorBundle<T> {
    fn index_mut(&mut self, color: Option<Color>) -> &mut Self::Output {
        match color {
            Some(Color::Black) => &mut self.black,
            None => &mut self.equal,
            Some(Color::White) => &mut self.white,
        }
    }
}

#[cfg(feature = "std")]
impl<T: Display> Display for OptColorBundle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Black: {}, White: {}, Equal: {}",
            self.black, self.white, self.equal
        )
    }
}

pub struct OptColorBundleIterator<'a, T> {
    bundle: &'a OptColorBundle<T>,
    index: usize,
}

impl<'a, T> Iterator for OptColorBundleIterator<'a, T> {
    type Item = (Option<Color>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let value = match self.index {
            0 => (Some(Color::Black), &self.bundle.black),
            1 => (None, &self.bundle.equal),
            2 => (Some(Color::White), &self.bundle.white),
            _ => return None,
        };

        self.index += 1;
        Some(value)
    }
}

impl<'a, T> IntoIterator for &'a OptColorBundle<T> {
    type Item = (Option<Color>, &'a T);
    type IntoIter = OptColorBundleIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        OptColorBundleIterator {
            bundle: self,
            index: 0,
        }
    }
}

impl<T> IntoIterator for OptColorBundle<T> {
    type Item = (Option<Color>, T);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            (Some(Color::Black), self.black),
            (None, self.equal),
            (Some(Color::White), self.white),
        ]
        .into_iter()
    }
}

impl<T: Default> FromIterator<(Option<Color>, T)> for OptColorBundle<T> {
    fn from_iter<I: IntoIterator<Item = (Option<Color>, T)>>(iter: I) -> Self {
        let mut bundle = OptColorBundle::default();
        for (color, value) in iter {
            match color {
                Some(Color::Black) => bundle.black = value,
                None => bundle.equal = value,
                Some(Color::White) => bundle.white = value,
            }
        }
        bundle
    }
}

impl<T> OptColorBundle<Option<T>> {
    pub fn complete(self) -> Option<OptColorBundle<T>> {
        Some(OptColorBundle {
            black: self.black?,
            equal: self.equal?,
            white: self.white?,
        })
    }
}

impl<T, E> OptColorBundle<Result<T, E>> {
    pub fn complete(self) -> Result<OptColorBundle<T>, E> {
        Ok(OptColorBundle {
            black: self.black?,
            equal: self.equal?,
            white: self.white?,
        })
    }
}
