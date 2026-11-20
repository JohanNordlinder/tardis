//! Läser och skriver flexfiler: en vanlig textfil per månad.
//!
//! ```text
//! # Kommentar
//! ingående: +2:15
//! normtid: 7:57
//! flex_start: 0:00
//! flex_slut: 24:00
//! min_lunch: 0:30
//!
//! # datum      började slutade lunch_bö lunch_sl normtid
//! 2026-11-02   08:00   17:00   12:00    12:45
//! 2026-11-03   07:30   16:15
//! 2026-11-05   08:00   12:29   -        -        3:59
//! ```
//! `-` betyder tomt fält. Saknade kolumner i slutet räknas som tomma.

use crate::berakning::{Dag, Installningar};
use crate::tid::Tid;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Flexfil {
    pub installningar: Installningar,
    pub dagar: Vec<Dag>,
}

fn falt(s: Option<&str>) -> Result<Option<Tid>, String> {
    match s {
        None | Some("-") | Some("") => Ok(None),
        Some(v) => Tid::tolka(v).map(Some),
    }
}

fn falt_text(t: Option<Tid>) -> String {
    t.map_or_else(|| "-".to_string(), |t| format!("{:0>5}", t.to_string()))
}

impl Flexfil {
    pub fn tolka(text: &str) -> Result<Flexfil, String> {
        let mut fil = Flexfil::default();
        for (nr, rad) in text.lines().enumerate() {
            let fel = |e: String| format!("rad {}: {e}", nr + 1);
            let (rad, notering) = match rad.split_once('#') {
                Some((r, n)) => (r.trim(), Some(n.trim()).filter(|n| !n.is_empty())),
                None => (rad.trim(), None),
            };
            if rad.is_empty() {
                continue;
            }
            if let Some((nyckel, varde)) = rad
                .split_once(':')
                .filter(|(k, _)| k.chars().all(|c| c.is_alphabetic() || c == '_'))
            {
                let tid = Tid::tolka(varde).map_err(fel)?;
                let inst = &mut fil.installningar;
                match nyckel.trim().to_lowercase().as_str() {
                    "ingående" | "ingaende" | "saldo" => inst.ingaende_saldo = tid,
                    "normtid" => inst.normtid = tid,
                    "flex_start" => inst.flex_start = tid,
                    "flex_slut" => inst.flex_slut = tid,
                    "min_lunch" => inst.min_lunch = tid,
                    annat => return Err(fel(format!("okänd inställning \"{annat}\""))),
                }
                continue;
            }
            let mut delar = rad.split_whitespace();
            let datum = delar.next().unwrap_or_default().to_string();
            let dag = Dag {
                datum,
                borjade: falt(delar.next()).map_err(fel)?,
                slutade: falt(delar.next()).map_err(fel)?,
                lunch_borjade: falt(delar.next()).map_err(fel)?,
                lunch_slutade: falt(delar.next()).map_err(fel)?,
                normtid: falt(delar.next()).map_err(fel)?,
                notering: notering.map(str::to_string),
            };
            if delar.next().is_some() {
                return Err(fel("för många kolumner".into()));
            }
            fil.dagar.push(dag);
        }
        Ok(fil)
    }

    pub fn till_text(&self) -> String {
        let i = &self.installningar;
        let mut ut = String::new();
        ut.push_str("# Tardis flexfil. Tider anges tt:mm, '-' betyder tomt.\n");
        ut.push_str(&format!("ingående: {}\n", i.ingaende_saldo.med_tecken()));
        ut.push_str(&format!("normtid: {}\n", i.normtid));
        ut.push_str(&format!("flex_start: {}\n", i.flex_start));
        ut.push_str(&format!("flex_slut: {}\n", i.flex_slut));
        ut.push_str(&format!("min_lunch: {}\n\n", i.min_lunch));
        ut.push_str("# datum      började slutade lunch_bö lunch_sl normtid\n");
        for d in &self.dagar {
            let mut kol = vec![
                format!("{:<12}", d.datum),
                format!("{:<7}", falt_text(d.borjade)),
                format!("{:<7}", falt_text(d.slutade)),
                format!("{:<8}", falt_text(d.lunch_borjade)),
                format!("{:<8}", falt_text(d.lunch_slutade)),
            ];
            if d.normtid.is_some() || d.notering.is_some() {
                kol.push(format!("{:<7}", falt_text(d.normtid)));
            }
            if let Some(n) = &d.notering {
                kol.push(format!("# {n}"));
            }
            ut.push_str(kol.join(" ").trim_end());
            ut.push('\n');
        }
        ut
    }

