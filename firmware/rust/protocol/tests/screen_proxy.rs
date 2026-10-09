// Le module de logique est aussi compilé sur l'hôte, sans esp-hal.
#[allow(dead_code)]
#[path = "../../screen/src/ota_proxy.rs"]
mod ota_proxy;

#[path = "../../screen/src/flash_words.rs"]
mod flash_words;

use embedded_storage::nor_flash::{ErrorType, NorFlash, ReadNorFlash};
struct Memory([u8; 8192]);
impl ErrorType for Memory {
    type Error = embedded_storage::nor_flash::NorFlashErrorKind;
}
impl ReadNorFlash for Memory {
    const READ_SIZE: usize = 4;
    fn capacity(&self) -> usize {
        self.0.len()
    }
    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        assert!(offset.is_multiple_of(4) && bytes.len().is_multiple_of(4));
        bytes.copy_from_slice(&self.0[offset as usize..offset as usize + bytes.len()]);
        Ok(())
    }
}
impl NorFlash for Memory {
    const WRITE_SIZE: usize = 4;
    const ERASE_SIZE: usize = 4096;
    fn erase(&mut self, _: u32, _: u32) -> Result<(), Self::Error> {
        panic!("streaming must not erase");
    }
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        assert!(offset.is_multiple_of(4) && bytes.len().is_multiple_of(4));
        for (old, new) in self.0[offset as usize..offset as usize + bytes.len()]
            .iter_mut()
            .zip(bytes)
        {
            assert_eq!(*old & *new, *new, "NOR cannot restore a zero bit");
            *old = *new;
        }
        Ok(())
    }
}

#[test]
fn arbitrary_http_fragments_preserve_flash_words_and_image_crc() {
    let mut flash = Memory([0xff; 8192]);
    let image: Vec<u8> = (0..2057).map(|i| (i % 251) as u8).collect();
    let mut at = 4096;
    for bytes in image.chunks(13) {
        flash_words::write(&mut flash, at, bytes).unwrap();
        at += bytes.len() as u32;
    }
    assert_eq!(&flash.0[4096..6153], image);
    assert!(flash.0[..4096].iter().all(|b| *b == 0xff));
    assert!(flash.0[6153..].iter().all(|b| *b == 0xff));
    let mut readback = vec![0; image.len()];
    for (n, bytes) in readback.chunks_mut(17).enumerate() {
        flash_words::read(&mut flash, 4096 + n as u32 * 17, bytes).unwrap();
    }
    assert_eq!(readback, image);
    assert_eq!(
        protocol::crc32_ieee(&readback),
        protocol::crc32_ieee(&image)
    );
}
