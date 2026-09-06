//! Recurrence expansion — computing a task's next occurrence from its
//! `Recurrence` (SCHEMA_SPEC §6, ARCHITECTURE.md §12).
//!
//! Three distinct models, each answering "when's next?" differently:
//! - **Fixed** — `interval` after the *original* due date, regardless of
//!   when it was actually completed (rent: due the 1st, always).
//! - **Flexible** — `interval` after *actual completion* (a recurring
//!   review: next one is a week after you finished this one).
//! - **Rrule** — a real iCalendar RRULE string (RFC 5545), for irregular
//!   patterns ("every 2nd Tuesday"). Parsed with the `rrule` crate rather
//!   than hand-rolled — BYDAY/BYMONTHDAY/WKST interactions are exactly the
//!   kind of thing worth not re-implementing.
//!
//! `until`/`count` end conditions on Fixed/Flexible are checked here too.
//! `Rrule` doesn't need separate fields for either — RRULE already has
//! native `UNTIL=`/`COUNT=` syntax, so duplicating them as struct fields
//! would just be two ways to say the same thing.

use chrono::{Datelike, NaiveDate, TimeZone};

use crate::error::{IrisError, IrisResult};
use crate::types::Recurrence;

/// Compute the next occurrence's date, or `None` if the recurrence has ended
/// (`until`/`count` exhausted for Fixed/Flexible, or the RRULE produces no
/// further occurrence).
///
/// `due_date` is the occurrence that was just completed; `completed_on` is
/// when it was actually completed (only `Flexible` uses it); `occurrences_so_far`
/// is how many times this recurrence has already fired (0 for the very first
/// completion), used to check `count`.
pub fn next_occurrence(
    recurrence: &Recurrence,
    due_date: NaiveDate,
    completed_on: NaiveDate,
    occurrences_so_far: u32,
) -> IrisResult<Option<NaiveDate>> {
    match recurrence {
        Recurrence::Fixed {
            interval,
            until,
            count,
        } => {
            if ended(*until, *count, occurrences_so_far, due_date) {
                return Ok(None);
            }
            let duration = parse_iso8601_duration(interval)?;
            Ok(Some(add_duration(due_date, &duration)?))
        }
        Recurrence::Flexible {
            interval,
            until,
            count,
        } => {
            if ended(*until, *count, occurrences_so_far, due_date) {
                return Ok(None);
            }
            let duration = parse_iso8601_duration(interval)?;
            Ok(Some(add_duration(completed_on, &duration)?))
        }
        Recurrence::Rrule { rrule, dtstart } => next_rrule_occurrence(rrule, *dtstart, due_date),
    }
}

