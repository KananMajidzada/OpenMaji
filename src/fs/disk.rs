use alloc::vec::Vec;
use crate::drivers::ata;


const MAGIC: u32 = 0x4D414A49; // "MAJI" in ASCII
const HEADER_LBA: u32 = 0;
const DATA_LBA:   u32 = 1;



pub fn save_fs(fs: &super::OpenMajiFs) -> Result<(), &'static str> {
    let data = serialize(fs)?;

   
    let mut header = [0u8; ata::SECTOR_SIZE];
    header[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    let data_len = data.len() as u32;
    header[4..8].copy_from_slice(&data_len.to_le_bytes());
    ata::write_sector(HEADER_LBA, &header)?;

   
    ata::write_sectors(DATA_LBA, &data)?;

    Ok(())
}

pub fn load_fs(fs: &mut super::OpenMajiFs) -> Result<(), &'static str> {
    
    let mut header = [0u8; ata::SECTOR_SIZE];
    ata::read_sector(HEADER_LBA, &mut header)?;

    let magic = u32::from_le_bytes(header[0..4].try_into().unwrap());
    if magic != MAGIC {
        return Err("disk not formatted — run 'format' first");
    }

    let data_len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    if data_len == 0 {
        return Err("disk is empty");
    }

    
    let sector_count = (data_len + ata::SECTOR_SIZE - 1) / ata::SECTOR_SIZE;
    let raw = ata::read_sectors(DATA_LBA, sector_count as u32)?;
    let data = &raw[..data_len];

    deserialize(fs, data)?;
    Ok(())
}

pub fn format_disk() -> Result<(), &'static str> {
    
    let mut header = [0u8; ata::SECTOR_SIZE];
    header[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    header[4..8].copy_from_slice(&0u32.to_le_bytes());
    ata::write_sector(HEADER_LBA, &header)?;
    Ok(())
}

pub fn disk_formatted() -> bool {
    let mut header = [0u8; ata::SECTOR_SIZE];
    if ata::read_sector(HEADER_LBA, &mut header).is_err() {
        return false;
    }
    let magic = u32::from_le_bytes(header[0..4].try_into().unwrap_or([0;4]));
    magic == MAGIC
}



fn serialize(fs: &super::OpenMajiFs) -> Result<Vec<u8>, &'static str> {
    let mut out: Vec<u8> = Vec::new();

    // write cwd
    write_u64(&mut out, fs.cwd as u64);

   
    let live: Vec<&super::Inode> = fs.inodes.iter()
        .filter_map(|i| i.as_ref())
        .collect();

    write_u32(&mut out, live.len() as u32);

    for inode in live {
      
        write_u64(&mut out, inode.id as u64);
        // kind: 0 = file, 1 = directory
        out.push(match inode.kind {
            super::InodeKind::File      => 0,
            super::InodeKind::Directory => 1,
        });
       
        write_u64(&mut out, inode.parent as u64);
       
        let name_bytes = inode.name.as_bytes();
        write_u16(&mut out, name_bytes.len() as u16);
        out.extend_from_slice(name_bytes);
        
        write_u32(&mut out, inode.data.len() as u32);
        out.extend_from_slice(&inode.data);
       
        write_u16(&mut out, inode.children.len() as u16);
        for (child_name, &child_id) in &inode.children {
            let cn = child_name.as_bytes();
            write_u16(&mut out, cn.len() as u16);
            out.extend_from_slice(cn);
            write_u64(&mut out, child_id as u64);
        }
    }

    Ok(out)
}

// ── Deserializer ──────────────────────────────────────────────────────────────

fn deserialize(fs: &mut super::OpenMajiFs, data: &[u8])
    -> Result<(), &'static str>
{
    let mut pos = 0;


    let cwd = read_u64(data, &mut pos)? as usize;

  
    let inode_count = read_u32(data, &mut pos)? as usize;

   
    for slot in fs.inodes.iter_mut() {
        *slot = None;
    }

    for _ in 0..inode_count {
        let id     = read_u64(data, &mut pos)? as usize;
        let kind_b = read_u8(data, &mut pos)?;
        let kind   = if kind_b == 0 {
            super::InodeKind::File
        } else {
            super::InodeKind::Directory
        };
        let parent = read_u64(data, &mut pos)? as usize;

        let name_len = read_u16(data, &mut pos)? as usize;
        let name = core::str::from_utf8(read_bytes(data, &mut pos, name_len)?)
            .map_err(|_| "invalid utf8 in name")?;

        let data_len = read_u32(data, &mut pos)? as usize;
        let inode_data = read_bytes(data, &mut pos, data_len)?.to_vec();

        let child_count = read_u16(data, &mut pos)? as usize;
        let mut children = alloc::collections::BTreeMap::new();
        for _ in 0..child_count {
            let cn_len = read_u16(data, &mut pos)? as usize;
            let cn = core::str::from_utf8(read_bytes(data, &mut pos, cn_len)?)
                .map_err(|_| "invalid utf8 in child name")?;
            let child_id = read_u64(data, &mut pos)? as usize;
            children.insert(alloc::string::String::from(cn), child_id);
        }

        if id < fs.inodes.len() {
            fs.inodes[id] = Some(super::Inode {
                id,
                kind,
                name: alloc::string::String::from(name),
                data: inode_data,
                children,
                parent,
            });
        }
    }

    fs.cwd = cwd;
    Ok(())
}



fn write_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}

fn write_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn read_u8(data: &[u8], pos: &mut usize) -> Result<u8, &'static str> {
    if *pos >= data.len() { return Err("disk data truncated"); }
    let v = data[*pos];
    *pos += 1;
    Ok(v)
}

fn read_u16(data: &[u8], pos: &mut usize) -> Result<u16, &'static str> {
    if *pos + 2 > data.len() { return Err("disk data truncated"); }
    let v = u16::from_le_bytes(data[*pos..*pos+2].try_into().unwrap());
    *pos += 2;
    Ok(v)
}

fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32, &'static str> {
    if *pos + 4 > data.len() { return Err("disk data truncated"); }
    let v = u32::from_le_bytes(data[*pos..*pos+4].try_into().unwrap());
    *pos += 4;
    Ok(v)
}

fn read_u64(data: &[u8], pos: &mut usize) -> Result<u64, &'static str> {
    if *pos + 8 > data.len() { return Err("disk data truncated"); }
    let v = u64::from_le_bytes(data[*pos..*pos+8].try_into().unwrap());
    *pos += 8;
    Ok(v)
}

fn read_bytes<'a>(data: &'a [u8], pos: &mut usize, len: usize)
    -> Result<&'a [u8], &'static str>
{
    if *pos + len > data.len() { return Err("disk data truncated"); }
    let slice = &data[*pos..*pos + len];
    *pos += len;
    Ok(slice)
}