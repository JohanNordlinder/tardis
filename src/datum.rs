//! Enkel kalenderhantering: månader, veckodagar och svenska helgdagar.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Manad {
    pub ar: i32,
    pub manad: u32,
}

impl Manad {
    /// Tolkar `ÅÅÅÅ-MM`.
    pub fn tolka(s: &str) -> Result<Manad, String> {
        let fel = || format!("ogiltig månad \"{s}\" (använd ÅÅÅÅ-MM, t.ex. 2026-11)");
        let (a, m) = s.trim().split_once('-').ok_or_else(fel)?;
        let ar: i32 = a.parse().map_err(|_| fel())?;
        let manad: u32 = m.parse().map_err(|_| fel())?;
        if !(1..=12).contains(&manad) || !(1900..=9999).contains(&ar) {
            return Err(fel());
        }
        Ok(Manad { ar, manad })
    }

    /// Hittar en månad `ÅÅÅÅ-MM` var som helst i en text, t.ex. ett filnamn.
    pub fn hitta_i(text: &str) -> Option<Manad> {
        let b = text.as_bytes();
        (0..b.len().saturating_sub(6)).find_map(|i| {
            let del = text.get(i..i + 7)?;
            let ok = del.bytes().enumerate().all(|(j, c)| {
                if j == 4 {
                    c == b'-'
                } else {
                    c.is_ascii_digit()
                }
            });
            let fore_ok = i == 0 || !b[i - 1].is_ascii_digit();
            let efter_ok = b.get(i + 7).is_none_or(|c| !c.is_ascii_digit());
            if ok && fore_ok && efter_ok {
                Manad::tolka(del).ok()
            } else {
                None
            }
        })
    }

    pub fn nasta(self) -> Manad {
        if self.manad == 12 {
            Manad {
                ar: self.ar + 1,
                manad: 1,
            }
        } else {
            Manad {
                ar: self.ar,
                manad: self.manad + 1,
            }
        }
    }

    pub fn antal_dagar(self) -> u32 {
        match self.manad {
            4 | 6 | 9 | 11 => 30,
            2 if skottar(self.ar) => 29,
            2 => 28,
            _ => 31,
        }
    }

    pub fn filnamn(self) -> String {
        format!("flex-{self}.txt")
    }
}

impl fmt::Display for Manad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.ar, self.manad)
    }
}

fn skottar(ar: i32) -> bool {
    (ar % 4 == 0 && ar % 100 != 0) || ar % 400 == 0
}

