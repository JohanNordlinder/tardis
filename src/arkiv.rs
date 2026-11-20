//! Månadsfilerna på disk: `flex-ÅÅÅÅ-MM.txt` i en mapp.

use crate::berakning::{Installningar, flexsaldo};
use crate::datum::Manad;
use crate::fil::Flexfil;
use crate::tid::Tid;
use std::path::{Path, PathBuf};

pub fn las(sokvag: &Path) -> Result<Flexfil, String> {
    let text = std::fs::read_to_string(sokvag)
        .map_err(|e| format!("kunde inte läsa {}: {e}", sokvag.display()))?;
    Flexfil::tolka(&text).map_err(|e| format!("{}: {e}", sokvag.display()))
}

pub fn skriv(sokvag: &Path, fil: &Flexfil) -> Result<(), String> {
    std::fs::write(sokvag, fil.till_text())
        .map_err(|e| format!("kunde inte skriva {}: {e}", sokvag.display()))
}

/// Skriver en ny fil, men vägrar skriva över en befintlig.
pub fn skriv_ny(sokvag: &Path, fil: &Flexfil) -> Result<(), String> {
    if sokvag.exists() {
        return Err(format!("{} finns redan", sokvag.display()));
    }
    skriv(sokvag, fil)
}

/// Alla månadsfiler i mappen, sorterade äldst först.
pub fn manadsfiler(mapp: &Path) -> Vec<(Manad, PathBuf)> {
    let mut filer: Vec<_> = std::fs::read_dir(mapp)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let namn = e.file_name().to_string_lossy().into_owned();
            let m = Manad::hitta_i(&namn)?;
            (namn == m.filnamn()).then(|| (m, e.path()))
        })
        .collect();
    filer.sort();
    filer
}

/// Närmast föregående månadsfil före `manad`, om någon finns.
pub fn foregaende(mapp: &Path, manad: Manad) -> Option<(Manad, PathBuf)> {
    manadsfiler(mapp)
        .into_iter()
        .rev()
        .find(|(m, _)| *m < manad)
}

/// Utgående flexsaldo för en månadsfil.
pub fn utgaende_saldo(sokvag: &Path) -> Result<Tid, String> {
    let fil = las(sokvag)?;
    Ok(flexsaldo(&fil.dagar, &fil.installningar))
}

/// Skapar filen för `manad` i `mapp`. Finns en tidigare månad förs dess
/// flexsaldo och inställningar över; annars används standardinställningar.
/// Returnerar sökvägen, ingående saldo och vilken fil saldot kom från.
pub fn skapa_manad(mapp: &Path, manad: Manad) -> Result<(PathBuf, Tid, Option<PathBuf>), String> {
    let (inst, fran) = match foregaende(mapp, manad) {
        Some((_, fran)) => {
            let gammal = las(&fran)?;
            let saldo = flexsaldo(&gammal.dagar, &gammal.installningar);
            (
                Installningar {
                    ingaende_saldo: saldo,
                    ..gammal.installningar
                },
                Some(fran),
            )
        }
        None => (Installningar::default(), None),
    };
    let sokvag = mapp.join(manad.filnamn());
    let saldo = inst.ingaende_saldo;
    skriv_ny(&sokvag, &Flexfil::ny_manad(manad, inst))?;
    Ok((sokvag, saldo, fran))
}

#[cfg(test)]
mod tester {
    use super::*;

    #[test]
    fn nasta_manad_far_saldot() {
        let mapp = std::env::temp_dir().join(format!("tardis-test-{}", std::process::id()));
        std::fs::create_dir_all(&mapp).unwrap();
        let nov = Manad::tolka("2026-11").unwrap();
        let mut fil = Flexfil::ny_manad(nov, Installningar::default());
        fil.dagar[0].borjade = Tid::tolka("08:00").ok();
        fil.dagar[0].slutade = Tid::tolka("17:00").ok(); // +0:33
        skriv(&mapp.join(nov.filnamn()), &fil).unwrap();

        let (sokvag, saldo, fran) = skapa_manad(&mapp, nov.nasta()).unwrap();
        assert_eq!(saldo, Tid(33));
        assert!(fran.is_some());
        assert_eq!(las(&sokvag).unwrap().installningar.ingaende_saldo, Tid(33));
        assert!(
            skapa_manad(&mapp, nov.nasta()).is_err(),
            "får inte skriva över"
        );
        std::fs::remove_dir_all(&mapp).unwrap();
    }
}
