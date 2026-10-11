use std::{collections::HashMap, net::IpAddr};
pub struct Limits {
    minute: u32,
    ip_day: u32,
    global_day: u32,
    day: u64,
    total: u32,
    ips: HashMap<IpAddr, Bucket>,
}
struct Bucket {
    day: u64,
    count: u32,
    minute: u64,
    minute_count: u32,
}
impl Limits {
    pub fn new(minute: u32, ip_day: u32, global_day: u32) -> Self {
        Self {
            minute,
            ip_day,
            global_day,
            day: 0,
            total: 0,
            ips: HashMap::new(),
        }
    }
    pub fn admit(&mut self, ip: IpAddr, seconds: u64) -> bool {
        let day = seconds / 86400;
        let minute = seconds / 60;
        if self.day != day {
            self.day = day;
            self.total = 0;
            self.ips.clear();
        }
        if self.total >= self.global_day {
            return false;
        }
        if self.ips.len() >= 10000 && !self.ips.contains_key(&ip) {
            return false;
        }
        let bucket = self.ips.entry(ip).or_insert(Bucket {
            day,
            count: 0,
            minute,
            minute_count: 0,
        });
        if bucket.day != day {
            bucket.day = day;
            bucket.count = 0;
        }
        if bucket.minute != minute {
            bucket.minute = minute;
            bucket.minute_count = 0;
        }
        if bucket.count >= self.ip_day || bucket.minute_count >= self.minute {
            return false;
        }
        bucket.count += 1;
        bucket.minute_count += 1;
        self.total += 1;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotas_reset_and_shared_budget() {
        let a = "127.0.0.1".parse().unwrap();
        let b = "127.0.0.2".parse().unwrap();
        let mut limits = Limits::new(1, 2, 3);
        assert!(limits.admit(a, 1));
        assert!(!limits.admit(a, 2));
        assert!(limits.admit(a, 61));
        assert!(!limits.admit(a, 121));
        assert!(limits.admit(b, 121));
        assert!(!limits.admit(b, 181));
        assert!(limits.admit(a, 86401));
    }
}