    /// Lägger till eller ersätter dagen med samma datum och sorterar raderna.
    /// En befintlig rads normtid och notering behålls om den nya saknar dem.
    pub fn satt_dag(&mut self, mut dag: Dag) {
        match self.dagar.iter_mut().find(|d| d.datum == dag.datum) {
            Some(befintlig) => {
                dag.normtid = dag.normtid.or(befintlig.normtid);
                dag.notering = dag.notering.or(befintlig.notering.take());
                *befintlig = dag;
            }
            None => self.dagar.push(dag),
        }
        self.dagar.sort_by(|a, b| a.datum.cmp(&b.datum));
    }
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn tolka_och_skriv_tillbaka() {
        let text = "ingående: -1:15\nnormtid: 7:57\n\n\
                    2026-11-02 08:00 17:00 12:00 12:45\n\
                    2026-11-03 07:30 16:15 # kommentar\n\
                    2026-11-04 - - - - 0:00 # helgdag\n\
                    2026-11-05 08:00 12:29 - - 3:59\n\
                    2026-11-06\n";
        let fil = Flexfil::tolka(text).unwrap();
        assert_eq!(fil.installningar.ingaende_saldo, Tid(-75));
        assert_eq!(fil.dagar.len(), 5);
        assert_eq!(fil.dagar[1].notering.as_deref(), Some("kommentar"));
        assert_eq!(fil.dagar[3].normtid, Some(Tid(239)));
        assert_eq!(fil.dagar[4].borjade, None);
        assert_eq!(Flexfil::tolka(&fil.till_text()).unwrap(), fil);
    }

    #[test]
    fn felaktig_rad_ger_radnummer() {
        let fel = Flexfil::tolka("\n2026-11-02 08:00 25:99\n").unwrap_err();
        assert!(fel.starts_with("rad 2:"), "{fel}");
    }
}

impl Flexfil {
    /// Skapar en ny månadsfil med en rad per vardag. Helgdagar som infaller
    /// på en vardag får normtid 0:00 och en notering.
    pub fn ny_manad(manad: crate::datum::Manad, installningar: Installningar) -> Flexfil {
        use crate::datum::{helgdag, veckodag};
        let dagar = (1..=manad.antal_dagar())
            .filter(|&d| veckodag(manad.ar, manad.manad, d) < 5)
            .map(|d| {
                let helg = helgdag(manad.ar, manad.manad, d);
                Dag {
                    datum: format!("{manad}-{d:02}"),
                    normtid: helg.map(|_| Tid::NOLL),
                    notering: helg.map(str::to_string),
                    ..Default::default()
                }
            })
            .collect();
        Flexfil {
            installningar,
            dagar,
        }
    }
}

#[cfg(test)]
mod tester_ny_manad {
    use super::*;
    use crate::datum::Manad;

    #[test]
    fn december_2026() {
        let fil = Flexfil::ny_manad(Manad::tolka("2026-12").unwrap(), Installningar::default());
        // 23 vardagar i december 2026.
        assert_eq!(fil.dagar.len(), 23);
        let jul = fil.dagar.iter().find(|d| d.datum == "2026-12-24").unwrap();
        assert_eq!(jul.normtid, Some(Tid::NOLL));
        assert_eq!(jul.notering.as_deref(), Some("julafton"));
        assert!(fil.dagar.iter().all(|d| d.datum != "2026-12-05"));
    }
}
