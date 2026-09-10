//! Strict, offline YouTube link normalization. User URLs are never navigated.
pub const MAX_SOURCES: usize = 10;
pub const MAX_INPUT_BYTES: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YouTubeSource {
    id: [u8; 64],
    len: u8,
    playlist: bool,
}

impl YouTubeSource {
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        if input.len() > 2048
            || !input.is_ascii()
            || input
                .bytes()
                .any(|b| b.is_ascii_whitespace() || b.is_ascii_control() || b == b'\\')
        {
            return None;
        }
        let rest = input
            .strip_prefix("https://")
            .or_else(|| input.strip_prefix("http://"))?;
        let (host, tail) = rest.split_once('/')?;
        let host = host.to_ascii_lowercase();
        if ![
            "youtube.com",
            "www.youtube.com",
            "m.youtube.com",
            "music.youtube.com",
            "youtu.be",
            "www.youtu.be",
        ]
        .contains(&host.as_str())
        {
            return None;
        }
        let tail = tail.split('#').next()?;
        let (path, query) = tail.split_once('?').unwrap_or((tail, ""));
        let values = |key: &str| -> Option<&str> {
            let mut found = query
                .split('&')
                .filter_map(|part| part.split_once('='))
                .filter(|(name, _)| *name == key)
                .map(|(_, value)| value);
            let first = found.next()?;
            if found.next().is_some() {
                None
            } else {
                Some(first)
            }
        };
        let short = host.ends_with("youtu.be");
        let (id, playlist) = if !short
            && (path == "playlist" || path == "watch")
            && query.split('&').any(|p| p.starts_with("list="))
        {
            (values("list")?, true)
        } else if short {
            (path, false)
        } else if path == "watch" {
            (values("v")?, false)
        } else {
            (
                path.strip_prefix("shorts/")
                    .or_else(|| path.strip_prefix("live/"))
                    .or_else(|| path.strip_prefix("embed/"))?,
                false,
            )
        };
        if !(if playlist {
            (10..=64).contains(&id.len())
        } else {
            id.len() == 11
        }) || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return None;
        }
        let mut bytes = [0; 64];
        bytes[..id.len()].copy_from_slice(id.as_bytes());
        Some(Self {
            id: bytes,
            len: id.len() as u8,
            playlist,
        })
    }

    pub fn id(&self) -> &str {
        // Constructed only by the ASCII ID validator above.
        std::str::from_utf8(&self.id[..usize::from(self.len)]).unwrap_or_default()
    }

    pub fn is_playlist(self) -> bool {
        self.playlist
    }

    pub fn url(self) -> String {
        format!(
            "https://www.youtube.com/{}{}",
            if self.playlist {
                "playlist?list="
            } else {
                "watch?v="
            },
            self.id()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceList([Option<YouTubeSource>; MAX_SOURCES]);

impl SourceList {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > MAX_INPUT_BYTES {
            return Err("來源文字過長。".into());
        }
        let mut result = Self::default();
        let mut count = 0;
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let source = YouTubeSource::parse(line)
                .ok_or_else(|| format!("第 {} 行不是有效的 YouTube 影片或清單連結。", index + 1))?;
            if result.0.contains(&Some(source)) {
                continue;
            }
            if count == MAX_SOURCES {
                return Err(format!("每個場景最多加入 {MAX_SOURCES} 個來源。"));
            }
            result.0[count] = Some(source);
            count += 1;
        }
        Ok(result)
    }

    pub fn iter(&self) -> impl Iterator<Item = YouTubeSource> + '_ {
        self.0.iter().flatten().copied()
    }

    pub fn text(self) -> String {
        self.iter()
            .map(YouTubeSource::url)
            .collect::<Vec<_>>()
            .join("\r\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_links_normalize_and_list_takes_precedence() {
        for url in [
            "https://youtu.be/Ee27soLzJ5c?si=abc",
            "https://www.youtube.com/watch?v=Ee27soLzJ5c&t=10",
            "https://m.youtube.com/shorts/Ee27soLzJ5c",
            "https://youtube.com/live/Ee27soLzJ5c",
            "http://youtube.com/embed/Ee27soLzJ5c",
        ] {
            assert_eq!(
                YouTubeSource::parse(url).unwrap().url(),
                "https://www.youtube.com/watch?v=Ee27soLzJ5c"
            );
        }
        let list =
            YouTubeSource::parse("https://www.youtube.com/watch?v=Ee27soLzJ5c&list=PLdsqwBj2O1Nw")
                .unwrap();
        assert!(list.is_playlist());
        assert_eq!(list.id(), "PLdsqwBj2O1Nw");
    }
    #[test]
    fn malicious_hosts_schemes_and_ambiguous_ids_are_rejected() {
        for url in [
            "file:///watch?v=Ee27soLzJ5c",
            "javascript:alert(1)",
            "https://youtube.com.evil/watch?v=Ee27soLzJ5c",
            "https://youtube.com@evil/watch?v=Ee27soLzJ5c",
            "https://evil@youtube.com/watch?v=Ee27soLzJ5c",
            "https://youtube.com:443/watch?v=Ee27soLzJ5c",
            "https://youtube.com/watch?v=Ee27soLzJ5c&v=aaaaaaaaaaa",
            "https://youtube.com/watch?v=%22script%22",
            "https://youtu.be/Ee27soLzJ5c/extra",
            "https://youtube.com/channel/UCabcdefghi",
            "https://youtu.be/short",
            "https://youtube.com/playlist?list=PLdsqwBj2O1Nw&list=PLBH60D9AGfu0",
        ] {
            assert!(YouTubeSource::parse(url).is_none(), "{url}");
        }
    }
    #[test]
    fn list_is_bounded_deduplicated_and_round_trips() {
        let list = SourceList::parse(
            "https://youtu.be/Ee27soLzJ5c\n\nhttps://youtube.com/watch?v=Ee27soLzJ5c",
        )
        .unwrap();
        assert_eq!(list.iter().count(), 1);
        assert_eq!(SourceList::parse(&list.text()).unwrap(), list);
        let many = (0..11)
            .map(|i| format!("https://youtu.be/{i:011}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(SourceList::parse(&many).is_err());
        assert!(SourceList::parse(&"x".repeat(MAX_INPUT_BYTES + 1)).is_err());
        assert_eq!(SourceList::parse("").unwrap(), SourceList::default());
    }
}
