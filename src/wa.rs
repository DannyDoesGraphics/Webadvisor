use crate::{run_with_timeout, Context};
use anyhow::anyhow;
use anyhow::Result;
use dotenv::var;
use headless_chrome::Tab;
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Navigates webadvisor

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub enum Semester {
    Winter = 3,
    Summer = 2,
    Fall = 1,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Date {
    pub semester: Semester,
    pub year: u32,
}
impl Date {
    pub fn from_str(input: &str) -> Result<Self> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid format"));
        }

        let semester = match parts[0] {
            "Winter" => Semester::Winter,
            "Summer" => Semester::Summer,
            "Fall" => Semester::Fall,
            _ => return Err(anyhow!("Invalid semester")),
        };

        let year = parts[1].parse::<u32>()?;

        Ok(Date { semester, year })
    }
}

impl PartialOrd for Date {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.year.cmp(&other.year) {
            Ordering::Less => Some(Ordering::Less),
            Ordering::Equal => self.semester.partial_cmp(&other.semester),
            Ordering::Greater => Some(Ordering::Greater),
        }
    }
}
fn extract_number(input: &str) -> Result<i64> {
    let mut number_str = String::new();
    for c in input.chars().rev() {
        if c.is_ascii_digit() {
            number_str.push(c);
        } else if !number_str.is_empty() {
            break;
        }
    }

    if number_str.is_empty() {
        Err(anyhow::anyhow!("No number found in the input string"))
    } else {
        let number: i64 = number_str.chars().rev().collect::<String>().parse()?;
        Ok(number)
    }
}

pub async fn wa_navigate_semester_precondition(context: Context) -> Result<bool> {
    let guard = context
        .inner
        .read()
        .map_err(|_| anyhow!("Poison error"))?;
    if guard.is_none() {
        return Err(anyhow!("Expected browser + tab, found `None`."));
    }
    let tab = guard.as_ref().unwrap().tab.clone();
    drop(guard);
    tab.wait_until_navigated()?;
    tab.find_element("#schedule-next-term")?;
    tab.find_element("#schedule-prev-term")?;
    tab.find_element("#schedule-activeterm-text")?;
    Ok(true)
}

async fn retry_interaction<F, T>(mut f: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let mut attempts = 0;
    while attempts < 3 {
        match f() {
            Ok(result) => return Ok(result),
            Err(e) => {
                eprintln!("Attempt {} failed: {}", attempts + 1, e);
                sleep(Duration::from_secs(1)).await;
                attempts += 1;
            }
        }
    }
    Err(anyhow!("Failed after 3 attempts"))
}

pub async fn wa_navigate_semester_function(context: Context) -> Result<()> {
    let tab = {
        let guard = context
            .inner
            .read()
            .map_err(|_| anyhow!("Poison error"))?;
        if guard.is_none() {
            return Err(anyhow!("Expected browser + tab, found `None`."));
        }
        guard.as_ref().unwrap().tab.clone()
    };

    let target_date = Date::from_str(var("TARGET_SEMESTER")?.as_str())?;
    run_with_timeout(
        async move {
            loop {
                tab.wait_until_navigated()?;

                let result = retry_interaction(|| {
                    let next = tab.find_element("#schedule-next-term")?;
                    let prev = tab.find_element("#schedule-prev-term")?;
                    let text = tab.find_element("#schedule-activeterm-text")?;
                    let text = text.get_inner_text()?;
                    let date_current = Date::from_str(&text)?;
                    match date_current.partial_cmp(&target_date) {
                        Some(ord) => {
                            match ord {
                                Ordering::Less => prev.click()?,
                                Ordering::Equal => return Ok(true),
                                Ordering::Greater => next.click()?,
                            };
                        }
                        _ => return Err(anyhow!("Failed to compare dates")),
                    }
                    Ok(false)
                })
                .await;
                if let Ok(true) = result {
                    return Ok(());
                }
                if let Err(e) = result {
                    return Err(anyhow!("Failed to interact with element: {}", e));
                }
            }
        },
        Duration::from_secs(20),
    )
    .await
    .map_or_else(|e| Err(anyhow!("Poison error, {e}")), Ok)?;
    Ok(())
}

pub async fn wa_register_precondition(context: Context) -> Result<bool> {
    let tab = {
        let guard = context
            .inner
            .read()
            .map_err(|_| anyhow!("Poison error"))?;
        if guard.is_none() {
            return Err(anyhow!("Expected browser + tab, found `None`."));
        }
        guard.as_ref().unwrap().tab.clone()
    };
    tab.find_element("#register-button")?;
    Ok(true)
}

pub fn button_pressing(tab: &Arc<Tab>) -> Result<f32> {
    unsafe {
        if let Ok(button) = tab.find_element("#register-button") {
            button.click()?;
        }
    }
    let button = tab.wait_for_element("#register-button")?;
    let script = format!(
        r#"
        document.querySelector("{}").removeAttribute("disabled");
    "#,
        "#register-button"
    );
    tab.evaluate(script.as_str(), true)?;
    button.click()?;
    Ok(1.0)
}

pub async fn wa_register_function(context: Context) -> Result<()> {
    let tab = {
        let guard = context
            .inner
            .read()
            .map_err(|_| anyhow!("Poison error"))?;
        if guard.is_none() {
            return Err(anyhow!("Expected browser + tab, found `None`."));
        }
        guard.as_ref().unwrap().tab.clone()
    };

    let mut lower_bound = 0.1;
    let mut upper_bound = 10.0;
    let mut wait_time: f32 = (lower_bound + upper_bound) / 2.0;

    let mut success_count = 0;
    let mut failure_count = 0;
    let mut consecutive_successes = 0;
    let mut total_fails = 0;

    const SUCCESS_THRESHOLD: usize = 5;
    const FAILURE_THRESHOLD: usize = 3;
    const MAX_TOTAL_FAILS: usize = 10;

    loop {
        let result = button_pressing(&tab);
        match result {
            Ok(_) => {
                println!("Button pressed successfully. Recording success.");
                success_count += 1;
                consecutive_successes += 1;
                failure_count = 0;

                if consecutive_successes >= SUCCESS_THRESHOLD {
                    // If we have enough consecutive successes, reduce the upper bound
                    upper_bound = wait_time;
                    wait_time = (lower_bound + upper_bound) / 2.0;
                    consecutive_successes = 0; // Reset consecutive success count
                    total_fails = 0;
                }
            }
            Err(e) => {
                eprintln!("Error pressing button: {}. Increasing wait time.", e);
                failure_count += 1;
                total_fails += 1;
                consecutive_successes = 0;

                if total_fails >= MAX_TOTAL_FAILS {
                    return Err(anyhow!(
                        "Failed to press button due to too many errors: {}",
                        e
                    ));
                }

                if failure_count >= FAILURE_THRESHOLD {
                    // If we have enough consecutive failures, increase the lower bound
                    lower_bound = wait_time;
                    wait_time = (lower_bound + upper_bound) / 2.0;
                    failure_count = 0; // Reset failure count
                }
            }
        }

        // Log current statistics
        println!(
            "Success count: {}, Failure count: {}, Current wait time: {}",
            success_count, failure_count, wait_time
        );

        // Adjust wait time to stay within bounds
        wait_time = wait_time.clamp(0.1, 10.0);
        sleep(Duration::from_secs_f32(wait_time)).await;
    }
}
