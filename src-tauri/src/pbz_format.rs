use flate2::read::ZlibDecoder;
use memchr::memmem::Finder;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Mutex,
    time::SystemTime,
};

const HEADER_SIZE: usize = 12;
const BLOCK_HEADER: &[u8; 8] = b"PBZS\x01\x01\0\0";
const BLOCK_HEADER_SIZE: usize = 36;
const MAX_DIRECTORY_SIZE: usize = 64 * 1024;
const MAX_ACHIEVEMENT_SIZE: usize = 256 * 1024;
const MAX_EXE_SIZE: u64 = 1024 * 1024 * 1024;
type SaveKey = [u8; 32];
type KeyCache = Vec<(PathBuf, SystemTime, u64, Vec<SaveKey>)>;
static KEY_CACHE: Mutex<KeyCache> = Mutex::new(Vec::new());

#[derive(Deserialize)]
struct Achievements {
    #[serde(rename = "UnlockedIds")]
    unlocked_ids: Vec<String>,
}

pub(super) fn achievements(bytes: &[u8], executables: &[PathBuf]) -> Result<Vec<String>, String> {
    let header = bytes.get(..HEADER_SIZE).ok_or("存档头不完整")?;
    if !header.starts_with(b"PBZS\x03\x01") {
        return Err("不支持此 PBZS 存档版本".into());
    }
    let directory_size = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
    if !(BLOCK_HEADER_SIZE..=MAX_DIRECTORY_SIZE).contains(&directory_size) {
        return Err("存档目录大小无效".into());
    }
    let directory_end = HEADER_SIZE
        .checked_add(directory_size)
        .ok_or("存档目录越界")?;
    let directory = bytes
        .get(HEADER_SIZE..directory_end)
        .ok_or("存档目录不完整")?;
    if !directory.starts_with(BLOCK_HEADER) {
        return Err("存档目录块无效".into());
    }
    for executable in executables.iter().filter(|path| path.is_file()) {
        for key in cached_keys(executable).map_err(|error| format!("无法读取游戏程序：{error}"))?
        {
            if let Some(index) = open_block(directory, &key) {
                return read_achievement_slot(&index, &bytes[directory_end..], header, &key);
            }
        }
    }
    Err("无法验证存档，请确认游戏程序与存档版本匹配".into())
}

fn open_block(block: &[u8], key: &SaveKey) -> Option<Vec<u8>> {
    if block.len() < BLOCK_HEADER_SIZE || !block.starts_with(BLOCK_HEADER) {
        return None;
    }
    let key = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).ok()?);
    let nonce = Nonce::try_assume_unique_for_key(&block[8..20]).ok()?;
    let mut data = block[BLOCK_HEADER_SIZE..].to_vec();
    data.extend_from_slice(&block[20..BLOCK_HEADER_SIZE]);
    let size = key
        .open_in_place(nonce, Aad::empty(), &mut data)
        .ok()?
        .len();
    data.truncate(size);
    Some(data)
}

