// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 OpenBuds contributors
//! Type erasure at the foreign boundary, not shared wire semantics across models.
use crate::error::{self, Failure};
use realme_buds::{Transport, devices::air7};
use std::{
    any::Any,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

pub trait Driver: Send {
    fn initialize(&mut self) -> Result<(), Failure>;
    fn set_timeout(&mut self, duration: Duration) -> Result<(), Failure>;
    fn call(&mut self, request: &[u8], cancel: &AtomicBool) -> Result<Vec<u8>, Failure>;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
pub struct Air7 {
    pub client: air7::Client<Box<dyn Transport>>,
    pub initialized: bool,
}
impl Driver for Air7 {
    fn initialize(&mut self) -> Result<(), Failure> {
        self.initialized = false;
        self.client.initialize()?;
        self.initialized = true;
        Ok(())
    }
    fn set_timeout(&mut self, duration: Duration) -> Result<(), Failure> {
        self.client.set_timeout(duration)?;
        Ok(())
    }
    fn call(&mut self, request: &[u8], cancel: &AtomicBool) -> Result<Vec<u8>, Failure> {
        let request: air7::api::Request = serde_json::from_slice(request)
            .map_err(|e| Failure::new(error::INVALID_ARGUMENT, e.to_string()))?;
        let init = request.is_initialize();
        if !init && !self.initialized {
            return Err(Failure::new(
                error::NOT_INITIALIZED,
                "initialize the selected model first",
            ));
        }
        if init {
            self.initialized = false;
        }
        let result =
            air7::api::execute(&mut self.client, request, || cancel.load(Ordering::Relaxed))?;
        if init {
            self.initialized = true;
        }
        serde_json::to_vec(&result).map_err(|e| Failure::new(error::INTERNAL, e.to_string()))
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
pub fn validate_model(id: &str) -> Result<(), Failure> {
    if realme_buds::devices::DRIVER_IDS.contains(&id) {
        Ok(())
    } else {
        Err(Failure::new(
            error::UNKNOWN_DRIVER,
            "unknown model; no protocol fallback is permitted",
        ))
    }
}
#[cfg(windows)]
pub fn windows_service(id: &str) -> Result<realme_buds::transport::ServiceUuid, Failure> {
    match id {
        air7::DRIVER_ID => Ok(air7::CONTROL_SERVICE),
        _ => Err(Failure::new(
            error::UNSUPPORTED,
            "driver has no native Windows RFCOMM service",
        )),
    }
}
pub fn create(id: &str, transport: Box<dyn Transport>) -> Result<Box<dyn Driver>, Failure> {
    match id {
        air7::DRIVER_ID => Ok(Box::new(Air7 {
            client: air7::Client::new(transport),
            initialized: false,
        })),
        _ => Err(Failure::new(
            error::UNKNOWN_DRIVER,
            "driver not implemented",
        )),
    }
}
pub fn initialized_air7(driver: &mut dyn Driver) -> Result<&mut Air7, Failure> {
    let d = driver.as_any_mut().downcast_mut::<Air7>().ok_or_else(|| {
        Failure::new(
            error::UNKNOWN_DRIVER,
            "this function belongs to the Air7 driver",
        )
    })?;
    if !d.initialized {
        return Err(Failure::new(
            error::NOT_INITIALIZED,
            "initialize the selected driver first",
        ));
    }
    Ok(d)
}
