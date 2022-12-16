/*
This file is part of the Building Aggregration Tool
Copyright (C) 2022 Novel-T

The Building Aggregration Tool is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/
use std::time::{Duration, Instant};

pub fn format_duration(d: Duration) -> String {
    let mut secs = d.as_secs();
    let hours = secs / 3600;
    secs = secs % 3600;
    let minutes = secs / 60;
    secs = secs % 60;

    let ms = d.as_millis() % 1000;

    format!("{}h {}m {}s {}ms", hours, minutes, secs, ms )
}


pub fn quote_csv_string(s: &str) -> String
{
    let mut r = String::new();

    r.push('"');

    for c in s.chars() {
        if c == '"' {
            r.push('\\');
            r.push('"');
            continue;
        }

        if c == '\\' {
            r.push('\\');
            r.push('\\');
            continue;
        }

        r.push(c);

    }

    r.push('"');

    r
}

pub fn print_remaining_time(now: &Instant, num_processed: u32, num_total: u32) {
    let now2 = Instant::now();
    let d = now2.duration_since(*now);
    let time_per_result = if num_processed == 0 {
        d / 1
    } else {
        d / num_processed
    };
    let est_remaining_time = time_per_result * (num_total - num_processed);
    let est_total_time = time_per_result * num_total;
    println!("Through {} of {}\nElapsed: {}\nEst. Remaining: {}\nEst total time: {}\n",
             num_processed, num_total,
             format_duration(d),
             format_duration(est_remaining_time),
             format_duration(est_total_time) );
}

pub fn print_remaining_time_msg(now: &Instant, num_processed: u32, num_total: u32, msg: &str) {
    let now2 = Instant::now();
    let d = now2.duration_since(*now);
    let time_per_result = if num_processed == 0 {
        d / 1
    } else {
        d / num_processed
    };
    let est_remaining_time = time_per_result * (num_total - num_processed);
    let est_total_time = time_per_result * num_total;
    println!("Through {} of {}\nElapsed: {}\nEst. Remaining: {}\nEst total time: {}\nMessage: {}\n",
             num_processed, num_total,
             format_duration(d),
             format_duration(est_remaining_time),
             format_duration(est_total_time),
        msg
    );
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_csv_string() {
        assert_eq!("\"hello\"", quote_csv_string("hello"));

        assert_eq!(quote_csv_string("hel\\l\"o"), "\"hel\\\\l\\\"o\"");
    }
}

