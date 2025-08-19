use chrono::{DateTime, Local};
use std::time::{SystemTime, UNIX_EPOCH};

/// 格式化时间为友好显示格式
pub fn format_time(time: SystemTime) -> String {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            if let Some(datetime) = DateTime::from_timestamp(duration.as_secs() as i64, 0) {
                let local_time = datetime.with_timezone(&Local);
                local_time.format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                "未知时间".to_string()
            }
        }
        Err(_) => "未知时间".to_string(),
    }
}

/// 格式化相对时间 (例如: "2 天前")
#[allow(dead_code)]
pub fn format_relative_time(time: SystemTime) -> String {
    let now = SystemTime::now();
    
    match now.duration_since(time) {
        Ok(duration) => {
            let seconds = duration.as_secs();
            
            match seconds {
                0..=59 => "刚刚".to_string(),
                60..=3599 => format!("{} 分钟前", seconds / 60),
                3600..=86399 => format!("{} 小时前", seconds / 3600),
                86400..=2591999 => format!("{} 天前", seconds / 86400),
                2592000..=31535999 => format!("{} 个月前", seconds / 2592000),
                _ => format!("{} 年前", seconds / 31536000),
            }
        }
        Err(_) => format_time(time),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_format_time() {
        let time = SystemTime::now();
        let formatted = format_time(time);
        assert!(formatted.contains("-"));
        assert!(formatted.contains(":"));
    }

    #[test]
    fn test_format_relative_time() {
        let now = SystemTime::now();
        
        // 测试"刚刚"
        assert_eq!(format_relative_time(now), "刚刚");
        
        // 测试"分钟前"
        let two_minutes_ago = now - std::time::Duration::from_secs(120);
        assert_eq!(format_relative_time(two_minutes_ago), "2 分钟前");
        
        // 测试"小时前"
        let two_hours_ago = now - std::time::Duration::from_secs(7200);
        assert_eq!(format_relative_time(two_hours_ago), "2 小时前");
        
        // 测试"天前"
        let two_days_ago = now - std::time::Duration::from_secs(172800);
        assert_eq!(format_relative_time(two_days_ago), "2 天前");
    }
}