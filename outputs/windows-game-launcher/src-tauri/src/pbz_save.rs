use crate::{
    model::{Game, Unlock},
    pbz_format,
};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

pub(crate) const APPID: &str = "4115450";
const SAVE_FILE: &str = "PBZSaveGame.sav";
const MAX_SAVE_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) fn is_save(game: &Game, path: &Path) -> bool {
    game.appid == APPID
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("sav"))
}

fn install_root(game: &Game) -> Option<&Path> {
    Path::new(&game.exe_path)
        .parent()?
        .ancestors()
        .take(6)
        .find(|folder| folder.join("ProjectPBZ").is_dir())
}

pub(crate) fn save_folder(game: &Game) -> Option<PathBuf> {
    Some(install_root(game)?.join("ProjectPBZ/Saved/SaveGames/Steam"))
}

pub(crate) fn discover(game: &Game) -> Result<Option<PathBuf>, String> {
    let Some(folder) = save_folder(game) else {
        return Ok(None);
    };
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("无法读取存档目录：{error}")),
    };
    let mut save = None;
    for entry in entries {
        let entry = entry.map_err(|error| format!("无法读取存档目录：{error}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.len() != 17 || !name.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let file = entry.path().join(SAVE_FILE);
        if file.is_file() && save.replace(file).is_some() {
            return Err("找到多个玩家存档，请选择要读取的 PBZSaveGame.sav".into());
        }
    }
    Ok(save)
}

pub(crate) fn read(game: &Game, file: &Path) -> Result<Vec<Unlock>, String> {
    let root = install_root(game).ok_or("请将游戏程序设为《影之刃零》安装目录内的 EXE")?;
    let executables = [
        root.join("platform/steam/PBZ.exe"),
        PathBuf::from(&game.exe_path),
    ];
    let handle = File::open(file).map_err(|error| format!("无法打开存档：{error}"))?;
    let mut bytes = Vec::new();
    handle
        .take(MAX_SAVE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("无法读取存档：{error}"))?;
    if bytes.len() as u64 > MAX_SAVE_BYTES {
        return Err("存档文件超过 64 MB 限制".into());
    }
    let mut names = pbz_format::achievements(&bytes, &executables)?;
    if names.iter().any(|name| {
        !name
            .strip_prefix("ACH_")
            .is_some_and(|id| id.len() == 3 && id.bytes().all(|byte| byte.is_ascii_digit()))
    }) {
        return Err("存档含有未知格式的成就标识".into());
    }
    names.sort_unstable();
    names.dedup();
    Ok(names
        .into_iter()
        .map(|api_name| Unlock {
            api_name,
            unlocked_at: None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_only_an_unambiguous_active_save() {
        let dir = tempfile::tempdir().unwrap();
        let game = Game {
            appid: APPID.into(),
            exe_path: dir
                .path()
                .join("platform/steam/PBZ.exe")
                .to_string_lossy()
                .into(),
            ..Default::default()
        };
        let folder = dir.path().join("ProjectPBZ/Saved/SaveGames/Steam");
        let player = folder.join("76561198000000001");
        fs::create_dir_all(player.join("1RealTime")).unwrap();
        fs::write(player.join("1RealTime/old.sav"), []).unwrap();
        assert_eq!(discover(&game).unwrap(), None);
        let active = player.join(SAVE_FILE);
        fs::write(&active, []).unwrap();
        assert_eq!(discover(&game).unwrap(), Some(active));
        let other = folder.join("76561198000000002");
        fs::create_dir(&other).unwrap();
        fs::write(other.join(SAVE_FILE), []).unwrap();
        assert!(discover(&game).is_err());
    }
}
