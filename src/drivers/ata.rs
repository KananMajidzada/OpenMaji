use x86_64::instructions::port::Port;
use x86_64::instructions::interrupts;
use spin::Mutex;
use lazy_static::lazy_static;

// ── Secondary ATA bus ports ───────────────────────────────────────────────────
const ATA_DATA:         u16 = 0x170;
const ATA_ERROR:        u16 = 0x171;
const ATA_SECTOR_COUNT: u16 = 0x172;
const ATA_LBA_LO:       u16 = 0x173;
const ATA_LBA_MID:      u16 = 0x174;
const ATA_LBA_HI:       u16 = 0x175;
const ATA_DRIVE_HEAD:   u16 = 0x176;
const ATA_STATUS:       u16 = 0x177;
const ATA_COMMAND:      u16 = 0x177;
const ATA_ALT_STATUS:   u16 = 0x376; // alternate status (reading won't clear IRQ)

const ATA_CMD_READ:     u8 = 0x20;
const ATA_CMD_WRITE:    u8 = 0x30;
const ATA_CMD_FLUSH:    u8 = 0xE7;
const ATA_CMD_IDENTIFY: u8 = 0xEC;

const ATA_SR_BSY:  u8 = 0x80;
const ATA_SR_DRQ:  u8 = 0x08;
const ATA_SR_ERR:  u8 = 0x01;
const ATA_SR_DF:   u8 = 0x20;

pub const SECTOR_SIZE: usize = 512;
const MAX_TRIES: u32 = 100_000;

pub struct AtaDrive {
    data:         Port<u16>,
    error:        Port<u8>,
    sector_count: Port<u8>,
    lba_lo:       Port<u8>,
    lba_mid:      Port<u8>,
    lba_hi:       Port<u8>,
    drive_head:   Port<u8>,
    status:       Port<u8>,
    command:      Port<u8>,
    alt_status:   Port<u8>,
}

impl AtaDrive {
    pub const fn new() -> Self {
        AtaDrive {
            data:         Port::new(ATA_DATA),
            error:        Port::new(ATA_ERROR),
            sector_count: Port::new(ATA_SECTOR_COUNT),
            lba_lo:       Port::new(ATA_LBA_LO),
            lba_mid:      Port::new(ATA_LBA_MID),
            lba_hi:       Port::new(ATA_LBA_HI),
            drive_head:   Port::new(ATA_DRIVE_HEAD),
            status:       Port::new(ATA_STATUS),
            command:      Port::new(ATA_COMMAND),
            alt_status:   Port::new(ATA_ALT_STATUS),
        }
    }

    // 400ns delay — read alt status 4 times
    fn delay_400ns(&mut self) {
        unsafe {
            self.alt_status.read();
            self.alt_status.read();
            self.alt_status.read();
            self.alt_status.read();
        }
    }

