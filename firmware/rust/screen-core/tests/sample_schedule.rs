//! Port de `archive/firmware/screen/test/test_sample_schedule.cpp`.

#![allow(clippy::field_reassign_with_default)]

use screen_core::sample_schedule::SampleSchedule;

const SCHEDULE: SampleSchedule = SampleSchedule::new(100_000, 25_000);

// Boucle de 50 ms réveillée sur la grille des ticks ; l'heure lue varie de
// 0 à 3 ms selon le travail qui précède, comme dans la tâche de télémétrie.
fn loop_now_us(pass: i64) -> i64 {
    pass * 50_000 + (pass * 7919 % 4) * 1000
}

#[test]
fn even_passes_between_samples() {
    // Tours pairs d'un échantillon à l'autre, même quand le travail raccourcit :
    // l'ancienne échéance « now + 100 ms » glissait alors d'un tour (150 ms).
    let mut deadline = loop_now_us(0);
    let mut previous: Option<i64> = None;
    let mut samples = 0;
    for pass in 0..400 {
        let now = loop_now_us(pass);
        if !SCHEDULE.due(now, deadline) {
            continue;
        }
        deadline = SCHEDULE.next(now, deadline);
        if let Some(previous) = previous {
            let interval = now - previous;
            assert!((97_000..=103_000).contains(&interval), "{interval}");
        }
        previous = Some(now);
        samples += 1;
    }
    assert_eq!(samples, 200);
}

#[test]
fn deadlines() {
    // Une échéance lue juste avant son heure est prise dans ce tour.
    assert!(SCHEDULE.due(99_990, 100_000));
    assert!(!SCHEDULE.due(74_000, 100_000));
    assert_eq!(SCHEDULE.next(99_990, 100_000), 200_000);

    // Un tour de boucle en retard garde la grille : la moyenne reste à 100 ms.
    assert_eq!(SCHEDULE.next(150_000, 100_000), 200_000);

    // Au-delà d'une période de retard, la grille repart de maintenant au lieu
    // de rattraper en rafale.
    assert_eq!(SCHEDULE.next(180_000, 100_000), 280_000);
    assert_eq!(SCHEDULE.next(420_000, 100_000), 520_000);
}
