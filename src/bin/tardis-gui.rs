//! Tardis – grafiskt gränssnitt för flextidsberäkningen.
//!
//! Läser och skriver samma månadsfiler (`flex-ÅÅÅÅ-MM.txt`) som
//! terminalprogrammet. Mappen väljs med första argumentet, miljövariabeln
//! `TARDIS_MAPP` eller blir annars `~/Tardis`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, RichText};
use std::path::{Path, PathBuf};
use tardis::arkiv;
use tardis::berakning::{Dag, flexsaldo};
use tardis::datum::{Manad, VECKODAGAR, helgdag, idag, tolka_datum, veckodag};
use tardis::fil::Flexfil;
use tardis::tid::Tid;

const MANADER: [&str; 12] = [
    "januari",
    "februari",
    "mars",
    "april",
    "maj",
    "juni",
    "juli",
    "augusti",
    "september",
    "oktober",
    "november",
    "december",
];

const GRON: Color32 = Color32::from_rgb(60, 160, 90);
const ROD: Color32 = Color32::from_rgb(210, 70, 60);

fn main() -> eframe::Result {
    let mapp = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("TARDIS_MAPP").ok())
        .map(PathBuf::from)
        .unwrap_or_else(standardmapp);
    let alternativ = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Tardis – flextid")
            .with_inner_size([1040.0, 760.0])
            .with_min_inner_size([760.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Tardis",
        alternativ,
        Box::new(move |_cc| Ok(Box::new(App::ny(mapp)))),
    )
}

fn standardmapp() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Tardis")
}

fn manadsnamn(m: Manad) -> String {
    format!("{} {}", MANADER[m.manad as usize - 1], m.ar)
}

fn tid_text(t: Option<Tid>) -> String {
    t.map_or_else(String::new, |t| format!("{:0>5}", t.to_string()))
}

/// Tomt fält betyder "ingen tid"; annars måste texten vara en giltig tid.
fn tolka_falt(s: &str) -> Result<Option<Tid>, ()> {
    let s = s.trim();
    if s.is_empty() || s == "-" {
        Ok(None)
    } else {
        Tid::tolka(s).map(Some).map_err(|_| ())
    }
}

/// Redigerbara textfält för en dag.
struct RadText {
    tider: [String; 5], // började, slutade, lunch bö, lunch sl, normtid
    notering: String,
}

impl RadText {
    fn fran(d: &Dag) -> RadText {
        RadText {
            tider: [
                d.borjade,
                d.slutade,
                d.lunch_borjade,
                d.lunch_slutade,
                d.normtid,
            ]
            .map(tid_text),
            notering: d.notering.clone().unwrap_or_default(),
        }
    }
}

/// Inställningar som textfält: ingående, normtid, flex start, flex slut, min lunch.
const INST_ETIKETTER: [&str; 5] = [
    "Ingående saldo",
    "Normtid",
    "Flextiden börjar",
    "Flextiden slutar",
    "Minsta lunch",
];

struct Meddelande {
    text: String,
    fel: bool,
}

struct App {
    mapp: PathBuf,
    mapp_text: String,
    manad: Manad,
    /// `None` när månadens fil inte finns ännu.
    fil: Option<Flexfil>,
    rader: Vec<RadText>,
    inst_text: [String; 5],
    /// Utgående saldo för föregående månad, om en sådan fil finns.
    foregaende_saldo: Option<(Manad, Tid)>,
    ny_dag: String,
    meddelande: Option<Meddelande>,
}

impl App {
    fn ny(mapp: PathBuf) -> App {
        let (ar, manad, _) = idag();
        let mut app = App {
            mapp_text: mapp.display().to_string(),
            mapp,
            manad: Manad { ar, manad },
            fil: None,
            rader: vec![],
            inst_text: Default::default(),
            foregaende_saldo: None,
            ny_dag: String::new(),
            meddelande: None,
        };
        app.ladda();
        app
    }

    fn sokvag(&self) -> PathBuf {
        self.mapp.join(self.manad.filnamn())
    }

    fn info(&mut self, text: impl Into<String>) {
        self.meddelande = Some(Meddelande {
            text: text.into(),
            fel: false,
        });
    }

    fn fel(&mut self, text: impl Into<String>) {
        self.meddelande = Some(Meddelande {
            text: text.into(),
            fel: true,
        });
    }

    /// Läser in den valda månaden från disk.
    fn ladda(&mut self) {
        let sokvag = self.sokvag();
        self.fil = None;
        self.rader.clear();
        if sokvag.exists() {
            match arkiv::las(&sokvag) {
                Ok(fil) => self.fil = Some(fil),
                Err(e) => self.fel(e),
            }
        }
        self.synka_text();
        self.foregaende_saldo = arkiv::foregaende(&self.mapp, self.manad)
            .and_then(|(m, p)| arkiv::utgaende_saldo(&p).ok().map(|s| (m, s)));
    }

