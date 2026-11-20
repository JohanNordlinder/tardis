//! Tardis – flextidsberäkning i terminalen.
//!
//! Gör samma beräkning som Excel-mallen "Mall beräkning flex", men sparar
//! varje månad som en läsbar textfil (`flex-ÅÅÅÅ-MM.txt`).

mod berakning;
mod datum;
mod fil;
mod tid;

use berakning::{Dag, Installningar, flexsaldo};
use datum::{Manad, VECKODAGAR, helgdag, tolka_datum, veckodag};
use fil::Flexfil;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tid::Tid;

const HJALP: &str = "\
Tardis – flextidsberäkning

ANVÄNDNING
  tardis ny <ÅÅÅÅ-MM> [--saldo ±t:mm] [--normtid t:mm]
        Skapar flex-ÅÅÅÅ-MM.txt med en rad per vardag (helgdagar får normtid 0:00).

  tardis nästa [fil]
        Skapar nästa månads fil. Månadens flexsaldo blir nästa månads ingående
        saldo, och inställningarna följer med. Utan fil används den senaste
        flex-ÅÅÅÅ-MM.txt i mappen.

  tardis in <datum|idag> <började> <slutade> [<lunch bö> <lunch sl>] [--normtid t:mm]
        Registrerar en dag i rätt månadsfil. Saknas filen skapas den automatiskt
        med saldot från föregående månad.

  tardis rapport [fil]
        Visar månadens tabell och flexsaldo (standard: senaste månaden).

  tardis dag <började> <slutade> [<lunch bö> <lunch sl>] [--normtid t:mm]
        Räknar ut en enskild dag utan att spara något.

FLAGGOR
  --mapp <mapp>   Mappen där månadsfilerna ligger (standard: aktuell mapp)

Tider skrivs tt:mm, t.ex. 08:15. Lunch räknas som minst 30 minuter; ange
lunchtider bara om lunchen var längre. Normtid är 7:57 om inget annat anges.";

struct Argument {
    positionella: Vec<String>,
    mapp: PathBuf,
    saldo: Option<Tid>,
    normtid: Option<Tid>,
}

fn tolka_argument(args: impl Iterator<Item = String>) -> Result<Argument, String> {
    let mut a = Argument {
        positionella: vec![],
        mapp: PathBuf::from("."),
        saldo: None,
        normtid: None,
    };
    let mut it = args.peekable();
    while let Some(arg) = it.next() {
        let mut varde = |namn: &str| it.next().ok_or(format!("{namn} kräver ett värde"));
        match arg.as_str() {
            "--mapp" => a.mapp = PathBuf::from(varde("--mapp")?),
            "--saldo" => a.saldo = Some(Tid::tolka(&varde("--saldo")?)?),
            "--normtid" => a.normtid = Some(Tid::tolka(&varde("--normtid")?)?),
            s if s.starts_with("--") && s.len() > 2 => return Err(format!("okänd flagga {s}")),
            _ => a.positionella.push(arg),
        }
    }
    Ok(a)
}

fn las(sokvag: &Path) -> Result<Flexfil, String> {
    let text = std::fs::read_to_string(sokvag)
        .map_err(|e| format!("kunde inte läsa {}: {e}", sokvag.display()))?;
    Flexfil::tolka(&text).map_err(|e| format!("{}: {e}", sokvag.display()))
}

fn skriv(sokvag: &Path, fil: &Flexfil) -> Result<(), String> {
    std::fs::write(sokvag, fil.till_text())
        .map_err(|e| format!("kunde inte skriva {}: {e}", sokvag.display()))
}

fn skriv_ny(sokvag: &Path, fil: &Flexfil) -> Result<(), String> {
    if sokvag.exists() {
        return Err(format!("{} finns redan", sokvag.display()));
    }
    skriv(sokvag, fil)
}

