//! Tid i hela minuter, med tolkning och utskrift i formatet tt:mm.

use std::fmt;
use std::ops::{Add, AddAssign, Neg, Sub};

/// En tidslängd eller ett klockslag, lagrat som (möjligen negativa) minuter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Tid(pub i32);

impl Tid {
    pub const NOLL: Tid = Tid(0);

    pub fn tt_mm(timmar: i32, minuter: i32) -> Tid {
        Tid(timmar * 60 + minuter)
    }

    /// Tolkar `tt:mm`, `t:mm`, `tt.mm` eller `tt` (hela timmar).
    /// Ett ledande `+` eller `-` tillåts för saldon.
    pub fn tolka(text: &str) -> Result<Tid, String> {
        let s = text.trim();
        let (tecken, s) = match s.strip_prefix('-') {
            Some(rest) => (-1, rest),
            None => (1, s.strip_prefix('+').unwrap_or(s)),
        };
        let fel = || format!("ogiltig tid \"{text}\" (använd tt:mm, t.ex. 08:15)");
        let (t, m) = match s.split_once([':', '.']) {
            Some((t, m)) => (t, m),
            None => (s, "0"),
        };
        if t.is_empty() || m.is_empty() || m.len() > 2 {
            return Err(fel());
        }
        let t: i32 = t.parse().map_err(|_| fel())?;
        let m: i32 = m.parse().map_err(|_| fel())?;
        if t < 0 || !(0..60).contains(&m) {
            return Err(fel());
        }
        Ok(Tid(tecken * (t * 60 + m)))
    }

    /// Timmar som decimaltal (hundradelstid), som mallens kolumn "Agressotid".
    pub fn hundradelar(self) -> f64 {
        self.0 as f64 / 60.0
    }

    pub fn max(self, annan: Tid) -> Tid {
        Tid(self.0.max(annan.0))
    }

    pub fn min(self, annan: Tid) -> Tid {
        Tid(self.0.min(annan.0))
    }

    /// Formaterar med tecken, t.ex. `+1:05` eller `-0:30`.
    pub fn med_tecken(self) -> String {
        if self.0 < 0 {
            format!("{self}")
        } else {
            format!("+{self}")
        }
    }
}

impl fmt::Display for Tid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tecken = if self.0 < 0 { "-" } else { "" };
        let a = self.0.abs();
        let s = format!("{tecken}{}:{:02}", a / 60, a % 60);
        f.pad(&s)
    }
}

impl Add for Tid {
    type Output = Tid;
    fn add(self, o: Tid) -> Tid {
        Tid(self.0 + o.0)
    }
}

impl AddAssign for Tid {
    fn add_assign(&mut self, o: Tid) {
        self.0 += o.0;
    }
}

impl Sub for Tid {
    type Output = Tid;
    fn sub(self, o: Tid) -> Tid {
        Tid(self.0 - o.0)
    }
}

impl Neg for Tid {
    type Output = Tid;
    fn neg(self) -> Tid {
        Tid(-self.0)
    }
}

impl std::iter::Sum for Tid {
    fn sum<I: Iterator<Item = Tid>>(iter: I) -> Tid {
        iter.fold(Tid::NOLL, |a, b| a + b)
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn tolkning() {
        assert_eq!(Tid::tolka("08:15"), Ok(Tid(495)));
        assert_eq!(Tid::tolka("7:57"), Ok(Tid(477)));
        assert_eq!(Tid::tolka("7.57"), Ok(Tid(477)));
        assert_eq!(Tid::tolka("8"), Ok(Tid(480)));
        assert_eq!(Tid::tolka("-1:30"), Ok(Tid(-90)));
        assert_eq!(Tid::tolka("+0:05"), Ok(Tid(5)));
        assert_eq!(Tid::tolka("24:00"), Ok(Tid(1440)));
        assert!(Tid::tolka("8:60").is_err());
        assert!(Tid::tolka("abc").is_err());
        assert!(Tid::tolka("8:").is_err());
    }

    #[test]
    fn utskrift() {
        assert_eq!(Tid(477).to_string(), "7:57");
        assert_eq!(Tid(-5).to_string(), "-0:05");
        assert_eq!(Tid(65).med_tecken(), "+1:05");
        assert_eq!(format!("{:>6}", Tid(65)), "  1:05");
    }
}