/// Dagnummer räknat från 1970-01-01 (som var en torsdag).
fn dagnummer(ar: i32, manad: u32, dag: u32) -> i64 {
    // Howard Hinnants algoritm "days_from_civil".
    let y = if manad <= 2 { ar - 1 } else { ar } as i64;
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = manad as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + dag as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Veckodag, 0 = måndag … 6 = söndag.
pub fn veckodag(ar: i32, manad: u32, dag: u32) -> u32 {
    (dagnummer(ar, manad, dag) + 3).rem_euclid(7) as u32
}

pub const VECKODAGAR: [&str; 7] = ["mån", "tis", "ons", "tor", "fre", "lör", "sön"];

/// Påskdagen (månad, dag) enligt den gregorianska algoritmen.
fn paskdagen(ar: i32) -> (u32, u32) {
    let a = ar % 19;
    let b = ar / 100;
    let c = ar % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let manad = (h + l - 7 * m + 114) / 31;
    let dag = (h + l - 7 * m + 114) % 31 + 1;
    (manad as u32, dag as u32)
}

/// Lägger till dagar till ett datum (fungerar för små förskjutningar).
fn forskjut(ar: i32, mut manad: u32, dag: u32, delta: i32) -> (u32, u32) {
    let mut d = dag as i32 + delta;
    while d < 1 {
        manad -= 1;
        d += Manad { ar, manad }.antal_dagar() as i32;
    }
    loop {
        let n = Manad { ar, manad }.antal_dagar() as i32;
        if d <= n {
            break;
        }
        d -= n;
        manad += 1;
    }
    (manad, d as u32)
}

/// Svenska helgdagar och dagar som i praktiken är lediga (midsommar-,
/// jul- och nyårsafton) som infaller på en vardag eller helg.
pub fn helgdag(ar: i32, manad: u32, dag: u32) -> Option<&'static str> {
    let fast = match (manad, dag) {
        (1, 1) => Some("nyårsdagen"),
        (1, 6) => Some("trettondedag jul"),
        (5, 1) => Some("första maj"),
        (6, 6) => Some("nationaldagen"),
        (12, 24) => Some("julafton"),
        (12, 25) => Some("juldagen"),
        (12, 26) => Some("annandag jul"),
        (12, 31) => Some("nyårsafton"),
        _ => None,
    };
    if fast.is_some() {
        return fast;
    }
    let (pm, pd) = paskdagen(ar);
    let rorliga = [
        (-2, "långfredagen"),
        (0, "påskdagen"),
        (1, "annandag påsk"),
        (39, "Kristi himmelsfärdsdag"),
    ];
    for (delta, namn) in rorliga {
        if forskjut(ar, pm, pd, delta) == (manad, dag) {
            return Some(namn);
        }
    }
    // Midsommarafton: fredagen mellan 19 och 25 juni.
    if manad == 6 && (19..=25).contains(&dag) && veckodag(ar, manad, dag) == 4 {
        return Some("midsommarafton");
    }
    None
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn veckodagar() {
        assert_eq!(veckodag(1970, 1, 1), 3); // torsdag
        assert_eq!(veckodag(2026, 11, 20), 4); // fredag
        assert_eq!(veckodag(2024, 2, 29), 3); // torsdag
    }

    #[test]
    fn helgdagar() {
        assert_eq!(paskdagen(2026), (4, 5));
        assert_eq!(paskdagen(2025), (4, 20));
        assert_eq!(helgdag(2026, 4, 3), Some("långfredagen"));
        assert_eq!(helgdag(2026, 4, 6), Some("annandag påsk"));
        assert_eq!(helgdag(2026, 5, 14), Some("Kristi himmelsfärdsdag"));
        assert_eq!(helgdag(2026, 6, 19), Some("midsommarafton"));
        assert_eq!(helgdag(2026, 11, 20), None);
    }

    #[test]
    fn manader() {
        let m = Manad::tolka("2026-12").unwrap();
        assert_eq!(m.nasta().to_string(), "2027-01");
        assert_eq!(Manad::tolka("2028-02").unwrap().antal_dagar(), 29);
        assert_eq!(
            Manad::hitta_i("mapp/flex-2026-11.txt"),
            Some(Manad {
                ar: 2026,
                manad: 11
            })
        );
        assert_eq!(Manad::hitta_i("flex.txt"), None);
    }
}

/// Dagens datum (UTC) som (år, månad, dag).
pub fn idag() -> (i32, u32, u32) {
    let sek = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    fran_dagnummer(sek.div_euclid(86_400))
}

/// Omvändningen av `dagnummer` (Hinnants "civil_from_days").
fn fran_dagnummer(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = (yoe + era * 400 + if m <= 2 { 1 } else { 0 }) as i32;
    (y, m, d)
}

/// Tolkar `ÅÅÅÅ-MM-DD` och kontrollerar att datumet finns.
pub fn tolka_datum(s: &str) -> Result<(i32, u32, u32), String> {
    let fel = || format!("ogiltigt datum \"{s}\" (använd ÅÅÅÅ-MM-DD eller \"idag\")");
    if s == "idag" {
        return Ok(idag());
    }
    let (am, d) = s.rsplit_once('-').ok_or_else(fel)?;
    let m = Manad::tolka(am).map_err(|_| fel())?;
    let d: u32 = d.parse().map_err(|_| fel())?;
    if d == 0 || d > m.antal_dagar() || s.len() != 10 {
        return Err(fel());
    }
    Ok((m.ar, m.manad, d))
}

#[cfg(test)]
mod tester_dagnummer {
    use super::*;

    #[test]
    fn fram_och_tillbaka() {
        for (a, m, d) in [(1970, 1, 1), (2000, 2, 29), (2026, 11, 20), (2100, 12, 31)] {
            assert_eq!(fran_dagnummer(dagnummer(a, m, d)), (a, m, d));
        }
        assert_eq!(tolka_datum("2026-02-28"), Ok((2026, 2, 28)));
        assert!(tolka_datum("2026-02-29").is_err());
    }
}
