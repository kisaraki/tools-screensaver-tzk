//! Calendar labels only: dates remain Gregorian in every presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalendarStyle {
    #[default]
    Chinese,
    English,
    Japanese,
}

impl CalendarStyle {
    pub fn from_registry(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Chinese),
            1 => Some(Self::English),
            2 => Some(Self::Japanese),
            _ => None,
        }
    }
    pub fn registry_value(self) -> u32 {
        match self {
            Self::Chinese => 0,
            Self::English => 1,
            Self::Japanese => 2,
        }
    }
    pub fn month(self, month: u16) -> String {
        let Some(index) = month.checked_sub(1).filter(|i| *i < 12) else {
            return String::new();
        };
        match self {
            Self::Chinese => format!("{month}月"),
            Self::English => [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ][usize::from(index)]
            .into(),
            Self::Japanese => [
                "睦月",
                "如月",
                "弥生",
                "卯月",
                "皐月",
                "水無月",
                "文月",
                "葉月",
                "長月",
                "神無月",
                "霜月",
                "師走",
            ][usize::from(index)]
            .into(),
        }
    }
    pub fn title(self, year: u16, month: u16) -> String {
        let label = self.month(month);
        match self {
            Self::English => format!("{label} {year}"),
            _ => format!("{year}年 {label}"),
        }
    }
    pub fn weekdays(self) -> [&'static str; 7] {
        match self {
            Self::Chinese => ["一", "二", "三", "四", "五", "六", "日"],
            Self::English => ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
            Self::Japanese => ["月曜", "火曜", "水曜", "木曜", "金曜", "土曜", "日曜"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_months_and_monday_first_labels_match_each_presentation() {
        assert_eq!(CalendarStyle::English.month(9), "September");
        assert_eq!(CalendarStyle::Japanese.month(6), "水無月");
        assert_eq!(CalendarStyle::Japanese.weekdays()[1], "火曜");
        assert_eq!(CalendarStyle::English.weekdays()[6], "Sun");
        assert_eq!(CalendarStyle::Chinese.title(2026, 9), "2026年 9月");
        for style in [
            CalendarStyle::Chinese,
            CalendarStyle::English,
            CalendarStyle::Japanese,
        ] {
            assert_eq!(
                CalendarStyle::from_registry(style.registry_value()),
                Some(style)
            );
            for month in 1..=12 {
                assert!(!style.month(month).is_empty());
            }
            assert!(style.month(0).is_empty());
            assert!(style.month(13).is_empty());
        }
        assert!(CalendarStyle::from_registry(3).is_none());
    }
}
