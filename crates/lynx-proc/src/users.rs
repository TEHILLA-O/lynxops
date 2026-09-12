use std::collections::HashMap;

use lynx_core::{Result, SysPaths, Uid};

use crate::util::read_to_string;

pub type UserMap = HashMap<u32, String>;

pub fn load_users(paths: &SysPaths) -> Result<UserMap> {
    match read_to_string(&paths.passwd) {
        Ok(text) => Ok(parse_passwd(&text)),
        Err(_) => Ok(UserMap::new()),
    }
}

pub fn parse_passwd(text: &str) -> UserMap {
    let mut map = UserMap::new();
    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut cols = line.split(':');
        let name = cols.next().unwrap_or("");
        let _passwd = cols.next();
        let uid = cols.next().and_then(|s| s.parse::<u32>().ok());
        if let Some(uid) = uid {
            if !name.is_empty() {
                map.insert(uid, name.to_string());
            }
        }
    }
    map
}

pub fn lookup(map: &UserMap, uid: Uid) -> Option<String> {
    map.get(&uid.0).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwd_maps_uid() {
        let map = parse_passwd("root:x:0:0:root:/root:/bin/bash\nwww-data:x:33:33:www-data:/var/www:/usr/sbin/nologin\n");
        assert_eq!(map.get(&33).map(String::as_str), Some("www-data"));
    }
}
