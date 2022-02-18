use std::collections::HashMap;

use chrono::{
    Date, DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc, Weekday,
};
use git2::{Config, Error, Repository, Sort};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    #[structopt(name = "user", short = "u")]
    /// the repository to analyze
    flag_user: Option<String>,
    #[structopt(name = "path", short = "p", default_value = ".")]
    /// the repositories to analyze
    paths: Option<Vec<String>>,
    #[structopt(name = "weeks", short = "w", default_value = "52")]
    /// the number of weeks in the past
    flag_nb_weeks: i64,
}

fn print_square(commit_nb: u8) -> () {
    let color = if commit_nb > 3 {
        255
    } else if commit_nb > 2 {
        251
    } else if commit_nb > 1 {
        249
    } else if commit_nb > 0 {
        246
    } else {
        238
    };
    print!("\x1b[38;5;{}m🟩\x1b[0m", color,);
    ()
}

fn main() -> Result<(), Error> {
    let args = Args::from_args();
    // Open repository
    let paths: Vec<String> = args.paths.ok_or(vec!["."]).unwrap();
    let repos = paths
        .iter()
        .map(|p| Repository::open(p).unwrap())
        .collect::<Vec<Repository>>();
    // Get user either from the command line or the config
    let user_email = args.flag_user.unwrap_or_else(|| {
        let config = Config::open_default().unwrap();
        return config.get_string("user.email").unwrap();
    });
    let nb_weeks = args.flag_nb_weeks;
    // The calendar will contain the count of commit per day
    let mut calendar: HashMap<Date<Utc>, u8> = HashMap::new();
    // Always start on a sunday
    let start_time = Utc
        .from_local_datetime(
            // Get the date of this week's sunday
            &NaiveDateTime::new(
                NaiveDate::from_isoywd(
                    Utc::now().year(),
                    Utc::now().iso_week().week(),
                    Weekday::Sun,
                )
                // Rewind nb_weeks in the past
                .checked_sub_signed(Duration::weeks(nb_weeks))
                .unwrap(),
                NaiveTime::from_hms(12, 0, 0),
            ),
        )
        .unwrap();

    for repo in repos {
        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(Sort::NONE | Sort::TIME)?;
        revwalk.push_head()?;

        // Walk the commit list
        for r_commit_id in revwalk {
            if let Ok(commit_id) = r_commit_id {
                if let Ok(commit) = repo.find_commit(commit_id) {
                    // If we reache a commit older than our limit, we stop.
                    if commit.time().seconds() < start_time.timestamp() {
                        break;
                    }
                    if user_email == commit.author().email().ok_or("unknown").unwrap() {
                        // Get the commit date
                        let commit_date = DateTime::<Utc>::from_utc(
                            NaiveDateTime::from_timestamp(commit.time().seconds(), 0),
                            Utc,
                        )
                        .date();
                        // Increment the date counter
                        match calendar.get_mut(&commit_date) {
                            Some(v) => {
                                *v += 1;
                            }
                            None => {
                                let _ = calendar.insert(commit_date, 1);
                            }
                        }
                    }
                }
            }
        }
    }

    let first_day: Date<Utc> = start_time.date();
    for shift in 0..7 {
        for i in 0..nb_weeks {
            let datei = first_day
                .checked_add_signed(Duration::days(shift + i * 7))
                .unwrap();
            let count = calendar.get(&datei).unwrap_or(&0);
            print_square(*count);
        }
        println!("");
    }

    Ok(())
}
