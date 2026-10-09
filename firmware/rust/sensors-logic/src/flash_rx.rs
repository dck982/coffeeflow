//! Réception d'une image par `FLASH_CTRL` / `FLASH_DATA`.
//!
//! Un seul transfert à la fois. Les `FLASH_DATA` (8 octets bruts) remplissent
//! un bloc de 2 Ko, écrit à son tour dans le slot inactif, puis acquitté avec
//! son CRC16. Le CRC32 de l'image entière se vérifie au `END`. Un rejeu de
//! bloc (acquittement perdu) est reconnu à son `BLOCK_START` et ses données
//! sont ignorées : l'écriture est idempotente.

use protocol::Crc32Incremental;

pub const BLOCK_SIZE: usize = 2048;
/// En-tête d'image ESP : 24 octets.
pub const HEAD_LEN: usize = 24;

/// Alignée sur 4 : esp-storage écrit alors directement depuis le tampon, sans
/// copie de 4 Ko sur la pile.
#[repr(C, align(4))]
struct Block([u8; BLOCK_SIZE]);

/// Ce que le récepteur demande au matériel.
pub trait FlashSink {
    /// Taille du slot inactif, `None` s'il n'y en a pas.
    fn slot_size(&self) -> Option<u32>;
    /// Efface le slot inactif en entier, avant l'acquittement du `BEGIN`.
    fn erase_slot(&mut self) -> bool;
    /// Écrit `data` à `offset` dans le slot. `data` fait un multiple de 4
    /// octets, complété de `0xFF` pour le dernier bloc.
    fn write(&mut self, offset: u32, data: &[u8]) -> bool;
    /// Contrôle l'image écrite (équivalent d'`esp_ota_end`) puis fait désigner
    /// le slot par `otadata`. `head` : les 24 premiers octets de l'image,
    /// `image_size` : sa taille.
    fn commit(&mut self, head: &[u8; HEAD_LEN], image_size: u32) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeginOutcome {
    /// Pas de slot, taille nulle ou plus grande que le slot.
    Refused,
    EraseFailed,
    Started,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStartOutcome {
    /// Aucun transfert en cours.
    Idle,
    /// Bloc déjà reçu : ses données seront ignorées, il faut réacquitter.
    Replay { block_number: u16, block_crc16: u16 },
    /// Le bloc suivant, attendu.
    Next,
    /// Numéro inattendu : transfert abandonné.
    Aborted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataOutcome {
    Nothing,
    BlockWritten {
        block_number: u16,
        block_crc16: u16,
        bytes_written: u32,
    },
    /// L'écriture a échoué : transfert abandonné.
    WriteFailed {
        bytes_written: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndOutcome {
    NotActive,
    /// Bloc final incomplet, taille ou CRC32 faux : transfert abandonné.
    Mismatch {
        bytes_written: u32,
    },
    /// Contrôle de l'image ou bascule d'`otadata` en échec.
    CommitFailed,
    Done,
}

pub struct FlashReceiver {
    active: bool,
    image_size: u32,
    bytes_written: u32,
    /// Nombre de blocs déjà acquittés.
    block_number: u16,
    last_block_crc16: u16,
    replay_ignored: bool,
    block: Block,
    block_len: usize,
    image_crc: Crc32Incremental,
    head: [u8; HEAD_LEN],
}

impl FlashReceiver {
    pub const fn new() -> Self {
        Self {
            active: false,
            image_size: 0,
            bytes_written: 0,
            block_number: 0,
            last_block_crc16: 0,
            replay_ignored: false,
            block: Block([0; BLOCK_SIZE]),
            block_len: 0,
            image_crc: Crc32Incremental::new(),
            head: [0; HEAD_LEN],
        }
    }

    /// Un transfert est en cours (`BEGIN` reçu, `END` ou `ABORT` pas encore
    /// traité).
    pub fn active(&self) -> bool {
        self.active
    }

    pub fn bytes_written(&self) -> u32 {
        self.bytes_written
    }

    fn reset(&mut self) {
        self.active = false;
        self.image_size = 0;
        self.bytes_written = 0;
        self.block_number = 0;
        self.last_block_crc16 = 0;
        self.replay_ignored = false;
        self.block_len = 0;
        self.image_crc = Crc32Incremental::new();
        self.head = [0; HEAD_LEN];
    }

    /// Abandonne le transfert en cours. `true` s'il y en avait un.
    pub fn abort(&mut self, reason: &str) -> bool {
        let was_active = self.active;
        if was_active {
            log::warn!("flash abandonné : {reason}");
        }
        self.reset();
        was_active
    }

    pub fn begin(&mut self, sink: &mut impl FlashSink, image_size: u32) -> BeginOutcome {
        if self.active {
            self.abort("nouveau BEGIN reçu avant la fin du précédent transfert");
        }
        match sink.slot_size() {
            Some(size) if image_size != 0 && image_size <= size => {}
            _ => return BeginOutcome::Refused,
        }
        // Tout le slot est effacé avant l'acquittement : plus aucun
        // effacement pendant le flux, donc plus la cause principale de
        // débordement de la file de réception.
        if !sink.erase_slot() {
            return BeginOutcome::EraseFailed;
        }
        self.reset();
        self.active = true;
        self.image_size = image_size;
        BeginOutcome::Started
    }

    pub fn block_start(&mut self, number: u16, crc16: u16) -> BlockStartOutcome {
        if !self.active {
            return BlockStartOutcome::Idle;
        }
        if number == self.block_number && self.block_number > 0 && crc16 == self.last_block_crc16 {
            // Les données rejouées sont ignorées jusqu'au prochain BLOCK_START.
            self.replay_ignored = true;
            BlockStartOutcome::Replay {
                block_number: self.block_number,
                block_crc16: self.last_block_crc16,
            }
        } else if u32::from(number) == u32::from(self.block_number) + 1 && self.block_len == 0 {
            self.replay_ignored = false;
            BlockStartOutcome::Next
        } else {
            self.abort("numéro de bloc inattendu");
            BlockStartOutcome::Aborted
        }
    }

    /// `FLASH_DATA` : la trame entière (jusqu'à 8 octets) est la donnée, sans
    /// en-tête. Un anonyme sans `BLOCK_START` reste accepté (anciens clients),
    /// mais sans déduplication.
    pub fn data(&mut self, sink: &mut impl FlashSink, bytes: &[u8]) -> DataOutcome {
        if !self.active || bytes.is_empty() || self.replay_ignored {
            return DataOutcome::Nothing;
        }
        let remaining_in_image = (self.image_size - self.bytes_written) as usize - self.block_len;
        // Jamais plus que ce que le tampon contient : une trame à cheval sur
        // deux blocs ne peut pas le déborder.
        let to_copy = bytes
            .len()
            .min(remaining_in_image)
            .min(BLOCK_SIZE - self.block_len);
        if to_copy == 0 {
            return DataOutcome::Nothing;
        }
        self.block.0[self.block_len..self.block_len + to_copy].copy_from_slice(&bytes[..to_copy]);
        self.block_len += to_copy;

        let bytes_left_total = (self.image_size - self.bytes_written) as usize;
        let expected_block_size = bytes_left_total.min(BLOCK_SIZE);
        if self.block_len < expected_block_size {
            return DataOutcome::Nothing;
        }

        let len = self.block_len;
        // Écriture par mots de 4 octets : le dernier bloc est complété de 0xFF.
        let padded = len.next_multiple_of(4);
        self.block.0[len..padded].fill(0xFF);
        if !sink.write(self.bytes_written, &self.block.0[..padded]) {
            self.abort("écriture en flash en échec");
            return DataOutcome::WriteFailed {
                bytes_written: self.bytes_written,
            };
        }
        if self.block_number == 0 {
            let n = len.min(HEAD_LEN);
            self.head[..n].copy_from_slice(&self.block.0[..n]);
        }
        self.image_crc.update(&self.block.0[..len]);
        let block_crc16 = protocol::crc16_ccitt(&self.block.0[..len]);
        self.last_block_crc16 = block_crc16;
        self.bytes_written += len as u32;
        self.block_len = 0;
        self.block_number += 1;
        DataOutcome::BlockWritten {
            block_number: self.block_number,
            block_crc16,
            bytes_written: self.bytes_written,
        }
    }

    pub fn end(&mut self, sink: &mut impl FlashSink, expected_crc32: u32) -> EndOutcome {
        if !self.active {
            return EndOutcome::NotActive;
        }
        // Bloc final incomplet : END n'aurait pas dû arriver avant le dernier
        // BLOCK_ACK. Traité comme un échec, pas rattrapable ici.
        if self.block_len != 0
            || self.bytes_written != self.image_size
            || self.image_crc.finish() != expected_crc32
        {
            let bytes_written = self.bytes_written;
            self.abort("CRC32 global ou taille reçue incohérente au END");
            return EndOutcome::Mismatch { bytes_written };
        }
        let head = self.head;
        let ok = image_header_ok(&head) && sink.commit(&head, self.image_size);
        self.reset();
        if ok {
            EndOutcome::Done
        } else {
            EndOutcome::CommitFailed
        }
    }
}

impl Default for FlashReceiver {
    fn default() -> Self {
        Self::new()
    }
}

/// En-tête d'image ESP : magic `0xE9`, 1 à 16 segments, puce ESP32-S3
/// (identifiant 9 à l'octet 12). C'est le contrôle minimal avant de désigner
/// le slot ; le CRC32 de l'image entière est déjà juste à ce stade.
pub fn image_header_ok(head: &[u8; HEAD_LEN]) -> bool {
    head[0] == 0xE9 && (1..=16).contains(&head[1]) && u16::from_le_bytes([head[12], head[13]]) == 9
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{crc16_ccitt, crc32_ieee};

    struct Sink {
        size: Option<u32>,
        erase_ok: bool,
        write_ok: bool,
        commit_ok: bool,
        erased: u32,
        flash: Vec<u8>,
        writes: Vec<(u32, usize)>,
        commits: u32,
    }

    impl Sink {
        fn new(size: u32) -> Self {
            Self {
                size: Some(size),
                erase_ok: true,
                write_ok: true,
                commit_ok: true,
                erased: 0,
                flash: vec![0xFF; size as usize],
                writes: vec![],
                commits: 0,
            }
        }
    }

    impl FlashSink for Sink {
        fn slot_size(&self) -> Option<u32> {
            self.size
        }
        fn erase_slot(&mut self) -> bool {
            self.erased += 1;
            self.flash.fill(0xFF);
            self.erase_ok
        }
        fn write(&mut self, offset: u32, data: &[u8]) -> bool {
            assert_eq!(data.len() % 4, 0);
            assert_eq!(offset % 4, 0);
            self.writes.push((offset, data.len()));
            self.flash[offset as usize..offset as usize + data.len()].copy_from_slice(data);
            self.write_ok
        }
        fn commit(&mut self, _head: &[u8; HEAD_LEN], _size: u32) -> bool {
            self.commits += 1;
            self.commit_ok
        }
    }

    fn image(len: usize) -> Vec<u8> {
        let mut img: Vec<u8> = (0..len).map(|i| (i * 7 + 3) as u8).collect();
        // En-tête d'image ESP32-S3 : magic, 3 segments, puce 9.
        if len >= 16 {
            img[0] = 0xE9;
            img[1] = 3;
            img[12] = 9;
            img[13] = 0;
        }
        img
    }

    fn feed(rx: &mut FlashReceiver, sink: &mut Sink, bytes: &[u8]) -> DataOutcome {
        let mut last = DataOutcome::Nothing;
        for chunk in bytes.chunks(8) {
            let out = rx.data(sink, chunk);
            if out != DataOutcome::Nothing {
                last = out;
            }
        }
        last
    }

    #[test]
    fn full_transfer_with_a_short_last_block() {
        let img = image(2048 * 2 + 13);
        let mut sink = Sink::new(4096 * 4);
        let mut rx = FlashReceiver::new();
        assert_eq!(rx.begin(&mut sink, img.len() as u32), BeginOutcome::Started);
        assert_eq!(sink.erased, 1);
        assert!(rx.active());

        for (n, block) in img.chunks(2048).enumerate() {
            let number = n as u16 + 1;
            let crc = crc16_ccitt(block);
            assert_eq!(rx.block_start(number, crc), BlockStartOutcome::Next);
            let out = feed(&mut rx, &mut sink, block);
            assert_eq!(
                out,
                DataOutcome::BlockWritten {
                    block_number: number,
                    block_crc16: crc,
                    bytes_written: (n * 2048 + block.len()) as u32,
                }
            );
        }
        assert_eq!(&sink.flash[..img.len()], &img[..]);
        // Le dernier bloc est complété à 4 octets par 0xFF.
        assert_eq!(sink.writes.last(), Some(&(4096, 16)));
        assert_eq!(&sink.flash[img.len()..img.len() + 3], &[0xFF; 3]);

        assert_eq!(rx.end(&mut sink, crc32_ieee(&img)), EndOutcome::Done);
        assert_eq!(sink.commits, 1);
        assert!(!rx.active());
    }

    #[test]
    fn begin_is_refused_for_zero_oversized_or_no_slot_and_erases_nothing() {
        let mut sink = Sink::new(4096);
        let mut rx = FlashReceiver::new();
        assert_eq!(rx.begin(&mut sink, 0), BeginOutcome::Refused);
        assert_eq!(rx.begin(&mut sink, 4097), BeginOutcome::Refused);
        sink.size = None;
        assert_eq!(rx.begin(&mut sink, 10), BeginOutcome::Refused);
        assert_eq!(sink.erased, 0);
        assert!(!rx.active());
        sink.size = Some(4096);
        assert_eq!(rx.begin(&mut sink, 4096), BeginOutcome::Started);
    }

    #[test]
    fn erase_failure_leaves_no_transfer() {
        let mut sink = Sink::new(4096);
        sink.erase_ok = false;
        let mut rx = FlashReceiver::new();
        assert_eq!(rx.begin(&mut sink, 100), BeginOutcome::EraseFailed);
        assert!(!rx.active());
    }

    #[test]
    fn a_replayed_block_is_acknowledged_again_and_its_data_ignored() {
        let img = image(4096);
        let mut sink = Sink::new(8192);
        let mut rx = FlashReceiver::new();
        rx.begin(&mut sink, 4096);
        let crc1 = crc16_ccitt(&img[..2048]);
        assert_eq!(rx.block_start(1, crc1), BlockStartOutcome::Next);
        feed(&mut rx, &mut sink, &img[..2048]);
        let writes = sink.writes.len();

        // L'ACK s'est perdu : l'écran rejoue le bloc 1.
        assert_eq!(
            rx.block_start(1, crc1),
            BlockStartOutcome::Replay {
                block_number: 1,
                block_crc16: crc1
            }
        );
        assert_eq!(feed(&mut rx, &mut sink, &img[..2048]), DataOutcome::Nothing);
        assert_eq!(sink.writes.len(), writes);
        assert_eq!(rx.bytes_written(), 2048);

        // Puis le bloc 2 normalement.
        let crc2 = crc16_ccitt(&img[2048..]);
        assert_eq!(rx.block_start(2, crc2), BlockStartOutcome::Next);
        feed(&mut rx, &mut sink, &img[2048..]);
        assert_eq!(rx.end(&mut sink, crc32_ieee(&img)), EndOutcome::Done);
        assert_eq!(&sink.flash[..4096], &img[..]);
    }

    #[test]
    fn a_replay_with_a_different_crc_aborts() {
        let img = image(4096);
        let mut sink = Sink::new(8192);
        let mut rx = FlashReceiver::new();
        rx.begin(&mut sink, 4096);
        let crc1 = crc16_ccitt(&img[..2048]);
        rx.block_start(1, crc1);
        feed(&mut rx, &mut sink, &img[..2048]);
        assert_eq!(rx.block_start(1, crc1 ^ 1), BlockStartOutcome::Aborted);
        assert!(!rx.active());
    }

    #[test]
    fn a_skipped_block_aborts() {
        let mut sink = Sink::new(8192);
        let mut rx = FlashReceiver::new();
        rx.begin(&mut sink, 4096);
        assert_eq!(rx.block_start(2, 0), BlockStartOutcome::Aborted);
        assert!(!rx.active());
        assert_eq!(rx.block_start(1, 0), BlockStartOutcome::Idle);
    }

    #[test]
    fn a_new_block_start_in_the_middle_of_a_block_aborts() {
        let img = image(4096);
        let mut sink = Sink::new(8192);
        let mut rx = FlashReceiver::new();
        rx.begin(&mut sink, 4096);
        rx.block_start(1, 0);
        rx.data(&mut sink, &img[..8]);
        assert_eq!(rx.block_start(2, 0), BlockStartOutcome::Aborted);
    }

    #[test]
    fn end_with_a_wrong_crc_or_a_short_image_aborts_without_commit() {
        let img = image(3000);
        let mut sink = Sink::new(8192);
        let mut rx = FlashReceiver::new();
        rx.begin(&mut sink, 3000);
        rx.block_start(1, crc16_ccitt(&img[..2048]));
        feed(&mut rx, &mut sink, &img[..2048]);
        // Il manque encore 952 octets.
        assert_eq!(
            rx.end(&mut sink, crc32_ieee(&img)),
            EndOutcome::Mismatch {
                bytes_written: 2048
            }
        );
        assert!(!rx.active());

        rx.begin(&mut sink, 3000);
        rx.block_start(1, crc16_ccitt(&img[..2048]));
        feed(&mut rx, &mut sink, &img[..2048]);
        rx.block_start(2, crc16_ccitt(&img[2048..]));
        feed(&mut rx, &mut sink, &img[2048..]);
        assert_eq!(
            rx.end(&mut sink, crc32_ieee(&img) ^ 1),
            EndOutcome::Mismatch {
                bytes_written: 3000
            }
        );
        assert_eq!(sink.commits, 0);
    }

    #[test]
    fn end_without_begin_is_not_active() {
        let mut sink = Sink::new(4096);
        let mut rx = FlashReceiver::new();
        assert_eq!(rx.end(&mut sink, 0), EndOutcome::NotActive);
    }

    #[test]
    fn commit_failure_is_reported_and_the_transfer_is_over() {
        let img = image(100);
        let mut sink = Sink::new(4096);
        sink.commit_ok = false;
        let mut rx = FlashReceiver::new();
        rx.begin(&mut sink, 100);
        rx.block_start(1, crc16_ccitt(&img));
        feed(&mut rx, &mut sink, &img);
        assert_eq!(
            rx.end(&mut sink, crc32_ieee(&img)),
            EndOutcome::CommitFailed
        );
        assert!(!rx.active());
    }

    #[test]
    fn an_image_without_the_esp_header_is_not_committed() {
        let mut img = image(100);
        img[0] = 0;
        let mut sink = Sink::new(4096);
        let mut rx = FlashReceiver::new();
        rx.begin(&mut sink, 100);
        rx.block_start(1, crc16_ccitt(&img));
        feed(&mut rx, &mut sink, &img);
        assert_eq!(
            rx.end(&mut sink, crc32_ieee(&img)),
            EndOutcome::CommitFailed
        );
        assert_eq!(sink.commits, 0);
    }

    #[test]
    fn write_failure_aborts() {
        let img = image(100);
        let mut sink = Sink::new(4096);
        sink.write_ok = false;
        let mut rx = FlashReceiver::new();
        rx.begin(&mut sink, 100);
        rx.block_start(1, 0);
        assert_eq!(
            feed(&mut rx, &mut sink, &img),
            DataOutcome::WriteFailed { bytes_written: 0 }
        );
        assert!(!rx.active());
    }

    #[test]
    fn data_past_the_image_end_is_dropped() {
        let img = image(16);
        let mut sink = Sink::new(4096);
        let mut rx = FlashReceiver::new();
        rx.begin(&mut sink, 16);
        rx.block_start(1, 0);
        rx.data(&mut sink, &img[..8]);
        let out = rx.data(&mut sink, &img[8..]);
        assert!(matches!(
            out,
            DataOutcome::BlockWritten {
                bytes_written: 16,
                ..
            }
        ));
        // Le transfert est complet : tout octet de plus est ignoré.
        assert_eq!(rx.data(&mut sink, &[1, 2, 3]), DataOutcome::Nothing);
    }

    #[test]
    fn a_second_begin_restarts_the_transfer() {
        let mut sink = Sink::new(4096);
        let mut rx = FlashReceiver::new();
        rx.begin(&mut sink, 4000);
        rx.block_start(1, 0);
        rx.data(&mut sink, &[0; 8]);
        assert_eq!(rx.begin(&mut sink, 100), BeginOutcome::Started);
        assert_eq!(sink.erased, 2);
        assert_eq!(rx.bytes_written(), 0);
        assert_eq!(rx.block_start(1, 0), BlockStartOutcome::Next);
    }

    #[test]
    fn a_failed_begin_aborts_the_previous_transfer() {
        let mut sink = Sink::new(4096);
        let mut rx = FlashReceiver::new();
        rx.begin(&mut sink, 100);
        assert_eq!(rx.begin(&mut sink, 0), BeginOutcome::Refused);
        assert!(!rx.active());
    }

    #[test]
    fn abort_reports_whether_a_transfer_was_running() {
        let mut sink = Sink::new(4096);
        let mut rx = FlashReceiver::new();
        assert!(!rx.abort("test"));
        rx.begin(&mut sink, 100);
        assert!(rx.abort("test"));
        assert!(!rx.active());
    }

    #[test]
    fn header_check() {
        let mut head = [0u8; HEAD_LEN];
        head[0] = 0xE9;
        head[1] = 5;
        head[12] = 9;
        assert!(image_header_ok(&head));
        head[12] = 5;
        assert!(!image_header_ok(&head));
        head[12] = 9;
        head[0] = 0;
        assert!(!image_header_ok(&head));
        head[0] = 0xE9;
        head[1] = 0;
        assert!(!image_header_ok(&head));
    }
}