/// Alla månadsfiler i mappen, sorterade äldst först.
fn manadsfiler(mapp: &Path) -> Vec<(Manad, PathBuf)> {
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

fn senaste_fil(a: &Argument, index: usize) -> Result<PathBuf, String> {
    if let Some(f) = a.positionella.get(index) {
        return Ok(PathBuf::from(f));
    }
    manadsfiler(&a.mapp).pop().map(|(_, p)| p).ok_or_else(|| {
        format!(
            "hittade ingen flex-ÅÅÅÅ-MM.txt i {}; skapa en med `tardis ny ÅÅÅÅ-MM`",
            a.mapp.display()
        )
    })
}

fn las_dag(datum: String, tider: &[String], normtid: Option<Tid>) -> Result<Dag, String> {
    let tid = |i: usize| tider.get(i).map(|s| Tid::tolka(s)).transpose();
    if tider.len() != 2 && tider.len() != 4 {
        return Err("ange <började> <slutade> och eventuellt <lunch bö> <lunch sl>".into());
    }
    Ok(Dag {
        datum,
        borjade: tid(0)?,
        slutade: tid(1)?,
        lunch_borjade: tid(2)?,
        lunch_slutade: tid(3)?,
        normtid,
        notering: None,
    })
}

/// Skapar månadsfilen för `manad` utifrån `fran` (föregående månads fil).
fn skapa_nasta(fran: &Path, manad: Manad, mapp: &Path) -> Result<PathBuf, String> {
    let gammal = las(fran)?;
    let saldo = flexsaldo(&gammal.dagar, &gammal.installningar);
    let inst = Installningar {
        ingaende_saldo: saldo,
        ..gammal.installningar
    };
    let sokvag = mapp.join(manad.filnamn());
    skriv_ny(&sokvag, &Flexfil::ny_manad(manad, inst))?;
    println!(
        "Skapade {} med ingående saldo {} (från {}).",
        sokvag.display(),
        saldo.med_tecken(),
        fran.display()
    );
    Ok(sokvag)
}

fn kor(a: Argument) -> Result<(), String> {
    let kommando = a
        .positionella
        .first()
        .map(String::as_str)
        .unwrap_or("hjälp");
    match kommando {
        "ny" => {
            let m = Manad::tolka(a.positionella.get(1).ok_or("ange månad, t.ex. 2026-11")?)?;
            let inst = Installningar {
                ingaende_saldo: a.saldo.unwrap_or(Tid::NOLL),
                normtid: a.normtid.unwrap_or(Installningar::default().normtid),
                ..Default::default()
            };
            let sokvag = a.mapp.join(m.filnamn());
            skriv_ny(&sokvag, &Flexfil::ny_manad(m, inst))?;
            println!("Skapade {}.", sokvag.display());
        }
        "nästa" | "nasta" => {
            let fran = senaste_fil(&a, 1)?;
            let m = Manad::hitta_i(&fran.to_string_lossy())
                .ok_or("filnamnet måste innehålla månaden, t.ex. flex-2026-11.txt")?;
            let mapp = fran
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(&a.mapp);
            skapa_nasta(&fran, m.nasta(), mapp)?;
        }
        "in" => {
            let datum_arg = a
                .positionella
                .get(1)
                .ok_or("ange datum, t.ex. idag eller 2026-11-20")?;
            let (ar, man, d) = tolka_datum(datum_arg)?;
            let manad = Manad { ar, manad: man };
            let mut dag = las_dag(format!("{manad}-{d:02}"), &a.positionella[2..], a.normtid)?;
            let ledig = veckodag(ar, man, d) >= 5 || helgdag(ar, man, d).is_some();
            if ledig && dag.normtid.is_none() {
                dag.normtid = Some(Tid::NOLL);
                dag.notering = Some(
                    helgdag(ar, man, d)
                        .unwrap_or(VECKODAGAR[veckodag(ar, man, d) as usize])
                        .into(),
                );
            }
            let sokvag = a.mapp.join(manad.filnamn());
            if !sokvag.exists() {
                let tidigare = manadsfiler(&a.mapp)
                    .into_iter()
                    .rev()
                    .find(|(m, _)| *m < manad);
                match tidigare {
                    Some((_, fran)) => {
                        skapa_nasta(&fran, manad, &a.mapp)?;
                    }
                    None => {
                        skriv_ny(&sokvag, &Flexfil::ny_manad(manad, Installningar::default()))?;
                        println!("Skapade {} (ingående saldo 0:00).", sokvag.display());
                    }
                }
            }
            let mut fil = las(&sokvag)?;
            fil.satt_dag(dag.clone());
            skriv(&sokvag, &fil)?;
            let r = dag.berakna(&fil.installningar);
            println!(
                "{}: arbetad tid {}, flex {}. Månadens saldo: {}",
                dag.datum,
                r.arbetad,
                r.flex.med_tecken(),
                flexsaldo(&fil.dagar, &fil.installningar).med_tecken()
            );
        }
        "rapport" => {
            let sokvag = senaste_fil(&a, 1)?;
            rapport(&sokvag, &las(&sokvag)?);
        }
        "dag" => {
            let dag = las_dag(String::new(), &a.positionella[1..], a.normtid)?;
            let r = dag.berakna(&Installningar::default());
            println!("Lunch:      {}", r.lunch);
            println!("Arbetad:    {}", r.arbetad);
            println!("Normtid:    {}", r.normtid);
            println!("Flex:       {}", r.flex.med_tecken());
            println!("Agressotid: {:.2}", r.arbetad.hundradelar());
        }
        "hjälp" | "hjalp" | "help" | "-h" | "--help" => println!("{HJALP}"),
        annat => return Err(format!("okänt kommando \"{annat}\"; kör `tardis hjälp`")),
    }
    Ok(())
}

fn rapport(sokvag: &Path, fil: &Flexfil) {
    let inst = &fil.installningar;
    let tom = |t: Option<Tid>| t.map_or(String::new(), |t| t.to_string());
    println!("{}\n", sokvag.display());
    println!(
        "{:<12} {:>7} {:>7} {:>6} {:>7} {:>7} {:>7} {:>8}",
        "Datum", "Började", "Slutade", "Lunch", "Arb tid", "Normtid", "Flex", "Agresso"
    );
    println!("{}", "-".repeat(71));
    let mut arbetat = Tid::NOLL;
    for d in &fil.dagar {
        let r = d.berakna(inst);
        arbetat += r.arbetad;
        let jobbat = r.arbetad != Tid::NOLL;
        let visa = |t: Tid| if jobbat { t.to_string() } else { String::new() };
        let mut rad = format!(
            "{:<12} {:>7} {:>7} {:>6} {:>7} {:>7} {:>7} {:>8}",
            d.datum,
            tom(d.borjade),
            tom(d.slutade),
            visa(r.lunch),
            visa(r.arbetad),
            r.normtid.to_string(),
            if jobbat {
                r.flex.med_tecken()
            } else {
                String::new()
            },
            if jobbat {
                format!("{:.2}", r.arbetad.hundradelar())
            } else {
                String::new()
            },
        );
        if let Some(n) = &d.notering {
            rad.push_str(&format!("  {n}"));
        }
        println!("{}", rad.trim_end());
    }
    println!("{}", "-".repeat(71));
    let saldo = flexsaldo(&fil.dagar, inst);
    println!("Arbetad tid totalt: {:>8}", arbetat.to_string());
    println!(
        "Månadens flex:      {:>8}",
        (saldo - inst.ingaende_saldo).med_tecken()
    );
    println!(
        "Ingående saldo:     {:>8}",
        inst.ingaende_saldo.med_tecken()
    );
    println!("FLEXSALDO:          {:>8}", saldo.med_tecken());
}

fn main() -> ExitCode {
    match tolka_argument(std::env::args().skip(1)).and_then(kor) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fel: {e}");
            ExitCode::FAILURE
        }
    }
}
