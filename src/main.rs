#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_closure)]

use core::{cell::RefCell, pin::Pin};

use bleps::{
    ad_structure::{
        create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
    },
    async_attribute_server::AttributeServer,
    asynch::Ble,
    attribute_server::NotificationData,
    gatt,
};
use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{
    ble::controller::asynch::BleConnector, initialize, EspWifiInitFor, EspWifiInitialization,
};

use esp32_hal as hal;
use hal::{
    clock::ClockControl, embassy, peripheral::Peripheral, peripherals::*, prelude::*,
    timer::TimerGroup, Rng, IO,
};

mod utilities {
    pub mod async_ble;
    pub mod ble;
}

use utilities::async_ble::connection;

#[main]
async fn main(_spawner: Spawner) -> ! {
    #[cfg(feature = "log")]
    esp_println::logger::init_logger(log::LevelFilter::Info);
    let peripherals = Peripherals::take();

    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    #[cfg(target_arch = "xtensa")]
    let timer = hal::timer::TimerGroup::new(peripherals.TIMG1, &clocks).timer0;
    #[cfg(target_arch = "riscv32")]
    let timer = hal::systimer::SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let init = initialize(
        EspWifiInitFor::Ble,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let pins = io.pins;
    // Async requires the GPIO interrupt to wake futures
    hal::interrupt::enable(
        hal::peripherals::Interrupt::GPIO,
        hal::interrupt::Priority::Priority1,
    )
    .unwrap();

    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timer_group0.timer0);
    let mut bluetooth = peripherals.BT;

    connection("ESP32", init, bluetooth, pins).await
}
