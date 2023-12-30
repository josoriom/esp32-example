#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_closure)]

use esp32_hal::{peripherals::*, prelude::*};

mod utilities {
    pub mod bluetooth;
}
use utilities::bluetooth::connection;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    pub const SOC_NAME: &str = "ESP32";
    connection(peripherals, SOC_NAME);
}
