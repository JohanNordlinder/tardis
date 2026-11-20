# Tardis – flextid i terminalen

Ett litet Rust-program som räknar flextid med samma regler som Excel-mallen
"Mall beräkning flex". Varje månad sparas som en vanlig textfil,
`flex-ÅÅÅÅ-MM.txt`, som också går att redigera för hand.

## Bygga

```bash
cargo build --release
```

Programmet hamnar i `target/release/tardis`. Det behöver inga externa beroenden.

## Använda

```bash
tardis ny 2026-11 --saldo +1:30          # skapa månadsfil med ingående saldo
tardis in idag 08:00 17:00               # registrera dagen (lunch 30 min)
tardis in 2026-11-03 07:30 16:00 11:30 12:30   # med längre lunch
tardis in 2026-04-02 08:00 12:29 --normtid 3:59  # dag med arbetstidsförkortning
tardis rapport                           # tabell + flexsaldo för senaste månaden
tardis nästa                             # skapa nästa månad, saldot förs över
tardis dag 08:00 17:00                   # räkna ut en dag utan att spara
```

### Nästa månad skapas automatiskt

* `tardis nästa` skapar nästa månads fil från den senaste i mappen.
  Månadens flexsaldo blir det nya ingående saldot, och normtid och övriga
  inställningar följer med.
* `tardis in` med ett datum i en månad som saknar fil skapar filen
  automatiskt, med saldot från närmast föregående månad.
* Nya månader får en rad per vardag. Svenska helgdagar på vardagar (samt
  midsommar-, jul- och nyårsafton) får normtid 0:00, så arbete då räknas som
  ren flex. Arbete på helger registreras också med normtid 0:00.

## Regler (från mallen)

* Arbetad tid = slutade − började − lunch, där lunchen räknas som **minst
  30 minuter**. Ange lunchtider bara om lunchen var längre.
* Bara tid inom flexramen (`flex_start`–`flex_slut`, standard 0:00–24:00) räknas.
* Flex per dag = arbetad tid − normtid, men bara för dagar med arbetad tid.
* Flexsaldo = summan av dagarnas flex + ingående saldo.
* "Agresso" är arbetad tid i hundradelar (8:30 → 8,50).

Normtid är 7:57 per dag (heltid). Den kan ändras för hela månaden i filen
(`normtid:`) eller per dag i sista kolumnen.

## Filformat

```text
ingående: +1:30
normtid: 7:57
flex_start: 0:00
flex_slut: 24:00
min_lunch: 0:30

# datum      började slutade lunch_bö lunch_sl normtid
2026-11-02   08:00   17:00   -        -
2026-11-03   07:30   16:00   11:30    12:30
2026-11-04   08:00   12:29   -        -        03:59   # förkortad dag
```

`-` betyder tomt fält och allt efter `#` är en anteckning.

> OBS! Detta ersätter inte tidsredovisningen i Agresso.
