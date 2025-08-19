use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc, Datelike};
use serde::{Deserialize, Serialize};

/// 日付時刻のフォーマット定数
pub const DATE_FORMAT: &str = "%Y-%m-%d";
pub const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
pub const ISO_DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

/// 現在のUTC時刻を取得
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// 文字列から日付を解析
pub fn parse_date(date_str: &str) -> Result<NaiveDate, chrono::ParseError> {
    NaiveDate::parse_from_str(date_str, DATE_FORMAT)
}

/// 文字列から日付時刻を解析
pub fn parse_datetime(datetime_str: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    let naive = NaiveDateTime::parse_from_str(datetime_str, DATETIME_FORMAT)?;
    Ok(DateTime::from_utc(naive, Utc))
}

/// ISO 8601形式から日付時刻を解析
pub fn parse_iso_datetime(iso_str: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(iso_str).map(|dt| dt.with_timezone(&Utc))
}

/// 日付を文字列にフォーマット
pub fn format_date(date: &NaiveDate) -> String {
    date.format(DATE_FORMAT).to_string()
}

/// 日付時刻を文字列にフォーマット
pub fn format_datetime(datetime: &DateTime<Utc>) -> String {
    datetime.format(DATETIME_FORMAT).to_string()
}

/// 日付時刻をISO 8601形式にフォーマット
pub fn format_iso_datetime(datetime: &DateTime<Utc>) -> String {
    datetime.format(ISO_DATETIME_FORMAT).to_string()
}

/// 人間が読みやすい相対時間を生成
pub fn humanize_duration(datetime: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*datetime);
    
    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{} minutes ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_days() < 30 {
        format!("{} weeks ago", duration.num_days() / 7)
    } else if duration.num_days() < 365 {
        format!("{} months ago", duration.num_days() / 30)
    } else {
        format!("{} years ago", duration.num_days() / 365)
    }
}

/// 日付範囲を表すヘルパー構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl DateRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }
    
    pub fn today() -> Self {
        let now = Utc::now();
        let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let end = now.date_naive().and_hms_opt(23, 59, 59).unwrap();
        
        Self {
            start: DateTime::from_utc(start, Utc),
            end: DateTime::from_utc(end, Utc),
        }
    }
    
    pub fn this_week() -> Self {
        let now = Utc::now();
        let days_since_monday = now.weekday().num_days_from_monday();
        let start_of_week = now.date_naive() - Duration::days(days_since_monday as i64);
        let end_of_week = start_of_week + Duration::days(6);
        
        Self {
            start: DateTime::from_utc(start_of_week.and_hms_opt(0, 0, 0).unwrap(), Utc),
            end: DateTime::from_utc(end_of_week.and_hms_opt(23, 59, 59).unwrap(), Utc),
        }
    }
    
    pub fn this_month() -> Self {
        let now = Utc::now();
        let start_of_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
        let end_of_month = if now.month() == 12 {
            NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap() - Duration::days(1)
        } else {
            NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1).unwrap() - Duration::days(1)
        };
        
        Self {
            start: DateTime::from_utc(start_of_month.and_hms_opt(0, 0, 0).unwrap(), Utc),
            end: DateTime::from_utc(end_of_month.and_hms_opt(23, 59, 59).unwrap(), Utc),
        }
    }
    
    pub fn last_n_days(n: i64) -> Self {
        let now = Utc::now();
        let start = now - Duration::days(n);
        
        Self {
            start,
            end: now,
        }
    }
    
    pub fn contains(&self, datetime: &DateTime<Utc>) -> bool {
        datetime >= &self.start && datetime <= &self.end
    }
    
    pub fn duration(&self) -> Duration {
        self.end.signed_duration_since(self.start)
    }
}

/// 営業日の計算
pub fn add_business_days(date: NaiveDate, days: i32) -> NaiveDate {
    let mut current = date;
    let mut remaining = days.abs();
    let direction = if days >= 0 { 1 } else { -1 };
    
    while remaining > 0 {
        current = current + Duration::days(direction);
        
        // 土曜日(6)と日曜日(7)をスキップ
        if current.weekday().number_from_monday() <= 5 {
            remaining -= 1;
        }
    }
    
    current
}

/// 年齢計算
pub fn calculate_age(birth_date: NaiveDate) -> i32 {
    let today = Utc::now().date_naive();
    let mut age = today.year() - birth_date.year();
    
    if today.month() < birth_date.month() || 
       (today.month() == birth_date.month() && today.day() < birth_date.day()) {
        age -= 1;
    }
    
    age
}

/// タイムゾーン変換ヘルパー
pub fn to_local_timezone(datetime: DateTime<Utc>, offset_hours: i32) -> DateTime<chrono::FixedOffset> {
    let offset = chrono::FixedOffset::east_opt(offset_hours * 3600).unwrap();
    datetime.with_timezone(&offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_parsing() {
        let date_str = "2023-12-25";
        let parsed = parse_date(date_str).unwrap();
        assert_eq!(parsed.year(), 2023);
        assert_eq!(parsed.month(), 12);
        assert_eq!(parsed.day(), 25);
    }

    #[test]
    fn test_datetime_formatting() {
        let dt = DateTime::parse_from_rfc3339("2023-12-25T15:30:45Z").unwrap().with_timezone(&Utc);
        let formatted = format_datetime(&dt);
        assert_eq!(formatted, "2023-12-25 15:30:45");
    }

    #[test]
    fn test_date_range_today() {
        let today = DateRange::today();
        let now = Utc::now();
        
        assert_eq!(today.start.date_naive(), now.date_naive());
        assert_eq!(today.end.date_naive(), now.date_naive());
    }

    #[test]
    fn test_business_days() {
        // 金曜日から3営業日後は水曜日
        let friday = NaiveDate::from_ymd_opt(2023, 12, 1).unwrap(); // 2023-12-01は金曜日
        let result = add_business_days(friday, 3);
        assert_eq!(result.weekday().number_from_monday(), 3); // 水曜日
    }

    #[test]
    fn test_age_calculation() {
        let birth_date = NaiveDate::from_ymd_opt(1990, 6, 15).unwrap();
        let age = calculate_age(birth_date);
        assert!(age >= 33); // 2023年以降であれば33歳以上
    }

    #[test]
    fn test_humanize_duration() {
        let now = Utc::now();
        let five_minutes_ago = now - Duration::minutes(5);
        let result = humanize_duration(&five_minutes_ago);
        assert!(result.contains("minutes ago"));
    }
}
