use std::env::var;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use chrono::Utc;
use otpauth::TOTP;

use crate::Context;

pub async fn login_microsoft_email_precondition(
    context: Context,
) -> Result<bool> {
        let guard = context
            .inner
            .read()
            .map_err(|_| anyhow::Error::from(anyhow!("Poison error")))?;
        if guard.is_none() {
            return Err(anyhow!("Expected browser + tab, found `None`."));
        }
        let tab = guard.as_ref().unwrap().tab.clone();
        drop(guard);
        tab.wait_until_navigated()?;
        tab.find_element("input[type='email']").map(|_| Ok(true))?
}

pub async fn login_microsoft_email_function(
    context: Context,
) -> Result<()> {
        let guard = context
            .inner
            .read()
            .map_err(|_| anyhow::Error::from(anyhow!("Poison error")))?;
        if guard.is_none() {
            return Err(anyhow!("Expected browser + tab, found `None`."));
        }
        let tab = guard.as_ref().unwrap().tab.clone();
        drop(guard);
        let email_input = tab.find_element("input[type='email']")?;
        email_input.type_into(var("EMAIL")?.as_str())?;
        email_input.parent.press_key("Enter")?;
        Ok(())
}

pub async fn login_microsoft_password_precondition(
    context: Context,
) -> Result<bool> {
        let guard = context
            .inner
            .read()
            .map_err(|_| anyhow::Error::from(anyhow!("Poison error")))?;
        if guard.is_none() {
            return Err(anyhow!("Expected browser + tab, found `None`."));
        }
        let tab = guard.as_ref().unwrap().tab.clone();
        drop(guard);
        tab.wait_until_navigated()?;
        tab.find_element("input[type='password']")
            .map(|_| Ok(true))?
}

pub async fn login_microsoft_password_function(
    context: Context,
) -> Result<()> {
        let guard = context
            .inner
            .read()
            .map_err(|_| anyhow::Error::from(anyhow!("Poison error")))?;
        if guard.is_none() {
            return Err(anyhow!("Expected browser + tab, found `None`."));
        }
        let tab = guard.as_ref().unwrap().tab.clone();
        drop(guard);
        let password_input = tab.find_element("input[type='password']")?;
        password_input.type_into(var("PASSWORD")?.as_str())?;
        password_input.parent.press_key("Enter")?;
        Ok(())
}

pub async fn acquire_2fa_code() -> Result<String> {
    let totp = TOTP::new::<String>(var("OTP").unwrap().as_str().parse()?)
        .generate(30, Utc::now().timestamp() as u64);
    Ok(totp.to_string())
}

pub async fn login_microsoft_otp_precondition(
    context: Context,
) -> Result<bool> {
        let guard = context
            .inner
            .read()
            .map_err(|_| anyhow::Error::from(anyhow!("Poison error")))?;
        if guard.is_none() {
            return Err(anyhow!("Expected browser + tab, found `None`."));
        }
        let tab = guard.as_ref().unwrap().tab.clone();
        drop(guard);
        tab.wait_until_navigated()?;
        tab.find_element("#idTxtBx_SAOTCC_OTC").map(|_| Ok(true))?
}

pub async fn login_microsoft_otp_function(
    context: Context,
) -> Result<()> {
    let guard = context
        .inner
        .read()
        .map_err(|_| anyhow::Error::from(anyhow!("Poison error")))?;
    if guard.is_none() {
        return Err(anyhow!("Expected browser + tab, found `None`."));
    }
    let tab = guard.as_ref().unwrap().tab.clone();
    drop(guard);
    tab.wait_until_navigated()?;
    let element = tab.find_element("#idTxtBx_SAOTCC_OTC")?;
    let code: String = {
        let totp = TOTP::from_base32(var("OTP")?.as_str()).expect("Failed to create TOTP instance");
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        totp.generate(30, current_time).to_string()
    };
    element.type_into(code.as_str())?;
    element.parent.press_key("Enter")?;

    Ok(())
}