fn read_achievement_slot(
    index: &[u8],
    blocks: &[u8],
    header: &[u8],
    key: &SaveKey,
) -> Result<Vec<String>, String> {
    if index.get(..HEADER_SIZE) != Some(header) {
        return Err("存档目录头不匹配".into());
    }
    let slots = u16::from_le_bytes(header[6..8].try_into().unwrap());
    let mut entries = &index[HEADER_SIZE..];
    let mut achievement = None;
    for _ in 0..slots {
        let (name_size, rest) = entries
            .split_first_chunk::<2>()
            .ok_or("存档目录条目不完整")?;
        let (name, rest) = rest
            .split_at_checked(u16::from_le_bytes(*name_size) as usize)
            .ok_or("存档目录名称不完整")?;
        let (entry, rest) = rest.split_first_chunk::<14>().ok_or("存档目录条目不完整")?;
        entries = rest;
        if name != b"__Achievements__" {
            continue;
        }
        if achievement.is_some() {
            return Err("存档含多个成就区块".into());
        }
        let decoded_size = u32::from_le_bytes(entry[2..6].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(entry[6..10].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(entry[10..14].try_into().unwrap()) as usize;
        let block = blocks
            .get(offset..offset.saturating_add(size))
            .ok_or("成就区块越界")?;
        let data = open_block(block, key).ok_or("成就区块校验失败")?;
        achievement = Some(decode_payload(entry[0], decoded_size, data)?);
    }
    if !entries.is_empty() {
        return Err("存档目录长度不匹配".into());
    }
    Ok(achievement.unwrap_or_default())
}

fn decode_payload(codec: u8, size: usize, data: Vec<u8>) -> Result<Vec<String>, String> {
    if size == 0 || size > MAX_ACHIEVEMENT_SIZE {
        return Err("成就区块大小无效".into());
    }
    let decoded = match codec {
        0 => data,
        1 => {
            let mut decoded = Vec::with_capacity(size);
            ZlibDecoder::new(data.as_slice())
                .take(size as u64 + 1)
                .read_to_end(&mut decoded)
                .map_err(|error| format!("成就区块解压失败：{error}"))?;
            decoded
        }
        2 => {
            let mut decoded = vec![0; size];
            let written = oozextract::Extractor::new()
                .read_from_slice(&data, &mut decoded)
                .map_err(|error| format!("成就区块解压失败：{error}"))?;
            decoded.truncate(written);
            decoded
        }
        _ => return Err("不支持成就区块的压缩格式".into()),
    };
    if decoded.len() != size {
        return Err("成就区块解压长度不匹配".into());
    }
    let achievements: Achievements =
        serde_json::from_slice(&decoded).map_err(|error| format!("成就区块格式无效：{error}"))?;
    Ok(achievements.unlocked_ids)
}

fn cached_keys(path: &Path) -> io::Result<Vec<SaveKey>> {
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?;
    let size = metadata.len();
    let mut cache = KEY_CACHE
        .lock()
        .map_err(|_| io::Error::other("存档缓存不可用"))?;
    if let Some((_, _, _, keys)) = cache
        .iter()
        .find(|(exe, time, len, _)| exe == path && *time == modified && *len == size)
    {
        return Ok(keys.clone());
    }
    if size > MAX_EXE_SIZE {
        return Err(io::Error::other("游戏程序文件过大"));
    }
    let keys = find_keys(File::open(path)?.take(MAX_EXE_SIZE))?;
    cache.retain(|(exe, _, _, _)| exe != path);
    if cache.len() == 8 {
        cache.remove(0);
    }
    cache.push((path.to_path_buf(), modified, size, keys.clone()));
    Ok(keys)
}

fn find_keys(mut reader: impl Read) -> io::Result<Vec<SaveKey>> {
    // PBZS v3 embeds two 32-byte halves after this UTF-16 string. The save's
    // authenticated directory determines which alignment contains its key.
    let anchor: Vec<u8> = "sealed but block is PLAINTEXT"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let search = Finder::new(&anchor);
    let overlap = anchor.len() + 15 + 64 - 1;
    let mut buffer = vec![0; 1024 * 1024 + overlap];
    let mut carry = 0;
    let mut keys = Vec::new();
    loop {
        let read = reader.read(&mut buffer[carry..])?;
        if read == 0 {
            return Ok(keys);
        }
        let end = carry + read;
        for offset in search.find_iter(&buffer[..end]) {
            let start = offset + anchor.len();
            for aligned in start..start + 16 {
                let material_end = aligned + 64;
                if material_end <= carry || material_end > end {
                    continue;
                }
                let material = &buffer[aligned..material_end];
                let key = std::array::from_fn(|i| material[i] ^ material[i + 32]);
                if !keys.contains(&key) {
                    keys.push(key);
                }
                if keys.len() > 256 {
                    return Err(io::Error::other("游戏程序中有过多存档密钥候选"));
                }
            }
        }
        carry = end.min(overlap);
        buffer.copy_within(end - carry..end, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_unlocked_ids_from_the_kraken_block() {
        let json = br#"{"UnlockedIds":["ACH_001"],"Counters":{"ACH_002":100},"PlatformPending":[{"Ids":["ACH_003"]}]}"#;
        // A Kraken block that stores its payload verbatim.
        let compressed = [&[0xcc, 0x06], json.as_slice()].concat();
        assert_eq!(
            decode_payload(2, json.len(), compressed).unwrap(),
            ["ACH_001"]
        );
        assert!(decode_payload(0, json.len(), b"{}".to_vec()).is_err());
    }

    #[test]
    fn reads_authenticated_slots_and_rejects_tampered_data() {
        fn seal(data: &[u8], key: &SaveKey, nonce_bytes: [u8; 12]) -> Vec<u8> {
            let key = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).unwrap());
            let nonce = Nonce::assume_unique_for_key(nonce_bytes);
            let mut payload = data.to_vec();
            let tag = key
                .seal_in_place_separate_tag(nonce, Aad::empty(), &mut payload)
                .unwrap();
            [
                BLOCK_HEADER.as_slice(),
                &nonce_bytes,
                tag.as_ref(),
                &payload,
            ]
            .concat()
        }

        let key = [42; 32];
        let json = br#"{"UnlockedIds":["ACH_001"]}"#;
        let name = b"__Achievements__";
        let payload = seal(json, &key, [1; 12]);
        let directory_size = BLOCK_HEADER_SIZE + HEADER_SIZE + 2 + name.len() + 14;
        let mut header = b"PBZS\x03\x01\x01\0".to_vec();
        header.extend_from_slice(&(directory_size as u32).to_le_bytes());
        let mut index = header.clone();
        index.extend_from_slice(&(name.len() as u16).to_le_bytes());
        index.extend_from_slice(name);
        index.extend_from_slice(&[0, 1]);
        index.extend_from_slice(&(json.len() as u32).to_le_bytes());
        index.extend_from_slice(&0_u32.to_le_bytes());
        index.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        let mut save = [header, seal(&index, &key, [2; 12]), payload].concat();
        let temp = tempfile::tempdir().unwrap();
        let executable = temp.path().join("PBZ.exe");
        let mut data = vec![0; 1024 * 1024 - 12];
        data.extend(
            "sealed but block is PLAINTEXT"
                .encode_utf16()
                .flat_map(u16::to_le_bytes),
        );
        data.extend_from_slice(&[0; 11]);
        data.extend_from_slice(&key);
        data.extend_from_slice(&[0; 32]);
        fs::write(&executable, data).unwrap();
        let paths = [executable];
        assert_eq!(achievements(&save, &paths).unwrap(), ["ACH_001"]);
        *save.last_mut().unwrap() ^= 1;
        assert_eq!(achievements(&save, &paths).unwrap_err(), "成就区块校验失败");
    }
}
