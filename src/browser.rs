use anyhow::{anyhow, Result};
use headless_chrome::{Browser, LaunchOptionsBuilder};

use crate::{Context, ContextInner};

pub async fn open_browser_precondition(_: Context) -> Result<bool> {
    Ok(true)
}

pub async fn open_browser_function(context: Context) -> Result<()> {
    let launch_options = LaunchOptionsBuilder::default().headless(false).build()?;
    let mut guard = context
        .inner
        .write()
        .map_err(|_| anyhow!("Poison error"))?;
    let browser = Browser::new(launch_options)?;
    let tab = browser.new_tab()?;
    *guard = Some(ContextInner { browser, tab });
    Ok(())
}

pub async fn navigate_to_webadvisor_function(context: Context) -> Result<()> {
    let guard = context
        .inner
        .read()
        .map_err(|_| anyhow!("Poison error"))?;
    if guard.is_none() {
        return Err(anyhow!("Expected browser + tab, found `None`."));
    }
    let tab = guard.as_ref().unwrap().tab.clone();
    drop(guard);
    tab.navigate_to("https://colleague-ss.uoguelph.ca/Student/Planning/DegreePlans")?
        .wait_until_navigated()?;
    Ok(())
}

pub async fn navigate_to_webadvisor_precondition(context: Context) -> Result<bool> {
    let guard = context
        .inner
        .read()
        .map_err(|_| anyhow!("Poison error"))?;
    if guard.is_none() {
        Ok(false)
    } else {
        Ok(true)
    }
}
