//! Simple cron schedule parser and evaluator.
//!
//! Supports standard 5-field cron expressions:
//! `minute hour day_of_month month day_of_week`
//!
//! Special values: `*` (any), specific numbers, and comma-separated lists.

use anyhow::Result;
use anyhow::bail;
use chrono::Datelike;
use chrono::Timelike;
use chrono::Utc;

/// A parsed cron schedule.
#[derive(Debug, Clone)]
pub struct CronSchedule {
    minutes: CronField,
    hours: CronField,
    days_of_month: CronField,
    months: CronField,
    days_of_week: CronField,
    raw: String,
}

#[derive(Debug, Clone)]
enum CronField {
    Any,
    Values(Vec<u32>),
}

impl CronField {
    fn matches(&self, value: u32) -> bool {
        match self {
            Self::Any => true,
            Self::Values(vals) => vals.contains(&value),
        }
    }

    fn parse(field: &str, min: u32, max: u32) -> Result<Self> {
        if field == "*" {
            return Ok(Self::Any);
        }

        let mut values = Vec::new();
        for part in field.split(',') {
            let part = part.trim();
            if let Some(step_parts) = part.strip_prefix("*/") {
                let step: u32 = step_parts
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid step value: {step_parts}"))?;
                if step == 0 {
                    bail!("step value cannot be zero");
                }
                let mut v = min;
                while v <= max {
                    values.push(v);
                    v += step;
                }
            } else if part.contains('-') {
                let mut range_parts = part.splitn(2, '-');
                let start: u32 = range_parts
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing range start"))?
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid range start"))?;
                let end: u32 = range_parts
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing range end"))?
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid range end"))?;
                if start > end || start < min || end > max {
                    bail!("invalid range {start}-{end} (valid: {min}-{max})");
                }
                for v in start..=end {
                    values.push(v);
                }
            } else {
                let v: u32 = part
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid cron value: {part}"))?;
                if v < min || v > max {
                    bail!("value {v} out of range {min}-{max}");
                }
                values.push(v);
            }
        }

        values.sort_unstable();
        values.dedup();
        Ok(Self::Values(values))
    }
}

impl CronSchedule {
    /// Parse a 5-field cron expression.
    ///
    /// Format: `minute hour day_of_month month day_of_week`
    ///
    /// Examples:
    /// - `0 3 * * *`     — every day at 03:00
    /// - `*/15 * * * *`  — every 15 minutes
    /// - `0 9,18 * * 1-5` — 9am and 6pm on weekdays
    pub fn parse(expression: &str) -> Result<Self> {
        let fields: Vec<&str> = expression.split_whitespace().collect();
        if fields.len() != 5 {
            bail!(
                "cron expression must have 5 fields (minute hour dom month dow), got {}",
                fields.len()
            );
        }

        Ok(Self {
            minutes: CronField::parse(fields[0], 0, 59)?,
            hours: CronField::parse(fields[1], 0, 23)?,
            days_of_month: CronField::parse(fields[2], 1, 31)?,
            months: CronField::parse(fields[3], 1, 12)?,
            days_of_week: CronField::parse(fields[4], 0, 6)?,
            raw: expression.to_string(),
        })
    }

    /// Check if the schedule matches the current UTC time (at minute granularity).
    pub fn matches_now(&self) -> bool {
        let now = Utc::now();
        self.matches_datetime(
            now.minute(),
            now.hour(),
            now.day(),
            now.month(),
            now.weekday().num_days_from_sunday(),
        )
    }

    /// Check if the schedule matches a specific datetime (for testing).
    pub fn matches_datetime(
        &self,
        minute: u32,
        hour: u32,
        day_of_month: u32,
        month: u32,
        day_of_week: u32,
    ) -> bool {
        self.minutes.matches(minute)
            && self.hours.matches(hour)
            && self.days_of_month.matches(day_of_month)
            && self.months.matches(month)
            && self.days_of_week.matches(day_of_week)
    }

    /// Return the raw cron expression string.
    pub fn expression(&self) -> &str {
        &self.raw
    }
}

impl std::fmt::Display for CronSchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_every_day_at_3am() {
        let schedule = CronSchedule::parse("0 3 * * *").expect("valid cron");
        assert_eq!(schedule.expression(), "0 3 * * *");
        // 3:00 AM, any day
        assert!(schedule.matches_datetime(0, 3, 15, 6, 1));
        // 3:01 AM — should NOT match
        assert!(!schedule.matches_datetime(1, 3, 15, 6, 1));
        // 4:00 AM — should NOT match
        assert!(!schedule.matches_datetime(0, 4, 15, 6, 1));
    }

    #[test]
    fn parse_every_15_minutes() {
        let schedule = CronSchedule::parse("*/15 * * * *").expect("valid cron");
        assert!(schedule.matches_datetime(0, 10, 1, 1, 0));
        assert!(schedule.matches_datetime(15, 10, 1, 1, 0));
        assert!(schedule.matches_datetime(30, 10, 1, 1, 0));
        assert!(schedule.matches_datetime(45, 10, 1, 1, 0));
        assert!(!schedule.matches_datetime(7, 10, 1, 1, 0));
    }

    #[test]
    fn parse_weekdays_9_and_18() {
        let schedule = CronSchedule::parse("0 9,18 * * 1-5").expect("valid cron");
        // Monday at 9:00
        assert!(schedule.matches_datetime(0, 9, 1, 1, 1));
        // Friday at 18:00
        assert!(schedule.matches_datetime(0, 18, 1, 1, 5));
        // Sunday at 9:00 — should NOT match
        assert!(!schedule.matches_datetime(0, 9, 1, 1, 0));
        // Monday at 12:00 — should NOT match
        assert!(!schedule.matches_datetime(0, 12, 1, 1, 1));
    }

    #[test]
    fn parse_invalid_expression() {
        assert!(CronSchedule::parse("0 3 *").is_err());
        assert!(CronSchedule::parse("60 3 * * *").is_err());
        assert!(CronSchedule::parse("0 25 * * *").is_err());
    }
}