fn ended(
    until: Option<NaiveDate>,
    count: Option<u32>,
    occurrences_so_far: u32,
    due_date: NaiveDate,
) -> bool {
    if let Some(count) = count {
        // occurrences_so_far doesn't include the one just completed (the
        // caller increments after this check), so the one just completed
        // was occurrence number `occurrences_so_far + 1`.
        if occurrences_so_far + 1 >= count {
            return true;
        }
    }
    if let Some(until) = until {
        if due_date >= until {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// ISO 8601 duration parsing (the subset SCHEMA_SPEC actually uses: P[n]Y[n]M[n]D
// or P[n]W — no time-of-day component, since due dates have no time of day).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Iso8601Duration {
    years: u32,
    months: u32,
    weeks: u32,
    days: u32,
}

fn parse_iso8601_duration(s: &str) -> IrisResult<Iso8601Duration> {
    let rest = s
        .strip_prefix('P')
        .ok_or_else(|| invalid_duration(s, "must start with 'P'"))?;
    if rest.is_empty() {
        return Err(invalid_duration(s, "no components after 'P'"));
    }

    let mut duration = Iso8601Duration::default();
    let mut number = String::new();
    for ch in rest.chars() {
        if ch.is_ascii_digit() {
            number.push(ch);
            continue;
        }
        let n: u32 = number
            .parse()
            .map_err(|_| invalid_duration(s, "expected a number before a unit letter"))?;
        number.clear();
        match ch {
            'Y' => duration.years = n,
            'M' => duration.months = n,
            'W' => duration.weeks = n,
            'D' => duration.days = n,
            other => return Err(invalid_duration(s, &format!("unsupported unit '{other}'"))),
        }
    }
    if !number.is_empty() {
        return Err(invalid_duration(s, "trailing number with no unit"));
    }
    Ok(duration)
}

fn invalid_duration(s: &str, why: &str) -> IrisError {
    IrisError::Validation(format!("invalid ISO 8601 duration `{s}`: {why}"))
}

fn add_duration(date: NaiveDate, duration: &Iso8601Duration) -> IrisResult<NaiveDate> {
    let months_total = duration.years * 12 + duration.months;
    let date = if months_total > 0 {
        date.checked_add_months(chrono::Months::new(months_total))
            .ok_or_else(|| IrisError::Validation("date overflow adding duration".into()))?
    } else {
        date
    };
    let days_total = i64::from(duration.weeks) * 7 + i64::from(duration.days);
    date.checked_add_signed(chrono::Duration::days(days_total))
        .ok_or_else(|| IrisError::Validation("date overflow adding duration".into()))
}

// ---------------------------------------------------------------------------
// RRULE (RFC 5545) via the `rrule` crate
// ---------------------------------------------------------------------------

fn next_rrule_occurrence(
    rrule: &str,
    dtstart: NaiveDate,
    due_date: NaiveDate,
) -> IrisResult<Option<NaiveDate>> {
    let dtstart_line = format!("DTSTART:{}T000000Z", dtstart.format("%Y%m%d"));
    let full = format!("{dtstart_line}\nRRULE:{rrule}");
    let rrule_set: ::rrule::RRuleSet = full
        .parse()
        .map_err(|e| IrisError::Validation(format!("invalid RRULE `{rrule}`: {e}")))?;

    let after = ::rrule::Tz::UTC
        .with_ymd_and_hms(due_date.year(), due_date.month(), due_date.day(), 0, 0, 0)
        .single()
        .ok_or_else(|| IrisError::Validation("invalid due date for RRULE expansion".into()))?;

    // `after` is inclusive in this crate (DTSTART itself matches its own
    // rule), so ask for a couple of candidates and take the first strictly
    // later than `due_date` rather than relying on undocumented boundary
    // behavior for exclusivity.
    let result = rrule_set.after(after).all(2);
    Ok(result
        .dates
        .into_iter()
        .map(|dt| dt.date_naive())
        .find(|d| *d > due_date))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn fixed_advances_from_original_due_date_not_completion() {
        let recurrence = Recurrence::Fixed {
            interval: "P1M".to_string(),
            until: None,
            count: None,
        };
        // Due Jan 1, completed 10 days late on Jan 11 — next is Feb 1, not Feb 11.
        let next = next_occurrence(&recurrence, date(2026, 1, 1), date(2026, 1, 11), 0)
            .unwrap()
            .unwrap();
        assert_eq!(next, date(2026, 2, 1));
    }

    #[test]
    fn flexible_advances_from_completion_not_due_date() {
        let recurrence = Recurrence::Flexible {
            interval: "P1W".to_string(),
            until: None,
            count: None,
        };
        // Due Jan 1, completed late on Jan 11 — next is a week after completion.
        let next = next_occurrence(&recurrence, date(2026, 1, 1), date(2026, 1, 11), 0)
            .unwrap()
            .unwrap();
        assert_eq!(next, date(2026, 1, 18));
    }

    #[test]
    fn count_stops_recurrence_after_the_nth_completion() {
        let recurrence = Recurrence::Fixed {
            interval: "P1D".to_string(),
            until: None,
            count: Some(3),
        };
        // Completions 1 and 2 (occurrences_so_far 0, 1) still recur.
        assert!(
            next_occurrence(&recurrence, date(2026, 1, 1), date(2026, 1, 1), 0)
                .unwrap()
                .is_some()
        );
        assert!(
            next_occurrence(&recurrence, date(2026, 1, 2), date(2026, 1, 2), 1)
                .unwrap()
                .is_some()
        );
        // The 3rd completion (occurrences_so_far 2) was the last allowed one.
        assert!(
            next_occurrence(&recurrence, date(2026, 1, 3), date(2026, 1, 3), 2)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn until_stops_recurrence_once_reached() {
        let recurrence = Recurrence::Fixed {
            interval: "P1D".to_string(),
            until: Some(date(2026, 1, 3)),
            count: None,
        };
        assert!(
            next_occurrence(&recurrence, date(2026, 1, 2), date(2026, 1, 2), 0)
                .unwrap()
                .is_some()
        );
        // due_date already at/past `until` — stop.
        assert!(
            next_occurrence(&recurrence, date(2026, 1, 3), date(2026, 1, 3), 1)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rrule_weekly_on_tuesday_finds_the_next_tuesday() {
        let recurrence = Recurrence::Rrule {
            rrule: "FREQ=WEEKLY;BYDAY=TU".to_string(),
            dtstart: date(2026, 1, 6),
        };
        // 2026-01-06 is a Tuesday; next Tuesday should be 2026-01-13.
        let next = next_occurrence(&recurrence, date(2026, 1, 6), date(2026, 1, 6), 0)
            .unwrap()
            .unwrap();
        assert_eq!(next, date(2026, 1, 13));
    }

    #[test]
    fn rrule_with_count_ends() {
        let recurrence = Recurrence::Rrule {
            rrule: "FREQ=DAILY;COUNT=2".to_string(),
            dtstart: date(2026, 1, 1),
        };
        // DTSTART (Jan 1) counts as occurrence 1; COUNT=2 allows one more.
        let next = next_occurrence(&recurrence, date(2026, 1, 1), date(2026, 1, 1), 0).unwrap();
        assert_eq!(next, Some(date(2026, 1, 2)));
        // Completing that 2nd occurrence: COUNT is relative to the fixed
        // dtstart above, NOT to this due_date — must stay exhausted, not
        // silently restart a fresh 2-occurrence series from here.
        let end = next_occurrence(&recurrence, date(2026, 1, 2), date(2026, 1, 2), 1).unwrap();
        assert_eq!(end, None);
    }

    #[test]
    fn parse_iso8601_duration_handles_years_months_days_and_weeks() {
        assert_eq!(
            parse_iso8601_duration("P1Y2M3D").unwrap(),
            Iso8601Duration {
                years: 1,
                months: 2,
                weeks: 0,
                days: 3
            }
        );
        assert_eq!(
            parse_iso8601_duration("P2W").unwrap(),
            Iso8601Duration {
                years: 0,
                months: 0,
                weeks: 2,
                days: 0
            }
        );
    }

    #[test]
    fn parse_iso8601_duration_rejects_garbage() {
        assert!(parse_iso8601_duration("1M").is_err()); // missing leading P
        assert!(parse_iso8601_duration("PX").is_err()); // no number before unit
        assert!(parse_iso8601_duration("P1Z").is_err()); // unsupported unit
    }
}
