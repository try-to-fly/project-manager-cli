use chrono::{DateTime, Utc, Local};

/// 格式化时间为友好显示格式
pub fn format_time(datetime: &DateTime<Utc>) -> String {
    let local_time = datetime.with_timezone(&Local);
    local_time.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 格式化相对时间 (例如: "2 天前")
pub fn format_relative_time(datetime: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*datetime);
    
    if let Ok(std_duration) = duration.to_std() {
        let seconds = std_duration.as_secs();
        
        match seconds {
            0..=59 => "刚刚".to_string(),
            60..=3599 => format!("{} 分钟前", seconds / 60),
            3600..=86399 => format!("{} 小时前", seconds / 3600),
            86400..=2591999 => format!("{} 天前", seconds / 86400),
            2592000..=31535999 => format!("{} 个月前", seconds / 2592000),
            _ => format!("{} 年前", seconds / 31536000),
        }
    } else {
        format_time(datetime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_format_time() {
        let time = Utc::now();
        let formatted = format_time(&time);
        assert!(formatted.contains("-"));
        assert!(formatted.contains(":"));
    }

    #[test]
    fn test_format_relative_time() {
        let now = Utc::now();
        
        // 测试"刚刚"
        assert_eq!(format_relative_time(&now), "刚刚");
        
        // 测试"分钟前"
        let two_minutes_ago = now - Duration::minutes(2);
        assert_eq!(format_relative_time(&two_minutes_ago), "2 分钟前");
        
        // 测试"小时前"
        let two_hours_ago = now - Duration::hours(2);
        assert_eq!(format_relative_time(&two_hours_ago), "2 小时前");
        
        // 测试"天前"
        let two_days_ago = now - Duration::days(2);
        assert_eq!(format_relative_time(&two_days_ago), "2 天前");
    }
}