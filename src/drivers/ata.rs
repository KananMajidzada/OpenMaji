use x86_64::instructions::port::Port;
use spin::Mutex;
use lazy_static::lazy_static;


const ATA_PRIMARY_DATA:         u16 = 0x1F0;
const ATA_PRIMARY_ERROR:        u16 = 0x1F1;
const ATA_PRIMARY_SECTOR_COUNT: u16 = 0x1F2;
const ATA_PRIMARY_LBA_LO:       u16 = 0x1F3;
const ATA_PRIMARY_LBA_MID:      u16 = 0x1F4;
const ATA_PRIMARY_LBA_HI:       u16 = 0x1F5;
const ATA_PRIMARY_DRIVE_HEAD:   u16 = 0x1F6;
const ATA_PRIMARY_STATUS:       u16 = 0x1F7;
const ATA_PRIMARY_COMMAND:      u16 = 0x1F7;

const ATA_CMD_READ_SECTORS:  u8 = 0x20;
const ATA_CMD_WRITE_SECTORS: u8 = 0x30;

const ATA_STATUS_BSY:  u8 = 0x80; 
const ATA_STATUS_DRQ:  u8 = 0x08; 
const ATA_STATUS_ERR:  u8 = 0x01; 

pub const SECTOR_SIZE: usize = 512;



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
}

impl AtaDrive {
    pub const fn new() -> Self {
        AtaDrive {
            data:         Port::new(ATA_PRIMARY_DATA),
            error:        Port::new(ATA_PRIMARY_ERROR),
            sector_count: Port::new(ATA_PRIMARY_SECTOR_COUNT),
            lba_lo:       Port::new(ATA_PRIMARY_LBA_LO),
            lba_mid:      Port::new(ATA_PRIMARY_LBA_MID),
            lba_hi:       Port::new(ATA_PRIMARY_LBA_HI),
            drive_head:   Port::new(ATA_PRIMARY_DRIVE_HEAD),
            status:       Port::new(ATA_PRIMARY_STATUS),
            command:      Port::new(ATA_PRIMARY_COMMAND),
        }
    }

    

    fn wait_ready(&mut self) -> Result<(), &'static str> {
        let mut tries = 0u32;
        loop {
            let status = unsafe { self.status.read() };
            if status & ATA_STATUS_ERR != 0 {
                return Err("ATA error");
            }
            if status & ATA_STATUS_BSY == 0 && status & ATA_STATUS_DRQ != 0 {
                return Ok(());
            }
            tries += 1;
            if tries > 1_000_000 {
                return Err("ATA timeout");
            }
        }
    }

    fn wait_not_busy(&mut self) -> Result<(), &'static str> {
        let mut tries = 0u32;
        loop {
            let status = unsafe { self.status.read() };
            if status & ATA_STATUS_ERR != 0 {
                return Err("ATA error");
            }
            if status & ATA_STATUS_BSY == 0 {
                return Ok(());
            }
            tries += 1;
            if tries > 1_000_000 {
                return Err("ATA timeout");
            }
        }
    }

    fn setup_lba28(&mut self, sector: u32, count: u8) {
        unsafe {
         
            self.drive_head.write(0xE0 | ((sector >> 24) as u8 & 0x0F));
            self.error.write(0x00);
            self.sector_count.write(count);
            self.lba_lo.write((sector & 0xFF) as u8);
            self.lba_mid.write(((sector >> 8) & 0xFF) as u8);
            self.lba_hi.write(((sector >> 16) & 0xFF) as u8);
        }
    }


   
    pub fn read_sector(&mut self, lba: u32, buf: &mut [u8; SECTOR_SIZE])
        -> Result<(), &'static str>
    {
        self.wait_not_busy()?;
        self.setup_lba28(lba, 1);
        unsafe { self.command.write(ATA_CMD_READ_SECTORS); }
        self.wait_ready()?;

        // read 256 u16 words = 512 bytes
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
        self.setup_lba28(lba, 1);
        unsafe { self.command.write(ATA_CMD_WRITE_SECTORS); }
        self.wait_ready()?;

  
        for i in 0..256 {
            let word = (buf[i * 2] as u16) | ((buf[i * 2 + 1] as u16) << 8);
            unsafe { self.data.write(word); }
        }

        // flush write cache
        unsafe { self.command.write(0xE7); }
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
            let offset = i as usize * SECTOR_SIZE;
            result[offset..offset + SECTOR_SIZE].copy_from_slice(&buf);
        }
        Ok(result)
    }

   
    pub fn write_sectors(&mut self, lba: u32, data: &[u8])
        -> Result<(), &'static str>
    {
        let sector_count = (data.len() + SECTOR_SIZE - 1) / SECTOR_SIZE;
        for i in 0..sector_count {
            let mut buf = [0u8; SECTOR_SIZE];
            let offset = i * SECTOR_SIZE;
            let end = (offset + SECTOR_SIZE).min(data.len());
            buf[..end - offset].copy_from_slice(&data[offset..end]);
            self.write_sector(lba as u32 + i as u32, &buf)?;
        }
        Ok(())
    }

    
    pub fn detect(&mut self) -> bool {
        unsafe {
            self.drive_head.write(0xA0);
            self.sector_count.write(0);
            self.lba_lo.write(0);
            self.lba_mid.write(0);
            self.lba_hi.write(0);
            self.command.write(0xEC); // IDENTIFY
        }
        let status = unsafe { self.status.read() };
        status != 0
    }
}



lazy_static! {
    pub static ref ATA: Mutex<AtaDrive> = Mutex::new(AtaDrive::new());
}



pub fn read_sector(lba: u32, buf: &mut [u8; SECTOR_SIZE]) -> Result<(), &'static str> {
    ATA.lock().read_sector(lba, buf)
}

pub fn write_sector(lba: u32, buf: &[u8; SECTOR_SIZE]) -> Result<(), &'static str> {
    ATA.lock().write_sector(lba, buf)
}

pub fn read_sectors(lba: u32, count: u32) -> Result<alloc::vec::Vec<u8>, &'static str> {
    ATA.lock().read_sectors(lba, count)
}

pub fn write_sectors(lba: u32, data: &[u8]) -> Result<(), &'static str> {
    ATA.lock().write_sectors(lba, data)
}

pub fn disk_present() -> bool {
    ATA.lock().detect()
}