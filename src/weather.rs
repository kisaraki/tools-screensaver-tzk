//! Current weather, bounded location settings and explicit display classification.
use crate::net;
use serde_json::Value;
use std::sync::mpsc::{self, Receiver};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CityName([u8; 80]);
impl Default for CityName {
    fn default() -> Self {
        Self([0; 80])
    }
}
impl CityName {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.len() > 80
            || (!text.is_empty()
                && (text.len() < 2
                    || !text.bytes().any(|b| b.is_ascii_alphabetic())
                    || !text
                        .bytes()
                        .all(|b| b.is_ascii_alphabetic() || b" -.',".contains(&b))))
        {
            return None;
        }
        let mut bytes = [0; 80];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        Some(Self(bytes))
    }
    pub fn text(&self) -> &str {
        std::str::from_utf8(&self.0[..self.0.iter().position(|b| *b == 0).unwrap_or(80)])
            .unwrap_or("")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeatherSettings {
    pub auto_location: bool,
    pub city_index: u32,
    pub custom_city: CityName,
}
impl Default for WeatherSettings {
    fn default() -> Self {
        Self {
            auto_location: true,
            city_index: 0,
            custom_city: CityName::default(),
        }
    }
}
#[derive(Clone, Copy)]
pub struct City {
    pub label: &'static str,
    pub name: &'static str,
    pub country: &'static str,
    pub lat: f64,
    pub lon: f64,
}
macro_rules! city {
    ($label:expr,$name:expr,$country:expr,$lat:expr,$lon:expr) => {
        City {
            label: $label,
            name: $name,
            country: $country,
            lat: $lat,
            lon: $lon,
        }
    };
}
pub const CITIES: &[City] = &[
    city!("臺灣 · 臺北 (Taipei)", "Taipei", "TW", 25.033, 121.565),
    city!(
        "臺灣 · 新北 (New Taipei)",
        "New Taipei",
        "TW",
        25.012,
        121.465
    ),
    city!("臺灣 · 桃園 (Taoyuan)", "Taoyuan", "TW", 24.994, 121.301),
    city!("臺灣 · 新竹 (Hsinchu)", "Hsinchu", "TW", 24.804, 120.971),
    city!("臺灣 · 臺中 (Taichung)", "Taichung", "TW", 24.147, 120.674),
    city!("臺灣 · 嘉義 (Chiayi)", "Chiayi", "TW", 23.481, 120.449),
    city!("臺灣 · 臺南 (Tainan)", "Tainan", "TW", 22.999, 120.227),
    city!(
        "臺灣 · 高雄 (Kaohsiung)",
        "Kaohsiung",
        "TW",
        22.627,
        120.301
    ),
    city!("臺灣 · 宜蘭 (Yilan)", "Yilan", "TW", 24.757, 121.753),
    city!("臺灣 · 花蓮 (Hualien)", "Hualien", "TW", 23.975, 121.601),
    city!("臺灣 · 臺東 (Taitung)", "Taitung", "TW", 22.758, 121.144),
    city!("臺灣 · 澎湖 (Penghu)", "Penghu", "TW", 23.566, 119.578),
    city!("臺灣 · 金門 (Kinmen)", "Kinmen", "TW", 24.436, 118.318),
    city!("臺灣 · 馬祖 (Matsu)", "Matsu", "TW", 26.16, 119.95),
    city!("日本 · 東京 (Tokyo)", "Tokyo", "JP", 35.676, 139.65),
    city!("日本 · 大阪 (Osaka)", "Osaka", "JP", 34.694, 135.502),
    city!("日本 · 京都 (Kyoto)", "Kyoto", "JP", 35.011, 135.768),
    city!("日本 · 札幌 (Sapporo)", "Sapporo", "JP", 43.061, 141.354),
    city!("日本 · 福岡 (Fukuoka)", "Fukuoka", "JP", 33.59, 130.402),
    city!("日本 · 那霸 (Naha)", "Naha", "JP", 26.212, 127.681),
    city!("美國 · 紐約 (New York)", "New York", "US", 40.713, -74.006),
    city!(
        "美國 · 洛杉磯 (Los Angeles)",
        "Los Angeles",
        "US",
        34.052,
        -118.244
    ),
    city!("美國 · 芝加哥 (Chicago)", "Chicago", "US", 41.878, -87.63),
    city!("美國 · 西雅圖 (Seattle)", "Seattle", "US", 47.606, -122.332),
    city!(
        "美國 · 舊金山 (San Francisco)",
        "San Francisco",
        "US",
        37.775,
        -122.419
    ),
    city!("澳洲 · 雪梨 (Sydney)", "Sydney", "AU", -33.869, 151.209),
    city!(
        "澳洲 · 墨爾本 (Melbourne)",
        "Melbourne",
        "AU",
        -37.814,
        144.963
    ),
    city!(
        "澳洲 · 布里斯本 (Brisbane)",
        "Brisbane",
        "AU",
        -27.47,
        153.026
    ),
    city!("澳洲 · 伯斯 (Perth)", "Perth", "AU", -31.952, 115.861),
    city!("新加坡 · Singapore", "Singapore", "SG", 1.352, 103.82),
    city!("中國 · 北京 (Beijing)", "Beijing", "CN", 39.904, 116.407),
    city!("中國 · 上海 (Shanghai)", "Shanghai", "CN", 31.23, 121.474),
    city!(
        "中國 · 廣州 (Guangzhou)",
        "Guangzhou",
        "CN",
        23.129,
        113.264
    ),
    city!("中國 · 深圳 (Shenzhen)", "Shenzhen", "CN", 22.543, 114.058),
    city!("英國 · 倫敦 (London)", "London", "GB", 51.507, -0.128),
    city!(
        "英國 · 曼徹斯特 (Manchester)",
        "Manchester",
        "GB",
        53.48,
        -2.242
    ),
    city!(
        "英國 · 愛丁堡 (Edinburgh)",
        "Edinburgh",
        "GB",
        55.953,
        -3.188
    ),
    city!("法國 · 巴黎 (Paris)", "Paris", "FR", 48.857, 2.352),
    city!("法國 · 里昂 (Lyon)", "Lyon", "FR", 45.764, 4.836),
    city!("法國 · 馬賽 (Marseille)", "Marseille", "FR", 43.296, 5.37),
    city!("德國 · 柏林 (Berlin)", "Berlin", "DE", 52.52, 13.405),
    city!("德國 · 慕尼黑 (Munich)", "Munich", "DE", 48.135, 11.582),
    city!("德國 · 漢堡 (Hamburg)", "Hamburg", "DE", 53.551, 9.994),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Sunny,
    Cloudy,
    Windy,
    Rain,
    Pouring,
    Storm,
    Snow,
    Blizzard,
}
impl Condition {
    pub const ALL: [Self; 8] = [
        Self::Sunny,
        Self::Cloudy,
        Self::Windy,
        Self::Rain,
        Self::Pouring,
        Self::Storm,
        Self::Snow,
        Self::Blizzard,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Sunny => "晴",
            Self::Cloudy => "曇",
            Self::Windy => "強風",
            Self::Rain => "雨",
            Self::Pouring => "土砂降り",
            Self::Storm => "嵐",
            Self::Snow => "雪",
            Self::Blizzard => "吹雪",
        }
    }
    pub(crate) fn png(self) -> &'static [u8] {
        match self {
            Self::Sunny => include_bytes!("../resources/wpic/background/晴.png"),
            Self::Cloudy => include_bytes!("../resources/wpic/background/曇.png"),
            Self::Windy => include_bytes!("../resources/wpic/background/強風.png"),
            Self::Rain => include_bytes!("../resources/wpic/background/雨.png"),
            Self::Pouring => include_bytes!("../resources/wpic/background/土砂降り.png"),
            Self::Storm => include_bytes!("../resources/wpic/background/嵐.png"),
            Self::Snow => include_bytes!("../resources/wpic/background/雪.png"),
            Self::Blizzard => include_bytes!("../resources/wpic/background/吹雪.png"),
        }
    }
}
pub fn classify(code: u32, rain_hour: Option<f64>, wind: f64) -> Condition {
    let snow = matches!(code, 71..=77 | 85 | 86);
    if snow && wind >= 10.0 {
        Condition::Blizzard
    } else if wind >= 20.0 || code >= 95 {
        Condition::Storm
    } else if snow {
        Condition::Snow
    } else if rain_hour.is_some_and(|r| r >= 20.0) {
        Condition::Pouring
    } else if wind >= 15.0 {
        Condition::Windy
    } else if matches!(code,51..=67|80..=82) || rain_hour.is_some_and(|r| r > 0.0) {
        Condition::Rain
    } else if code >= 2 {
        Condition::Cloudy
    } else {
        Condition::Sunny
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub place: String,
    pub condition: Condition,
    pub temperature: Option<f64>,
    pub humidity: Option<f64>,
    pub wind: Option<f64>,
    pub rain_hour: Option<f64>,
    pub observed: String,
    pub source: String,
    pub location: String,
    pub status: String,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            place: "Taipei, TW".into(),
            condition: Condition::Sunny,
            temperature: None,
            humidity: None,
            wind: None,
            rain_hour: None,
            observed: String::new(),
            source: String::new(),
            location: String::new(),
            status: "等待氣象資料（離線預覽）".into(),
        }
    }
}
struct Location {
    name: String,
    country: String,
    lat: f64,
    lon: f64,
    method: String,
}
fn valid_coords(lat: f64, lon: f64) -> bool {
    lat.is_finite()
        && lon.is_finite()
        && (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
}
fn civil_minutes(year: u16, month: u16, day: u16, hour: u16, minute: u16) -> Option<u64> {
    if year < 1970
        || !(1..=12).contains(&month)
        || day == 0
        || day > crate::model::days_in_month(year, month)
        || hour > 23
        || minute > 59
    {
        return None;
    }
    let days = (1970..year)
        .map(|y| {
            if crate::model::days_in_month(y, 2) == 29 {
                366u64
            } else {
                365
            }
        })
        .sum::<u64>()
        + (1..month)
            .map(|m| u64::from(crate::model::days_in_month(year, m)))
            .sum::<u64>()
        + u64::from(day - 1);
    Some(days * 1440 + u64::from(hour) * 60 + u64::from(minute))
}
fn recent(now: u64, observed: u64) -> bool {
    observed <= now.saturating_add(5) && now.saturating_sub(observed) <= 120
}
fn current_minutes() -> Result<u64, String> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "系統時間無效")?
        .as_secs()
        / 60)
}
fn safe_text(v: &Value, key: &str, max: usize) -> Option<String> {
    v[key]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control))
        .map(str::to_owned)
}
fn ip_location() -> Result<Location, String> {
    let v = net::json(
        "ipwho.is",
        "/?fields=success,city,country_code,latitude,longitude",
    )?;
    if v["success"].as_bool() != Some(true) {
        return Err("IP 定位失敗".into());
    }
    let lat = v["latitude"].as_f64().ok_or("定位缺少緯度")?;
    let lon = v["longitude"].as_f64().ok_or("定位缺少經度")?;
    let country = safe_text(&v, "country_code", 2)
        .filter(|s| s.len() == 2 && s.bytes().all(|b| b.is_ascii_uppercase()))
        .ok_or("定位國碼無效")?;
    if !valid_coords(lat, lon) {
        return Err("定位座標無效".into());
    }
    Ok(Location {
        name: safe_text(&v, "city", 100).ok_or("定位城市無效")?,
        country,
        lat,
        lon,
        method: "IP 約略定位".into(),
    })
}
fn configured_location(settings: WeatherSettings) -> Result<Location, String> {
    let city = CITIES
        .get(settings.city_index as usize)
        .unwrap_or(&CITIES[0]);
    if settings.custom_city.text().is_empty() {
        return Ok(Location {
            name: city.name.into(),
            country: city.country.into(),
            lat: city.lat,
            lon: city.lon,
            method: "控制臺城市".into(),
        });
    }
    let name = settings
        .custom_city
        .text()
        .bytes()
        .map(|b| format!("%{b:02X}"))
        .collect::<String>();
    let v = net::json(
        "geocoding-api.open-meteo.com",
        &format!(
            "/v1/search?name={name}&count=10&language=en&format=json&countryCode={}",
            city.country
        ),
    )?;
    let result = v["results"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|r| r["country_code"].as_str() == Some(city.country))
        })
        .ok_or("無法辨識英文城市名，請確認名稱及所選國家")?;
    let lat = result["latitude"].as_f64().ok_or("城市座標缺失")?;
    let lon = result["longitude"].as_f64().ok_or("城市座標缺失")?;
    if !valid_coords(lat, lon) {
        return Err("城市座標無效".into());
    }
    Ok(Location {
        name: safe_text(result, "name", 100).ok_or("城市名無效")?,
        country: city.country.into(),
        lat,
        lon,
        method: "自訂英文城市／GeoNames".into(),
    })
}
fn number(v: &Value, key: &str, min: f64, max: f64) -> Option<f64> {
    v[key]
        .as_f64()
        .filter(|n| n.is_finite() && (min..=max).contains(n))
}
fn global(location: &Location) -> Result<Snapshot, String> {
    let v=net::json("api.open-meteo.com",&format!("/v1/forecast?latitude={:.4}&longitude={:.4}&current=temperature_2m,relative_humidity_2m,precipitation,weather_code,wind_speed_10m&hourly=rain&forecast_days=1&wind_speed_unit=ms&timezone=auto",location.lat,location.lon))?;
    let current = &v["current"];
    let observed = safe_text(current, "time", 32).ok_or("氣象時間缺失")?;
    let parts = observed
        .get(..16)
        .ok_or("氣象時間格式無效")?
        .split(['-', 'T', ':'])
        .map(str::parse::<u16>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "氣象時間格式無效")?;
    let stamp = if parts.len() == 5 {
        civil_minutes(parts[0], parts[1], parts[2], parts[3], parts[4])
    } else {
        None
    }
    .ok_or("氣象時間格式無效")?;
    let offset = v["utc_offset_seconds"]
        .as_i64()
        .filter(|s| s.abs() <= 14 * 3600)
        .ok_or("氣象時區無效")?
        / 60;
    let now = (current_minutes()? as i64 + offset).max(0) as u64;
    if !recent(now, stamp) {
        return Err("目前天氣模型時間已過期".into());
    }
    let temperature = number(current, "temperature_2m", -100.0, 70.0).ok_or("氣溫缺失")?;
    let wind = number(current, "wind_speed_10m", 0.0, 150.0).ok_or("風速缺失")?;
    let code = current["weather_code"]
        .as_u64()
        .filter(|c| *c <= 99)
        .ok_or("天氣代碼無效")? as u32;
    let hour = observed.get(..13).ok_or("時間格式無效")?;
    let index = v["hourly"]["time"].as_array().and_then(|times| {
        times
            .iter()
            .position(|t| t.as_str().is_some_and(|s| s.starts_with(hour)))
    });
    let rain = index
        .and_then(|i| v["hourly"]["rain"][i].as_f64())
        .filter(|r| (0.0..=1000.0).contains(r));
    Ok(Snapshot {
        place: format!("{}, {}", location.name, location.country),
        condition: classify(code, rain, wind),
        temperature: Some(temperature),
        humidity: number(current, "relative_humidity_2m", 0.0, 100.0),
        wind: Some(wind),
        rain_hour: rain,
        observed,
        source: "Open-Meteo · CC BY 4.0 · 目前天氣模型".into(),
        location: location.method.clone(),
        status: "已更新".into(),
    })
}
const STATIONS: &[(&str, &str, f64, f64)] = &[
    ("46692", "臺北", 25.037, 121.514),
    ("46688", "板橋", 25.013, 121.442),
    ("46705", "新屋", 25.007, 121.047),
    ("46757", "新竹", 24.827, 121.015),
    ("46749", "臺中", 24.145, 120.684),
    ("46748", "嘉義", 23.497, 120.425),
    ("46741", "臺南", 22.993, 120.205),
    ("46744", "高雄", 22.567, 120.315),
    ("46708", "宜蘭", 24.765, 121.748),
    ("46699", "花蓮", 23.976, 121.605),
    ("46766", "臺東", 22.752, 121.154),
    ("46735", "澎湖", 23.566, 119.563),
    ("46711", "金門", 24.407, 118.289),
    ("46799", "馬祖", 26.168, 119.923),
    ("46694", "基隆", 25.135, 121.732),
];
fn cell<'a>(row: &'a str, key: &str) -> Option<&'a str> {
    let start = row.find(&format!("headers=\"{key}\""))?;
    let rest = &row[start..];
    let rest = &rest[rest.find('>')? + 1..];
    rest.split("</td>").next()
}
fn span<'a>(text: &'a str, class: &str) -> Option<&'a str> {
    let rest = &text[text.find(&format!("class=\"{class}"))?..];
    let rest = &rest[rest.find('>')? + 1..];
    rest.split('<').next()
}
fn numeric(text: Option<&str>, min: f64, max: f64) -> Option<f64> {
    text?
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && (min..=max).contains(n))
}
fn strip_tags(text: &str) -> String {
    let mut inside = false;
    text.chars()
        .filter(|c| {
            if *c == '<' {
                inside = true;
                false
            } else if *c == '>' {
                inside = false;
                false
            } else {
                !inside
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn row_time(row: &str) -> Option<String> {
    row.split("headers=\"time\"")
        .nth(1)?
        .split_once('>')?
        .1
        .split("</th>")
        .next()
        .map(strip_tags)
}
fn time_parts(text: &str) -> Option<[u32; 4]> {
    let fields: Vec<_> = text.split_whitespace().collect();
    if fields.len() != 2 {
        return None;
    }
    let (month, day) = fields[0].split_once('/')?;
    let (hour, minute) = fields[1].split_once(':')?;
    let parts = [
        month.parse().ok()?,
        day.parse().ok()?,
        hour.parse().ok()?,
        minute.parse().ok()?,
    ];
    (parts[0] >= 1
        && parts[0] <= 12
        && parts[1] >= 1
        && parts[1] <= 31
        && parts[2] < 24
        && parts[3] < 60)
        .then_some(parts)
}
fn adjacent_observations(newer: &str, older: &str) -> bool {
    let Some(a) = row_time(newer).and_then(|s| time_parts(&s)) else {
        return false;
    };
    let Some(b) = row_time(older).and_then(|s| time_parts(&s)) else {
        return false;
    };
    let am = a[2] * 60 + a[3];
    let bm = b[2] * 60 + b[3];
    if a[..2] == b[..2] {
        am.checked_sub(bm) == Some(10)
    } else {
        am == 0 && bm == 1430
    }
}
pub(crate) fn parse_cwa(
    html: &str,
    place: &str,
    station: &str,
    method: &str,
) -> Result<Snapshot, String> {
    let rows: Vec<_> = html.split("</tr>").take(7).collect();
    let first = *rows.first().ok_or("觀測表缺失")?;
    let temperature = numeric(
        cell(first, "temp").and_then(|s| span(s, "tem-C")),
        -100.0,
        70.0,
    )
    .ok_or("CWA 氣溫缺失")?;
    let wind = numeric(
        cell(first, "w-2").and_then(|s| span(s, "wind_2")),
        0.0,
        150.0,
    );
    let weather = cell(first, "weather")
        .and_then(|s| s.split("title=\"").nth(1))
        .and_then(|s| s.split('"').next())
        .ok_or("CWA 天氣現象缺失")?;
    let rain = if rows.len() == 7 {
        let mut sum = 0.0;
        let mut valid = true;
        for pair in rows.windows(2) {
            let newest = numeric(cell(pair[0], "rain"), 0.0, 3000.0);
            let oldest = numeric(cell(pair[1], "rain"), 0.0, 3000.0);
            if !adjacent_observations(pair[0], pair[1]) {
                valid = false;
                continue;
            }
            if let (Some(a), Some(b)) = (newest, oldest) {
                sum += if a >= b { a - b } else { a };
            } else {
                valid = false;
            }
        }
        valid.then_some(sum)
    } else {
        None
    };
    let code = if weather.contains('雪') {
        71
    } else if weather.contains('雷') {
        95
    } else if weather.contains('雨') {
        61
    } else if weather.contains('雲') || weather.contains('陰') || weather.contains('霧') {
        3
    } else {
        0
    };
    let time = row_time(first)
        .filter(|s| time_parts(s).is_some())
        .ok_or("CWA 時間缺失")?;
    Ok(Snapshot {
        place: format!("{place}, TW"),
        condition: classify(code, rain, wind.unwrap_or(0.0)),
        temperature: Some(temperature),
        humidity: numeric(cell(first, "hum"), 0.0, 100.0),
        wind,
        rain_hour: rain,
        observed: time,
        source: format!("中央氣象署 · {station}測站觀測"),
        location: method.into(),
        status: "已更新".into(),
    })
}
fn taiwan(location: &Location) -> Result<Snapshot, String> {
    let distance = |s: &(&str, &str, f64, f64)| {
        ((location.lat - s.2).powi(2)
            + ((location.lon - s.3) * location.lat.to_radians().cos()).powi(2))
        .sqrt()
    };
    let station = STATIONS
        .iter()
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
        .ok_or("CWA 測站缺失")?;
    if distance(station) > 1.5 {
        return Err("附近沒有已支援的 CWA 測站".into());
    }
    let bytes = net::get(
        "www.cwa.gov.tw",
        &format!("/V8/C/W/Observe/MOD/24hr/{}.html", station.0),
        512 * 1024,
    )?;
    let snapshot = parse_cwa(
        std::str::from_utf8(&bytes).map_err(|_| "CWA 編碼無效")?,
        &location.name,
        station.1,
        &location.method,
    )?;
    let parts = time_parts(&snapshot.observed).ok_or("CWA 時間無效")?;
    let mut utc = Default::default();
    unsafe {
        windows_sys::Win32::System::SystemInformation::GetSystemTime(&mut utc);
    }
    let now = current_minutes()?.saturating_add(480);
    if ![
        utc.wYear.saturating_sub(1),
        utc.wYear,
        utc.wYear.saturating_add(1),
    ]
    .iter()
    .any(|year| {
        civil_minutes(
            *year,
            parts[0] as u16,
            parts[1] as u16,
            parts[2] as u16,
            parts[3] as u16,
        )
        .is_some_and(|stamp| recent(now, stamp))
    }) {
        return Err("CWA 測站觀測已超過兩小時".into());
    }
    Ok(snapshot)
}
pub(crate) fn fetch(settings: WeatherSettings) -> Result<Snapshot, String> {
    let location = if settings.auto_location {
        ip_location().or_else(|_| configured_location(settings))?
    } else {
        configured_location(settings)?
    };
    if location.country == "TW" {
        taiwan(&location)
    } else {
        global(&location)
    }
}
pub(crate) fn fetch_async(settings: WeatherSettings) -> Receiver<Result<Snapshot, String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(fetch(settings));
    });
    rx
}
pub(crate) fn preview(settings: WeatherSettings) -> Snapshot {
    let city = CITIES
        .get(settings.city_index as usize)
        .unwrap_or(&CITIES[0]);
    Snapshot {
        place: format!(
            "{}, {}",
            if settings.custom_city.text().is_empty() {
                city.name
            } else {
                settings.custom_city.text()
            },
            city.country
        ),
        ..Snapshot::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_bounded_and_never_urls() {
        assert!(CityName::parse("San Francisco").is_some());
        assert!(CityName::parse("臺北").is_none());
        assert!(CityName::parse("https://test.com").is_none());
        assert!(CityName::parse(&"A".repeat(81)).is_none());
        assert_eq!(CityName::parse("").unwrap().text(), "");
    }
    #[test]
    fn classification_boundaries_and_precedence() {
        assert_eq!(classify(0, None, 0.0), Condition::Sunny);
        assert_eq!(classify(3, None, 0.0), Condition::Cloudy);
        assert_eq!(classify(61, Some(19.9), 0.0), Condition::Rain);
        assert_eq!(classify(61, Some(20.0), 0.0), Condition::Pouring);
        assert_eq!(classify(0, None, 14.9), Condition::Sunny);
        assert_eq!(classify(0, None, 15.0), Condition::Windy);
        assert_eq!(classify(0, None, 20.0), Condition::Storm);
        assert_eq!(classify(95, None, 0.0), Condition::Storm);
        assert_eq!(classify(71, None, 9.9), Condition::Snow);
        assert_eq!(classify(71, None, 10.0), Condition::Blizzard);
    }
    #[test]
    fn cwa_table_parsing_does_not_execute_remote_script() {
        let row = |index: u32, rain: f64| {
            let clock = 1250 - index * 10;
            let (hour, minute) = (clock / 60, clock % 60);
            format!("<tr><th headers=\"time\">09/15<br> {hour:02}:{minute:02}</th><td headers=\"temp\"><span class=\"tem-C is-active\">28</span></td><td headers=\"weather\"><img title=\"雨\"></td><td headers=\"w-2\"><span class=\"wind_2 is-active\">2</span></td><td headers=\"hum\">80</td><td headers=\"rain\">{rain}</td></tr>")
        };
        let html = (0..7)
            .map(|i| row(i, 30.0 - i as f64 * 4.0))
            .collect::<String>();
        let snapshot = parse_cwa(&html, "Taipei", "臺北", "控制臺").unwrap();
        assert_eq!(snapshot.temperature, Some(28.0));
        assert_eq!(snapshot.rain_hour, Some(24.0));
        assert_eq!(snapshot.condition, Condition::Pouring);
        let duplicate = (0..7)
            .map(|i| row(0, 30.0 - i as f64 * 4.0))
            .collect::<String>();
        assert_eq!(
            parse_cwa(&duplicate, "Taipei", "臺北", "")
                .unwrap()
                .rain_hour,
            None
        );
        assert!(parse_cwa("<script>bad()</script>", "Taipei", "臺北", "").is_err());
    }
    #[test]
    fn stale_data_is_rejected_and_calendar_dates_are_validated() {
        assert!(recent(200, 80));
        assert!(!recent(200, 79));
        assert!(recent(200, 205));
        assert!(!recent(200, 206));
        assert!(civil_minutes(2024, 2, 29, 12, 0).is_some());
        assert!(civil_minutes(2026, 2, 29, 12, 0).is_none());
    }
    #[test]
    #[ignore = "explicit bounded HTTPS weather/location probe; no UI"]
    fn public_weather_network_probe() {
        let mut results = Vec::new();
        for settings in [
            WeatherSettings {
                auto_location: false,
                ..WeatherSettings::default()
            },
            WeatherSettings {
                auto_location: false,
                city_index: 14,
                ..WeatherSettings::default()
            },
            WeatherSettings {
                auto_location: false,
                city_index: 14,
                custom_city: CityName::parse("Yokohama").unwrap(),
            },
        ] {
            let snapshot = fetch(settings).unwrap();
            assert!(snapshot.temperature.is_some());
            results.push(serde_json::json!({"place":snapshot.place,"temperature":snapshot.temperature,"condition":snapshot.condition.label(),"source":snapshot.source,"observed":snapshot.observed,"location":snapshot.location,"rain_hour":snapshot.rain_hour,"wind":snapshot.wind}));
        }
        let ip = ip_location().unwrap();
        assert!(valid_coords(ip.lat, ip.lon));
        results.push(serde_json::json!({"ip_location_probe":"PASS","country":ip.country}));
        let text = serde_json::to_string_pretty(&results).unwrap();
        println!("{text}");
        if let Some(path) = std::env::var_os("WEATHER_PROBE_OUTPUT") {
            std::fs::write(path, text).unwrap();
        }
    }
}