    fn synka_text(&mut self) {
        if let Some(fil) = &self.fil {
            self.rader = fil.dagar.iter().map(RadText::fran).collect();
            let i = &fil.installningar;
            self.inst_text = [
                i.ingaende_saldo.med_tecken(),
                i.normtid.to_string(),
                i.flex_start.to_string(),
                i.flex_slut.to_string(),
                i.min_lunch.to_string(),
            ];
        }
    }

    fn spara(&mut self) {
        if let Some(fil) = &self.fil
            && let Err(e) = arkiv::skriv(&self.sokvag(), fil)
        {
            self.fel(e);
        }
    }

    fn byt_manad(&mut self, manad: Manad) {
        self.manad = manad;
        self.meddelande = None;
        self.ladda();
    }

    fn skapa_manad(&mut self, manad: Manad) {
        if let Err(e) = std::fs::create_dir_all(&self.mapp) {
            return self.fel(format!("kunde inte skapa mappen: {e}"));
        }
        match arkiv::skapa_manad(&self.mapp, manad) {
            Ok((_, saldo, fran)) => {
                self.byt_manad(manad);
                self.info(match fran {
                    Some(_) => format!(
                        "Skapade {} med ingående saldo {} från föregående månad.",
                        manadsnamn(manad),
                        saldo.med_tecken()
                    ),
                    None => format!("Skapade {}.", manadsnamn(manad)),
                });
            }
            Err(e) => self.fel(e),
        }
    }

    fn lagg_till_dag(&mut self) {
        let text = self.ny_dag.trim().to_string();
        let datum = match text.parse::<u32>() {
            Ok(d) => format!("{}-{d:02}", self.manad),
            Err(_) => text,
        };
        let (ar, man, d) = match tolka_datum(&datum) {
            Ok(v) => v,
            Err(e) => return self.fel(e),
        };
        if (Manad { ar, manad: man }) != self.manad {
            return self.fel(format!("{datum} hör inte till {}", manadsnamn(self.manad)));
        }
        let Some(fil) = &mut self.fil else { return };
        if fil.dagar.iter().any(|d| d.datum == datum) {
            return self.fel(format!("{datum} finns redan i listan"));
        }
        let ledig = veckodag(ar, man, d) >= 5 || helgdag(ar, man, d).is_some();
        fil.satt_dag(Dag {
            datum: datum.clone(),
            normtid: ledig.then_some(Tid::NOLL),
            notering: helgdag(ar, man, d).map(str::to_string),
            ..Default::default()
        });
        self.ny_dag.clear();
        self.synka_text();
        self.spara();
        self.info(format!("La till {datum}."));
    }

    // ---------------------------------------------------------------- vyer