    fn wait_not_busy(&mut self) -> Result<(), &'static str> {
        for _ in 0..MAX_TRIES {
            let s = unsafe { self.alt_status.read() };
            if s & ATA_SR_ERR != 0 { return Err("ATA: drive error"); }
            if s & ATA_SR_DF  != 0 { return Err("ATA: drive fault"); }
            if s & ATA_SR_BSY == 0 { return Ok(()); }
        }
        Err("ATA: timeout waiting for not-busy")
    }

    fn wait_drq(&mut self) -> Result<(), &'static str> {
        for _ in 0..MAX_TRIES {
            let s = unsafe { self.alt_status.read() };
            if s & ATA_SR_ERR != 0 { return Err("ATA: drive error"); }
            if s & ATA_SR_DF  != 0 { return Err("ATA: drive fault"); }
            if s & ATA_SR_BSY == 0 && s & ATA_SR_DRQ != 0 {
                return Ok(());
            }
        }
        Err("ATA: timeout waiting for DRQ")
    }

    fn select_drive(&mut self, lba: u32) {
        unsafe {
            // master drive (0xE0), LBA mode
            self.drive_head.write(0xE0 | ((lba >> 24) as u8 & 0x0F));
        }
        self.delay_400ns();
    }

    fn setup(&mut self, lba: u32) {
        unsafe {
            self.error.write(0);
            self.sector_count.write(1);
            self.lba_lo.write((lba & 0xFF) as u8);
            self.lba_mid.write(((lba >> 8) & 0xFF) as u8);
            self.lba_hi.write(((lba >> 16) & 0xFF) as u8);
        }
    }

    pub fn read_sector(&mut self, lba: u32, buf: &mut [u8; SECTOR_SIZE])
        -> Result<(), &'static str>
    {
        self.wait_not_busy()?;
        self.select_drive(lba);
        self.wait_not_busy()?;
        self.setup(lba);
        unsafe { self.command.write(ATA_CMD_READ); }
        self.delay_400ns();
        self.wait_drq()?;

        for i in 0..256 {
            let word = unsafe { self.data.read() };
            buf[i * 2]     = (word & 0xFF) as u8;
            buf[i * 2 + 1] = (word >> 8)   as u8;
        }
        Ok(())
    }

    pub fn write_sector(&mut self, lba: u32, buf: &[u8; SECTOR_SIZE])
        -> Result<(), &'static str>
    {
        self.wait_not_busy()?;
        self.select_drive(lba);
        self.wait_not_busy()?;
        self.setup(lba);
        unsafe { self.command.write(ATA_CMD_WRITE); }
        self.delay_400ns();
        self.wait_drq()?;

        for i in 0..256 {
            let word = (buf[i * 2] as u16) | ((buf[i * 2 + 1] as u16) << 8);
            unsafe { self.data.write(word); }
        }

        // flush write cache
        unsafe { self.command.write(ATA_CMD_FLUSH); }
        self.wait_not_busy()?;
        Ok(())
    }

    pub fn read_sectors(&mut self, lba: u32, count: u32)
        -> Result<alloc::vec::Vec<u8>, &'static str>
    {
        let mut result = alloc::vec![0u8; count as usize * SECTOR_SIZE];
        for i in 0..count {
            let mut buf = [0u8; SECTOR_SIZE];
            self.read_sector(lba + i, &mut buf)?;
            let off = i as usize * SECTOR_SIZE;
            result[off..off + SECTOR_SIZE].copy_from_slice(&buf);
        }
        Ok(result)
    }

    pub fn write_sectors(&mut self, lba: u32, data: &[u8])
        -> Result<(), &'static str>
    {
        let count = (data.len() + SECTOR_SIZE - 1) / SECTOR_SIZE;
        for i in 0..count {
            let mut buf = [0u8; SECTOR_SIZE];
            let off = i * SECTOR_SIZE;
            let end = (off + SECTOR_SIZE).min(data.len());
            buf[..end - off].copy_from_slice(&data[off..end]);
            self.write_sector(lba as u32 + i as u32, &buf)?;
        }
        Ok(())
    }

    pub fn detect(&mut self) -> bool {
        unsafe {
            // select master
            self.drive_head.write(0xA0);
        }
        self.delay_400ns();

        // check mid/hi ports — if both 0, drive exists
        let mid = unsafe { self.lba_mid.read() };
        let hi  = unsafe { self.lba_hi.read()  };
        if mid != 0 || hi != 0 {
            return false; // not ATA
        }

        unsafe { self.command.write(ATA_CMD_IDENTIFY); }
        self.delay_400ns();

        let status = unsafe { self.alt_status.read() };
        if status == 0 {
            return false; // no drive
        }

        // wait for BSY to clear
        for _ in 0..MAX_TRIES {
            let s = unsafe { self.alt_status.read() };
            if s & ATA_SR_BSY == 0 { return true; }
        }
        false
    }
}

// ── Global instance ───────────────────────────────────────────────────────────

lazy_static! {
    pub static ref ATA: Mutex<AtaDrive> = Mutex::new(AtaDrive::new());
}

// ── Safe public API (interrupts disabled during I/O) ─────────────────────────

pub fn read_sector(lba: u32, buf: &mut [u8; SECTOR_SIZE]) -> Result<(), &'static str> {
    interrupts::without_interrupts(|| ATA.lock().read_sector(lba, buf))
}

pub fn write_sector(lba: u32, buf: &[u8; SECTOR_SIZE]) -> Result<(), &'static str> {
    interrupts::without_interrupts(|| ATA.lock().write_sector(lba, buf))
}

pub fn read_sectors(lba: u32, count: u32) -> Result<alloc::vec::Vec<u8>, &'static str> {
    interrupts::without_interrupts(|| ATA.lock().read_sectors(lba, count))
}

pub fn write_sectors(lba: u32, data: &[u8]) -> Result<(), &'static str> {
    interrupts::without_interrupts(|| ATA.lock().write_sectors(lba, data))
}

pub fn disk_present() -> bool {
    interrupts::without_interrupts(|| ATA.lock().detect())
}