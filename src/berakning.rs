//! Flexberäkningen, samma regler som Excel-mallen "Mall beräkning flex".

use crate::tid::Tid;

/// Inställningar som i mallens rutor E38–E41 och I2–I3.
#[derive(Debug, Clone, PartialEq)]
pub struct Installningar {
    /// Ingående flexsaldo (positivt eller negativt).
    pub ingaende_saldo: Tid,
    /// Normtid per dag om inget annat anges (7:57 vid heltid).
    pub normtid: Tid,
    /// Flextiden börjar vid detta klockslag; tid innan räknas inte.
    pub flex_start: Tid,
    /// Flextiden slutar vid detta klockslag; tid efter räknas inte.
    pub flex_slut: Tid,
    /// Minimal längd på lunchen.
    pub min_lunch: Tid,
}

impl Default for Installningar {
    fn default() -> Self {
        Installningar {
            ingaende_saldo: Tid::NOLL,
            normtid: Tid::tt_mm(7, 57),
            flex_start: Tid::tt_mm(0, 0),
            flex_slut: Tid::tt_mm(24, 0),
            min_lunch: Tid::tt_mm(0, 30),
        }
    }
}

/// En dagsrad. Tomma fält motsvarar tomma celler i mallen.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dag {
    pub datum: String,
    pub borjade: Option<Tid>,
    pub slutade: Option<Tid>,
    pub lunch_borjade: Option<Tid>,
    pub lunch_slutade: Option<Tid>,
    /// Normtid för just denna dag (t.ex. 3:59 vid arbetstidsförkortning).
    pub normtid: Option<Tid>,
    /// Fri text efter `#` på raden, t.ex. "julafton".
    pub notering: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resultat {
    pub lunch: Tid,
    pub arbetad: Tid,
    pub normtid: Tid,
    pub flex: Tid,
}

impl Dag {
    pub fn berakna(&self, inst: &Installningar) -> Resultat {
        let normtid = self.normtid.unwrap_or(inst.normtid);
        let b = self.borjade.unwrap_or(Tid::NOLL);
        let c = self.slutade.unwrap_or(Tid::NOLL);
        let lunch_angiven =
            self.lunch_slutade.unwrap_or(Tid::NOLL) - self.lunch_borjade.unwrap_or(Tid::NOLL);

        // Kolumn F: arbetad tid. Lunch räknas med minst `min_lunch`, och bara
        // tid inom flexramen (flex_start–flex_slut) räknas.
        let (lunch, arbetad) = if c - b > Tid::NOLL {
            let lunch = if lunch_angiven < inst.min_lunch {
                inst.min_lunch
            } else {
                lunch_angiven
            };
            let slut = c.min(inst.flex_slut);
            let start = b.max(inst.flex_start);
            (lunch, slut - start - lunch)
        } else {
            (Tid::NOLL, Tid::NOLL)
        };

        // Kolumn I: flextid, bara för dagar med arbetad tid.
        let flex = if arbetad == Tid::NOLL {
            Tid::NOLL
        } else {
            arbetad - normtid
        };

        Resultat {
            lunch,
            arbetad,
            normtid,
            flex,
        }
    }
}

/// Ruta I37: summan av dagarnas flex korrigerad med ingående saldo.
pub fn flexsaldo(dagar: &[Dag], inst: &Installningar) -> Tid {
    dagar.iter().map(|d| d.berakna(inst).flex).sum::<Tid>() + inst.ingaende_saldo
}

#[cfg(test)]
mod tester {
    use super::*;

    fn t(s: &str) -> Tid {
        Tid::tolka(s).unwrap()
    }

    fn dag(b: &str, c: &str, lb: Option<&str>, ls: Option<&str>) -> Dag {
        Dag {
            datum: "1".into(),
            borjade: Some(t(b)),
            slutade: Some(t(c)),
            lunch_borjade: lb.map(t),
            lunch_slutade: ls.map(t),
            normtid: None,
            notering: None,
        }
    }

    #[test]
    fn utan_lunch_dras_minst_30_min() {
        let r = dag("08:00", "17:00", None, None).berakna(&Installningar::default());
        assert_eq!(r.arbetad, t("8:30"));
        assert_eq!(r.flex, t("0:33"));
    }

    #[test]
    fn kort_lunch_avrundas_upp_till_30_min() {
        let r =
            dag("08:00", "16:27", Some("12:00"), Some("12:15")).berakna(&Installningar::default());
        assert_eq!(r.arbetad, t("7:57"));
        assert_eq!(r.flex, Tid::NOLL);
    }

    #[test]
    fn lang_lunch_dras_fullt() {
        let r =
            dag("07:30", "16:00", Some("11:30"), Some("12:30")).berakna(&Installningar::default());
        assert_eq!(r.arbetad, t("7:30"));
        assert_eq!(r.flex, t("-0:27"));
    }

    #[test]
    fn tom_dag_ger_ingen_flex() {
        let r = Dag::default().berakna(&Installningar::default());
        assert_eq!(r.arbetad, Tid::NOLL);
        assert_eq!(r.flex, Tid::NOLL);
    }

    #[test]
    fn flexram_begransar() {
        let inst = Installningar {
            flex_start: t("07:00"),
            flex_slut: t("19:00"),
            ..Default::default()
        };
        let r = dag("06:00", "20:00", None, None).berakna(&inst);
        assert_eq!(r.arbetad, t("11:30"));
    }

    #[test]
    fn dagens_normtid_gar_fore() {
        let mut d = dag("08:00", "12:29", None, None);
        d.normtid = Some(t("3:59"));
        let r = d.berakna(&Installningar::default());
        assert_eq!(r.arbetad, t("3:59"));
        assert_eq!(r.flex, Tid::NOLL);
    }

    #[test]
    fn saldo_med_ingaende() {
        let inst = Installningar {
            ingaende_saldo: t("-1:00"),
            ..Default::default()
        };
        let dagar = vec![
            dag("08:00", "17:00", None, None),                   // +0:33
            dag("07:30", "16:00", Some("11:30"), Some("12:30")), // -0:27
        ];
        assert_eq!(flexsaldo(&dagar, &inst), t("-0:54"));
    }
}