    fn topprad(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("⏴").on_hover_text("Föregående månad").clicked() {
                let m = self.manad;
                let fore = if m.manad == 1 {
                    Manad {
                        ar: m.ar - 1,
                        manad: 12,
                    }
                } else {
                    Manad {
                        ar: m.ar,
                        manad: m.manad - 1,
                    }
                };
                self.byt_manad(fore);
            }
            ui.add_sized(
                [170.0, 24.0],
                egui::Label::new(RichText::new(manadsnamn(self.manad)).heading().strong()),
            );
            if ui.button("⏵").on_hover_text("Nästa månad").clicked() {
                self.byt_manad(self.manad.nasta());
            }
            if ui.button("Idag").clicked() {
                let (ar, manad, _) = idag();
                self.byt_manad(Manad { ar, manad });
            }
            ui.separator();
            let nasta = self.manad.nasta();
            let finns = self.mapp.join(nasta.filnamn()).exists();
            if ui
                .add_enabled(
                    self.fil.is_some() && !finns,
                    egui::Button::new("Skapa nästa månad"),
                )
                .on_hover_text("Skapar nästa månad med en rad per vardag. Flexsaldot förs över.")
                .on_disabled_hover_text(if finns {
                    "Nästa månad finns redan"
                } else {
                    "Skapa den här månaden först"
                })
                .clicked()
            {
                self.skapa_manad(nasta);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(fil) = &self.fil {
                    let saldo = flexsaldo(&fil.dagar, &fil.installningar);
                    let farg = if saldo < Tid::NOLL { ROD } else { GRON };
                    ui.label(
                        RichText::new(saldo.med_tecken())
                            .heading()
                            .strong()
                            .color(farg),
                    );
                    ui.label(RichText::new("Flexsaldo").heading());
                }
            });
        });
    }

    fn installningar(&mut self, ui: &mut egui::Ui) {
        let mut andrat = false;
        egui::CollapsingHeader::new("Inställningar")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Mapp:");
                    ui.add(egui::TextEdit::singleline(&mut self.mapp_text).desired_width(360.0));
                    if ui.button("Öppna").clicked() {
                        self.mapp = PathBuf::from(self.mapp_text.trim());
                        self.ladda();
                        self.info(format!("Använder mappen {}.", self.mapp.display()));
                    }
                });
                if self.fil.is_none() {
                    return;
                }
                ui.horizontal_wrapped(|ui| {
                    for (i, etikett) in INST_ETIKETTER.iter().enumerate() {
                        ui.label(*etikett);
                        let ok = Tid::tolka(&self.inst_text[i]).is_ok();
                        let mut te =
                            egui::TextEdit::singleline(&mut self.inst_text[i]).desired_width(56.0);
                        if !ok {
                            te = te.text_color(ROD);
                        }
                        andrat |= ui.add(te).changed();
                        ui.add_space(10.0);
                    }
                });
            });
        if andrat {
            let tider: Result<Vec<Tid>, _> = self.inst_text.iter().map(|s| Tid::tolka(s)).collect();
            if let (Ok(t), Some(fil)) = (tider, &mut self.fil) {
                let i = &mut fil.installningar;
                [
                    i.ingaende_saldo,
                    i.normtid,
                    i.flex_start,
                    i.flex_slut,
                    i.min_lunch,
                ] = [t[0], t[1], t[2], t[3], t[4]];
                self.spara();
            }
        }
    }

    fn saldo_varning(&mut self, ui: &mut egui::Ui) {
        let (Some(fil), Some((m, saldo))) = (&self.fil, self.foregaende_saldo) else {
            return;
        };
        if fil.installningar.ingaende_saldo == saldo {
            return;
        }
        let text = format!(
            "Ingående saldo ({}) stämmer inte med utgående saldo för {} ({}).",
            fil.installningar.ingaende_saldo.med_tecken(),
            manadsnamn(m),
            saldo.med_tecken()
        );
        ui.horizontal(|ui| {
            ui.label(RichText::new(text).color(Color32::from_rgb(200, 140, 30)));
            if ui.button("Uppdatera").clicked()
                && let Some(fil) = &mut self.fil
            {
                fil.installningar.ingaende_saldo = saldo;
                self.synka_text();
                self.spara();
            }
        });
    }

    fn saknad_manad(&mut self, ui: &mut egui::Ui) {
        ui.add_space(60.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(format!(
                    "Det finns ingen fil för {} ännu.",
                    manadsnamn(self.manad)
                ))
                .size(18.0),
            );
            ui.add_space(8.0);
            let text = match self.foregaende_saldo {
                Some((m, s)) => format!(
                    "Den skapas med en rad per vardag och ingående saldo {} från {}.",
                    s.med_tecken(),
                    manadsnamn(m)
                ),
                None => "Den skapas med en rad per vardag och ingående saldo 0:00.".into(),
            };
            ui.label(text);
            ui.add_space(12.0);
            if ui
                .button(RichText::new("Skapa månaden").size(16.0))
                .clicked()
            {
                self.skapa_manad(self.manad);
            }
        });
    }

    fn tabell(&mut self, ui: &mut egui::Ui) {
        let Some(fil) = &self.fil else { return };
        let inst = fil.installningar.clone();
        let idag = {
            let (a, m, d) = idag();
            format!("{a:04}-{m:02}-{d:02}")
        };
        let mut andrad: Vec<usize> = vec![];

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                egui::Grid::new("dagar")
                    .striped(true)
                    .spacing([10.0, 4.0])
                    .min_col_width(0.0)
                    .show(ui, |ui| {
                        for rubrik in [
                            "Datum",
                            "Började",
                            "Slutade",
                            "Lunch bö",
                            "Lunch sl",
                            "Normtid",
                            "Lunch",
                            "Arb tid",
                            "Flex",
                            "Agresso",
                            "Notering",
                        ] {
                            ui.label(RichText::new(rubrik).strong());
                        }
                        ui.end_row();

                        let dagar = &self.fil.as_ref().unwrap().dagar;
                        for (i, rad) in self.rader.iter_mut().enumerate() {
                            let dag = &dagar[i];
                            let datum = match tolka_datum(&dag.datum) {
                                Ok((a, m, d)) => {
                                    let vd = VECKODAGAR[veckodag(a, m, d) as usize];
                                    format!("{vd} {d:>2}")
                                }
                                Err(_) => dag.datum.clone(),
                            };
                            let mut etikett = RichText::new(datum).monospace();
                            if dag.datum == idag {
                                etikett = etikett.strong().color(ui.visuals().hyperlink_color);
                            }
                            ui.label(etikett).on_hover_text(&dag.datum);

                            for (k, falt) in rad.tider.iter_mut().enumerate() {
                                let ok = tolka_falt(falt).is_ok();
                                let mut te = egui::TextEdit::singleline(falt)
                                    .desired_width(50.0)
                                    .align(egui::Align2::CENTER_CENTER);
                                if k == 4 {
                                    te = te.hint_text(inst.normtid.to_string());
                                }
                                if !ok {
                                    te = te.text_color(ROD);
                                }
                                if ui.add_sized([54.0, 20.0], te).changed() {
                                    andrad.push(i);
                                }
                            }

                            let r = dag.berakna(&inst);
                            if r.arbetad == Tid::NOLL {
                                for _ in 0..4 {
                                    ui.label("");
                                }
                            } else {
                                let farg = if r.flex < Tid::NOLL { ROD } else { GRON };
                                ui.label(r.lunch.to_string());
                                ui.label(r.arbetad.to_string());
                                ui.label(RichText::new(r.flex.med_tecken()).color(farg).strong());
                                ui.label(format!("{:.2}", r.arbetad.hundradelar()));
                            }
                            if ui
                                .add_sized(
                                    [160.0, 20.0],
                                    egui::TextEdit::singleline(&mut rad.notering),
                                )
                                .changed()
                            {
                                andrad.push(i);
                            }
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("Lägg till dag (t.ex. en lördag):");
                    let svar = ui.add(
                        egui::TextEdit::singleline(&mut self.ny_dag)
                            .hint_text("dag eller ÅÅÅÅ-MM-DD")
                            .desired_width(140.0),
                    );
                    let enter = svar.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Lägg till").clicked() || enter {
                        self.lagg_till_dag();
                    }
                });
            });

        // Spara ändrade rader där alla fält är giltiga.
        andrad.dedup();
        if andrad.is_empty() {
            return;
        }
        let Some(fil) = &mut self.fil else { return };
        let mut sparas = false;
        for i in andrad {
            let rad = &self.rader[i];
            let tider: Result<Vec<Option<Tid>>, ()> =
                rad.tider.iter().map(|s| tolka_falt(s)).collect();
            if let Ok(t) = tider {
                let dag = &mut fil.dagar[i];
                [
                    dag.borjade,
                    dag.slutade,
                    dag.lunch_borjade,
                    dag.lunch_slutade,
                    dag.normtid,
                ] = [t[0], t[1], t[2], t[3], t[4]];
                let n = rad.notering.trim();
                dag.notering = (!n.is_empty()).then(|| n.to_string());
                sparas = true;
            }
        }
        if sparas {
            self.spara();
        }
    }

    fn summering(&self, ui: &mut egui::Ui) {
        let Some(fil) = &self.fil else { return };
        let inst = &fil.installningar;
        let arbetat: Tid = fil.dagar.iter().map(|d| d.berakna(inst).arbetad).sum();
        let saldo = flexsaldo(&fil.dagar, inst);
        ui.horizontal(|ui| {
            ui.label(format!("Arbetad tid: {arbetat}"));
            ui.separator();
            ui.label(format!(
                "Månadens flex: {}",
                (saldo - inst.ingaende_saldo).med_tecken()
            ));
            ui.separator();
            ui.label(format!(
                "Ingående saldo: {}",
                inst.ingaende_saldo.med_tecken()
            ));
            ui.separator();
            ui.label(RichText::new(format!("Flexsaldo: {}", saldo.med_tecken())).strong());
        });
    }

    fn statusrad(&self, ui: &mut egui::Ui, sokvag: &Path) {
        ui.horizontal(|ui| match &self.meddelande {
            Some(m) => {
                let farg = if m.fel { ROD } else { GRON };
                ui.label(RichText::new(&m.text).color(farg));
            }
            None => {
                ui.label(
                    RichText::new(format!(
                        "{}  ·  Tider skrivs tt:mm. Lunch räknas som minst {}; ange lunchtider bara vid längre lunch. Ändringar sparas direkt.",
                        sokvag.file_name().unwrap_or_default().to_string_lossy(),
                        self.fil.as_ref().map_or(Tid(30), |f| f.installningar.min_lunch)
                    ))
                    .weak(),
                );
            }
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let sokvag = self.sokvag();
        egui::Panel::top("topp").show(ui, |ui| {
            ui.add_space(6.0);
            self.topprad(ui);
            self.installningar(ui);
            self.saldo_varning(ui);
            ui.add_space(4.0);
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.add_space(4.0);
            self.summering(ui);
            self.statusrad(ui, &sokvag);
            ui.add_space(4.0);
        });
        egui::CentralPanel::default().show(ui, |ui| {
            if self.fil.is_some() {
                self.tabell(ui);
            } else {
                self.saknad_manad(ui);
            }
        });
    }
}
