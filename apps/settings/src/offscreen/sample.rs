//! 截图用的样例统计：近一年每天的输入量与 Token、今天各时段，数是造的（固定种子），只为看图表排得怎样。

use std::collections::BTreeMap;

use jiff::civil::Date;
use qingjian_core::Usage;
use qingjian_predict::{ApiUsageHour, ApiUsageStats};

use crate::stats::{CloudStats, InputStats, last_24};

/// 今天是 `today`、现在 `hour` 点时的样例。
pub fn stats(today: Date, hour: u8) -> (InputStats, CloudStats) {
    let mut seed = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = move |range: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % range.max(1)
    };
    let mut days = BTreeMap::new();
    let mut cloud_days = BTreeMap::new();
    for back in 0..300_i64 {
        let date = today.saturating_sub(jiff::Span::new().days(back));
        // 周末少打、偶尔整天不打
        let weekend = date.weekday().to_monday_zero_offset() >= 5;
        if next(10) < if weekend { 4 } else { 1 } {
            continue;
        }
        let hanzi = 300 + next(if weekend { 1_500 } else { 5_000 });
        days.insert(date, usage(hanzi, &mut next));
        if back < 120 && next(10) < 8 {
            let requests = hanzi / 40 + next(20);
            cloud_days.insert(date, api(requests, &mut next));
        }
    }
    // 昨天整天、今天到现在，按钟点排；忙在上午、下午和晚上
    let day_hours = |until: usize, next: &mut dyn FnMut(u64) -> u64| {
        let mut hours = [Usage::default(); 24];
        let mut cloud = [ApiUsageHour::default(); 24];
        for clock in 8..=until {
            let busy = matches!(clock, 9..=11 | 14..=17 | 20..=22);
            if next(10) < 8 {
                hours[clock] = usage(if busy { 200 + next(600) } else { next(150) }, next);
                cloud[clock] = api(hours[clock].hanzi / 40 + next(5), next);
            }
        }
        (hours, cloud)
    };
    let (yesterday_hours, yesterday_cloud) = day_hours(23, &mut next);
    let (today_hours, today_cloud) = day_hours(usize::from(hour), &mut next);
    for (date, hours, cloud) in [
        (
            today.yesterday().unwrap_or(today),
            &yesterday_hours,
            &yesterday_cloud,
        ),
        (today, &today_hours, &today_cloud),
    ] {
        days.insert(
            date,
            hours.iter().fold(Usage::default(), |sum, hour| sum + *hour),
        );
        let mut total = ApiUsageHour::default();
        for hour in cloud {
            total += *hour;
        }
        cloud_days.insert(date, total);
    }
    let hours = last_24(yesterday_hours, today_hours, hour);
    let cloud_hours = last_24(yesterday_cloud, today_cloud, hour);
    let mut totals = ApiUsageStats::default();
    for day in cloud_days.values() {
        totals.requests += day.requests;
        totals.responses += day.requests - day.failures;
        totals.failures += day.failures;
        totals.input_tokens += day.input_tokens;
        totals.output_tokens += day.output_tokens;
        totals.total_tokens += day.total_tokens;
        totals.cached_input_tokens += day.input_tokens / 3;
    }
    let input = InputStats {
        today,
        hour,
        days,
        hours,
    };
    let cloud = CloudStats {
        today,
        hour,
        totals: Some(totals),
        days: cloud_days,
        hours: cloud_hours,
    };
    (input, cloud)
}

fn usage(hanzi: u64, next: &mut dyn FnMut(u64) -> u64) -> Usage {
    let choices = hanzi / 2 + 1;
    Usage {
        hanzi,
        words: hanzi / 2,
        english_words: hanzi / 30,
        commits: choices + hanzi / 50,
        choices,
        first_choices: choices * (80 + next(12)) / 100,
        typed_hanzi: hanzi,
        keys: hanzi * (28 + next(8)) / 10,
        typing_ms: hanzi * 60_000 / (55 + next(40)),
        edited: choices * (8 + next(10)) / 100,
    }
}

fn api(requests: u64, next: &mut dyn FnMut(u64) -> u64) -> ApiUsageHour {
    let input = requests * (380 + next(120));
    let output = requests * (30 + next(20));
    ApiUsageHour {
        requests,
        failures: u64::from(requests > 0 && next(10) == 0),
        input_tokens: input,
        output_tokens: output,
        total_tokens: input + output,
    }
}
